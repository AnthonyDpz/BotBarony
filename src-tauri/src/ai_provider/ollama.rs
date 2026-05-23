/// Ollama local provider.
///
/// Speaks to the Ollama REST API (default: http://localhost:11434).
/// No API key required — Ollama runs fully offline.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::{AIProvider, build_http_client};

// ─── Wire types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

// Ollama returns {"message": {...}, "done": true, ...} on success
// or {"error": "model not found"} on failure — we handle both.
#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: Option<MessageContent>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    content: String,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    name: String,
}

// ─── Provider struct ──────────────────────────────────────────────────────────

pub struct OllamaProvider {
    base_url: String,
    client: reqwest::Client,
    /// Model to use for completions (defaults to the first available model).
    pub model: String,
}

impl OllamaProvider {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: build_http_client().expect("failed to build HTTP client"),
            model: "llama3".to_string(),
        }
    }

    /// Override the model used for completions.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

// ─── Trait impl ───────────────────────────────────────────────────────────────

#[async_trait]
impl AIProvider for OllamaProvider {
    fn name(&self) -> &str {
        "Ollama"
    }

    #[instrument(skip(self), fields(provider = "Ollama"))]
    async fn list_models(&self) -> anyhow::Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        debug!("GET {url}");
        let resp: TagsResponse = self.client.get(&url).send().await?.error_for_status()?.json().await?;
        Ok(resp.models.into_iter().map(|m| m.name).collect())
    }

    #[instrument(skip(self, system, user), fields(provider = "Ollama", model = %self.model))]
    async fn complete(&self, system: &str, user: &str) -> anyhow::Result<String> {
        let url = format!("{}/api/chat", self.base_url);
        let body = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage { role: "system", content: system },
                ChatMessage { role: "user",   content: user   },
            ],
            stream: false,
        };
        debug!("POST {url}");
        let resp: ChatResponse = self.client.post(&url).json(&body).send().await?.json().await?;
        if let Some(err) = resp.error {
            anyhow::bail!("Ollama error: {err}");
        }
        resp.message
            .map(|m| m.content)
            .ok_or_else(|| anyhow::anyhow!("Ollama returned no message content"))
    }

    #[instrument(skip(self), fields(provider = "Ollama"))]
    async fn health_check(&self) -> anyhow::Result<bool> {
        let url = format!("{}/api/tags", self.base_url);
        let status = self.client.get(&url).send().await?.status();
        Ok(status.is_success())
    }
}
