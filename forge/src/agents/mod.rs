pub mod architect;
pub mod ceo;
pub mod tech_lead;

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

use crate::llm::Llm;
use crate::plan::Task;
use crate::schema::SchemaRegistry;

/// Tolerate models that wrap JSON in markdown code fences despite JSON mode.
pub fn extract_json(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let after = rest.find('\n').map(|i| &rest[i + 1..]).unwrap_or(rest);
        if let Some(end) = after.rfind("```") {
            return after[..end].trim();
        }
        return after.trim();
    }
    trimmed
}

/// Does a hat exist for this role? (Single source of truth shared with the
/// orchestrator so "skipped: no hat" never disagrees with dispatch.)
pub fn has_hat(role: &str) -> bool {
    matches!(
        role.trim().to_ascii_lowercase().as_str(),
        "solution architect" | "architect" | "tech lead" | "tech-lead"
    )
}

/// Everything a hat needs to do its job: the LLM, the schema registry (for
/// mechanical review + the contract text), and the examples dir (templates).
pub struct HatContext<'a> {
    pub llm: &'a Llm,
    pub registry: &'a SchemaRegistry,
    pub examples_dir: PathBuf,
}

/// Dispatch a task to its owning hat. Gains an arm per hat.
pub async fn run_task(task: &Task, ctx: &HatContext<'_>) -> Result<Value> {
    match task.role.trim().to_ascii_lowercase().as_str() {
        "solution architect" | "architect" => {
            let entity_text = ctx.registry.schema_text(architect::ENTITY_SCHEMA_ID)?;
            let field_text = ctx.registry.schema_text(architect::FIELD_SCHEMA_ID)?;
            let example_text = read(ctx.examples_dir.join("entity.json"))?;
            architect::run_architect(
                ctx.llm,
                ctx.registry,
                task,
                entity_text,
                field_text,
                &example_text,
            )
            .await
        }
        "tech lead" | "tech-lead" => {
            let wf = ctx.registry.schema_text(tech_lead::WORKFLOW_SCHEMA_ID)?;
            let jl = ctx.registry.schema_text(tech_lead::JSON_LOGIC_SCHEMA_ID)?;
            let ac = ctx.registry.schema_text(tech_lead::ACTION_SCHEMA_ID)?;
            let example_text = read(ctx.examples_dir.join("workflow.json"))?;
            tech_lead::run_tech_lead(ctx.llm, ctx.registry, task, wf, jl, ac, &example_text).await
        }
        other => Err(anyhow!(
            "no agent implemented for role {:?} (hats so far: Solution Architect, Tech Lead)",
            other
        )),
    }
}

fn read(path: PathBuf) -> Result<String> {
    std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))
}
