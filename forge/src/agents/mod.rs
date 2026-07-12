pub mod architect;
pub mod ceo;

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

/// Everything a hat needs to do its job: the LLM, the schema registry (for
/// mechanical review + the contract text), and the examples dir (templates).
pub struct HatContext<'a> {
    pub llm: &'a Llm,
    pub registry: &'a SchemaRegistry,
    pub examples_dir: PathBuf,
}

/// Dispatch a task to its owning hat. Only the Architect is wired so far; the
/// match grows as more hats land (Domain Modeler, Tech Lead, QA, ...).
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
        _other => Err(anyhow!(
            "no agent implemented for role {:?} (only Solution Architect so far)",
            task.role
        )),
    }
}

fn read(path: PathBuf) -> Result<String> {
    std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))
}
