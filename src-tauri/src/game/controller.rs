/// BotAPI client — connects to Barony's TCP JSON-Lines interface (port 27015).
///
/// Each `send_cmd()` call registers a oneshot channel in `pending`, sends the
/// JSON line, then waits for the reader task to route the matching response back.
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{debug, info, instrument};

static NEXT_ID: AtomicU32 = AtomicU32::new(1);
fn next_id() -> u32 { NEXT_ID.fetch_add(1, Ordering::Relaxed) }

// ─── Protocol types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoveDirection {
    North, South, East, West, Forward, Backward,
}
impl std::fmt::Display for MoveDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            MoveDirection::North    => "north",
            MoveDirection::South    => "south",
            MoveDirection::East     => "east",
            MoveDirection::West     => "west",
            MoveDirection::Forward  => "forward",
            MoveDirection::Backward => "backward",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum GameEvent {
    Connected { version: String },
    Death { killer: Option<String>, level: Option<u32>, ticks: Option<u64> },
    LevelChange { level: u32 },
    Combat { damage: i32, hp: i32, maxhp: i32 },
    ItemPickup { name: String, #[serde(rename = "type")] item_type: i32 },
    GameOver,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerState {
    pub hp: i32, pub maxhp: i32,
    pub mp: i32, pub maxmp: i32,
    pub x: f32, pub y: f32, pub yaw: f32,
    pub level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterInfo {
    pub uid: u32,
    #[serde(rename = "type")] pub monster_type: i32,
    pub hp: i32, pub maxhp: i32,
    pub x: f32, pub y: f32, pub dist: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    #[serde(rename = "type")] pub item_type: i32,
    pub count: i32,
    pub identified: bool,
}

// ─── Pending request map ──────────────────────────────────────────────────────

type PendingMap = Arc<Mutex<HashMap<u32, oneshot::Sender<anyhow::Result<Value>>>>>;

// ─── Client inner state ───────────────────────────────────────────────────────

struct Inner {
    writer:  Mutex<Option<tokio::net::tcp::OwnedWriteHalf>>,
    pending: PendingMap,
    events:  mpsc::Sender<GameEvent>,
}

// ─── BaronyClient ─────────────────────────────────────────────────────────────

pub struct BaronyClient {
    inner:      Arc<Inner>,
    pub events: Mutex<mpsc::Receiver<GameEvent>>,
}

impl BaronyClient {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(128);
        let inner = Arc::new(Inner {
            writer:  Mutex::new(None),
            pending: Arc::new(Mutex::new(HashMap::new())),
            events:  tx,
        });
        Self { inner, events: Mutex::new(rx) }
    }

    /// Connect (or reconnect) to Barony on localhost:27015.
    pub async fn connect(&self) -> anyhow::Result<()> {
        self.connect_to("127.0.0.1:27015").await
    }

    pub async fn connect_to(&self, addr: &str) -> anyhow::Result<()> {
        let stream = TcpStream::connect(addr)
            .await
            .with_context(|| format!("Cannot connect to BotAPI at {addr}"))?;

        let (read_half, write_half) = stream.into_split();
        *self.inner.writer.lock().await = Some(write_half);

        // Spawn reader task — owns read_half, shares pending + events.
        let pending = Arc::clone(&self.inner.pending);
        let events  = self.inner.events.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(read_half).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                debug!("← {}", line);
                let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };

                if let Some(id_val) = v.get("id") {
                    // Command response.
                    let id = id_val.as_u64().unwrap_or(0) as u32;
                    let result = if v["ok"].as_bool().unwrap_or(false) {
                        Ok(v["data"].clone())
                    } else {
                        let msg = v["error"].as_str().unwrap_or("unknown error").to_owned();
                        Err(anyhow::anyhow!(msg))
                    };
                    if let Some(reply) = pending.lock().await.remove(&id) {
                        let _ = reply.send(result);
                    }
                } else if v.get("event").is_some() {
                    // Pushed event.
                    if let Ok(ev) = serde_json::from_value::<GameEvent>(v) {
                        let _ = events.send(ev).await;
                    }
                }
            }
            info!("[BotAPI] Connection closed by server");
        });

        info!("[BotAPI] Connected to Barony at {addr}");
        Ok(())
    }

    /// Send a raw JSON command line and wait up to 5 s for the response.
    #[instrument(skip(self))]
    pub async fn send_cmd(&self, cmd: &str, args_json: Option<&str>) -> anyhow::Result<Value> {
        let id   = next_id();
        let line = build_cmd_line(id, cmd, args_json);
        debug!("→ {}", line.trim());

        let (tx, rx) = oneshot::channel();
        self.inner.pending.lock().await.insert(id, tx);

        // Send to game.
        {
            let mut guard = self.inner.writer.lock().await;
            match guard.as_mut() {
                Some(w) => w.write_all(line.as_bytes()).await
                    .with_context(|| "Write to BotAPI failed")?,
                None => bail!("Not connected to BotAPI"),
            }
        }

        // Wait for response.
        match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_))     => bail!("BotAPI reply channel dropped"),
            Err(_) => {
                self.inner.pending.lock().await.remove(&id);
                bail!("BotAPI command timed out: {cmd}")
            }
        }
    }

    /// Drain all buffered events without blocking.
    pub async fn poll_events(&self) -> Vec<GameEvent> {
        let mut rx = self.events.lock().await;
        let mut out = Vec::new();
        while let Ok(ev) = rx.try_recv() { out.push(ev); }
        out
    }
}

