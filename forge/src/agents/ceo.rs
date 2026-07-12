use anyhow::{anyhow, Result};

use super::extract_json;
use crate::llm::Llm;
use crate::plan::CompanyPlan;

pub const CEO_SYSTEM: &str = r#"You are the CEO of a virtual software company made of AI agents. Each agent wears a "hat" (a role). The company builds software end-to-end, as one big company would.

You will be given a COMPANY BRIEF (the founders' goal and hard constraints) and the ARCHITECTURE DECISION RECORDS (the immutable technical decisions already made). They are IMMUTABLE — plan within them.

Produce the company's plan:
1. Mission — restate it in one or two sentences.
2. Organization — the hats/roles and what each owns. Be lean but complete (cover planning, domain/business analysis, architecture, data, security, UX, engineering leadership, implementation, QA, DevOps, release, documentation). SPECIALIZE NARROWLY so each agent has a small, clear scope.
3. Roadmap — an ordered list of phases, each with a concise goal and explicit, checkable exit criteria.
4. Contribution model — a short paragraph: how a unit of work goes from idea to merged (GitHub issues = tasks, branches = work, PRs = review, tags = milestones).
5. First phase task breakdown — a concrete list of tasks to begin the first phase. Each task gets an id (e.g. T1), a short title, the owning role/hat, a type, a one-paragraph description, and dependency ids.

Guiding principles:
- The product is MODEL-DRIVEN: a platform of engines + metadata modules.
- The executing model is cheap and weak; favor many small atomic tasks, contracts-first (schemas before code), templates, and reviewer loops.
- Use Microsoft Dynamics 365 as the reference for all domain decisions.
- Financials + Supply Chain Management are the v1 scope.

Respond with ONLY a JSON object with EXACTLY this shape:
{
  "mission": string,
  "organization": { "hats": [ { "name": string, "role": string, "responsibilities": [string] } ] },
  "roadmap": [ { "name": string, "goal": string, "exit_criteria": [string] } ],
  "contribution_model": string,
  "first_phase": { "name": string, "tasks": [ { "id": string, "title": string, "role": string, "type": string, "description": string, "depends_on": [string] } ] }
}
Output nothing but the JSON object."#;

/// Run the CEO hat: read the brief (+ ADRs, if provided), return a structured
/// company plan.
pub async fn run_ceo(llm: &Llm, brief: &str, adrs: &str) -> Result<CompanyPlan> {
    let mut user = format!("COMPANY BRIEF:\n\n{brief}\n\n");
    if !adrs.trim().is_empty() {
        user.push_str(
            "ARCHITECTURE DECISION RECORDS (immutable constraints — plan within them):\n\n",
        );
        user.push_str(adrs);
        user.push_str("\n\n");
    }
    user.push_str("Now produce the company plan as a JSON object.");

    // A weak model in JSON mode usually succeeds, but can occasionally emit
    // invalid JSON or a plan that fails our integrity checks. Per ADR-0005
    // (reviewer loops), retry a few times and feed the specific rejection back
    // so the model can correct it, rather than failing the whole run.
    const MAX_ATTEMPTS: u32 = 3;
    let mut last_err: Option<String> = None;
    for attempt in 0..MAX_ATTEMPTS {
        let prompt = match &last_err {
            None => user.clone(),
            Some(err) => format!(
                "{user}\n\nYour previous attempt was rejected by the reviewer:\n{err}\n\n\
                 Return ONLY a corrected JSON object with EXACTLY the required shape.",
            ),
        };
        let raw = llm.chat_json(CEO_SYSTEM, &prompt).await?;
        let json = extract_json(&raw);
        match serde_json::from_str::<CompanyPlan>(json) {
            Ok(plan) => match plan.validate() {
                Ok(()) => return Ok(plan),
                Err(e) => {
                    let msg = e.to_string();
                    tracing::warn!(
                        attempt,
                        error = %msg,
                        "CEO plan failed validation; retrying"
                    );
                    last_err = Some(msg);
                }
            },
            Err(e) => {
                let preview: String = raw.chars().take(800).collect();
                let msg = format!("invalid JSON ({e}). Preview:\n{preview}");
                tracing::warn!(attempt, "CEO returned invalid JSON; retrying");
                last_err = Some(msg);
            }
        }
    }
    Err(anyhow!(
        "CEO failed to produce a valid plan after {MAX_ATTEMPTS} attempts: {}",
        last_err.unwrap_or_else(|| "unknown error".into())
    ))
}
