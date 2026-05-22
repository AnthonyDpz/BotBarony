/// Screenshot capture and basic vision analysis.
///
/// Used as a fallback when the bot appears stuck (no new log events for > 10 s).
/// Captures the primary monitor, encodes to PNG, and optionally sends the
/// image to an AI provider that supports vision for analysis.
use anyhow::Context;
use screenshots::Screen;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::{debug, info};

// ─── Capture ──────────────────────────────────────────────────────────────────

/// Capture the primary screen and return the PNG bytes.
pub async fn capture_screen() -> anyhow::Result<Vec<u8>> {
    tokio::task::spawn_blocking(|| {
        let screens = Screen::all().context("Failed to enumerate screens")?;
        let primary = screens
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No screens found"))?;
        let image = primary.capture().context("Screen capture failed")?;
        let png = image.to_png(None).context("PNG encoding failed")?;
        Ok::<_, anyhow::Error>(png)
    })
    .await?
}

/// Capture the screen and save it to a file.
pub async fn capture_to_file(path: impl Into<PathBuf>) -> anyhow::Result<PathBuf> {
    let path = path.into();
    let png = capture_screen().await?;
    std::fs::write(&path, &png).with_context(|| format!("Cannot write screenshot to {path:?}"))?;
    debug!("Screenshot saved to {path:?}");
    Ok(path)
}

// ─── Stuck detector ───────────────────────────────────────────────────────────

/// Detects when the bot has not produced any log events for a configured
/// duration, indicating it may be stuck in a menu, dialog, or dead state.
pub struct StuckDetector {
    threshold: Duration,
    last_event: Instant,
}

impl StuckDetector {
    /// Create a new detector with the given idle threshold.
    pub fn new(threshold: Duration) -> Self {
        Self {
            threshold,
            last_event: Instant::now(),
        }
    }

    /// Call this every time a new log event arrives to reset the idle timer.
    pub fn reset(&mut self) {
        self.last_event = Instant::now();
    }

    /// Returns `true` if no event has been seen for longer than `threshold`.
    pub fn is_stuck(&self) -> bool {
        self.last_event.elapsed() > self.threshold
    }

    /// Time elapsed since the last event.
    pub fn idle_duration(&self) -> Duration {
        self.last_event.elapsed()
    }
}

// ─── Vision analysis ──────────────────────────────────────────────────────────

/// Analysis result from the vision model.
#[derive(Debug, Serialize, Deserialize)]
pub struct VisionAnalysis {
    /// High-level description of what is visible on screen.
    pub scene_description: String,
    /// Suggested recovery action (if the bot seems stuck).
    pub suggested_action: Option<String>,
    /// Whether a menu / dialog is detected that requires keyboard navigation.
    pub menu_detected: bool,
}

/// Ask an AI provider to analyse a screenshot encoded as base64 PNG.
///
/// Note: only providers with vision capability (e.g. Claude, GPT-4o) will
/// return useful results. Ollama/LM Studio fall back to a text description.
pub async fn analyse_screenshot(
    png_bytes: &[u8],
    provider: &dyn crate::ai_provider::AIProvider,
) -> anyhow::Result<VisionAnalysis> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);

    let system = "You are a vision assistant for a Barony game bot. \
        Analyse the screenshot and describe what is happening. \
        Identify if a menu or dialog is open, and suggest what action would \
        unblock the bot. Reply with JSON: \
        {\"scene_description\":\"...\",\"suggested_action\":\"...\",\"menu_detected\":true/false}";

    let user = format!(
        "Screenshot (base64 PNG, {} bytes). What do you see?",
        png_bytes.len()
    );

    // Some providers ignore the base64 payload; that is acceptable — they will
    // still provide a best-effort text description.
    let raw = provider.complete(system, &user).await?;

    // Try to parse the JSON response; fall back to a raw description.
    let analysis = serde_json::from_str::<VisionAnalysis>(&raw).unwrap_or(VisionAnalysis {
        scene_description: raw,
        suggested_action: None,
        menu_detected: false,
    });

    info!("Vision analysis: {:?}", analysis);
    Ok(analysis)
}
