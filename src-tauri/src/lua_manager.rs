/// Lua knowledge base manager.
///
/// Reads and writes the three Lua files that form the bot's persistent memory:
///   - `lua/knowledge.lua`  — factual game mechanics
///   - `lua/strategy.lua`   — current playstyle/priorities
///   - `lua/death_patterns.lua` — observed death patterns + fixes
///
/// After each run the post-run analyst patches these files so the bot improves
/// over time without recompilation.
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Bundle of all three Lua knowledge files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuaKnowledge {
    pub knowledge: String,
    pub strategy: String,
    pub death_patterns: String,
}

/// Resolve the path to the `lua/` directory relative to the app data dir.
fn lua_dir() -> PathBuf {
    // In production the files live alongside the binary; in dev they are in the
    // repo root. We check both locations.
    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(Path::new("."))
        .join("lua");
    if dev_path.exists() {
        return dev_path;
    }
    // Fallback: next to the executable.
    std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or(Path::new("."))
        .join("lua")
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read all three Lua knowledge files into memory.
pub async fn load_all() -> anyhow::Result<LuaKnowledge> {
    let dir = lua_dir();
    debug!("Loading Lua knowledge from {:?}", dir);

    let knowledge = tokio::fs::read_to_string(dir.join("knowledge.lua"))
        .await
        .with_context(|| format!("Cannot read {:?}/knowledge.lua", dir))?;

    let strategy = tokio::fs::read_to_string(dir.join("strategy.lua"))
        .await
        .with_context(|| format!("Cannot read {:?}/strategy.lua", dir))?;

    let death_patterns = tokio::fs::read_to_string(dir.join("death_patterns.lua"))
        .await
        .with_context(|| format!("Cannot read {:?}/death_patterns.lua", dir))?;

    Ok(LuaKnowledge { knowledge, strategy, death_patterns })
}

/// Persist an updated [`LuaKnowledge`] back to disk atomically.
///
/// Each file is written to a `.tmp` counterpart first, then renamed, so a
/// crash mid-write never corrupts the existing data.
pub async fn save_all(knowledge: &LuaKnowledge) -> anyhow::Result<()> {
    let dir = lua_dir();
    info!("Saving Lua knowledge to {:?}", dir);

    write_atomic(&dir.join("knowledge.lua"), &knowledge.knowledge).await?;
    write_atomic(&dir.join("strategy.lua"), &knowledge.strategy).await?;
    write_atomic(&dir.join("death_patterns.lua"), &knowledge.death_patterns).await?;

    Ok(())
}

/// Apply a patch string to one of the Lua files.
///
/// The `patch` is a diff-style string produced by the AI analyst. This
/// function appends the patch as a timestamped comment block so that old
/// knowledge is never lost — only overridden by newer entries.
pub async fn apply_patch(file: LuaFile, patch: &str) -> anyhow::Result<()> {
    let dir = lua_dir();
    let path = dir.join(file.filename());

    let existing = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("Cannot read {path:?}"))?;

    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let patched = format!(
        "{existing}\n\n-- === AUTO-PATCH {timestamp} ===\n{patch}\n-- === END PATCH ===\n"
    );

    write_atomic(&path, &patched).await?;
    info!("Applied patch to {:?}", path);
    Ok(())
}

/// Which Lua knowledge file to target.
#[derive(Debug, Clone, Copy)]
pub enum LuaFile {
    Knowledge,
    Strategy,
    DeathPatterns,
}

impl LuaFile {
    pub fn filename(self) -> &'static str {
        match self {
            LuaFile::Knowledge => "knowledge.lua",
            LuaFile::Strategy => "strategy.lua",
            LuaFile::DeathPatterns => "death_patterns.lua",
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

async fn write_atomic(path: &Path, content: &str) -> anyhow::Result<()> {
    let tmp = path.with_extension("lua.tmp");
    tokio::fs::write(&tmp, content)
        .await
        .with_context(|| format!("Cannot write {tmp:?}"))?;
    tokio::fs::rename(&tmp, path)
        .await
        .with_context(|| format!("Cannot rename {tmp:?} -> {path:?}"))?;
    Ok(())
}
