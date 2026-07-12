pub mod architect;
pub mod ceo;
pub mod domain_modeler;
pub mod qa;
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
    let r = role.trim().to_ascii_lowercase();
    matches!(
        r.as_str(),
        "solution architect" | "architect" | "tech lead" | "tech-lead" | "qa engineer" | "qa"
    ) || r.starts_with("domain modeler")
}

/// Everything a hat needs to do its job: the LLM, the schema registry (for
/// mechanical review + the contract text), and the examples dir (templates).
pub struct HatContext<'a> {
    pub llm: &'a Llm,
    pub registry: &'a SchemaRegistry,
    pub examples_dir: PathBuf,
    pub out_dir: PathBuf,
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
        dm if dm.starts_with("domain modeler") => {
            let area = domain_modeler::area_for_role(&task.role)?;
            let schema_text = ctx.registry.schema_text(domain_modeler::DOMAIN_REFERENCE_SCHEMA_ID)?;
            let example_text = read(ctx.examples_dir.join("domain-reference.json"))?;
            domain_modeler::run_domain_modeler(
                ctx.llm,
                ctx.registry,
                task,
                schema_text,
                &example_text,
                area,
            )
            .await
        }
        "qa engineer" | "qa" => {
            let tps = ctx.registry.schema_text(qa::TEST_PLAN_SCHEMA_ID)?;
            let example_text = read(ctx.examples_dir.join("test-plan.json"))?;
            let sut_id = qa::infer_schema_under_test(&task.description)?;
            // Fold the supporting schemas (field / json-logic / action) into the
            // schema-under-test text so the hat knows the rules its samples must
            // satisfy (e.g. picklist needs options, decimal needs precision).
            let mut sut_text = ctx.registry.schema_text(sut_id)?.to_string();
            for sid in qa::supporting_schema_ids(sut_id) {
                sut_text.push_str("\n\n---- SUPPORTING SCHEMA ----\n");
                sut_text.push_str(ctx.registry.schema_text(sid)?);
            }
            let reference_text = read(ctx.out_dir.join(qa::infer_reference_file(&task.description)?))?;
            qa::run_qa(
                ctx.llm,
                ctx.registry,
                task,
                tps,
                &example_text,
                sut_id,
                &sut_text,
                &reference_text,
            )
            .await
        }
        other => Err(anyhow!(
            "no agent implemented for role {:?} (hats so far: Solution Architect, Tech Lead, Domain Modeler)",
            other
        )),
    }
}

/// If an artifact has a human-readable companion (e.g. domain-reference →
/// markdown), return `(filename, content)`. Used by the runner to write a
/// sibling file alongside the JSON in interactive (non-PR) mode.
pub fn render_companion(value: &Value) -> Option<(String, String)> {
    match value.get("$kind").and_then(|v| v.as_str()) {
        Some("domain-reference") => {
            let id = value
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("reference");
            Some((format!("{id}.md"), domain_modeler::render_markdown(value)))
        }
        _ => None,
    }
}

fn read(path: PathBuf) -> Result<String> {
    std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))
}
