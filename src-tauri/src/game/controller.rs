/// Game controller — translates AI decisions into keyboard/mouse actions.
///
/// `enigo` is `!Send` on macOS (holds a raw CoreGraphics pointer), so we cannot
/// share it across async tasks via `Arc<Mutex<>>` + `spawn_blocking`. Instead we
/// own the `Enigo` instance on a dedicated OS thread and communicate with it via
/// a `std::sync::mpsc` channel, converting results back to async via
/// `tokio::sync::oneshot`.
use anyhow::Context;
use enigo::{Direction::Click, Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::sleep;
use tracing::{debug, instrument};

// ─── Action model ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameAction {
    Move { direction: Direction, steps: u8 },
    Attack { direction: Option<Direction> },
    ToggleInventory,
    UseItem { slot: u8 },
    CastSpell { slot: u8 },
    Interact,
    Descend,
    Wait,
    RawKey { key: String },
}

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

// ─── Internal message types ────────────────────────────────────────────────────

enum InputMsg {
    PressKey(Key, oneshot::Sender<anyhow::Result<()>>),
    PressChar(char, oneshot::Sender<anyhow::Result<()>>),
}

// ─── Controller ───────────────────────────────────────────────────────────────

pub struct GameController {
    sender: mpsc::Sender<InputMsg>,
    key_delay_ms: u64,
}

impl GameController {
    /// Spawn the dedicated input thread and return a controller handle.
    pub fn new() -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel::<InputMsg>();

        std::thread::Builder::new()
            .name("botbarony-input".into())
            .spawn(move || {
                let mut enigo = match Enigo::new(&Settings::default()) {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("[input thread] Failed to init enigo: {e}");
                        return;
                    }
                };
                while let Ok(msg) = rx.recv() {
                    match msg {
                        InputMsg::PressKey(key, reply) => {
                            let res = enigo.key(key, Click).map_err(anyhow::Error::from);
                            let _ = reply.send(res);
                        }
                        InputMsg::PressChar(c, reply) => {
                            let res = enigo.key(Key::Unicode(c), Click).map_err(anyhow::Error::from);
                            let _ = reply.send(res);
                        }
                    }
                }
            })
            .context("Failed to spawn input thread")?;

        Ok(Self { sender: tx, key_delay_ms: 50 })
    }

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
                let digit = b'1' + slot.min(8);
                self.press_char(digit as char).await?;
            }
            GameAction::CastSpell { slot } => {
                debug!("CastSpell slot={slot} — not yet implemented");
            }
            GameAction::Interact => self.press_key(Key::Return).await?,
            GameAction::Descend  => self.press_char('>').await?,
            GameAction::Wait     => self.press_char('.').await?,
            GameAction::RawKey { key } => {
                debug!("RawKey: {key}");
            }
        }
        Ok(())
    }

    async fn press_key(&self, key: Key) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(InputMsg::PressKey(key, tx))
            .map_err(|_| anyhow::anyhow!("Input thread is no longer running"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Input thread dropped reply channel"))?
    }

    async fn press_char(&self, c: char) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(InputMsg::PressChar(c, tx))
            .map_err(|_| anyhow::anyhow!("Input thread is no longer running"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Input thread dropped reply channel"))?
    }
}

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
