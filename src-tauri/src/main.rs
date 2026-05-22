// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod ai_provider;
mod game;
mod lua_manager;
mod run_analyst;

// ─── Tauri Commands ────────────────────────────────────────────────────────────

/// List available models from a specific AI provider.
#[tauri::command]
async fn list_models(
    provider: String,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<Vec<String>, String> {
    let provider = ai_provider::build_provider(&provider, base_url.as_deref(), api_key.as_deref())
        .map_err(|e| e.to_string())?;
    provider.list_models().await.map_err(|e| e.to_string())
}

/// Health-check a provider (returns true if reachable).
#[tauri::command]
async fn check_provider_health(
    provider: String,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<bool, String> {
    let provider = ai_provider::build_provider(&provider, base_url.as_deref(), api_key.as_deref())
        .map_err(|e| e.to_string())?;
    provider.health_check().await.map_err(|e| e.to_string())
}

/// Start a new bot run.
#[tauri::command]
async fn start_run(
    app: tauri::AppHandle,
    config: run_analyst::RunConfig,
) -> Result<String, String> {
    // TODO: spawn the bot main loop in a background task, return run_id
    let run_id = uuid::Uuid::new_v4().to_string();
    info!("Starting run {run_id} with config: {:?}", config);
    let _ = app; // will be used to emit events
    Ok(run_id)
}

/// Stop the currently running bot.
#[tauri::command]
async fn stop_run(run_id: String) -> Result<(), String> {
    // TODO: signal the running task to stop
    info!("Stopping run {run_id}");
    Ok(())
}

/// Fetch the list of past runs from disk.
#[tauri::command]
async fn get_run_history() -> Result<Vec<run_analyst::RunSummary>, String> {
    run_analyst::load_history().await.map_err(|e| e.to_string())
}

/// Read the current Lua knowledge base files.
#[tauri::command]
async fn get_lua_knowledge() -> Result<lua_manager::LuaKnowledge, String> {
    lua_manager::load_all().await.map_err(|e| e.to_string())
}

// ─── Entry Point ──────────────────────────────────────────────────────────────

fn main() {
    // Initialise structured logging; RUST_LOG overrides the default filter.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            info!("BotBarony starting up — version {}", app.package_info().version);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_models,
            check_provider_health,
            start_run,
            stop_run,
            get_run_history,
            get_lua_knowledge,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
