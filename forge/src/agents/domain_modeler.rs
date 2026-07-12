//! The Domain Modeler hat. Author of a structured, schema-validated **domain
//! reference** digest for one business area (Financials or Supply Chain) — the
//! curated reference KB that is ground truth for downstream domain agents
//! (ADR-0005). Output is JSON validated against [`DOMAIN_REFERENCE_SCHEMA_ID`],
//! and rendered to markdown for humans.
//!
//! Mirrors the Architect/Tech Lead loop: LLM → JSON → schema-validate →
//! retry-on-rejection with the errors fed back.

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::llm::Llm;
use crate::plan::Task;
use crate::schema::SchemaRegistry;

pub const DOMAIN_REFERENCE_SCHEMA_ID: &str =
    "https://agentic.dev/platform-spec/schemas/domain-reference.schema.json";

pub const DOMAIN_MODELER_SYSTEM: &str = r#"You are the DOMAIN MODELER hat for one business area of a model-driven cloud ERP modeled on Microsoft Dynamics 365.

Your job: author ONE "domain reference" — a structured JSON digest of the area's core domain knowledge. You are given the DOMAIN-REFERENCE SCHEMA (the contract you MUST satisfy) and a TEMPLATE EXAMPLE to mirror. Fill the template — do not invent new structure, and emit no markdown or commentary.

What to capture:
- The core business ENTITIES (their key fields and how they relate), categorized as transactional / master / reference.
- The main business PROCESSES (ordered steps, which entities they touch, triggers).
- The important business RULES / constraints (what they enforce, severity).

Reconstruct from your knowledge of Dynamics 365; be concrete and correct. Set "source" to note that the digest is model-reconstructed from D365 public documentation.

Hard requirements:
- "$kind" MUST be "domain-reference".
- "area" MUST be the area given in the task (one of: financials, supply-chain).
- Field names are camelCase; entity/concept "name" is PascalCase; the digest "id" is kebab-case.

Output ONLY the domain-reference JSON object — nothing before or after."#;

/// Infer the area from the task role, e.g. "Domain Modeler (Financials)".
/// Returns Err if the role isn't a recognized domain-modeler specialization.
pub fn area_for_role(role: &str) -> Result<&'static str> {
    let r = role.trim().to_ascii_lowercase();
    if !r.starts_with("domain modeler") {
        anyhow::bail!("not a domain modeler role: {role:?}");
    }
    if r.contains("financial") {
        Ok("financials")
    } else if r.contains("supply") {
        Ok("supply-chain")
    } else {
        anyhow::bail!("cannot infer area (financials/supply-chain) from role {role:?}")
    }
}

pub async fn run_domain_modeler(
    llm: &Llm,
    registry: &SchemaRegistry,
    task: &Task,
    schema_text: &str,
    example_text: &str,
    area: &str,
) -> Result<Value> {
    let validator = registry.validator(DOMAIN_REFERENCE_SCHEMA_ID)?;

    let base_prompt = format!(
        "TASK {id}: {title}\n(type: {ty}, role: {role})\n\n\
         {desc}\n\n\
         TARGET AREA: {area}\n\n\
         ================ DOMAIN-REFERENCE SCHEMA (the contract) ================\n{schema}\n\n\
         ================ TEMPLATE EXAMPLE (mirror this shape) ================\n{example}\n\n\
         =================================================================\n\
         Author ONE domain reference for D365 **{area}** that fulfills the task. Cover the\n\
         core entities (with key fields + relationships), the main processes, and the\n\
         important rules. Set \"$kind\" to \"domain-reference\" and \"area\" to \"{area}\".\n\
         Return ONLY the JSON.",
        id = task.id,
        title = task.title.trim(),
        ty = task.task_type,
        role = task.role,
        desc = task.description.trim(),
        area = area,
        schema = schema_text,
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
                 Return ONLY a corrected domain-reference JSON object that passes the schema.",
                base = base_prompt,
            ),
        };
        let raw = llm.chat_json(DOMAIN_MODELER_SYSTEM, &prompt).await?;
        let json = super::extract_json(&raw);
        let value: Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(e) => {
                let preview: String = raw.chars().take(400).collect();
                let msg = format!("invalid JSON ({e}). Preview:\n{preview}");
                tracing::warn!(attempt, "domain-modeler returned invalid JSON; retrying");
                last_err = Some(msg);
                continue;
            }
        };
        let errs = crate::schema::collect_errors(&validator, &value);
        if errs.is_empty() {
            let id = value.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let n = value
                .get("entities")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            tracing::info!(attempt, ref_id = %id, entities = n, "domain reference passed schema validation");
            return Ok(value);
        }
        tracing::warn!(
            attempt,
            "domain reference failed schema validation; retrying"
        );
        last_err = Some(errs);
    }
    Err(anyhow!(
        "domain-modeler failed to produce a schema-valid reference after {MAX_ATTEMPTS} attempts:\n{}",
        last_err.unwrap_or_else(|| "unknown error".into())
    ))
}

