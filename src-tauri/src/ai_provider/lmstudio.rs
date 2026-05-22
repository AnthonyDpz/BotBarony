/// LM Studio local provider.
///
/// LM Studio exposes an OpenAI-compatible REST API on port 1234 by default.
/// No API key required.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::{AIProvider, build_http_client};

// ─── Wire types (OpenAI-compatible) ───────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
    max_tokens: i32,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelData>,
}

#[derive(Debug, Deserialize)]
struct ModelData {
    id: String,
}

// ─── Provider struct ──────────────────────────────────────────────────────────

pub struct LmStudioProvider {
    base_url: String,
    client: reqwest::Client,
    pub model: String,
}

impl LmStudioProvider {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: build_http_client().expect("failed to build HTTP client"),
            model: "local-model".to_string(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

// ─── Trait impl ───────────────────────────────────────────────────────────────

#[async_trait]
impl AIProvider for LmStudioProvider {
    fn name(&self) -> &str {
        "LM Studio"
    }

    #[instrument(skip(self), fields(provider = "LmStudio"))]
    async fn list_models(&self) -> anyhow::Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);
        debug!("GET {url}");
        let resp: ModelsResponse = self.client.get(&url).send().await?.json().await?;
        Ok(resp.data.into_iter().map(|m| m.id).collect())
    }

    #[instrument(skip(self, system, user), fields(provider = "LmStudio", model = %self.model))]
    async fn complete(&self, system: &str, user: &str) -> anyhow::Result<String> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage { role: "system", content: system },
                ChatMessage { role: "user",   content: user   },
            ],
            temperature: 0.7,
            max_tokens: -1,
            stream: false,
        };
        debug!("POST {url}");
        let resp: ChatResponse = self.client.post(&url).json(&body).send().await?.json().await?;
        resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("LM Studio returned an empty choices array"))
    }

    #[instrument(skip(self), fields(provider = "LmStudio"))]
    async fn health_check(&self) -> anyhow::Result<bool> {
        let url = format!("{}/v1/models", self.base_url);
        let status = self.client.get(&url).send().await?.status();
        Ok(status.is_success())
    }
}
