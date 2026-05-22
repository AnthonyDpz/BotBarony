# BotBarony

An AI-powered desktop application that plays [Barony](https://github.com/AnthonyDpz/Barony) autonomously. The bot watches the game log in real time, builds a structured context, asks an AI model for the next action, and executes it via keyboard/mouse simulation — all from a single self-contained binary.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tauri Desktop App                        │
│                                                                 │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────────┐   │
│  │  Setup Page  │   │  Dashboard   │   │  History Page    │   │
│  │  (Svelte)    │   │  (Svelte)    │   │  (Svelte)        │   │
│  └──────┬───────┘   └──────┬───────┘   └────────┬─────────┘   │
│         │                  │                     │             │
│         └──────────────────┼─────────────────────┘             │
│                            │ tauri::invoke / events            │
│                    ┌───────▼────────┐                          │
│                    │   Rust Backend │                          │
│                    └───────┬────────┘                          │
│                            │                                   │
│      ┌─────────────────────┼──────────────────────┐            │
│      │                     │                      │            │
│  ┌───▼──────────┐  ┌───────▼──────┐  ┌───────────▼────────┐  │
│  │  LogWatcher  │  │  AIProvider  │  │   RunAnalyst       │  │
│  │  (notify)    │  │  (trait)     │  │   (post-mortem)    │  │
│  └───┬──────────┘  └───────┬──────┘  └───────────┬────────┘  │
│      │                     │                      │            │
│  ┌───▼──────────┐  ┌───────▼──────────────────┐  │            │
│  │  GameState   │  │  Ollama / LMStudio /      │  │            │
│  │  (in-memory) │  │  Claude API / OpenAI API  │  │            │
│  └───┬──────────┘  └──────────────────────────┘  │            │
│      │                                            │            │
│  ┌───▼──────────────┐                    ┌────────▼─────────┐ │
│  │  GameController  │                    │  LuaManager      │ │
│  │  (enigo input)   │                    │  (knowledge base)│ │
│  └──────────────────┘                    └──────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                │                              │
                ▼                              ▼
          Barony.exe                    lua/*.lua files
          (game process)           (persistent AI memory)
```

### Data Flow (per turn)

```
barony.log (new line)
    │
    ▼
LogWatcher::parse_line() → LogEvent
    │
    ▼
GameState::apply(event)  → updated GameState
    │
    ▼
Build JSON context        → { game_state, lua_context, vision_hint? }
    │
    ▼
AIProvider::complete()    → JSON action string
    │
    ▼
GameController::execute() → keyboard/mouse event → Barony
    │
    ▼  (on death)
RunAnalyst::analyse()     → cause_of_death + Lua patches + RunSummary
    │
    ▼
lua_manager::apply_patch() → updated strategy/death_patterns.lua
```

---

## Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| **Rust** | ≥ 1.77 | Install via [rustup](https://rustup.rs/) |
| **Node.js** | ≥ 20 | For Svelte/Vite frontend build |
| **Tauri CLI** | v2 | `cargo install tauri-cli --version "^2"` |
| **Barony** | any | The open-source roguelike game |

At least one AI provider must be running or configured:

| Provider | Requirements |
|----------|-------------|
| **Ollama** | Running on `localhost:11434` |
| **LM Studio** | Running on `localhost:1234` with a model loaded |
| **Claude API** | Anthropic API key |
| **OpenAI API** | OpenAI API key |

---

## Build & Run

```bash
# 1. Clone the repo
git clone https://github.com/AnthonyDpz/BotBarony.git
cd BotBarony

# 2. Install frontend dependencies
npm install

# 3. Development mode (hot-reload)
npm run tauri dev

# 4. Production build
npm run tauri build
# Binary output: src-tauri/target/release/bot-barony
```

---

## Supported AI Providers

### Ollama (local, free)
- Default URL: `http://localhost:11434`
- No API key required
- Recommended models: `llama3`, `mistral`, `qwen2.5`
- Best for: offline use, fast iteration

### LM Studio (local, free)
- Default URL: `http://localhost:1234`
- No API key required — load any GGUF model in LM Studio
- Uses OpenAI-compatible API
- Best for: testing quantized models with a GUI

### Claude API (cloud)
- Endpoint: `https://api.anthropic.com`
- Requires: `ANTHROPIC_API_KEY`
- Recommended models: `claude-sonnet-4-5`, `claude-haiku-4-5`
- Best for: highest reasoning quality

### OpenAI API (cloud)
- Endpoint: `https://api.openai.com`
- Requires: `OPENAI_API_KEY`
- Recommended models: `gpt-4o`, `gpt-4o-mini`
- Best for: vision analysis (stuck detection)

---

## Project Structure

```
BotBarony/
├── src-tauri/src/
│   ├── main.rs               Tauri entry point + command handlers
│   ├── ai_provider/          AIProvider trait + provider implementations
│   ├── game/                 Game launcher, controller, log watcher, screenshot
│   ├── lua_manager.rs        Lua knowledge base read/write
│   └── run_analyst.rs        Post-run AI analysis + history persistence
├── src/                      Svelte frontend
│   ├── lib/Setup.svelte       Provider config + health check
│   ├── lib/Dashboard.svelte   Live run control + log stream
│   └── lib/History.svelte     Past runs + AI summaries
├── lua/
│   ├── knowledge.lua          Barony game mechanics facts
│   ├── strategy.lua           Bot decision priorities
│   └── death_patterns.lua     Known failure modes + fixes
└── agents/prompts/
    ├── system_bot.md          In-game bot system prompt
    └── analyst.md             Post-run analyst system prompt
```

---

## Contributing

Issues and PRs welcome. See [CLAUDE.md](CLAUDE.md) for architecture details and coding conventions.

## License

MIT
