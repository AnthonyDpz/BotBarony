/// Barony log event types (legacy — kept for GameState compatibility).
///
/// Events are now pushed directly from the BotAPI socket (see controller.rs).
/// This module no longer performs file watching.
use serde::{Deserialize, Serialize};

/// A single parsed event from the game.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LogEvent {
    Move { direction: String },
    Attack { target: String, damage: u32 },
    Damaged { source: String, amount: u32 },
    PickedUp { item: String },
    UsedItem { item: String },
    Descended { new_level: u32 },
    Died,
    LevelUp { new_level: u32 },
    Raw { line: String },
}
