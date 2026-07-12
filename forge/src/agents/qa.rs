//! The QA Engineer hat. Author of a schema-validated **test plan** — conformance
//! assertions proving a platform-spec schema can represent the concepts in a
//! domain reference (the QA gate of ADR-0005). The harness then *executes* the
//! plan via [`check_plan`], closing the loop: the QA hat writes tests and the
//! harness immediately runs them.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use jsonschema::Validator;
use serde_json::Value;

use crate::llm::Llm;
use crate::plan::Task;
use crate::schema::SchemaRegistry;

pub const TEST_PLAN_SCHEMA_ID: &str =
    "https://agentic.dev/platform-spec/schemas/test-plan.schema.json";

pub const QA_SYSTEM: &str = r#"You are the QA ENGINEER hat of a virtual software company building a model-driven cloud ERP modeled on Microsoft Dynamics 365.

Your job: author ONE "test plan" — a set of conformance assertions that prove a given schema can represent every concept in a given domain reference digest. You are given the TEST-PLAN SCHEMA (the contract you MUST satisfy), a TEMPLATE EXAMPLE, the SCHEMA UNDER TEST (the contract whose coverage you are checking), and the DOMAIN REFERENCE (the concepts to cover). Emit no markdown or commentary.

For each entity/concept in the reference, write ONE assertion with expect "valid" and a sample instance that conforms to the schema under test and faithfully represents that concept. Then add 1-3 NEGATIVE assertions (expect "invalid") for the schema's most important constraints (e.g. a transactional company-scoped entity missing companyId).

Common mistakes to AVOID in your samples (the schema will reject these):
- picklist fields MUST include a non-empty "picklist.options" array (each {value,label}).
- decimal/money fields MUST include "precision" and "scale".
- lookup fields MUST include "lookup.target".
- transactional companyScoped entities MUST include a "companyId" lookup field (target "Company").
- every entity MUST have a guid field named "id".

Hard requirements:
- "$kind" MUST be "test-plan".
- Every assertion's "schema" MUST be the $id of the schema under test (given to you).
- "sample" for a "valid" assertion MUST actually satisfy the schema; for "invalid" it MUST violate it in the way the rationale states.
- Assertion "name" is kebab-case; plan "name" is PascalCase; plan "id" is kebab-case.

Output ONLY the test-plan JSON object — nothing before or after."#;

/// Infer the schema under test from the task description ("entity" | "workflow").
pub fn infer_schema_under_test(desc: &str) -> Result<&'static str> {
    let d = desc.to_ascii_lowercase();
    if d.contains("workflow") {
        Ok(crate::agents::tech_lead::WORKFLOW_SCHEMA_ID)
    } else if d.contains("entity") {
        Ok(crate::agents::architect::ENTITY_SCHEMA_ID)
    } else {
        anyhow::bail!(
            "cannot infer schema under test from task description (need 'entity' or 'workflow')"
        )
    }
}

/// Infer the reference digest filename (in out_dir) from the task description.
pub fn infer_reference_file(desc: &str) -> Result<&'static str> {
    let d = desc.to_ascii_lowercase();
    if d.contains("financial") {
        Ok("d365-financials-reference.json")
    } else if d.contains("supply") {
        Ok("d365-supply-chain-reference.json")
    } else {
        anyhow::bail!(
            "cannot infer reference digest from task description (need 'financial' or 'supply')"
        )
    }
}