/// Render a validated domain reference to human-readable markdown.
pub fn render_markdown(value: &Value) -> String {
    let s = |p: &str| value.get(p).and_then(|v| v.as_str()).unwrap_or("").trim();
    let label = localized(value.get("label"));
    let area = s("area");
    let source = s("source");
    let mut out = String::new();
    out.push_str(&format!("# {label}\n\n"));
    out.push_str("_Authored by the **Domain Modeler** hat via `forge`; schema-validated._\n\n");
    if !area.is_empty() || !source.is_empty() {
        out.push_str(&format!(
            "**Area:** {area}  \n**Source:** {source}\n\n",
            area = if area.is_empty() { "—" } else { area },
            source = if source.is_empty() { "—" } else { source },
        ));
    }
    let desc = s("description");
    if !desc.is_empty() {
        out.push_str(&format!("{desc}\n\n"));
    }

    if let Some(entities) = value.get("entities").and_then(|v| v.as_array()) {
        out.push_str("## Entities\n\n");
        for e in entities {
            let name = e.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let elabel = localized(e.get("label"));
            let cat = e.get("category").and_then(|v| v.as_str()).unwrap_or("?");
            out.push_str(&format!("### {name} — {elabel} _({cat})_\n\n"));
            if let Some(d) = e.get("description").and_then(|v| v.as_str()) {
                if !d.trim().is_empty() {
                    out.push_str(&format!("{d}\n\n"));
                }
            }
            if let Some(fields) = e.get("fields").and_then(|v| v.as_array()) {
                out.push_str("| Field | Type | Required | Description |\n|---|---|---|---|\n");
                for f in fields {
                    let fname = f.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let ftype = f.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let req = if f.get("required").and_then(|v| v.as_bool()).unwrap_or(false) {
                        "yes"
                    } else {
                        ""
                    };
                    let fdesc = f.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    out.push_str(&format!(
                        "| `{fname}` | {ftype} | {req} | {} |\n",
                        fdesc.replace('|', "\\|")
                    ));
                }
                out.push('\n');
            }
            if let Some(rels) = e.get("relationships").and_then(|v| v.as_array()) {
                if !rels.is_empty() {
                    out.push_str("**Relationships:** ");
                    let bits: Vec<String> = rels
                        .iter()
                        .map(|r| {
                            format!(
                                "`{}` → {} ({})",
                                r.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                                r.get("target").and_then(|v| v.as_str()).unwrap_or(""),
                                r.get("cardinality").and_then(|v| v.as_str()).unwrap_or("")
                            )
                        })
                        .collect();
                    out.push_str(&bits.join("; "));
                    out.push_str("\n\n");
                }
            }
        }
    }

    if let Some(procs) = value.get("processes").and_then(|v| v.as_array()) {
        out.push_str("## Processes\n\n");
        for p in procs {
            let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let plabel = localized(p.get("label"));
            out.push_str(&format!("### {name} — {plabel}\n\n"));
            if let Some(d) = p.get("description").and_then(|v| v.as_str()) {
                if !d.trim().is_empty() {
                    out.push_str(&format!("{d}\n\n"));
                }
            }
            if let Some(steps) = p.get("steps").and_then(|v| v.as_array()) {
                for (i, st) in steps.iter().enumerate() {
                    if let Some(t) = st.as_str() {
                        out.push_str(&format!("{}. {t}\n", i + 1));
                    }
                }
                out.push('\n');
            }
        }
    }

    if let Some(rules) = value.get("rules").and_then(|v| v.as_array()) {
        out.push_str("## Rules\n\n");
        for r in rules {
            let name = r.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let sev = r.get("severity").and_then(|v| v.as_str()).unwrap_or("");
            let enf = r.get("enforced").and_then(|v| v.as_str()).unwrap_or("");
            let rdesc = r.get("description").and_then(|v| v.as_str()).unwrap_or("");
            out.push_str(&format!(
                "- **{name}** _({sev}, {enf})_ — {rdesc}\n",
                sev = if sev.is_empty() { "—" } else { sev },
                enf = if enf.is_empty() { "—" } else { enf },
            ));
        }
    }

    out
}

/// Pick an English label from a localizedText object, falling back to any value.
fn localized(v: Option<&Value>) -> String {
    let Some(obj) = v.and_then(|v| v.as_object()) else {
        return String::new();
    };
    if let Some(en) = obj.get("en").and_then(|v| v.as_str()) {
        return en.to_string();
    }
    obj.iter()
        .next()
        .and_then(|(_, v)| v.as_str())
        .unwrap_or("")
        .to_string()
}
