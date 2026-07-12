use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::Config;

/// Hard ceiling for the auto-grow token budget on truncation. Keeps a runaway
/// retry from requesting an unsupported output size.
const MAX_TOKEN_CAP: u32 = 32_768;

/// Max transport-level retries for transient failures (HTTP 408/429/5xx or
/// network errors), with exponential backoff.
const HTTP_MAX_RETRIES: u32 = 3;

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
    /// OpenAI-style stop reason: "stop", "length" (truncated), "content_filter", ...
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ResponseMessage {
    // content may be null when JSON mode hiccups (handled by the caller).
    content: Option<String>,
}

/// One chat turn's payload, the caller can branch on.
struct ChatOutcome {
    content: Option<String>,
    finish_reason: Option<String>,
}

/// Minimal OpenAI-compatible chat client (DeepSeek). Requests JSON output.
pub struct Llm {
    client: Client,
    config: Config,
}

impl Llm {
    pub fn new(config: Config) -> Result<Self> {
        let timeout = config.timeout;
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("build HTTP client")?;
        Ok(Self { client, config })
    }

    /// Send a system + user message and return the assistant's content as JSON
    /// text.
    ///
    /// Robustness for a weak model in JSON mode:
    /// - retries on empty content (a known JSON-mode quirk), and
    /// - on `finish_reason == "length"` (output truncated by `max_tokens`),
    ///   retries with a larger budget up to [`MAX_TOKEN_CAP`] so a large plan
    ///   doesn't fail as a cryptic JSON parse error.
    pub async fn chat_json(&self, system: &str, user: &str) -> Result<String> {
        let mut budget = self.config.max_tokens;
        let mut saw_truncation = false;
        for attempt in 0..3 {
            let out = self.chat_once(system, user, budget).await?;
            if let Some("length") = out.finish_reason.as_deref() {
                saw_truncation = true;
                if budget >= MAX_TOKEN_CAP {
                    bail!(
                        "LLM output truncated (finish_reason=length) at the {} token \
                         cap; raise DEEPSEEK_MAX_TOKENS, switch to deepseek-v4-pro, or \
                         simplify the task",
                        MAX_TOKEN_CAP
                    );
                }
                let next = budget.saturating_mul(2).min(MAX_TOKEN_CAP);
                tracing::warn!(
                    attempt,
                    prev = budget,
                    next,
                    "LLM output truncated; retrying with a larger token budget"
                );
                budget = next;
                continue;
            }
            if let Some(c) = &out.content {
                if !c.trim().is_empty() {
                    return Ok(c.clone());
                }
            }
            tracing::warn!(attempt, "LLM returned empty content; retrying");
        }
        if saw_truncation {
            bail!(
                "LLM output kept getting truncated; simplify the task or raise \
                 DEEPSEEK_MAX_TOKENS"
            );
        }
        bail!("LLM returned empty content repeatedly; tweak the prompt or retry later")
    }

    async fn chat_once(&self, system: &str, user: &str, max_tokens: u32) -> Result<ChatOutcome> {
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
            response_format: ResponseFormat { typ: "json_object" },
            max_tokens,
            thinking: Thinking {
                typ: if self.config.thinking {
                    "enabled"
                } else {
                    "disabled"
                },
            },
            // temperature/top_p are ignored in thinking mode, so omit them.
            temperature: if self.config.thinking {
                None
            } else {
                Some(0.2)
            },
            reasoning_effort: self.config.reasoning_effort.clone(),
        };

        let url = format!("{}/chat/completions", self.config.base_url);

        // Transport-level retries: transient network errors and retryable HTTP
        // statuses (408/429/5xx) get exponential backoff, honoring Retry-After.
        // Application-level retries (truncation/empty content) are handled by
        // `chat_json` on top of a successful response.
        for attempt in 0..=HTTP_MAX_RETRIES {
            let resp = match self
                .client
                .post(&url)
                .bearer_auth(&self.config.api_key)
                .json(&req)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) if attempt < HTTP_MAX_RETRIES && is_transient_err(&e) => {
                    let backoff = backoff_secs(attempt);
                    tracing::warn!(
                        attempt, backoff_secs = backoff, error = %e,
                        "transient network error; retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    continue;
                }
                Err(e) => return Err(e).context("LLM request send failed"),
            };

            let status = resp.status();
            if status.is_success() {
                return decode(resp).await;
            }
            if attempt < HTTP_MAX_RETRIES && is_retryable_status(status) {
                let retry_after = retry_after_secs(&resp);
                let _ = resp.text().await; // drain so the connection can be reused
                let backoff = retry_after.unwrap_or_else(|| backoff_secs(attempt));
                tracing::warn!(
                    attempt, %status, backoff_secs = backoff,
                    "retryable HTTP status; retrying"
                );
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                continue;
            }
            let body = resp.text().await.unwrap_or_default();
            bail!(
                "LLM request to {url} failed ({status}): {}",
                truncate(&body, 1000)
            );
        }
        bail!("LLM request to {url} failed: retries exhausted")
    }
}

/// Parse a successful (2xx) response into a `ChatOutcome`.
async fn decode(resp: reqwest::Response) -> Result<ChatOutcome> {
    // Read as text (handles keep-alive blank lines) then parse; serde
    // tolerates surrounding whitespace.
    let text = resp.text().await.context("read response body")?;
    let chat: ChatResponse = serde_json::from_str(&text)
        .with_context(|| format!("decode chat response: {}", truncate(&text, 500)))?;
    let choice = chat
        .choices
        .into_iter()
        .next()
        .context("no choices in response")?;
    Ok(ChatOutcome {
        content: choice.message.content,
        finish_reason: choice.finish_reason,
    })
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429) || status.is_server_error()
}

fn is_transient_err(e: &reqwest::Error) -> bool {
    e.is_timeout() || e.is_connect()
}

/// Exponential backoff (1s, 2s, 4s, 8s, ...), capped at 30s.
fn backoff_secs(attempt: u32) -> u64 {
    (1u64 << attempt).min(30)
}

/// Best-effort parse of the HTTP `Retry-After` header as whole seconds.
/// (Only the delta-seconds form is honored, not the HTTP-date form.)
fn retry_after_secs(resp: &reqwest::Response) -> Option<u64> {
    resp.headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
