use anyhow::{anyhow, Result};

/// Environment-driven configuration for the LLM (DeepSeek, OpenAI-compatible).
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load forge/.env then .env if present (existing env vars always win).
        for path in ["forge/.env", ".env"] {
            let _ = dotenvy::from_path(path);
        }

        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .map_err(|_| anyhow!("DEEPSEEK_API_KEY is not set (see forge/.env.example)"))?;
        let base_url =
            std::env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| "https://api.deepseek.com".into());
        let model =
            std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-chat".into());

        Ok(Self {
            api_key,
            base_url,
            model,
        })
    }
}
