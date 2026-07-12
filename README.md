# agentic_dev — an AI-built, model-driven cloud ERP

An end-to-end project in which a **virtual software company of AI agents**
builds a **model-driven cloud ERP** (Financials + Supply Chain Management),
modeled on **Microsoft Dynamics 365**, from blank repo to shipped v1.0.

The company is run by a **Rust harness (`forge`)** that schedules role-based
agents, talks to GitHub, and enforces an artifact-driven, gated workflow. The
executing model is a cheap/weak model (DeepSeek V4 Flash); quality comes from
**process structure**, not model strength.

> **Status:** Phase 0 — Foundation ✅ complete · next: Phase 1 — Discovery

## What's in this repo

```
agentic_dev/
├── erp/     the PRODUCT — a model-driven ERP (engines + metadata modules)
└── forge/   the COMPANY — Rust harness that runs the agents (TBD)
```

- **[`erp/`](erp/)** — the ERP product itself: a platform of engines
  (metadata, data, workflow, auth, reporting, notification, audit, gateway)
  plus business modules as metadata. Start at **[`erp/README.md`](erp/README.md)**.
- **[`forge/`](forge/)** — the Rust orchestrator that instantiates the virtual
  company's 15 "hats" as agents, drives the GitHub-native workflow, and keeps
  state resumable.

## Why it works with a weak model

The ERP is **model-driven**: ~90% of "building it" is authoring validated JSON
metadata (entities, forms, workflows, rules), which a weak model handles well —
especially with reviewer agents, JSON-schema gates, a contracts-first flow, and
a curated D365 reference knowledge base as ground truth. See
[`erp/docs/adr/`](erp/docs/adr/).

## Quick links

- [Org & artifact catalog](erp/docs/governance/org-charter.md)
- [Project plan & phases](erp/docs/governance/project-plan.md)
- [Architecture decisions (ADRs)](erp/docs/adr/)

---

_Built by AI agents coordinated by `forge`. See `erp/docs/governance/` for how
the "company" works._
