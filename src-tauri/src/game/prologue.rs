/// Prologue — runs immediately after the game starts (in_game = true).
///
/// Responsibilities:
///   1. Wait for the dungeon to be fully loaded.
///   2. Scan initial inventory and record every item by name.
///   3. Read all spellbooks to learn spells.
///   4. Query player state (HP, MP, position).
///   5. Return a `StartContext` that will be stored and fed to the AI.
use anyhow::Context;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::game::controller::{GameAPI, InventoryItem, PlayerState};

// ─── Barony item-type helpers ─────────────────────────────────────────────────

/// Returns true if the Barony item_type integer is a spellbook.
/// Ranges derived from items.hpp enum (SPELLBOOK_FORCEBOLT = 104).
pub fn is_spellbook(type_id: i32) -> bool {
    matches!(type_id,
        // Base game spellbooks
        104..=124 |
        // DLC1 (Myths & Outcasts)
        201..=209 |
        // DLC2 (Legends & Pariahs)
        232..=253 |
        // DLC3 (Deserters & Disciples)
        495..=523 |
        // Generic / colour-coded spellbooks
        531 | 631..=638
    )
}

/// Human-readable category for an item_type.
pub fn item_category(type_id: i32) -> &'static str {
    match type_id {
        0..=19   => "shield/weapon",
        20..=75  => "armor/equipment",
        77..=91  | 234 | 242 | 246..=248 => "potion",
        92..=105 => "scroll",
        t if is_spellbook(t) => "spellbook",
        149..=166 => "gem",
        178..=209 | 243 => "food",
        _ => "misc",
    }
}

// ─── StartContext ─────────────────────────────────────────────────────────────

/// Everything the bot knows at the very start of a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartContext {
    /// Class the bot is playing.
    pub character_class: String,
    /// Race the bot is playing.
    pub character_race: String,
    /// Player stats at dungeon entry.
    pub player: PlayerState,
    /// All items present in the starting inventory.
    pub inventory: Vec<ItemRecord>,
    /// Spells successfully learned during the prologue.
    pub spells_learned: Vec<String>,
    /// ISO-8601 timestamp when the prologue finished.
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemRecord {
    pub item_type: i32,
    pub name: String,
    pub category: String,
    pub count: i32,
    pub identified: bool,
    pub beatitude: i32, // -1 cursed / 0 normal / 1 blessed
    pub status: i32,    // 0 broken … 5 legendary
}

impl From<&InventoryItem> for ItemRecord {
    fn from(i: &InventoryItem) -> Self {
        Self {
            item_type: i.item_type,
            name: i.name.clone(),
            category: item_category(i.item_type).to_string(),
            count: i.count,
            identified: i.identified,
            beatitude: i.beatitude,
            status: i.status,
        }
    }
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Run the prologue for this game session.
///
/// Call this once `in_game = true` (game is in the dungeon).
/// The function blocks until all spellbooks have been read.
pub async fn run_prologue(
    api: &GameAPI,
    character_class: &str,
    character_race: &str,
) -> anyhow::Result<StartContext> {
    info!("Prologue started — class={character_class} race={character_race}");

    // ── 1. Get initial inventory ──────────────────────────────────────────────
    let raw_inv = api.get_inventory().await.context("getInventory failed")?;
    let player  = api.get_player().await.context("getPlayer failed")?;

    info!("Starting inventory: {} items", raw_inv.len());
    for item in &raw_inv {
        info!("  [{:>3}] {} (×{}) cat={}", item.item_type, item.name, item.count, item_category(item.item_type));
    }

    // ── 2. Learn all spellbooks ───────────────────────────────────────────────
    let mut spells_learned = Vec::new();

    // Detect spellbooks by name (most robust) with type-ID as a fallback.
    let spellbooks: Vec<i32> = raw_inv.iter()
        .filter(|i| {
            let name_lc = i.name.to_lowercase();
            name_lc.starts_with("spellbook") || is_spellbook(i.item_type)
        })
        .map(|i| i.item_type)
        .collect();

    if spellbooks.is_empty() {
        info!("No spellbooks in starting inventory");
    } else {
        info!("Found {} spellbook(s) — learning spells…", spellbooks.len());
    }

    for type_id in spellbooks {
        let item_name = raw_inv.iter()
            .find(|i| i.item_type == type_id)
            .map(|i| i.name.as_str())
            .unwrap_or("unknown spellbook");

        info!("  Reading: {} (type {})", item_name, type_id);
        match api.use_item_of_type(type_id).await {
            Ok(name) => {
                let spell = if name.is_empty() { item_name.to_string() } else { name };
                info!("    → Learned: {spell}");
                spells_learned.push(spell);
                sleep(Duration::from_millis(300)).await; // let game process the use
            }
            Err(e) => {
                warn!("    → Failed to use {item_name}: {e}");
            }
        }
    }

    // ── 3. Re-scan inventory (some items may be consumed or changed) ──────────
    let final_inv = api.get_inventory().await.context("getInventory (post-prologue) failed")?;
    let inventory: Vec<ItemRecord> = final_inv.iter().map(ItemRecord::from).collect();

    let ctx = StartContext {
        character_class: character_class.to_string(),
        character_race: character_race.to_string(),
        player,
        inventory,
        spells_learned,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    info!("Prologue complete — {} item(s), {} spell(s) learned",
          ctx.inventory.len(), ctx.spells_learned.len());

    Ok(ctx)
}

// ─── Persistence ──────────────────────────────────────────────────────────────

/// Path where the start context is saved (one file per run, latest overwrites).
fn context_path() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("BotBarony")
        .join("start_context.json")
}

/// Persist the context to disk.
pub async fn save_context(ctx: &StartContext) -> anyhow::Result<()> {
    let path = context_path();
    if let Some(p) = path.parent() {
        tokio::fs::create_dir_all(p).await?;
    }
    let json = serde_json::to_string_pretty(ctx)?;
    tokio::fs::write(&path, json).await?;
    info!("Start context saved to {path:?}");
    Ok(())
}

/// Load the most recent start context, if any.
pub async fn load_context() -> anyhow::Result<Option<StartContext>> {
    let path = context_path();
    if !path.exists() { return Ok(None); }
    let json = tokio::fs::read_to_string(&path).await?;
    Ok(Some(serde_json::from_str(&json)?))
}
