pub mod controller;
pub mod launcher;
pub mod log_watcher;
pub mod prologue;
pub mod screenshot;

/// The game state as reconstructed from log events.
///
/// This struct is serialised to JSON and included in every AI prompt as
/// the "current game context".
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GameState {
    /// Current dungeon level (1-based).
    pub level: u32,
    /// Player HP (current / max).
    pub hp: (i32, i32),
    /// Player MP (current / max).
    pub mp: (i32, i32),
    /// Player XP and character level.
    pub xp: u32,
    pub char_level: u32,
    /// Items in the inventory (name → quantity).
    pub inventory: std::collections::HashMap<String, u32>,
    /// Active status effects.
    pub status_effects: Vec<String>,
    /// Last N log events (kept for context window).
    pub recent_events: Vec<String>,
    /// Number of turns elapsed this run.
    pub turn: u64,
}

impl GameState {
    /// Apply a parsed [`log_watcher::LogEvent`] to update the state.
    pub fn apply(&mut self, event: &log_watcher::LogEvent) {
        use log_watcher::LogEvent;
        match event {
            LogEvent::PickedUp { item } => {
                *self.inventory.entry(item.clone()).or_insert(0) += 1;
            }
            LogEvent::Damaged { amount, .. } => {
                self.hp.0 = (self.hp.0 - *amount as i32).max(0);
            }
            LogEvent::Descended { new_level } => {
                self.level = *new_level;
            }
            LogEvent::LevelUp { new_level } => {
                self.char_level = *new_level;
            }
            _ => {}
        }
        // Keep the last 20 events for the AI prompt.
        let pretty = format!("{:?}", event);
        self.recent_events.push(pretty);
        if self.recent_events.len() > 20 {
            self.recent_events.remove(0);
        }
        self.turn += 1;
    }
}
