# CLAUDE.md — BotBarony Developer Guide for AI Agents

This file is the primary reference for any AI agent (Claude Code or other) working on the BotBarony codebase. Read it entirely before making any changes.

---

## Project Overview

BotBarony is a Tauri v2 desktop application. It launches Barony (a roguelike game), watches its log file, and feeds events to an AI model that decides what action to take next. After the player dies, a second AI call analyses the run and patches the Lua knowledge files to improve future runs.

**Stack:** Rust (backend) + Svelte (frontend, compiled into the binary via Tauri).

---

## Repository Layout

```
src-tauri/src/
├── main.rs               # Tauri app setup + #[tauri::command] handlers
├── ai_provider/
│   ├── mod.rs            # AIProvider trait + build_provider() factory
│   ├── ollama.rs         # Ollama REST implementation
│   ├── lmstudio.rs       # LM Studio (OpenAI-compat) implementation
│   └── api.rs            # Claude + OpenAI cloud implementations
├── game/
│   ├── mod.rs            # GameState struct + apply()
│   ├── launcher.rs       # Spawns the Barony process
│   ├── controller.rs     # Translates GameAction → enigo input
│   ├── log_watcher.rs    # Watches barony.log, emits LogEvent
│   └── screenshot.rs     # Screen capture + stuck detection
├── lua_manager.rs        # Reads/writes lua/ files atomically
└── run_analyst.rs        # Post-run analysis + RunSummary persistence

src/lib/
├── Setup.svelte          # Provider selector + health check UI
├── Dashboard.svelte      # Live run control + event log
└── History.svelte        # Past runs browser

lua/
├── knowledge.lua         # Barony mechanics (facts, read-only by bot)
├── strategy.lua          # Bot decision priorities (patched after each run)
└── death_patterns.lua    # Failure modes + solutions (patched after each run)

agents/prompts/
├── system_bot.md         # System prompt for the in-game decision bot
└── analyst.md            # System prompt for the post-run analyst
```

---

## Core Trait: AIProvider

Every AI backend must implement this trait (defined in `src-tauri/src/ai_provider/mod.rs`):

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Return available model identifiers on this backend.
    async fn list_models(&self) -> anyhow::Result<Vec<String>>;

    /// Send a system + user message pair and return the model's text reply.
    async fn complete(&self, system: &str, user: &str) -> anyhow::Result<String>;

    /// Return true if the backend is reachable and accepting requests.
    async fn health_check(&self) -> anyhow::Result<bool>;

    /// Human-readable name for UI display.
    fn name(&self) -> &str;
}
```

### Implementing a new provider

1. Create a new file in `src-tauri/src/ai_provider/your_provider.rs`.
2. Define a struct with `base_url`, `api_key` (if needed), `client: reqwest::Client`, and `model: String`.
3. `impl AIProvider for YourProvider` — all methods must be async, all errors returned as `anyhow::Result`.
4. Register the provider in `build_provider()` in `mod.rs`.
5. Add it to the `PROVIDERS` array in `src/lib/Setup.svelte`.

**Rules:**
- Never use `unwrap()` in provider code. Use `?` or `.map_err()`.
- Use `build_http_client()` from `mod.rs` — it sets the 120s timeout.
- Use `#[instrument(skip(self, ...))]` on all async methods for tracing.

---

## GameState & LogEvent

`GameState` (in `game/mod.rs`) is the in-memory model of the current run. It is updated via `apply(&LogEvent)` and serialised to JSON for each AI prompt.

`LogEvent` (in `game/log_watcher.rs`) is an enum of parsed log lines:

```rust
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
```

When adding a new variant: add parsing in `parse_line()`, update `GameState::apply()`, and update the bot system prompt if the AI needs to be aware of it.

---

## JSON Context Sent to the AI (per turn)

The bot main loop (to be implemented) must build this JSON and pass it to `AIProvider::complete()`:

```json
{
  "game_state": {
    "level": 3,
    "hp": [45, 100],
    "mp": [20, 50],
    "xp": 120,
    "char_level": 2,
    "turn": 47,
    "inventory": { "healing potion": 2, "dagger": 1 },
    "status_effects": [],
    "recent_events": ["...last 20 log events..."]
  },
  "lua_context": {
    "knowledge": "<full contents of lua/knowledge.lua>",
    "strategy": "<full contents of lua/strategy.lua>",
    "death_patterns": "<full contents of lua/death_patterns.lua>"
  },
  "vision_hint": null
}
```

