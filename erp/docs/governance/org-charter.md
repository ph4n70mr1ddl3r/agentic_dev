# Org Charter — The Virtual Software Company

**Status:** Phase 0 · **Owner:** Product Owner · **Stability:** foundational

This document defines the virtual company that builds the ERP: its mission, the
"hats" (roles) each AI agent can wear, reporting lines, and a RACI for the
recurring activities. It is the social contract the `forge` harness enforces.

## 1. Mission

Build a production-quality, model-driven cloud ERP (Financials + Supply Chain,
modeled on Dynamics 365) end-to-end, entirely by AI agents, on a cheap/weak
model — by making the *process* do the work that a strong model would otherwise
do.

## 2. Guiding principles

1. **Artifacts over chat.** Every decision and deliverable is a file in this
   repo. If it isn't written down, it didn't happen.
2. **Model-driven.** The platform is engines; the ERP is metadata. Prefer
   authoring JSON over writing code (see [ADR-0001](../adr/0001-model-driven-architecture.md)).
3. **Structure compensates for the model.** Atomic tasks, templates,
   contracts-first, reviewer loops, curated reference material
   (see [ADR-0005](../adr/0005-gated-delivery-and-weak-model-strategy.md)).
4. **Gated delivery.** No phase exits without a human approval at the gate.
5. **Everything in GitHub.** Issues = tasks, branches = work units, PRs = review,
   tags = releases. The repo *is* the project state.

## 3. The hats

Specialization is deliberate: narrow scope per agent keeps a weak model accurate.

| # | Hat | Core responsibility |
|---|---|---|
| 1 | **Product Owner (PO)** | Vision, roadmap, go/no-go gates, final acceptance |
| 2 | **Product Manager (PM)** | PRD, backlog, epics & stories, release plan |
| 3 | **Business Analyst / Domain Expert (BA)** | Business processes (BPMN), domain model, glossary, regulatory (GAAP/IFRS), **curates the D365 reference knowledge base** |
| 4 | **Solution Architect (SA)** | Architecture vision, C4 model, ADRs, NFRs, integration design |
| 5 | **Data Architect (DA)** | Conceptual→physical data models, data dictionary, master-data & migration strategy |
| 6 | **Security Architect (Sec)** | Threat model (STRIDE), security architecture, compliance matrix, Segregation-of-Duties matrix, RBAC matrix |
| 7 | **UX Designer (UX)** | Design system, wireframes, user flows, accessibility |
| 8 | **Tech Lead (TL)** | Coding standards, service templates, API contracts (OpenAPI), code review, DoR/DoD, tech-debt log |
| 9 | **Platform Engineer (PE)** *(per engine)* | Implement one engine (data, workflow, auth, metadata, reporting, notification, audit, gateway) |
| 10 | **Frontend Engineer (FE)** | Generic ERP shell + Studio, component library, data layer |
| 11 | **Domain Modeler (DM)** *(per module)* | Author module metadata (entities, forms, workflows, rules, permissions) + seed data |
| 12 | **QA Engineer (QA)** | Test strategy/plans/cases, automation, quality gates, financial-reconciliation tests |
| 13 | **DevOps / SRE (DO)** | Infra, Docker, CI/CD, environments, observability, IaC, runbooks |
| 14 | **Release Manager (RM)** | Versioning policy, release notes, deploy/rollback runbooks |
| 15 | **Technical Writer (TW)** | Architecture/API/user/admin docs, onboarding |

## 4. Reporting lines

```
Product Owner
 ├── Product Manager ── Business Analyst / Domain Expert
 ├── Solution Architect
 │    ├── Data Architect
 │    ├── Security Architect
 │    └── UX Designer
 ├── Tech Lead
 │    ├── Platform Engineers (per engine)
 │    ├── Frontend Engineer
 │    ├── Domain Modelers (per module)
 │    └── QA Engineer
 └── Release Manager ── DevOps / SRE
 (Technical Writer is shared, consulted by all)
```

## 5. RACI for recurring activities

R = Responsible · A = Accountable · C = Consulted · I = Informed

| Activity | PO | PM | BA | SA | DA | Sec | UX | TL | PE/FE | DM | QA | DO | RM | TW |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Set vision & roadmap | A/R | R | C | C | I | I | I | I | I | I | I | I | I | I |
| Define requirements & domain | A | R | R | C | C | C | I | I | I | I | I | I | I | I |
| Approve architecture (gate) | A | C | C | R | C | C | I | C | I | I | I | I | I | I |
| Author module metadata | I | C | C | C | C | C | C | C | I | R/A | I | I | I | I |
| Implement engines | I | I | C | C | C | C | C | A | R | I | C | C | I | I |
| Review & merge PRs | I | I | C | C | C | C | C | A/R | C | C | C | C | I | I |
| Test & quality gate | I | C | C | I | I | C | I | C | I | C | A/R | C | I | I |
| CI/CD & release | A | I | I | I | I | C | I | C | I | I | C | R | R | I |
| Ship release | A | I | I | I | I | I | I | C | I | I | C | R | R | C |

## 6. Hat lifecycle

A single `forge` agent can wear multiple hats, but **never two hats on the same
artifact's R/A line** (segregation of authorship and approval). The harness
assigns hats per task from the issue's role label.

## 7. Change control for this charter

This document is itself a governed artifact. Changes require a PR reviewed by
the Solution Architect and approved by the Product Owner.
