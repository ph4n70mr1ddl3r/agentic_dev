use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message>,
    response_format: ResponseFormat,
    temperature: f32,
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
    content: String,
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

    /// Send a system + user message and return the assistant's content.
    pub async fn chat_json(&self, system: &str, user: &str) -> Result<String> {
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
            temperature: 0.2,
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
            bail!("LLM request to {url} failed ({status}): {body}");
        }

        let chat: ChatResponse = resp.json().await.context("decode chat response")?;
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
