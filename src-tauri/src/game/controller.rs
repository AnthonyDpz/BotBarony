/// Game controller — translates AI decisions into keyboard/mouse actions.
///
/// Uses `enigo` to send OS-level input events. All actions are async to allow
/// delays between key presses without blocking the executor.
use anyhow::Context;
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Mouse, Settings,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, instrument};

// ─── Action model ─────────────────────────────────────────────────────────────

/// A single action the bot can perform.
///
/// This enum is the bridge between the AI's JSON response and raw input events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameAction {
    /// Move in a cardinal direction for `steps` tiles.
    Move { direction: Direction, steps: u8 },
    /// Attack in a direction (or forward if omitted).
    Attack { direction: Option<Direction> },
    /// Open / close inventory.
    ToggleInventory,
    /// Use the item in the given inventory slot (0-indexed).
    UseItem { slot: u8 },
    /// Cast the spell in the given spell slot.
    CastSpell { slot: u8 },
    /// Interact with an object (door, chest, lever…).
    Interact,
    /// Descend to the next level via a staircase.
    Descend,
    /// Wait one turn in place.
    Wait,
    /// Press a raw key by name (escape hatch for edge cases).
    RawKey { key: String },
}

/// Cardinal + diagonal directions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

// ─── Controller ───────────────────────────────────────────────────────────────

/// Thread-safe wrapper around `enigo`.
///
/// `enigo` is `!Send` on some platforms, so we wrap it in `Arc<Mutex<>>` and
/// run input operations in a `spawn_blocking` closure.
pub struct GameController {
    enigo: Arc<Mutex<Enigo>>,
    /// Delay between individual key events (ms). Keep ≥ 30 ms for Barony.
    key_delay_ms: u64,
}

impl GameController {
    pub fn new() -> anyhow::Result<Self> {
        let enigo = Enigo::new(&Settings::default())
            .context("Failed to initialise enigo input backend")?;
        Ok(Self {
            enigo: Arc::new(Mutex::new(enigo)),
            key_delay_ms: 50,
        })
    }

    /// Execute a [`GameAction`], waiting for inter-key delays as required.
    #[instrument(skip(self), fields(action = ?action))]
    pub async fn execute(&self, action: GameAction) -> anyhow::Result<()> {
        match action {
            GameAction::Move { direction, steps } => {
                let key = direction_key(direction);
                for _ in 0..steps {
                    self.press_key(key).await?;
                    sleep(Duration::from_millis(self.key_delay_ms)).await;
                }
            }
            GameAction::Attack { direction } => {
                let key = direction.map(direction_key).unwrap_or(Key::Space);
                self.press_key(key).await?;
            }
            GameAction::ToggleInventory => self.press_key(Key::Tab).await?,
            GameAction::UseItem { slot } => {
                // Barony item slots: press the number key matching (slot + 1).
                let digit = b'1' + slot.min(8);
                self.press_char(digit as char).await?;
            }
            GameAction::CastSpell { slot } => {
                // TODO: map spell slots to Barony key bindings.
                debug!("CastSpell slot={slot} — not yet implemented");
            }
            GameAction::Interact => self.press_key(Key::Return).await?,
            GameAction::Descend => self.press_char('>').await?,
            GameAction::Wait => self.press_char('.').await?,
            GameAction::RawKey { key } => {
                // Parse the key name and press it.
                debug!("RawKey: {key}");
                // TODO: implement a richer key-name parser.
            }
        }
        Ok(())
    }

    // ─── Internal helpers ─────────────────────────────────────────────────────

    async fn press_key(&self, key: Key) -> anyhow::Result<()> {
        let enigo = Arc::clone(&self.enigo);
        tokio::task::spawn_blocking(move || {
            let mut e = enigo.lock().expect("enigo mutex poisoned");
            e.key(key, Click)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;
        Ok(())
    }

    async fn press_char(&self, c: char) -> anyhow::Result<()> {
        let enigo = Arc::clone(&self.enigo);
        tokio::task::spawn_blocking(move || {
            let mut e = enigo.lock().expect("enigo mutex poisoned");
            e.key(Key::Unicode(c), Click)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;
        Ok(())
    }
}

/// Map a [`Direction`] to the corresponding Barony movement key.
fn direction_key(dir: Direction) -> Key {
    match dir {
        Direction::North     => Key::Unicode('w'),
        Direction::South     => Key::Unicode('s'),
        Direction::East      => Key::Unicode('d'),
        Direction::West      => Key::Unicode('a'),
        Direction::NorthEast => Key::Unicode('e'),
        Direction::NorthWest => Key::Unicode('q'),
        Direction::SouthEast => Key::Unicode('c'),
        Direction::SouthWest => Key::Unicode('z'),
    }
}
