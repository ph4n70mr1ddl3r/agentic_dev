//! Minimal GitHub REST client (issues, labels, milestones) for the `sync`
//! command. Uses the same transport-retry posture as the LLM client.

use anyhow::{bail, Context, Result};
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::util::{retry_after_secs, truncate};

const HTTP_MAX_RETRIES: u32 = 3;

#[derive(Debug, Clone)]
pub struct Repo {
    pub owner: String,
    pub name: String,
}

impl Repo {
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// Parse owner/name from a GitHub git remote URL (HTTPS or SSH) or an explicit
/// `owner/name` shorthand. Returns `None` for non-GitHub URLs / garbage.
pub fn parse_repo(s: &str) -> Option<Repo> {
    let s = s.trim();
    let rest = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
        .or_else(|| s.strip_prefix("git@github.com:"))
        .or_else(|| s.strip_prefix("ssh://git@github.com/"))
        .or_else(|| s.strip_prefix("ssh://github.com/"));
    let rest = match rest {
        Some(r) => r,
        // Bare "owner/name" shorthand (no scheme, no @).
        None if !s.contains("://") && !s.contains('@') => s,
        _ => return None,
    };
    split_owner_name(rest)
}

fn split_owner_name(rest: &str) -> Option<Repo> {
    let rest = rest.trim_end_matches(".git").trim_end_matches('/');
    let (owner, name) = rest.split_once('/')?;
    let owner = owner.trim();
    let name = name.trim();
    if owner.is_empty() || name.is_empty() || name.contains('/') {
        return None;
    }
    Some(Repo {
        owner: owner.to_string(),
        name: name.to_string(),
    })
}

/// Detect the GitHub repo from the current git repo's `origin` remote.
pub fn detect_repo() -> Option<Repo> {
    let out = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_repo(&String::from_utf8_lossy(&out.stdout))
}

/// Percent-encode a string for use as a single URL path segment. Encodes `/`
/// (so it doesn't become a path separator) and spaces; passes through the
/// RFC 3986 unreserved set. Operates on bytes, so non-ASCII is fully encoded.
fn percent_encode_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    /// Present on PRs only (the `/issues` endpoint lists both).
    #[serde(default)]
    pub pull_request: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct CreateLabel<'a> {
    name: &'a str,
    color: &'a str,
}

#[derive(Serialize)]
pub struct CreateIssue<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone: Option<u64>,
}

#[derive(Deserialize)]
struct CreatedIssue {
    number: u64,
}

#[derive(Deserialize)]
struct Milestone {
    number: u64,
    title: String,
}

pub struct GitHub {
    client: Client,
    token: String,
    repo: Repo,
}

