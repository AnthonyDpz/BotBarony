/// Barony log watcher and parser.
///
/// Watches `barony.log` for new lines using `notify` (inotify/kqueue/FSEvents),
/// parses each line into a [`LogEvent`], and forwards events through a Tokio
/// mpsc channel to the bot main loop.
///
/// Barony log format (examples):
///   [12:34:56] Player1 moves north.
///   [12:34:57] Player1 attacks Skeleton for 8 damage.
///   [12:34:58] Player1 receives 3 damage from Skeleton.
///   [12:34:59] Player1 picks up Dagger.
///   [12:35:01] Player1 dies.
use std::path::{Path, PathBuf};
use anyhow::Context;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebounceEventResult};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Seek, SeekFrom};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

// ─── Event model ──────────────────────────────────────────────────────────────

/// A single parsed event from the Barony log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LogEvent {
    /// Player moved in a direction.
    Move { direction: String },
    /// Player attacked an entity.
    Attack { target: String, damage: u32 },
    /// Player received damage.
    Damaged { source: String, amount: u32 },
    /// Player picked up an item.
    PickedUp { item: String },
    /// Player used an item.
    UsedItem { item: String },
    /// Player descended a level.
    Descended { new_level: u32 },
    /// Player died.
    Died,
    /// Player gained a level.
    LevelUp { new_level: u32 },
    /// An unrecognised log line (stored verbatim for debugging).
    Raw { line: String },
}

// ─── Parser ───────────────────────────────────────────────────────────────────

/// Parse a raw log line into a [`LogEvent`].
///
/// Returns `None` if the line is empty or a header/metadata line.
pub fn parse_line(line: &str) -> Option<LogEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Strip the timestamp prefix "[HH:MM:SS] " if present.
    let content = if line.starts_with('[') {
        line.splitn(2, "] ").nth(1).unwrap_or(line)
    } else {
        line
    };

    // Pattern matching on well-known line shapes.
    if content.contains(" moves ") {
        let direction = content
            .split(" moves ")
            .nth(1)
            .and_then(|s| s.split('.').next())
            .unwrap_or("unknown")
            .to_string();
        return Some(LogEvent::Move { direction });
    }

    if content.contains(" attacks ") && content.contains(" for ") {
        // "X attacks Y for N damage."
        let mut parts = content.splitn(2, " attacks ");
        let _ = parts.next(); // actor
        if let Some(rest) = parts.next() {
            let mut rp = rest.splitn(2, " for ");
            let target = rp.next().unwrap_or("unknown").to_string();
            let damage = rp
                .next()
                .and_then(|s| s.split(' ').next())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            return Some(LogEvent::Attack { target, damage });
        }
    }

    if content.contains(" receives ") && content.contains(" damage from ") {
        let mut parts = content.splitn(2, " receives ");
        let _ = parts.next();
        if let Some(rest) = parts.next() {
            let mut rp = rest.splitn(2, " damage from ");
            let amount = rp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let source = rp
                .next()
                .and_then(|s| s.split('.').next())
                .unwrap_or("unknown")
                .to_string();
            return Some(LogEvent::Damaged { source, amount });
        }
    }

    if content.contains(" picks up ") {
        let item = content
            .splitn(2, " picks up ")
            .nth(1)
            .and_then(|s| s.split('.').next())
            .unwrap_or("unknown")
            .to_string();
        return Some(LogEvent::PickedUp { item });
    }

    if content.ends_with(" dies.") || content.ends_with(" dies!") {
        return Some(LogEvent::Died);
    }

    // Fallback: store the raw line.
    Some(LogEvent::Raw { line: content.to_string() })
}

// ─── Watcher ──────────────────────────────────────────────────────────────────

pub struct LogWatcher {
    log_path: PathBuf,
    sender: mpsc::Sender<LogEvent>,
}

impl LogWatcher {
    pub fn new(log_path: impl AsRef<Path>, sender: mpsc::Sender<LogEvent>) -> Self {
        Self {
            log_path: log_path.as_ref().to_path_buf(),
            sender,
        }
    }

    /// Start watching the log file. This function runs until the sender is
    /// dropped or an unrecoverable error occurs.
    pub async fn run(self) -> anyhow::Result<()> {
        info!("Watching {:?}", self.log_path);

        // Open the file and seek to the end — we only care about new lines.
        let file = std::fs::File::open(&self.log_path)
            .with_context(|| format!("Cannot open log file: {:?}", self.log_path))?;
        let mut reader = std::io::BufReader::new(file);
        reader.seek(SeekFrom::End(0))?;

        // Set up a debounced file-system watcher.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<DebounceEventResult>(32);
        let mut debouncer = new_debouncer(
            std::time::Duration::from_millis(100),
            move |res: DebounceEventResult| {
                let _ = tx.blocking_send(res);
            },
        )?;

        debouncer
            .watcher()
            .watch(&self.log_path, RecursiveMode::NonRecursive)
            .with_context(|| format!("Cannot watch {:?}", self.log_path))?;

        while let Some(result) = rx.recv().await {
            match result {
                Ok(_events) => {
                    // Drain all new lines from the file.
                    let mut line = String::new();
                    loop {
                        line.clear();
                        match reader.read_line(&mut line) {
                            Ok(0) => break, // EOF — no more new lines yet
                            Ok(_) => {
                                if let Some(event) = parse_line(&line) {
                                    debug!("Log event: {:?}", event);
                                    if self.sender.send(event).await.is_err() {
                                        info!("Log event receiver dropped — stopping watcher");
                                        return Ok(());
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error reading log: {e}");
                                break;
                            }
                        }
                    }
                }
                Err(e) => warn!("File-watch debounce error: {:?}", e),
            }
        }

        Ok(())
    }
}
