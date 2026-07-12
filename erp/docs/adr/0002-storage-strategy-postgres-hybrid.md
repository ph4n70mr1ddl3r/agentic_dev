# ADR-0002 — Storage: Postgres hybrid (real table per entity + JSONB extras)

- **Status:** Accepted
- **Date:** 2026-07-12
- **Deciders:** Solution Architect, Data Architect
- **Owner:** Data Architect

## Context

In a model-driven system, entity schemas are dynamic (defined by metadata), but
this is an **ERP**: financial reporting, inventory valuation, and master
planning require performant queries, joins, referential integrity, and indexing.
We must reconcile dynamic schemas with ERP-grade query needs.

## Decision

Use **PostgreSQL** with a **hybrid** storage model:

- The **Schema Engine** owns a real table per declared entity, named
  `ent_<entity>`, with **typed columns** for each declared field (type mapped
  from the field metadata).
- Each entity table also carries an `extras JSONB` column for custom/extension
  fields not promoted to columns.
- **Generated columns** and selective **indexes** are created for commonly
  filtered/sorted fields, driven by metadata flags.
- Schema changes (add/alter/drop field) are **versioned migrations** generated
  and applied by the Schema Engine — never ad-hoc DDL.
- **Metadata itself** (the catalogs: entities, fields, forms, workflows, …)
  lives in a normal relational schema `meta.*`, not in the dynamic tables.
- Audit/version history is append-only (`audit.*`), per [ADR-0001].

## Consequences

**Positive**
- ERP-grade query/reporting performance, foreign-key integrity, and clean SQL.
- JSONB retains flexibility for extension/custom fields.
- Strong typing catches agent errors at the DB layer.

**Negative / costs**
- The Schema Engine must reliably generate and migrate DDL (testable, gated).
- Every field change = a migration (intentional — it's traceable).

## Alternatives considered

### Alternative A — Entity-Attribute-Value (EAV)
Rejected: notoriously slow and complex queries; poor reporting; hard for agents
to get right; bad fit for an ERP.

### Alternative B — Pure JSONB per entity
Rejected: weaker queries, indexing, and validation; reporting pain; loses FK
integrity. Suitable only for extension fields (which we keep in `extras`).

### Alternative C — Database-per-tenant dynamic DDL
Deferred to deployment strategy (see [ADR-0004](./0004-multi-company-tenancy.md));
not an API/data-model concern.

## Compliance / follow-up

- Multi-company isolation must apply to every `ent_*` table → [ADR-0004].
- Schema Engine migrations are CI-gated and reversible.
- `extras JSONB` fields are never used for fields that need indexing/reporting
  (promote to columns instead).