/// Schemas referenced by the schema-under-test, whose rules the QA hat's samples
/// must also satisfy (entity -> field; workflow -> json-logic + action). The
/// dispatch folds these into the schema-under-test text shown to the hat.
pub fn supporting_schema_ids(schema_under_test: &str) -> Vec<&'static str> {
    const FIELD: &str = "https://agentic.dev/platform-spec/schemas/field.schema.json";
    const JSON_LOGIC: &str = "https://agentic.dev/platform-spec/schemas/json-logic.schema.json";
    const ACTION: &str = "https://agentic.dev/platform-spec/schemas/action.schema.json";
    match schema_under_test {
        crate::agents::architect::ENTITY_SCHEMA_ID => vec![FIELD],
        crate::agents::tech_lead::WORKFLOW_SCHEMA_ID => vec![JSON_LOGIC, ACTION],
        _ => vec![],
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run_qa(
    llm: &Llm,
    registry: &SchemaRegistry,
    task: &Task,
    test_plan_schema_text: &str,
    example_text: &str,
    schema_under_test_id: &str,
    schema_under_test_text: &str,
    reference_text: &str,
) -> Result<Value> {
    let validator = registry.validator(TEST_PLAN_SCHEMA_ID)?;

    let base_prompt = format!(
        "TASK {id}: {title}\n(type: {ty}, role: {role})\n\n\
         {desc}\n\n\
         SCHEMA UNDER TEST ($id {sid}):\n{sut}\n\n\
         DOMAIN REFERENCE (cover its concepts):\n{ref}\n\n\
         ================ TEST-PLAN SCHEMA (the contract) ================\n{tps}\n\n\
         ================ TEMPLATE EXAMPLE (mirror this shape) ================\n{ex}\n\n\
         =================================================================\n\
         Author ONE test plan that covers every concept in the reference with a\n\
         \"valid\" assertion, plus 1-3 \"invalid\" assertions for the schema's key\n\
         constraints. Set \"$kind\" to \"test-plan\" and every assertion's \"schema\" to\n\
         \"{sid}\". Return ONLY the JSON.",
        id = task.id,
        title = task.title.trim(),
        ty = task.task_type,
        role = task.role,
        desc = task.description.trim(),
        sid = schema_under_test_id,
        sut = schema_under_test_text,
        r#ref = reference_text,
        tps = test_plan_schema_text,
        ex = example_text,
    );

    const MAX_ATTEMPTS: u32 = 6;
    let mut last_err: Option<String> = None;
    for attempt in 0..MAX_ATTEMPTS {
        let prompt = match &last_err {
            None => base_prompt.clone(),
            Some(err) => format!(
                "{base}\n\n\
                 Your previous attempt FAILED schema validation:\n{err}\n\n\
                 Return ONLY a corrected test-plan JSON object that passes the schema.",
                base = base_prompt,
            ),
        };
        let raw = llm.chat_json(QA_SYSTEM, &prompt).await?;
        let json = super::extract_json(&raw);
        let value: Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(e) => {
                let preview: String = raw.chars().take(400).collect();
                let msg = format!("invalid JSON ({e}). Preview:\n{preview}");
                tracing::warn!(attempt, "qa returned invalid JSON; retrying");
                last_err = Some(msg);
                continue;
            }
        };
        // Gate 1: structural — the plan must satisfy the test-plan schema.
        let structural = crate::schema::collect_errors(&validator, &value);
        if !structural.is_empty() {
            tracing::warn!(attempt, "test plan failed structural validation; retrying");
            last_err = Some(structural);
            continue;
        }
        // Gate 2: semantic — execute the plan. Every assertion must actually hold,
        // or the plan is defective (e.g. a 'valid' sample that violates the
        // schema-under-test). If a concept truly can't be represented, retries
        // exhaust and surface a real schema gap.
        let report = match check_plan(registry, &value) {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("could not execute plan: {e}");
                tracing::warn!(attempt, "test plan execution error; retrying");
                last_err = Some(msg);
                continue;
            }
        };
        if report.all_passed() {
            tracing::info!(
                attempt,
                assertions = report.total,
                "test plan passed schema + semantic validation"
            );
            return Ok(value);
        }
        let mut msg = format!(
            "{} of {} assertions did not hold:\n",
            report.failed.len(),
            report.total
        );
        for f in &report.failed {
            let first = f.errors.lines().next().unwrap_or("(no detail)");
            msg.push_str(&format!(
                "  - {} (expected {}, was {}): {}\n",
                f.name, f.expected, f.actual, first
            ));
        }
        tracing::warn!(
            attempt,
            failed = report.failed.len(),
            "test plan assertions did not hold; retrying"
        );
        last_err = Some(msg);
    }
    Err(anyhow!(
        "qa failed to produce a schema-valid test plan after {MAX_ATTEMPTS} attempts:\n{}",
        last_err.unwrap_or_else(|| "unknown error".into())
    ))
}

#[derive(Debug)]
pub struct FailedAssertion {
    pub name: String,
    pub expected: String,
    pub actual: String,
    pub errors: String,
}

#[derive(Debug)]
pub struct CheckReport {
    pub total: usize,
    pub passed: usize,
    pub failed: Vec<FailedAssertion>,
}

impl CheckReport {
    pub fn all_passed(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Execute a test plan: validate each assertion's sample against its schema and
/// compare to the expected outcome. Validators are compiled once per $id.
pub fn check_plan(registry: &SchemaRegistry, plan: &Value) -> Result<CheckReport> {
    let assertions = plan
        .get("assertions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("test plan has no assertions array"))?;

    let mut validators: HashMap<String, Validator> = HashMap::new();
    let mut failed = Vec::new();
    let mut passed = 0usize;

    for a in assertions {
        let name = a
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        let schema_id = a
            .get("schema")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("assertion {name:?} has no schema"))?;
        let expect = a.get("expect").and_then(|v| v.as_str()).unwrap_or("valid");
        let sample = a.get("sample").unwrap_or(&Value::Null);

        if !validators.contains_key(schema_id) {
            validators.insert(schema_id.to_string(), registry.validator(schema_id)?);
        }
        let validator = validators.get(schema_id).expect("just-inserted");

        let errs = crate::schema::collect_errors(validator, sample);
        let actual = if errs.is_empty() { "valid" } else { "invalid" };
        if actual == expect {
            passed += 1;
        } else {
            failed.push(FailedAssertion {
                name,
                expected: expect.to_string(),
                actual: actual.to_string(),
                errors: errs,
            });
        }
    }

    Ok(CheckReport {
        total: assertions.len(),
        passed,
        failed,
    })
}
