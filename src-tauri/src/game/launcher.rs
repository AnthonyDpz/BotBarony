/// Game launcher — starts Barony, connects to BotAPI, navigates menus.
///
/// Menu sequence (keyboard/mouse mode):
///   Space → Play Game → New → Local → (lobby) →
///   class → {class_name} → back_button →
///   race  → {race_name}  → back_button →
///   ready
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::{bail, Context};
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::game::controller::{BaronyClient, GameAPI};
use crate::game::prologue;

// ─── Configuration ────────────────────────────────────────────────────────────

/// All class names accepted by Barony's characterCardClassMenu.
pub const VALID_CLASSES: &[&str] = &[
    "barbarian", "warrior", "healer", "rogue", "wanderer", "cleric",
    "merchant", "wizard", "arcanist", "joker", "sexton", "ninja", "monk",
    "conjurer", "accursed", "mesmer", "brewer", "mechanist", "punisher",
    "shaman", "hunter", "bard", "sapper", "scion", "hermit", "paladin",
];

/// All race names accepted by Barony's characterCardRaceMenu (English).
pub const VALID_RACES: &[&str] = &[
    "Human", "Skeleton", "Vampire", "Succubus", "Goatman",
    "Automaton", "Incubus", "Goblin", "Insectoid",
    "Gnome", "Gremlin", "Dryad", "Myconid", "Salamander",
];

/// Configuration for a new game run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LaunchConfig {
    /// Absolute path to the Barony executable (or .app on macOS).
    pub barony_executable: PathBuf,
    /// Character class (e.g. "wizard").  Defaults to "wanderer".
    pub character_class: String,
    /// Character race (e.g. "Human").  Defaults to "Human".
    pub character_race: String,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            barony_executable: auto_detect_barony()
                .unwrap_or_else(|| PathBuf::from("barony")),
            character_class: "wanderer".to_string(),
            character_race: "Human".to_string(),
        }
    }
}

