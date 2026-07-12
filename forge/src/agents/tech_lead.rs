//! The Tech Lead hat. Author of model-driven workflow metadata.
//!
//! Mirrors the Architect hat: given a task, the workflow + JSON-logic + action
//! schemas (the contract), and a template example, it emits ONE workflow. The
//! artifact is validated against [`WORKFLOW_SCHEMA_ID`] in-process; on failure
//! the errors are fed back and the hat retries (ADR-0005 reviewer loop).

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::llm::Llm;
use crate::plan::Task;
use crate::schema::SchemaRegistry;

pub const WORKFLOW_SCHEMA_ID: &str =
    "https://agentic.dev/platform-spec/schemas/workflow.schema.json";
pub const JSON_LOGIC_SCHEMA_ID: &str =
    "https://agentic.dev/platform-spec/schemas/json-logic.schema.json";
pub const ACTION_SCHEMA_ID: &str = "https://agentic.dev/platform-spec/schemas/action.schema.json";

pub const TECH_LEAD_SYSTEM: &str = r#"You are the TECH LEAD hat of a virtual software company building a model-driven cloud ERP modeled on Microsoft Dynamics 365.

Your job: author ONE workflow as metadata JSON. You are given the WORKFLOW JSON SCHEMA (the contract you MUST satisfy), the JSON-LOGIC schema (for transition guards), the ACTION vocabulary (for transition/state actions), and a TEMPLATE EXAMPLE to mirror. Fill the template — do not invent structure, and emit no markdown or commentary.

Immutable rules (ADRs):
- A workflow has "states" (at least one), an "initialState" that names one of them, and "transitions".
- Each transition has "from" (array of state names that exist in states), "to" (a state that exists in states), an optional "guard" (a JSON-logic expression), and optional "actions".
- Actions MUST come from the curated vocabulary (set-field, transition-workflow, post-to-ledger, send-notification, ...). NEVER invent action verbs.
- Guards are JSON-logic over the record, e.g. {"==":[{"var":"status"},"Active"]}.
- The loader checks referential integrity (initialState / transition targets exist in states); keep it correct anyway.

Output ONLY the workflow JSON object — nothing before or after."#;

pub async fn run_tech_lead(
    llm: &Llm,
    registry: &SchemaRegistry,
    task: &Task,
    workflow_schema_text: &str,
    json_logic_schema_text: &str,
    action_schema_text: &str,
    example_text: &str,
) -> Result<Value> {
    let validator = registry.validator(WORKFLOW_SCHEMA_ID)?;

    let base_prompt = format!(
        "TASK {id}: {title}\n(type: {ty}, role: {role})\n\n\
         {desc}\n\n\
         ================ WORKFLOW SCHEMA (the contract) ================\n{wf}\n\n\
         ================ JSON-LOGIC SCHEMA (for guards) ================\n{jl}\n\n\
         ================ ACTION VOCABULARY (for actions) ================\n{ac}\n\n\
         ================ TEMPLATE EXAMPLE (mirror this shape) ================\n{example}\n\n\
         =================================================================\n\
         Author ONE workflow that fulfills the task. A good first workflow is a lifecycle\n\
         for an entity that already exists, e.g. the Company master entity (states like\n\
         Prospect -> Active -> Inactive), unless the task plainly asks for something else.\n\
         Return ONLY the workflow JSON.",
        id = task.id,
        title = task.title.trim(),
        ty = task.task_type,
        role = task.role,
        desc = task.description.trim(),
        wf = workflow_schema_text,
        jl = json_logic_schema_text,
        ac = action_schema_text,
        example = example_text,
    );

    const MAX_ATTEMPTS: u32 = 3;
    let mut last_err: Option<String> = None;
    for attempt in 0..MAX_ATTEMPTS {
        let prompt = match &last_err {
            None => base_prompt.clone(),
            Some(err) => format!(
                "{base}\n\n\
                 Your previous attempt FAILED schema validation:\n{err}\n\n\
                 Return ONLY a corrected workflow JSON object that passes the schema.",
                base = base_prompt,
            ),
        };
        let raw = llm.chat_json(TECH_LEAD_SYSTEM, &prompt).await?;
        let json = super::extract_json(&raw);
        let value: Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(e) => {
                let preview: String = raw.chars().take(400).collect();
                let msg = format!("invalid JSON ({e}). Preview:\n{preview}");
                tracing::warn!(attempt, "tech-lead returned invalid JSON; retrying");
                last_err = Some(msg);
                continue;
            }
        };
        let errs = crate::schema::collect_errors(&validator, &value);
        if errs.is_empty() {
            let id = value.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            tracing::info!(attempt, workflow_id = %id, "tech-lead workflow passed schema validation");
            return Ok(value);
        }
        tracing::warn!(
            attempt,
            "tech-lead workflow failed schema validation; retrying"
        );
        last_err = Some(errs);
    }
    Err(anyhow!(
        "tech-lead failed to produce a schema-valid workflow after {MAX_ATTEMPTS} attempts:\n{}",
        last_err.unwrap_or_else(|| "unknown error".into())
    ))
}
