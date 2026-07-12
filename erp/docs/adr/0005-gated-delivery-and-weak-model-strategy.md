# ADR-0005 — Gated delivery & weak-model strategy

- **Status:** Accepted
- **Date:** 2026-07-12
- **Deciders:** Product Owner, Solution Architect
- **Owner:** Product Owner

## Context

The executing model (DeepSeek V4 Flash) is cheap but **weaker** than frontier
models, and the project is a **financial system** — correctness and
recoverability matter. Quality must come from **process structure**, not from
model intelligence. The project is long-running and must be **resumable**.

## Decision

### 1. Gated delivery
Phased delivery with **human approval gates** between phases (see
[project plan](../governance/project-plan.md)). No work from a later phase may
start before the prior gate is approved. Gate artifacts are explicitly listed.

### 2. Weak-model mitigations (applied throughout)
- **Atomic tasks** — one concept/file per issue; explicit inputs and DoD.
- **Templates & scaffolding** — agents fill blanks rather than invent structure.
- **Contracts-first** — OpenAPI + JSON schemas exist *before* the code/metadata
  that implements them.
- **Reviewer loops** — every artifact is reviewed by a *different* hat (segregation
  of authorship/approval); CI (lint/test/schema-validate) is a hard gate.
- **Structured I/O** — agents emit JSON per a strict schema; the harness parses
  and acts, no free-form "here's what I did."
- **Minimal, reconstructed context** — each agent receives only what its task
  needs, assembled by the harness (never "the whole repo").
- **Durable memory** — ADRs + living spec/domain docs; the system never relies
  on the model remembering across runs.
- **Curated reference knowledge base** — a D365-grounded reference (built in
  Phase 1) is ground truth for domain agents. *Weak model + authoritative
  reference > strong model guessing.*

### 3. GitHub-native & resumable
Everything in GitHub: issues = tasks, branches = work, PRs = review, tags =
milestones. The repo *is* the project state, so the harness can resume anytime.

## Consequences

**Positive**
- Predictable, reviewable, recoverable; compensates for the weak model.
- Financial correctness protected by gates + SoD + reconciliation tests.

**Negative / costs**
- Slower (gates, review loops, upfront schema/template/reference investment).
- Some ceremony for simple artifacts (intentional trade-off for a financial
  system).

## Alternatives considered

### Alternative A — Fully autonomous end-to-end
Rejected for v1: too risky with a weak model on a financial system. We will
*increase* per-phase autonomy as confidence and test coverage grow, possibly
relaxing gates in later phases.

### Alternative B — Per-artifact human approval (no phase gates)
Rejected: too granular and slow; phase gates balance safety with throughput.

## Compliance / follow-up

- Gate exit checklists live in [project plan](../governance/project-plan.md).
- Segregation rules live in [contribution model](../governance/contribution-model.md).
- Reviewer + schema-validation CI is a Phase 2/3 deliverable.