`vision_hint` is `null` normally. It is set to a `VisionAnalysis` object when the `StuckDetector` fires (no log events for > 10 seconds).

---

## GameAction Enum

The AI returns a JSON action; the controller parses it into:

```rust
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
```

Add new variants to `controller.rs` and update `system_bot.md` with the new action type.

---

## Lua Knowledge Base Format

The three Lua files are read as plain text and embedded verbatim in the AI prompt. They use Lua table syntax but are **not executed** — the AI reads them as structured documentation.

### knowledge.lua
Facts about the game that never change (enemy stats, item descriptions, trap mechanics, dungeon structure). The analyst should add new facts here if it discovers something the bot didn't know.

### strategy.lua
Priority-ordered decision rules for the bot. Format:

```lua
PRIORITY = {
  { condition = "...", action = "...", reason = "..." },
  ...
}
```

The analyst patches this when the bot's overall strategy needs adjustment.

### death_patterns.lua
Known failure modes. Format:

```lua
DEATH_PATTERNS[N] = {
  trigger   = "...",
  cause     = "...",
  solution  = "...",
  run_count = N,
}
```

The analyst increments `run_count` on existing patterns and adds new patterns after each run.

---

## Tauri Commands (Frontend ↔ Backend)

All Tauri commands are in `src-tauri/src/main.rs` and annotated `#[tauri::command]`:

| Command | Args | Returns |
|---------|------|---------|
| `list_models` | `provider, base_url?, api_key?` | `Vec<String>` |
| `check_provider_health` | `provider, base_url?, api_key?` | `bool` |
| `start_run` | `config: RunConfig` | `run_id: String` |
| `stop_run` | `run_id: String` | `()` |
| `get_run_history` | — | `Vec<RunSummary>` |
| `get_lua_knowledge` | — | `LuaKnowledge` |

Errors are surfaced as `Result<T, String>` (Tauri serialises `anyhow::Error` via `.to_string()`).

## Tauri Events (Backend → Frontend)

Use `app.emit("event_name", payload)` from the Rust side:

| Event | Payload | Description |
|-------|---------|-------------|
| `bot:log` | `String` | New log line to display in Dashboard |
| `bot:state` | `GameState` | Updated game state after each turn |
| `bot:run_ended` | `RunSummary` | Emitted when the bot dies and analysis completes |

---

## Rust Code Conventions

- **Edition:** 2021
- **Error handling:** `anyhow::Result<T>` for fallible functions; `thiserror` for domain error enums. Never `unwrap()` in production code.
- **Async:** `tokio` runtime throughout. All I/O, HTTP, and file operations must be async.
- **Logging:** `tracing` macros (`info!`, `debug!`, `warn!`, `error!`). Add `#[instrument]` to significant async functions.
- **Serialization:** `serde` with `#[derive(Serialize, Deserialize)]`. Use `#[serde(rename_all = "snake_case")]` on enums.
- **Naming:** snake_case for functions/variables, CamelCase for types, SCREAMING_SNAKE_CASE for constants.
- **Modules:** Each logical unit in its own file. Re-export via `mod.rs` as needed.
- **Tests:** Unit tests in `#[cfg(test)]` blocks in the same file. Integration tests in `tests/`. Mock the `AIProvider` trait for bot loop tests.

## Svelte Code Conventions

- Svelte 4 component syntax (no Svelte 5 runes).
- Styles are scoped (inside `<style>` block).
- Use `invoke` from `@tauri-apps/api/core` for backend calls.
- Use `listen` from `@tauri-apps/api/event` for real-time events.
- No external UI library — hand-written CSS with the dark gold theme (`#c9a84c` accent, `#0d0d0d` background).

---

## Running Tests

```bash
# Rust unit tests
cd src-tauri && cargo test

# Rust with logging
RUST_LOG=debug cargo test -- --nocapture
```

---

## Common Pitfalls

1. **enigo is `!Send` on some platforms** — always wrap in `Arc<Mutex<>>` and use `spawn_blocking`.
2. **barony.log path** — it varies by OS and Steam install location. Always let the user configure it in Setup.
3. **Lua files are not executed** — they are read as text and embedded in the prompt. Do not add executable Lua that the Rust side needs to run.
4. **Provider timeout** — the default HTTP timeout is 120 seconds. For local models, this may need to be raised for large context windows.
5. **Atomic file writes** — always use `lua_manager::write_atomic()` when patching Lua files to avoid corruption on crash.
