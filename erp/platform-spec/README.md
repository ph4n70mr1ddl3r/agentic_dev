# `platform-spec/` — Metadata JSON schemas (the contract)

The JSON Schema (draft 2020-12) definitions that constitute the **model-driven
contract** of the ERP. Every downstream artifact — engine behavior (Phase 3) and
business-module metadata (Phase 4+) — is validated against these schemas before
merge. **No `MOD` artifact may be authored before its schema here is `accepted`.**
See [ADR-0001](../docs/adr/0001-model-driven-architecture.md) and
[ADR-0003](../docs/adr/0003-expression-and-action-language.md).

> **Phase:** 1 (T3 + T4 — Solution Architect owns entity/field/form/list;
> Tech Lead owns workflow/rule/json-logic/action).

## Schemas

| Schema | Owner hat | Describes |
|---|---|---|
| [`_common`](schemas/_common.schema.json) | Solution Architect | Shared primitives: localized text, identifier patterns |
| [`entity`](schemas/entity.schema.json) | Solution Architect | A business entity → Postgres `ent_<id>` (ADR-0002); enforces `id` PK +, for transactional/company-scoped entities, a `companyId` lookup (ADR-0004) |
| [`field`](schemas/field.schema.json) | Solution Architect | A typed field; `promoted` = column vs `extras` JSONB; picklist/lookup/decimal constraints (ADR-0002) |
| [`relationship`](schemas/relationship.schema.json) | Solution Architect | Explicit entity relationships (incl. many-to-many) |
| [`form`](schemas/form.schema.json) | Frontend Developer | A form layout rendered by the generic shell |
| [`list`](schemas/list.schema.json) | Frontend Developer | A list/grid view |
| [`workflow`](schemas/workflow.schema.json) | Tech Lead | A state machine: states + transitions with JSON-logic guards and curated actions |
| [`rule`](schemas/rule.schema.json) | Tech Lead | An event-triggered rule (condition → actions) |
| [`json-logic`](schemas/json-logic.schema.json) | Tech Lead | The deterministic JSON-logic subset used for conditions/formulas (ADR-0003) |
| [`action`](schemas/action.schema.json) | Tech Lead | The curated action vocabulary (ADR-0003) |
| [`domain-reference`](schemas/domain-reference.schema.json) | Domain Modeler | A structured D365 area digest (entities/processes/rules) — the curated KB (ADR-0005); `$kind`-tagged, rendered to markdown |
| [`test-plan`](schemas/test-plan.schema.json) | QA Engineer | Conformance assertions (sample + expect valid/invalid) over a schema; executed by `forge check` — the QA gate (ADR-0005) |

Each schema declares a stable `$id` under `https://agentic.dev/platform-spec/schemas/`
and cross-references siblings by that `$id`. The loader (see
[`../tools/`](../tools/)) registers them all, then validates module artifacts and
performs **referential-integrity** checks JSON Schema can't express:

- every `lookup.target` / relationship `from`/`to` / `entity` reference resolves
  to a declared entity;
- every workflow `initialState` and transition `from`/`to` exists in `states`;
- every `field` referenced by a form/list/section exists on its entity;
- every `action` is in the curated vocabulary (enforced by the schema).

## Examples

[`examples/`](examples/) holds one valid instance per artifact type, built around
a single `JournalEntry` running example (financials), so the schemas are shown to
compose end-to-end:

- [`entity.json`](examples/entity.json) — a transactional, company-scoped entity
- [`workflow.json`](examples/workflow.json) — Draft → UnderReview → Posted, with
  a JSON-logic guard and a `post-to-ledger` action
- [`rule.json`](examples/rule.json) — a `before-create` rule that defaults `status`
- [`form.json`](examples/form.json), [`list.json`](examples/list.json)

## Validate locally

The canonical gate is the `metadata-loader` ([`../tools/`](../tools/)), which
loads the whole registry by `$id` and adds the referential-integrity checks
listed above. For a quick structural check, any **draft 2020-12** JSON Schema
validator works (e.g. ajv 8 via `require('ajv/dist/2020')`, registering every
schema before validating). All examples — plus a suite of negatives (transactional
entity missing `companyId`, unknown action verb, unknown JSON-logic operator,
picklist without options, …) — were verified this way.
