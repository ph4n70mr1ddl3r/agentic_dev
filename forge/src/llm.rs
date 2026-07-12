use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message>,
    response_format: ResponseFormat,
    max_tokens: u32,
    thinking: Thinking,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
}

#[derive(Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    typ: &'static str,
}

#[derive(Serialize)]
struct Thinking {
    #[serde(rename = "type")]
    typ: &'static str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    // content may be null when JSON mode hiccups (handled by the caller).
    content: Option<String>,
}

/// Minimal OpenAI-compatible chat client (DeepSeek). Requests JSON output.
pub struct Llm {
    client: Client,
    config: Config,
}

impl Llm {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder().build().context("build HTTP client")?;
        Ok(Self { client, config })
    }

    /// Send a system + user message and return the assistant's content as JSON
    /// text. Retries once on empty content (a known JSON-mode quirk).
    pub async fn chat_json(&self, system: &str, user: &str) -> Result<String> {
        for _ in 0..2 {
            match self.chat_once(system, user).await? {
                Some(c) if !c.trim().is_empty() => return Ok(c),
                _ => tracing::warn!("LLM returned empty content; retrying once"),
            }
        }
        bail!("LLM returned empty content twice; tweak the prompt or retry later")
    }

    async fn chat_once(&self, system: &str, user: &str) -> Result<Option<String>> {
        let req = ChatRequest {
            model: &self.config.model,
            messages: vec![
                Message {
                    role: "system",
                    content: system.to_string(),
                },
                Message {
                    role: "user",
                    content: user.to_string(),
                },
            ],
            response_format: ResponseFormat {
                typ: "json_object",
            },
            max_tokens: self.config.max_tokens,
            thinking: Thinking {
                typ: if self.config.thinking { "enabled" } else { "disabled" },
            },
            // temperature/top_p are ignored in thinking mode, so omit then.
            temperature: if self.config.thinking { None } else { Some(0.2) },
            reasoning_effort: self.config.reasoning_effort.clone(),
        };

        let url = format!("{}/chat/completions", self.config.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&req)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("LLM request to {url} failed ({status}): {}", truncate(&body, 1000));
        }

        // Read as text (handles keep-alive blank lines) then parse; serde
        // tolerates surrounding whitespace.
        let text = resp.text().await.context("read response body")?;
        let chat: ChatResponse = serde_json::from_str(&text)
            .with_context(|| format!("decode chat response: {}", truncate(&text, 500)))?;

        let content = chat
            .choices
            .into_iter()
            .next()
            .context("no choices in response")?
            .message
            .content;

        Ok(content)
    }
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