/// Auto-detect Barony on common macOS / Linux Steam paths.
pub fn auto_detect_barony() -> Option<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        // macOS Steam .app bundle
        PathBuf::from(&home)
            .join("Library/Application Support/Steam/steamapps/common/Barony/Barony.app"),
        // macOS Steam raw binary
        PathBuf::from(&home)
            .join("Library/Application Support/Steam/steamapps/common/Barony/barony.app/Contents/MacOS/barony"),
        // Linux Steam
        PathBuf::from(&home)
            .join(".steam/steam/steamapps/common/Barony/barony"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

// ─── Launcher ─────────────────────────────────────────────────────────────────

/// Launch Barony, wait for BotAPI, and navigate menus to start a new local game.
///
/// Returns `Ok(Arc<BaronyClient>)` once the game is loading (after clicking Ready).
/// The caller is responsible for keeping the `Child` process alive — this function
/// intentionally leaks the `Child` so Tauri's main thread can manage the lifetime.
pub async fn launch_and_start_game(config: LaunchConfig) -> anyhow::Result<Arc<BaronyClient>> {
    // ── Resolve class / race (random if requested) ────────────────────────────
    let class_input = config.character_class.to_lowercase();
    let class: String = if class_input == "random" || class_input.is_empty() {
        let idx = (rand_u32() as usize) % VALID_CLASSES.len();
        VALID_CLASSES[idx].to_string()
    } else {
        class_input
    };

    let race_input = config.character_race.clone();
    let race: String = if race_input.eq_ignore_ascii_case("random") || race_input.is_empty() {
        let idx = (rand_u32() as usize) % VALID_RACES.len();
        VALID_RACES[idx].to_string()
    } else {
        race_input
    };

    if !VALID_CLASSES.contains(&class.as_str()) {
        bail!("Unknown class {:?}. Valid classes: {:?}", class, VALID_CLASSES);
    }
    if !VALID_RACES.contains(&race.as_str()) {
        bail!("Unknown race {:?}. Valid races: {:?}", race, VALID_RACES);
    }

    info!("Run will use class={class} race={race}");

    // ── Spawn Barony ──────────────────────────────────────────────────────────
    let exe = &config.barony_executable;
    if !exe.exists() {
        bail!("Barony executable not found at {:?}", exe);
    }

    info!("Launching Barony: {:?}", exe);

    // On macOS the path may point to a .app bundle; use `open` to launch it.
    let _child = if exe.extension().and_then(|e| e.to_str()) == Some("app") {
        std::process::Command::new("open")
            .arg("-n")          // new instance even if already running
            .arg("--wait-apps") // don't exit until all apps are quit (ignored here)
            .arg(exe)
            .spawn()
            .with_context(|| format!("Failed to open {:?}", exe))?
    } else {
        std::process::Command::new(exe)
            .spawn()
            .with_context(|| format!("Failed to spawn {:?}", exe))?
    };
    // Keep _child alive for the process lifetime.  In a real implementation we'd
    // store it in app state; for now leaking is safe because Tauri exits with the user.
    std::mem::forget(_child);

    // ── Wait for BotAPI to open (up to 30 s) ─────────────────────────────────
    let client = Arc::new(BaronyClient::new());
    let connected = retry_connect(Arc::clone(&client), 30).await;
    if !connected {
        bail!("Barony BotAPI did not open within 30 s (port 27015)");
    }
    info!("Connected to BotAPI");

    let api = GameAPI::new(Arc::clone(&client));

    // ── Title screen (Press to Start) ────────────────────────────────────────
    // Wait for the intro animation, then click the title screen button.
    sleep(Duration::from_secs(5)).await;
    click(&api, "button").await?;   // "button" = Press-to-Start on title screen
    sleep(Duration::from_millis(1500)).await;

    // ── Main menu navigation ──────────────────────────────────────────────────
    click(&api, "Play Game").await?;
    sleep(Duration::from_millis(800)).await;

    click(&api, "new").await?;
    sleep(Duration::from_millis(800)).await;

    click(&api, "local").await?;
    sleep(Duration::from_secs(2)).await; // lobby initialises, createStartButton is set up

    // ── Create character card ─────────────────────────────────────────────────
    // The lobby start-button tick checks for KeyboardLogin (Space) to call
    // createCharacterCard(). Inject Space now.
    let _ = api.menu_key("space", 3).await;
    sleep(Duration::from_millis(1200)).await;

    // ── Character creation — class ────────────────────────────────────────────
    // menuClick searches the selected widget's context first, so "back_button"
    // resolves to the sub-menu's back button, not the main-menu one.
    click(&api, "class").await?;
    sleep(Duration::from_millis(500)).await;

    click(&api, &class).await?;
    sleep(Duration::from_millis(500)).await;

    click(&api, "back_button").await?;
    sleep(Duration::from_millis(600)).await;

    // ── Character creation — race ─────────────────────────────────────────────
    click(&api, "race").await?;
    sleep(Duration::from_millis(500)).await;

    click(&api, &race).await?;
    sleep(Duration::from_millis(500)).await;

    click(&api, "back_button").await?;
    sleep(Duration::from_millis(600)).await;

    // ── Ready! ────────────────────────────────────────────────────────────────
    click(&api, "ready").await?;
    info!("Clicked Ready — game is loading");

    // ── Wait until dungeon is loaded (in_game = true) ─────────────────────────
    let in_game = wait_in_game(&api, 60).await;
    if !in_game {
        warn!("Timeout waiting for dungeon to load — prologue skipped");
        return Ok(client);
    }
    info!("Dungeon loaded — starting prologue");

    // ── Prologue: scan inventory + learn spells ───────────────────────────────
    match prologue::run_prologue(&api, &class, &race).await {
        Ok(ctx) => {
            if let Err(e) = prologue::save_context(&ctx).await {
                warn!("Could not save start context: {e}");
            }
            info!("Prologue: learned {} spell(s), {} item(s) recorded",
                  ctx.spells_learned.len(), ctx.inventory.len());
        }
        Err(e) => warn!("Prologue failed: {e}"),
    }

    Ok(client)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Simple 32-bit pseudo-random number using the system time as seed.
/// Avoids a `rand` dependency — good enough for picking class/race.
fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(12345);
    // xorshift32
    let mut x = nanos ^ 0x9e3779b9;
    x ^= x << 13; x ^= x >> 17; x ^= x << 5;
    x
}

/// Poll `status` every second until `in_game = true` (max `max_secs`).
async fn wait_in_game(api: &GameAPI, max_secs: u32) -> bool {
    for _ in 0..max_secs {
        if let Ok(s) = api.status().await {
            if s["in_game"].as_bool().unwrap_or(false) {
                return true;
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
    false
}

/// Try to connect to BotAPI every second for up to `max_secs` seconds.
async fn retry_connect(client: Arc<BaronyClient>, max_secs: u32) -> bool {
    for _ in 0..max_secs {
        match client.connect().await {
            Ok(()) => return true,
            Err(e) => {
                warn!("BotAPI not ready yet: {e}");
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    false
}

/// Send a menuClick command; log but don't abort on "button not found" errors.
async fn click(api: &GameAPI, button: &str) -> anyhow::Result<()> {
    match api.menu_click(button).await {
        Ok(()) => { info!("menuClick OK: {button}"); Ok(()) }
        Err(e) => {
            warn!("menuClick {button:?} failed: {e}");
            // Non-fatal — menu state may have changed; continue the sequence.
            Ok(())
        }
    }
}