fn build_cmd_line(id: u32, cmd: &str, args_json: Option<&str>) -> String {
    match args_json {
        None => format!("{{\"id\":{id},\"cmd\":\"{cmd}\"}}\n"),
        Some(args) => {
            // args is a JSON object like {"direction":"north","ticks":30}
            // Merge: {"id":N,"cmd":"move","direction":"north","ticks":30}
            let inner = args.trim().trim_start_matches('{').trim_end_matches('}');
            if inner.is_empty() {
                format!("{{\"id\":{id},\"cmd\":\"{cmd}\"}}\n")
            } else {
                format!("{{\"id\":{id},\"cmd\":\"{cmd}\",{inner}}}\n")
            }
        }
    }
}

// ─── High-level typed API ─────────────────────────────────────────────────────

pub struct GameAPI {
    client: Arc<BaronyClient>,
}

impl GameAPI {
    pub fn new(client: Arc<BaronyClient>) -> Self { Self { client } }

    pub async fn get_player(&self) -> anyhow::Result<PlayerState> {
        let data = self.client.send_cmd("getPlayer", None).await?;
        serde_json::from_value(data).context("parse getPlayer")
    }

    pub async fn get_monsters(&self) -> anyhow::Result<Vec<MonsterInfo>> {
        let data = self.client.send_cmd("getMonsters", None).await?;
        serde_json::from_value(data).context("parse getMonsters")
    }

    pub async fn get_inventory(&self) -> anyhow::Result<Vec<InventoryItem>> {
        let data = self.client.send_cmd("getInventory", None).await?;
        serde_json::from_value(data).context("parse getInventory")
    }

    pub async fn get_level(&self) -> anyhow::Result<u32> {
        let data = self.client.send_cmd("getLevel", None).await?;
        Ok(data["level"].as_u64().unwrap_or(0) as u32)
    }

    pub async fn move_dir(&self, dir: MoveDirection, ticks: u32) -> anyhow::Result<()> {
        let args = format!("{{\"direction\":\"{dir}\",\"ticks\":{ticks}}}");
        self.client.send_cmd("move", Some(&args)).await?;
        Ok(())
    }

    pub async fn turn(&self, angle: f32) -> anyhow::Result<()> {
        let args = format!("{{\"angle\":{angle}}}");
        self.client.send_cmd("turn", Some(&args)).await?;
        Ok(())
    }

    pub async fn attack(&self) -> anyhow::Result<()> {
        self.client.send_cmd("attack", None).await?; Ok(())
    }

    pub async fn use_item(&self) -> anyhow::Result<()> {
        self.client.send_cmd("use", None).await?; Ok(())
    }

    pub async fn wait_action(&self) -> anyhow::Result<()> {
        self.client.send_cmd("wait", None).await?; Ok(())
    }

    pub async fn descend(&self) -> anyhow::Result<()> {
        self.client.send_cmd("descend", None).await?; Ok(())
    }

    pub async fn poll_events(&self) -> Vec<GameEvent> {
        self.client.poll_events().await
    }
}
