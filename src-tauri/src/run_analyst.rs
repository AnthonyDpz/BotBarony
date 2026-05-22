/// Post-run analyst.
///
/// After the player dies, this module:
///   1. Collects the full run transcript (log events + final state).
///   2. Asks the AI to analyse the run and identify what went wrong.
///   3. Generates Lua patches to improve future runs.
///   4. Persists the run summary to the history file.
use crate::ai_provider::AIProvider;
use crate::game::GameState;
use crate::lua_manager::{self, LuaFile};
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

// ─── Data models ──────────────────────────────────────────────────────────────

/// Configuration for a bot run, sent from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    /// Which AI provider to use.
    pub provider: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    /// Model identifier (e.g. "llama3", "claude-sonnet-4-5").
    pub model: String,
    /// Character class chosen for this run.
    pub character_class: String,
    /// Path to the Barony executable.
    pub barony_executable: Option<String>,
}

/// Summary of a completed run, written to history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duration_secs: u64,
    pub character_class: String,
    pub dungeon_level_reached: u32,
    pub character_level: u32,
    pub turns: u64,
    pub cause_of_death: String,
    pub ai_analysis: String,
    pub lua_patches_applied: Vec<String>,
    pub provider: String,
    pub model: String,
}

// ─── Analyst ──────────────────────────────────────────────────────────────────

pub struct RunAnalyst {
    provider: Box<dyn AIProvider>,
    analyst_prompt: String,
}

impl RunAnalyst {
    /// Load the analyst prompt from disk and build the analyst.
    pub async fn new(provider: Box<dyn AIProvider>) -> anyhow::Result<Self> {
        let prompt_path = {
            let base = std::env::current_exe()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf();
            // Dev path
            let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("agents/prompts/analyst.md");
            if dev.exists() { dev } else { base.join("agents/prompts/analyst.md") }
        };

        let analyst_prompt = tokio::fs::read_to_string(&prompt_path)
            .await
            .with_context(|| format!("Cannot read analyst prompt at {prompt_path:?}"))?;

        Ok(Self { provider, analyst_prompt })
    }

    /// Run a post-mortem analysis and write Lua patches + history entry.
    pub async fn analyse(
        &self,
        config: &RunConfig,
        state: &GameState,
        started_at: DateTime<Utc>,
    ) -> anyhow::Result<RunSummary> {
        let ended_at = Utc::now();
        let duration_secs = (ended_at - started_at).num_seconds().max(0) as u64;

        let knowledge = lua_manager::load_all().await?;

        // Build the user message with full run context.
        let user_msg = serde_json::json!({
            "run_config": config,
            "final_state": state,
            "lua_knowledge": {
                "knowledge": knowledge.knowledge,
                "strategy": knowledge.strategy,
                "death_patterns": knowledge.death_patterns,
            },
            "request": "Analyse this run. Identify the cause of death. \
                        Suggest Lua patches for strategy.lua and death_patterns.lua \
                        to improve future runs. Return JSON matching RunAnalysisResult."
        })
        .to_string();

        info!("Sending post-run analysis to {}", self.provider.name());
        let raw = self
            .provider
            .complete(&self.analyst_prompt, &user_msg)
            .await?;

        // Parse the AI response.
        let analysis: RunAnalysisResult = serde_json::from_str(&raw).unwrap_or(RunAnalysisResult {
            cause_of_death: "Unknown (analysis parse error)".to_string(),
            summary: raw.clone(),
            strategy_patch: None,
            death_patterns_patch: None,
        });

        // Apply patches.
        let mut patches_applied = Vec::new();
        if let Some(ref patch) = analysis.strategy_patch {
            if let Err(e) = lua_manager::apply_patch(LuaFile::Strategy, patch).await {
                error!("Failed to apply strategy patch: {e}");
            } else {
                patches_applied.push("strategy.lua".to_string());
            }
        }
        if let Some(ref patch) = analysis.death_patterns_patch {
            if let Err(e) = lua_manager::apply_patch(LuaFile::DeathPatterns, patch).await {
                error!("Failed to apply death_patterns patch: {e}");
            } else {
                patches_applied.push("death_patterns.lua".to_string());
            }
        }

        let summary = RunSummary {
            id: Uuid::new_v4().to_string(),
            started_at,
            ended_at,
            duration_secs,
            character_class: config.character_class.clone(),
            dungeon_level_reached: state.level,
            character_level: state.char_level,
            turns: state.turn,
            cause_of_death: analysis.cause_of_death,
            ai_analysis: analysis.summary,
            lua_patches_applied: patches_applied,
            provider: config.provider.clone(),
            model: config.model.clone(),
        };

        // Persist to history.
        append_history(&summary).await?;

        Ok(summary)
    }
}

/// Expected JSON shape returned by the AI analyst.
#[derive(Debug, Deserialize)]
struct RunAnalysisResult {
    cause_of_death: String,
    summary: String,
    strategy_patch: Option<String>,
    death_patterns_patch: Option<String>,
}

// ─── History persistence ───────────────────────────────────────────────────────

fn history_path() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("BotBarony/history.jsonl")
}

/// Append a run summary to the JSONL history file.
async fn append_history(summary: &RunSummary) -> anyhow::Result<()> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let line = serde_json::to_string(summary)? + "\n";
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await?;
    file.write_all(line.as_bytes()).await?;
    info!("Run history written to {:?}", path);
    Ok(())
}

/// Load all past runs from the JSONL history file.
pub async fn load_history() -> anyhow::Result<Vec<RunSummary>> {
    let path = history_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = tokio::fs::read_to_string(&path).await?;
    let runs = content
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    Ok(runs)
}
