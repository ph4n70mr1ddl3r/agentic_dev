# ERP — Model-Driven Cloud ERP (Financials + Supply Chain)

A **metadata-driven** cloud ERP, built end-to-end as if by a virtual software
company of AI agents. v1 scope: **Financials** and **Supply Chain Management**,
modeled on **Microsoft Dynamics 365** — whose excellent public documentation is
used as ground truth by the agents.

> **Status:** Phase 0 — Foundation ✅ complete · next: Phase 1 — Discovery

## What this is

This repository is the **product** built by the virtual company. The company
itself — an AI-agent harness written in Rust — lives in a separate `forge`
repository and operates *on* this repo through GitHub (issues, branches, PRs).

The ERP is **model-driven**: a small platform of engines reads metadata
(entities, fields, forms, workflows, rules, permissions) from the database and
renders/executes everything. Business modules (General Ledger, Procurement,
Inventory, …) are **metadata packages**, not bespoke code.

## Architecture in one breath

- **Platform (code, written once):** engines as microservices — `metadata`,
  `data`, `workflow`, `auth`, `reporting`, `notification`, `audit`, `gateway` —
  plus one generic frontend shell and an admin "Studio".
- **Modules (metadata, scalable & parallel):** Financials and Supply Chain
  packages under `modules/`.
- **Storage:** Postgres hybrid — a real table per entity + JSONB extras.
- **Rules & workflow:** JSON-logic conditions + a curated action vocabulary.
  No arbitrary code execution.

See [`docs/adr/`](docs/adr/) for the reasoning behind every one of these.

## Repository layout

| Path | Purpose | Phase |
|---|---|---|
| `docs/governance/` | Org charter, artifact catalog, project plan, contribution model | 0 |
| `docs/adr/` | Architecture Decision Records | ongoing |
| `docs/templates/` | Artifact templates | 0 |
| `docs/domain/` | D365 reference base, glossary, domain model | 1 |
| `docs/architecture/` | C4 model, NFRs, integration design | 2 |
| `platform-spec/` | Metadata JSON schemas — the contract everything depends on | 2 |
| `services/` | The engines (microservices) | 3 |
| `frontend/` | Generic ERP shell + Studio | 3 |
| `modules/financials/`, `modules/supply-chain/` | ERP modules as metadata | 4 |
| `infra/` | Docker, CI/CD, IaC | 3+ |
| `tools/` | CLI helpers, metadata loader | 2/3 |

## How work happens

The company runs in gated phases (see
[`docs/governance/project-plan.md`](docs/governance/project-plan.md)). Each
artifact is produced on a branch, reviewed, and merged via PR (see
[`docs/governance/contribution-model.md`](docs/governance/contribution-model.md)).

## Quick links

- [Org & artifact catalog](docs/governance/org-charter.md)
- [Project plan & phases](docs/governance/project-plan.md)
- [Decision records (ADRs)](docs/adr/)

---

_This project is built by AI agents coordinated by the `forge` harness. See
`docs/governance/` for how the "company" works._
