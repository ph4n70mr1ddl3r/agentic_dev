# ADR-0003 — Expression & action language: JSON-logic + curated action vocabulary

- **Status:** Accepted
- **Date:** 2026-07-12
- **Deciders:** Solution Architect, Tech Lead, Security Architect
- **Owner:** Tech Lead

## Context

Workflow conditions, business rules, field formulas, and workflow actions must
be authored by (weak) agents and executed by the engine. Requirements:

- **Safe & deterministic** — no arbitrary code execution anywhere.
- **Mechanically reviewable** — JSON-schema-validatable, so QA can fail bad
  output automatically (key weak-model mitigation).
- **Auditable** — every rule/action traceable for financial controls.

## Decision

- **Conditions & formulas:** [JSON-logic](https://jsonlogic.com/) (boolean,
  comparison, arithmetic, var access into the record/context). JSON-logic is a
  small, deterministic, well-specified standard.
- **Actions:** a **curated, versioned action vocabulary**, each action a JSON
  object with a typed argument schema. v1 actions:
  - `set-field`, `update-record`, `create-record`
  - `send-notification`
  - `call-service` (named service + operation, args validated by OpenAPI)
  - `assign-user`, `transition-workflow`, `start-subflow`
  - `schedule-job`, `run-report`, `post-to-ledger` (financial)
- The engine **interprets** these; it **never evaluates arbitrary code**. There
  is no `eval`, no inline JS, no shell-out from rules.
- New capabilities require **extending the vocabulary** — a governed change
  (ADR + schema update + tests).

## Consequences

**Positive**
- Deterministic, sandboxed, and reviewable; trivially JSON-schema-validatable.
- Auditable: the rule/action JSON *is* the audit record.
- Weak-model-friendly: agents emit structured JSON, reviewers validate it.

**Negative / costs**
- Less expressive than code; advanced needs require adding governed actions.
- JSON-logic has a learning curve (mitigated by templates + examples).

## Alternatives considered

### Alternative A — Custom DSL + parser
Rejected: harder for a weak model to produce correctly and harder to validate
than JSON; parser is its own bug surface.

### Alternative B — Sandboxed JS (e.g. `vm2`/QuickJS)
Rejected: security and determinism risks; harder mechanical review; contrary to
the "no arbitrary execution" safety stance for a financial system.

## Compliance / follow-up

- The action vocabulary + JSON-logic subset are published as JSON schemas in
  `platform-spec/` (Phase 2 gate).
- Every `workflow` and `rule` metadata artifact is validated against these
  schemas before merge.
- `post-to-ledger` and other financial actions additionally require SoD checks.
