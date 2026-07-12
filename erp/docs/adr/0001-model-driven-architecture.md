# ADR-0001 — Model-driven architecture (platform of engines + metadata)

- **Status:** Accepted
- **Date:** 2026-07-12
- **Deciders:** Product Owner, Solution Architect
- **Owner:** Solution Architect

## Context

We are building a full cloud ERP (Financials + Supply Chain) end-to-end using
AI agents on a cheap/weak model (DeepSeek V4 Flash). We need:

- **Scalability without proportional code growth** — many business modules
  without N bespoke services.
- **Reviewability** — a weak model's output must be mechanically checkable.
- **Low risk of repeated novel logic** — the model shouldn't re-invent
  distributed-systems behavior for every module.

Real cloud ERPs that meet these needs (Dynamics 365, Salesforce, Odoo, SAP) are
all **metadata-driven**: a platform interprets metadata to render UI and execute
logic, and business modules are data.

## Decision

Build a **model-driven platform**:

- A small set of generic **engines** (as microservices: metadata, data,
  workflow, auth, reporting, notification, audit, gateway) reads metadata from
  the database to render screens/forms and execute workflows/rules.
- Business modules (GL, AP, Procurement, Inventory, …) are **metadata packages**
  — entities, fields, forms, workflows, rules, permissions — stored as JSON and
  loaded into the DB via a versioned migration/loader.
- One generic **frontend shell** renders any form/list/dashboard from metadata;
  an admin **Studio** authors metadata.

## Consequences

**Positive**
- ~90% of "building the ERP" becomes authoring validated JSON — ideal for a weak
  model with reviewer + schema-validation loops.
- Engines are written once, small in surface, heavily reviewed/tested.
- Modules can be authored in parallel by many Domain Modeler agents.
- Changes to forms/workflows are metadata migrations, not code deployments.

**Negative / costs**
- The engines (especially workflow + dynamic schema + generic UI renderer) are
  non-trivial — this is the bulk of real engineering, concentrated in Phase 3.
- Requires early investment in `platform-spec` JSON schemas + a loader/validator
  (Phase 2/3) that everything downstream depends on.

## Alternatives considered

### Alternative A — Bespoke microservices per module (hardcoded forms/workflows)
Rejected: unbounded code surface for a weak model; poor reviewability; cost
scales linearly with module count; doesn't match how real ERPs are built.

### Alternative B — Build on an existing low-code platform
Rejected: out of scope; we build from scratch (and want full control of the
metadata model).

## Compliance / follow-up

- Storage strategy for dynamic entities → [ADR-0002](./0002-storage-strategy-postgres-hybrid.md).
- Rule/workflow expression language → [ADR-0003](./0003-expression-and-action-language.md).
- Multi-company model → [ADR-0004](./0004-multi-company-tenancy.md).
- `platform-spec` schemas are a Phase 2 gate before any `MOD` authoring.
