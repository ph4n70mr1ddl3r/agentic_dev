# `platform-spec/` — Metadata JSON schemas (the contract)

*(Phase 2)* This holds the JSON schemas that define the model-driven contract:
`entity`, `field`, `form`, `list`, `workflow`, `rule`, `permission`, `number-sequence`,
`posting-profile`, plus the JSON-logic subschema and the curated action vocabulary.

Everything downstream — engines (Phase 3) and module metadata (Phase 4) — is
validated against these schemas. **No `MOD` artifact may be authored before its
schema here is `accepted`.** See [ADR-0001](../docs/adr/0001-model-driven-architecture.md)
and [ADR-0003](../docs/adr/0003-expression-and-action-language.md).
