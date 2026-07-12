use anyhow::{anyhow, Result};
use std::time::Duration;

/// Environment-driven configuration for the LLM (DeepSeek, OpenAI-compatible).
///
/// See https://api-docs.deepseek.com/ . Notable points reflected here:
/// - models are `deepseek-v4-flash` (cheap) / `deepseek-v4-pro`;
///   `deepseek-chat`/`deepseek-reasoner` are deprecated 2026-07-24.
/// - thinking mode defaults to ENABLED server-side, so we send an explicit
///   toggle to keep the cheap non-thinking path by default.
/// - JSON output (`response_format: json_object`) can occasionally return empty
///   content; `max_tokens` must be set to avoid mid-JSON truncation.
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub thinking: bool,
    pub reasoning_effort: Option<String>,
    pub max_tokens: u32,
    pub timeout: Duration,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load forge/.env then .env if present (existing env vars always win).
        for path in ["forge/.env", ".env"] {
            let _ = dotenvy::from_path(path);
        }

        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .map_err(|_| anyhow!("DEEPSEEK_API_KEY is not set (see forge/.env.example)"))?;
        let base_url = std::env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com".into());
        let model = std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".into());

        let thinking = std::env::var("DEEPSEEK_THINKING")
            .map(|v| matches!(v.as_str(), "1" | "true" | "enabled"))
            .unwrap_or(false);

        // `reasoning_effort` default is applied by `finalize_reasoning()` once
        // the final `thinking` flag is known (commands may override thinking).
        let reasoning_effort = std::env::var("DEEPSEEK_REASONING_EFFORT")
            .ok()
            .filter(|s| !s.is_empty());

        let max_tokens = std::env::var("DEEPSEEK_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8192);

        let timeout = Duration::from_secs(
            std::env::var("DEEPSEEK_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(180),
        );

        Ok(Self {
            api_key,
            base_url,
            model,
            thinking,
            reasoning_effort,
            max_tokens,
            timeout,
        })
    }

    /// Apply the default reasoning effort once the final `thinking` flag is
    /// settled. Commands (e.g. the CEO forcing thinking on) call this after
    /// they may have flipped `thinking`, so the "high by default when thinking"
    /// rule lives in exactly one place.
    pub fn finalize_reasoning(&mut self) {
        if self.thinking && self.reasoning_effort.is_none() {
            self.reasoning_effort = Some("high".into());
        }
    }
}