impl GitHub {
    pub fn new(token: String, repo: Repo) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("forge/0.1")
            .build()
            .context("build GitHub HTTP client")?;
        Ok(Self {
            client,
            token,
            repo,
        })
    }

    pub fn repo_slug(&self) -> String {
        self.repo.slug()
    }

    fn url(&self, path: &str) -> String {
        format!("https://api.github.com/repos/{}/{}", self.repo.slug(), path)
    }

    async fn get(&self, url: &str) -> Result<reqwest::Response> {
        self.send_retry("GET", self.client.get(url)).await
    }

    async fn post<B: Serialize>(&self, url: &str, body: &B) -> Result<reqwest::Response> {
        self.send_retry("POST", self.client.post(url).json(body))
            .await
    }

    /// Send a request, retrying transient failures (network errors, 429, 5xx)
    /// with exponential backoff and `Retry-After`. Returns the response on
    /// success, 404, or 422 (callers branch on status); bails otherwise.
    async fn send_retry(&self, label: &str, rb: RequestBuilder) -> Result<reqwest::Response> {
        let rb = rb.bearer_auth(&self.token);
        for attempt in 0..=HTTP_MAX_RETRIES {
            let resp = match rb
                .try_clone()
                .ok_or_else(|| anyhow::anyhow!("{label}: body is not retryable"))?
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) if attempt < HTTP_MAX_RETRIES => {
                    let backoff = 1u64 << attempt;
                    tracing::warn!(attempt, backoff, error = %e, "{label}: send error; retrying");
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    continue;
                }
                Err(e) => return Err(e).with_context(|| label.to_string()),
            };
            let st = resp.status();
            if st.is_success() || st.as_u16() == 404 || st.as_u16() == 422 {
                return Ok(resp);
            }
            if attempt < HTTP_MAX_RETRIES && (st.as_u16() == 429 || st.is_server_error()) {
                let backoff = retry_after_secs(&resp).unwrap_or_else(|| 1u64 << attempt);
                tracing::warn!(attempt, %st, backoff, "{label}: retryable status; retrying");
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                continue;
            }
            let text = resp.text().await.unwrap_or_default();
            bail!("{label} failed ({st}): {}", truncate(&text, 1000));
        }
        bail!("{label}: retries exhausted")
    }

    /// Open issues (excluding PRs), up to 100.
    pub async fn list_open_issues(&self) -> Result<Vec<Issue>> {
        let resp = self
            .get(&self.url("issues?state=open&per_page=100"))
            .await?;
        if !resp.status().is_success() {
            bail!("list issues: unexpected status {}", resp.status());
        }
        let issues: Vec<Issue> = resp.json().await.context("decode issues")?;
        Ok(issues
            .into_iter()
            .filter(|i| i.pull_request.is_none())
            .collect())
    }

    /// Create a label if it doesn't exist. Idempotent.
    pub async fn ensure_label(&self, name: &str, color: &str) -> Result<()> {
        let lookup = format!("labels/{}", percent_encode_segment(name));
        let resp = self.get(&self.url(&lookup)).await?;
        if resp.status().is_success() {
            return Ok(()); // already exists
        }
        // send_retry only returns Ok for 2xx/404/422; non-success here is 404.
        let body = CreateLabel { name, color };
        let resp = self.post(&self.url("labels"), &body).await?;
        let st = resp.status();
        if st.is_success() || st.as_u16() == 422 {
            return Ok(()); // created, or already existed (race)
        }
        bail!("create label {name:?}: {st}");
    }

    /// Return the milestone number for `title`, creating it if needed.
    pub async fn ensure_milestone(&self, title: &str) -> Result<Option<u64>> {
        let resp = self
            .get(&self.url("milestones?state=open&per_page=100"))
            .await?;
        if !resp.status().is_success() {
            bail!("list milestones: unexpected status {}", resp.status());
        }
        let ms: Vec<Milestone> = resp.json().await.context("decode milestones")?;
        if let Some(m) = ms.iter().find(|m| m.title.eq_ignore_ascii_case(title)) {
            return Ok(Some(m.number));
        }
        #[derive(Serialize)]
        struct Body<'a> {
            title: &'a str,
            state: &'a str,
        }
        let resp = self
            .post(
                &self.url("milestones"),
                &Body {
                    title,
                    state: "open",
                },
            )
            .await?;
        let st = resp.status();
        if st.is_success() {
            let m: Milestone = resp.json().await.context("decode created milestone")?;
            return Ok(Some(m.number));
        }
        bail!("create milestone {title:?}: {st}");
    }

    /// Create an issue; returns its number.
    pub async fn create_issue(&self, req: &CreateIssue<'_>) -> Result<u64> {
        let resp = self.post(&self.url("issues"), req).await?;
        let st = resp.status();
        if st.is_success() {
            let c: CreatedIssue = resp.json().await.context("decode created issue")?;
            return Ok(c.number);
        }
        bail!("create issue {:?}: {st}", req.title);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_https_remote() {
        let r = parse_repo("https://github.com/ph4n70mr1ddl3r/agentic_dev.git").unwrap();
        assert_eq!(r.slug(), "ph4n70mr1ddl3r/agentic_dev");
    }

    #[test]
    fn parse_ssh_remote() {
        assert_eq!(
            parse_repo("git@github.com:owner/name.git").unwrap().slug(),
            "owner/name"
        );
    }

    #[test]
    fn parse_ssh_scheme_remote() {
        assert_eq!(
            parse_repo("ssh://git@github.com/owner/name")
                .unwrap()
                .slug(),
            "owner/name"
        );
    }

    #[test]
    fn parse_owner_name_shorthand() {
        assert_eq!(parse_repo("owner/name").unwrap().slug(), "owner/name");
    }

    #[test]
    fn parse_non_github_is_none() {
        assert!(parse_repo("https://gitlab.com/a/b").is_none());
        assert!(parse_repo("not a repo").is_none());
        assert!(parse_repo("").is_none());
        assert!(parse_repo("git@gitlab.com:a/b").is_none());
    }

    #[test]
    fn percent_encode_encodes_slash_and_space() {
        assert_eq!(percent_encode_segment("phase-1"), "phase-1");
        assert_eq!(percent_encode_segment("a/b c"), "a%2Fb%20c");
    }
}
