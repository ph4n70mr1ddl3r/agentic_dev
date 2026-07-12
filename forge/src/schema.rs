//! The platform-spec schema registry: loads every `*.schema.json`, feeds them
//! to agents as the contract, and compiles JSON-Schema validators that resolve
//! cross-file `$ref`s locally (by `$id`) instead of fetching over HTTP.
//!
//! This is the mechanical-review half of the agent loop (ADR-0005): an agent's
//! artifact is validated against the schema here before it is accepted.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use jsonschema::{Retrieve, Uri, Validator};
use serde_json::Value;

pub struct SchemaRegistry {
    /// `$id` -> raw file text (fed to agents as the contract).
    texts: HashMap<String, String>,
    /// `$id` -> parsed schema (used to compile validators).
    values: Arc<HashMap<String, Value>>,
}

impl SchemaRegistry {
    /// Load every `*.schema.json` under `dir`, indexed by its `$id`.
    pub fn load_dir(dir: &Path) -> Result<Self> {
        let mut texts: HashMap<String, String> = HashMap::new();
        let mut values: HashMap<String, Value> = HashMap::new();

        let entries =
            std::fs::read_dir(dir).with_context(|| format!("read schema dir {}", dir.display()))?;
        for entry in entries {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            let value: Value =
                serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
            let id = value
                .get("$id")
                .and_then(|i| i.as_str())
                .ok_or_else(|| anyhow!("{} has no $id", path.display()))?
                .to_string();
            texts.insert(id.clone(), text);
            values.insert(id, value);
        }
        tracing::info!(schemas = values.len(), dir = %dir.display(), "loaded schema registry");
        Ok(Self {
            texts,
            values: Arc::new(values),
        })
    }

    /// Raw file text for the schema with this `$id` (to embed in an agent prompt).
    pub fn schema_text(&self, id: &str) -> Result<&str> {
        self.texts
            .get(id)
            .map(|s| s.as_str())
            .ok_or_else(|| anyhow!("no schema with $id {id}"))
    }

    /// Compile a validator for the schema with this `$id`.
    pub fn validator(&self, id: &str) -> Result<Validator> {
        let root = self
            .values
            .get(id)
            .ok_or_else(|| anyhow!("no schema with $id {id}"))?;
        let retriever = IdRetriever {
            schemas: self.values.clone(),
        };
        jsonschema::options()
            .with_retriever(retriever)
            .build(root)
            .map_err(|e| anyhow!("compile schema {id}: {e}"))
    }
}

/// Resolves a `$ref` `$id` to a parsed schema in the registry.
#[derive(Clone)]
struct IdRetriever {
    schemas: Arc<HashMap<String, Value>>,
}

impl Retrieve for IdRetriever {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> std::result::Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let s = uri.as_str();
        let doc = s.split('#').next().unwrap_or(s);
        if let Some(v) = self.schemas.get(doc) {
            return Ok(v.clone());
        }
        // Fallback: match by filename, in case the resolver normalizes the scheme.
        if let Some(name) = doc.rsplit('/').next() {
            if let Some((_, v)) = self
                .schemas
                .iter()
                .find(|(k, _)| k.rsplit('/').next() == Some(name))
            {
                return Ok(v.clone());
            }
        }
        Err(format!("schema not found in registry: {doc}").into())
    }
}

/// All validation errors (up to 8) as a multi-line string; empty when valid.
/// This string is fed back to the agent on retry (ADR-0005 reviewer loop).
pub fn collect_errors(validator: &Validator, instance: &Value) -> String {
    let mut errs: Vec<String> = Vec::new();
    for e in validator.iter_errors(instance).take(8) {
        errs.push(format!("  - at {}: {}", e.instance_path(), e));
    }
    errs.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Loads the real platform-spec schemas from the erp product repo (located
    /// one level up from the forge crate). Skips if absent (e.g. published).
    fn real_registry() -> Option<SchemaRegistry> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../erp/platform-spec/schemas");
        if dir.is_dir() {
            SchemaRegistry::load_dir(&dir).ok()
        } else {
            None
        }
    }

    #[test]
    fn loads_all_schemas() {
        let reg = match real_registry() {
            Some(r) => r,
            None => {
                eprintln!("skipping: erp/platform-spec not found");
                return;
            }
        };
        // every named schema is present and addressable
        for id in [
            "_common.schema.json",
            "entity.schema.json",
            "field.schema.json",
            "workflow.schema.json",
            "action.schema.json",
            "json-logic.schema.json",
        ] {
            let full = format!("https://agentic.dev/platform-spec/schemas/{id}");
            assert!(reg.schema_text(&full).is_ok(), "missing {id}");
        }
    }

    #[test]
    fn entity_validator_accepts_good_and_rejects_bad() {
        let reg = match real_registry() {
            Some(r) => r,
            None => {
                eprintln!("skipping: erp/platform-spec not found");
                return;
            }
        };
        let v = reg
            .validator("https://agentic.dev/platform-spec/schemas/entity.schema.json")
            .expect("compile entity");

        let good = json!({
            "id": "company", "name": "Company", "label": {"en": "Company"},
            "scope": "master", "companyScoped": false,
            "fields": [{"name": "id", "type": "guid", "label": {"en": "ID"}}]
        });
        assert!(collect_errors(&v, &good).is_empty(), "good entity rejected");

        let bad = json!({
            "id": "je", "name": "JournalEntry", "label": {"en": "JE"},
            "scope": "transactional", "companyScoped": true,
            "fields": [{"name": "id", "type": "guid", "label": {"en": "ID"}}]
        });
        let errs = collect_errors(&v, &bad);
        assert!(!errs.is_empty(), "bad entity (no companyId) should fail");
        // the missing-companyId rule is an allOf if/then `contains` over fields
        assert!(
            errs.contains("/fields"),
            "failure should point at fields: {errs}"
        );
    }
}
