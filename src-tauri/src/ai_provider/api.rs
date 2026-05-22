/// Cloud API providers: Anthropic Claude and OpenAI.
///
/// Both providers require an API key. Base URLs default to official endpoints
/// but can be overridden (e.g. for proxies or local mirrors).
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::{AIProvider, build_http_client};

// ═══════════════════════════════════════════════════════════════════════════════
//  Claude (Anthropic Messages API)
// ═══════════════════════════════════════════════════════════════════════════════

// ─── Wire types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<ClaudeMessage<'a>>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeModelsResponse {
    data: Vec<ClaudeModel>,
}

#[derive(Debug, Deserialize)]
struct ClaudeModel {
    id: String,
}

// ─── Provider struct ──────────────────────────────────────────────────────────

pub struct ClaudeProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
    pub model: String,
}

impl ClaudeProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            client: build_http_client().expect("failed to build HTTP client"),
            model: "claude-sonnet-4-6".to_string(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

// ─── Trait impl ───────────────────────────────────────────────────────────────

#[async_trait]
impl AIProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "Claude API"
    }

    #[instrument(skip(self), fields(provider = "Claude"))]
    async fn list_models(&self) -> anyhow::Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);
        debug!("GET {url}");
        let resp: ClaudeModelsResponse = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.data.into_iter().map(|m| m.id).collect())
    }

    #[instrument(skip(self, system, user), fields(provider = "Claude", model = %self.model))]
    async fn complete(&self, system: &str, user: &str) -> anyhow::Result<String> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = ClaudeRequest {
            model: &self.model,
            max_tokens: 4096,
            system,
            messages: vec![ClaudeMessage { role: "user", content: user }],
        };
        debug!("POST {url}");
        let resp: ClaudeResponse = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        resp.content
            .into_iter()
            .find(|c| c.content_type == "text")
            .and_then(|c| c.text)
            .ok_or_else(|| anyhow::anyhow!("Claude returned no text content"))
    }

    #[instrument(skip(self), fields(provider = "Claude"))]
    async fn health_check(&self) -> anyhow::Result<bool> {
        // A lightweight models list request is the cheapest possible ping.
        Ok(self.list_models().await.is_ok())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  OpenAI Chat Completions API
// ═══════════════════════════════════════════════════════════════════════════════

// ─── Wire types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct OpenAIRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAIMessage<'a>>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct OpenAIMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessageContent,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessageContent {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
}

// ─── Provider struct ──────────────────────────────────────────────────────────

pub struct OpenAIProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
    pub model: String,
}

impl OpenAIProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            client: build_http_client().expect("failed to build HTTP client"),
            model: "gpt-4o".to_string(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

// ─── Trait impl ───────────────────────────────────────────────────────────────

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "OpenAI API"
    }

    #[instrument(skip(self), fields(provider = "OpenAI"))]
    async fn list_models(&self) -> anyhow::Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);
        debug!("GET {url}");
        let resp: OpenAIModelsResponse = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        // Filter to only chat-capable models.
        let mut models: Vec<String> = resp
            .data
            .into_iter()
            .map(|m| m.id)
            .filter(|id| id.contains("gpt") || id.contains("o1") || id.contains("o3"))
            .collect();
        models.sort();
        Ok(models)
    }

    #[instrument(skip(self, system, user), fields(provider = "OpenAI", model = %self.model))]
    async fn complete(&self, system: &str, user: &str) -> anyhow::Result<String> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = OpenAIRequest {
            model: &self.model,
            messages: vec![
                OpenAIMessage { role: "system", content: system },
                OpenAIMessage { role: "user",   content: user   },
            ],
            temperature: 0.7,
        };
        debug!("POST {url}");
        let resp: OpenAIResponse = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("OpenAI returned an empty choices array"))
    }

    #[instrument(skip(self), fields(provider = "OpenAI"))]
    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(self.list_models().await.is_ok())
    }
}
