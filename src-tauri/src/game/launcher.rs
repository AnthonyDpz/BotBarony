/// Game launcher — starts Barony and creates a new character/game.
///
/// Strategy:
///   1. Locate the Barony executable (configurable path, or auto-detect via
///      Steam library / common install locations).
///   2. Spawn the process and wait until the main menu is detected (via log
///      sentinel or a timed screenshot check).
///   3. Send the key sequence required to: New Game → choose class → enter
///      dungeon.
use std::path::{Path, PathBuf};
use std::process::Child;
use anyhow::{bail, Context};
use tracing::{info, warn};

/// Configuration for a new game run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LaunchConfig {
    /// Absolute path to the Barony executable.
    pub barony_executable: PathBuf,
    /// Character class to pick (e.g. "human", "skeleton").
    pub character_class: String,
    /// Seed for the run (0 = random).
    pub seed: u32,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            barony_executable: auto_detect_barony().unwrap_or_else(|| PathBuf::from("barony")),
            character_class: "human".to_string(),
            seed: 0,
        }
    }
}

/// Try to locate the Barony executable on common install paths.
pub fn auto_detect_barony() -> Option<PathBuf> {
    let candidates = [
        // macOS Steam
        PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Library/Application Support/Steam/steamapps/common/Barony/barony"),
        // Linux Steam
        PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".steam/steam/steamapps/common/Barony/barony"),
        // Windows Steam (cross-compile placeholder)
        PathBuf::from("C:/Program Files (x86)/Steam/steamapps/common/Barony/barony.exe"),
    ];

    candidates.into_iter().find(|p| p.exists())
}

// ─── Launcher ─────────────────────────────────────────────────────────────────

pub struct GameLauncher {
    config: LaunchConfig,
    process: Option<Child>,
}

impl GameLauncher {
    pub fn new(config: LaunchConfig) -> Self {
        Self { config, process: None }
    }

    /// Spawn Barony and wait for it to be ready.
    ///
    /// Returns `Err` if the executable is not found or the process fails to start.
    pub async fn launch(&mut self) -> anyhow::Result<()> {
        if !self.config.barony_executable.exists() {
            bail!(
                "Barony executable not found at {:?}",
                self.config.barony_executable
            );
        }

        info!("Launching Barony: {:?}", self.config.barony_executable);

        let child = std::process::Command::new(&self.config.barony_executable)
            .spawn()
            .with_context(|| {
                format!("Failed to spawn {:?}", self.config.barony_executable)
            })?;

        self.process = Some(child);

        // Give the game time to reach the main menu.
        // TODO: replace with log-sentinel detection once log_watcher is ready.
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;

        info!("Barony is up — proceeding to character creation");
        Ok(())
    }

    /// Navigate menus to start a new game with the configured class.
    pub async fn start_new_game(&self) -> anyhow::Result<()> {
        // TODO: implement menu navigation via GameController (keyboard)
        info!(
            "Starting new game — class: {}, seed: {}",
            self.config.character_class, self.config.seed
        );
        Ok(())
    }

    /// Kill the Barony process if it is still running.
    pub fn kill(&mut self) {
        if let Some(ref mut child) = self.process {
            match child.kill() {
                Ok(_) => info!("Barony process terminated"),
                Err(e) => warn!("Failed to kill Barony process: {e}"),
            }
        }
        self.process = None;
    }

    /// Returns `true` if the process is alive.
    pub fn is_running(&mut self) -> bool {
        match &mut self.process {
            None => false,
            Some(child) => matches!(child.try_wait(), Ok(None)),
        }
    }
}

impl Drop for GameLauncher {
    fn drop(&mut self) {
        self.kill();
    }
}
