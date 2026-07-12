//! The Solution Architect hat. Author of model-driven entity metadata.
//!
//! Given a task, the entity + field schemas (the contract), and a template
//! example, it emits ONE entity. The artifact is validated against
//! [`ENTITY_SCHEMA_ID`] using the in-process schema registry; on failure the
//! specific errors are fed back and the hat retries (ADR-0005 reviewer loop).

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::llm::Llm;
use crate::plan::Task;
use crate::schema::SchemaRegistry;

/// `$id` of the schema every entity artifact must satisfy.
pub const ENTITY_SCHEMA_ID: &str = "https://agentic.dev/platform-spec/schemas/entity.schema.json";
/// `$id` of the field schema (for `fields[]` items).
pub const FIELD_SCHEMA_ID: &str = "https://agentic.dev/platform-spec/schemas/field.schema.json";

pub const ARCHITECT_SYSTEM: &str = r#"You are the SOLUTION ARCHITECT hat of a virtual software company building a model-driven cloud ERP modeled on Microsoft Dynamics 365.

Your job: author ONE business entity as metadata JSON. You are given the ENTITY JSON SCHEMA (the contract you MUST satisfy), the FIELD SCHEMA (for items in the fields array), and a TEMPLATE EXAMPLE to mirror. Fill the template — do not invent new structure, and emit no markdown or commentary.

Immutable rules (these are ADRs; honor them):
- Every entity has a guid primary key field named "id".
- A transactional, companyScoped entity MUST include a "companyId" field of type "lookup" with lookup.target "Company".
- Field names are camelCase; the entity "name" is PascalCase; the entity "id" is kebab-case.
- picklist fields need options; lookup fields need target; decimal/money fields need precision and scale.
- Prefer typed, indexed columns (the default). Use a type "json" field with promoted=false (extras JSONB) only for genuinely freeform extension data.

Output ONLY the entity JSON object — nothing before or after."#;

/// Run the Architect hat on `task`: return a schema-valid entity, or an error
/// after a few attempts. The reviewer is the JSON Schema itself.
pub async fn run_architect(
    llm: &Llm,
    registry: &SchemaRegistry,
    task: &Task,
    entity_schema_text: &str,
    field_schema_text: &str,
    example_text: &str,
) -> Result<Value> {
    let validator = registry.validator(ENTITY_SCHEMA_ID)?;

    let base_prompt = format!(
        "TASK {id}: {title}\n(type: {ty}, role: {role})\n\n\
         {desc}\n\n\
         ================ ENTITY SCHEMA (the contract) ================\n{entity}\n\n\
         ================ FIELD SCHEMA (fields[] items) ================\n{field}\n\n\
         ================ TEMPLATE EXAMPLE (mirror this shape) ================\n{example}\n\n\
         =================================================================\n\
         Author ONE entity that fulfills the task. A strong default is the \"Company\"\n\
         master entity (every other entity references it via companyId, and it exercises\n\
         the master scope path) — unless the task plainly asks for something else.\n\
         Return ONLY the entity JSON.",
        id = task.id,
        title = task.title.trim(),
        ty = task.task_type,
        role = task.role,
        desc = task.description.trim(),
        entity = entity_schema_text,
        field = field_schema_text,
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
                 Return ONLY a corrected entity JSON object that passes the schema.",
                base = base_prompt,
            ),
        };
        let raw = llm.chat_json(ARCHITECT_SYSTEM, &prompt).await?;
        let json = super::extract_json(&raw);
        let value: Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(e) => {
                let preview: String = raw.chars().take(400).collect();
                let msg = format!("invalid JSON ({e}). Preview:\n{preview}");
                tracing::warn!(attempt, "architect returned invalid JSON; retrying");
                last_err = Some(msg);
                continue;
            }
        };
        let errs = crate::schema::collect_errors(&validator, &value);
        if errs.is_empty() {
            let id = value.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            tracing::info!(attempt, entity_id = %id, "architect entity passed schema validation");
            return Ok(value);
        }
        tracing::warn!(
            attempt,
            "architect entity failed schema validation; retrying"
        );
        last_err = Some(errs);
    }
    Err(anyhow!(
        "architect failed to produce a schema-valid entity after {MAX_ATTEMPTS} attempts:\n{}",
        last_err.unwrap_or_else(|| "unknown error".into())
    ))
}
