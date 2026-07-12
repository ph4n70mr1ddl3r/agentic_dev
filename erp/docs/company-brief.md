# Company Brief — the seed

This is the **founders' seed** for the virtual software company. It states the
**goal** and the **hard constraints** that are already decided. Everything else —
the organization, the roadmap, the artifact set, the contribution model, the task
breakdown — is **decided by the CEO hat** (and the hats it delegates to) through
the `forge` harness, and written into this repo as the company's own plan.

> **Read this first. Then plan.** (You are the CEO.)

## Goal

Build a **model-driven cloud ERP**, end to end, as if by one big software
company made of AI agents. v1 scope: **Financials** and **Supply Chain
Management**, modeled on **Microsoft Dynamics 365** — whose public documentation
is the ground truth for all domain decisions. Ship a working **v1.0**.

## Hard constraints (immutable)

Decided by the founders. Agents work *within* these; they are not re-litigated.

1. **Model-driven.** A platform of engines reads metadata to render UI and
   execute logic; business modules are metadata, not code.
   ([ADR-0001](adr/0001-model-driven-architecture.md))
2. **Postgres hybrid storage** — a real table per entity + JSONB extras.
   ([ADR-0002](adr/0002-storage-strategy-postgres-hybrid.md))
3. **JSON-logic + curated action vocabulary** for rules/workflow; no arbitrary
   code execution anywhere.
   ([ADR-0003](adr/0003-expression-and-action-language.md))
4. **Multi-company from day one** — legal entities are first-class.
   ([ADR-0004](adr/0004-multi-company-tenancy.md))
5. **Gated delivery + weak-model mitigations.** Human approval gates between
   phases; quality comes from structure (atomic tasks, templates,
   contracts-first, reviewer loops, structured I/O, curated reference material,
   durable ADR/spec memory).
   ([ADR-0005](adr/0005-gated-delivery-and-weak-model-strategy.md))

## Stack & operating constraints

- **ERP product:** full-stack JavaScript/TypeScript, microservices, Docker.
- **Harness:** Rust (`forge/`) — coordinates the hat-wearing agents, drives
  GitHub, and keeps state resumable.
- **Executing model:** DeepSeek (OpenAI-compatible endpoint), cheap/fast. **Assume
  a weaker model** and design the process accordingly (see ADR-0005).
- **Everything in GitHub:** issues = tasks, branches = work, PRs = review,
  tags = milestones. The repo *is* the project state.

## The CEO's first job

Produce the company's own plan from this seed:

1. The **organization** — hats/roles and what each owns (keep it lean but
   complete; specialize narrowly so each agent has a small, clear scope).
2. The **roadmap** — an ordered list of phases, each with a goal and explicit
   exit criteria.
3. The **artifact set** the company produces.
4. The **contribution model** — how a unit of work goes from idea to merged.
5. The **first phase's task breakdown** — as discrete tasks (later: GitHub
   issues), each with an owner hat, a type, a description, and dependencies.

Then delegate to the hats and begin. Use Dynamics 365 as the reference for all
domain decisions.
