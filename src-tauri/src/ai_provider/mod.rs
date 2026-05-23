/// AI Provider abstraction layer.
///
/// All AI providers implement the [`AIProvider`] trait, which exposes three
/// async operations: model discovery, completion, and health check.
/// A concrete provider is built at runtime with [`build_provider`].
use async_trait::async_trait;

pub mod api;
pub mod lmstudio;
pub mod ollama;

// ─── Core Trait ───────────────────────────────────────────────────────────────

/// Unified interface for every AI backend supported by BotBarony.
///
/// Implementations must be `Send + Sync` because they are shared across async
/// tasks (bot loop + UI thread).
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Return the list of models available on this backend.
    async fn list_models(&self) -> anyhow::Result<Vec<String>>;

    /// Send a system + user prompt and return the model's text response.
    async fn complete(&self, system: &str, user: &str) -> anyhow::Result<String>;

    /// Return `true` if the backend is reachable and ready to serve requests.
    async fn health_check(&self) -> anyhow::Result<bool>;

    /// Human-readable provider name (e.g. "Ollama", "Claude API").
    fn name(&self) -> &str;
}

// ─── Provider Kind ────────────────────────────────────────────────────────────

/// Identifies which concrete provider to instantiate.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Ollama,
    LmStudio,
    Claude,
    OpenAI,
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderKind::Ollama => write!(f, "Ollama"),
            ProviderKind::LmStudio => write!(f, "LM Studio"),
            ProviderKind::Claude => write!(f, "Claude API"),
            ProviderKind::OpenAI => write!(f, "OpenAI API"),
        }
    }
}

// ─── Factory ──────────────────────────────────────────────────────────────────

/// Build a boxed [`AIProvider`] from a provider name string.
///
/// `base_url` is required for local providers (Ollama, LM Studio) and optional
/// for API providers (Claude, OpenAI) — they fall back to their official URLs.
/// `api_key` is required for Claude and OpenAI.
pub fn build_provider(
    kind: &str,
    base_url: Option<&str>,
    api_key: Option<&str>,
    model: Option<&str>,
) -> anyhow::Result<Box<dyn AIProvider>> {
    match kind.to_lowercase().as_str() {
        "ollama" => {
            let url = base_url.unwrap_or("http://localhost:11434").to_string();
            let mut p = ollama::OllamaProvider::new(url);
            if let Some(m) = model { p = p.with_model(m); }
            Ok(Box::new(p))
        }
        "lmstudio" | "lm_studio" | "lm studio" => {
            let url = base_url.unwrap_or("http://localhost:1234").to_string();
            let mut p = lmstudio::LmStudioProvider::new(url);
            if let Some(m) = model { p = p.with_model(m); }
            Ok(Box::new(p))
        }
        "claude" => {
            let key = api_key
                .ok_or_else(|| anyhow::anyhow!("Claude API requires an API key"))?
                .to_string();
            let url = base_url.unwrap_or("https://api.anthropic.com").to_string();
            let mut p = api::ClaudeProvider::new(url, key);
            if let Some(m) = model { p = p.with_model(m); }
            Ok(Box::new(p))
        }
        "openai" => {
            let key = api_key
                .ok_or_else(|| anyhow::anyhow!("OpenAI API requires an API key"))?
                .to_string();
            let url = base_url.unwrap_or("https://api.openai.com").to_string();
            let mut p = api::OpenAIProvider::new(url, key);
            if let Some(m) = model { p = p.with_model(m); }
            Ok(Box::new(p))
        }
        other => anyhow::bail!("Unknown provider: {other}"),
    }
}

// ─── Shared HTTP helper ────────────────────────────────────────────────────────

/// Build a shared `reqwest::Client` with a reasonable timeout for AI requests.
pub(crate) fn build_http_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(Into::into)
}
