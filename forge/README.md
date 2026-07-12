# `forge/` — the Rust harness (the "company")

`forge` is the executive office of the virtual software company. It is **not**
part of the ERP product — it operates *on* the [`erp/`](../erp/) repository
through GitHub. It instantiates the company's hats as AI agents and drives the
artifact-driven, gated workflow defined in the company plan.

> **Status:** MVP — the **CEO hat** plans, **`sync`** materializes the plan as
> GitHub Issues, and **`run`** orchestrates the agent loop: four hats —
> Architect, Tech Lead, Domain Modeler, QA — produce schema-validated artifacts
> (6/8 Phase-1 tasks run unattended), DAG-aware, retry-on-rejection, each landing
> as a reviewable GitHub PR with **`--pr`**; **`forge check`** executes the QA
> test-plans, runs are **resumable** (`forge status`, `run --force`), and phases
> close behind a **human gate** (`forge gate`). The last two hats come next.

## Run

```bash
# 1. configure (copy and fill in your DeepSeek key)
cp forge/.env.example forge/.env   # or just export DEEPSEEK_API_KEY=...

# 2. from the repo root, run the CEO (uses thinking mode by default)
cargo run --manifest-path forge/Cargo.toml -- ceo --repo erp --write
# add --no-thinking for the cheaper non-thinking path
```

Environment (see [`forge/.env.example`](.env.example)):
- `DEEPSEEK_API_KEY` — **required**.
- `DEEPSEEK_BASE_URL` — default `https://api.deepseek.com` (OpenAI-compatible).
- `DEEPSEEK_MODEL` — default `deepseek-v4-flash` (the cheap model). `deepseek-v4-pro`
  is the smarter option. (`deepseek-chat`/`deepseek-reasoner` are deprecated
  2026-07-24.)
- `DEEPSEEK_THINKING` — `disabled` (default for future hats, cheapest) or `enabled`.
  Note: the **CEO thinks by default**; pass `ceo --no-thinking` to force the
  cheap path for the CEO.
- `DEEPSEEK_REASONING_EFFORT` — `high`/`max` (only when thinking enabled).
- `DEEPSEEK_MAX_TOKENS` — default 8192 (prevents JSON truncation).
- `DEEPSEEK_TIMEOUT_SECS` — default 180 (per-request HTTP timeout).

Without `--write`, the plan is printed as JSON to stdout (useful for inspecting
before committing). Increase verbosity with `RUST_LOG=forge=debug`.

## Sync the plan to GitHub Issues

After `forge ceo --write` has produced `plan.json`, turn the first-phase tasks
into GitHub Issues — one per task, with `phase-*` / `type-*` / `role-*` labels,
a milestone matching the phase, and the dependency list in the body:

```bash
# dry run (prints what would be created; no token needed)
cargo run --manifest-path forge/Cargo.toml -- sync --repo erp

# create them for real (needs GITHUB_TOKEN with Issues: write)
export GITHUB_TOKEN="$(gh auth token)"   # or a fine-grained PAT
cargo run --manifest-path forge/Cargo.toml -- sync --repo erp --write
```

The GitHub repo is auto-detected from git's `origin` remote; override with
`--github-repo owner/name`. Sync is **idempotent**: each issue body carries a
`<!-- forge:task:Tn -->` marker, so re-running skips tasks that already have an
open issue.

## Run — the agent loop

`forge run` drives the agent loop. With a task id it runs that one task;
without it, the **orchestrator** walks the first-phase DAG and runs every task
whose owning role has a hat and whose dependencies are satisfied, skipping the
rest with a reason (no hat / blocked deps), stopping at the phase boundary.

Each hat reads its task, the relevant `platform-spec` schema(s) (the contract),
and a template example, then emits a structured artifact that is **validated
against the schema in-process** before acceptance. On rejection the specific
errors are fed back and the hat retries (the ADR-0005 reviewer loop).

```bash
# run the whole phase (DAG-aware): runs what it can, skips the rest with reasons
cargo run --manifest-path forge/Cargo.toml -- run --repo erp
# run a single task
cargo run --manifest-path forge/Cargo.toml -- run --repo erp T3
# run the whole phase AND open a PR per artifact
GITHUB_TOKEN="$(gh auth token)" cargo run --manifest-path forge/Cargo.toml -- run --repo erp --pr
# execute a QA test-plan (validate its assertions)
cargo run --manifest-path forge/Cargo.toml -- check --repo erp modules/generated/entity-schema-financials-conformance.json
# show persisted progress
cargo run --manifest-path forge/Cargo.toml -- status --repo erp
# approve a phase gate (human approval between phases — ADR-0005)
cargo run --manifest-path forge/Cargo.toml -- gate --repo erp 1 --note "accepted"
# re-run, ignoring persisted done-state
cargo run --manifest-path forge/Cargo.toml -- run --repo erp --force
# artifacts → erp/modules/generated/<id>.json (or a PR with --pr)
```

With **`--pr`**, each produced artifact is published as a reviewable GitHub PR
(one branch per artifact — `forge/<task>-<id>`, a scoped commit, pushed, then a
PR opened with the task context + the schema it was validated against). Needs a
clean working tree and `GITHUB_TOKEN` (e.g. `$(gh auth token)`); the base branch
defaults to `main` (`--base`).

Implemented hats:

- **Solution Architect** — authors entity metadata, validated against
  `entity.schema.json` (guid `id` PK, `companyId` on transactional entities,
  typed fields, picklist/lookup/decimal constraints, ...).
- **Tech Lead** — authors workflow metadata, validated against
  `workflow.schema.json` (states/transitions, JSON-logic guards, curated actions).
- **Domain Modeler** — authors a structured, schema-validated D365 reference
  digest (entities/processes/rules) for Financials or Supply Chain, the curated
  KB (ADR-0005); rendered to a markdown companion locally.
- **QA Engineer** — authors a schema-validated test-plan (conformance assertions
  over a reference + schema), self-checked semantically before acceptance, then
  executed by `forge check` / the orchestrator.

Other hats (Domain Modeler, QA, DevOps) land as the harness grows.

The reviewer **is the JSON Schema itself** — structural correctness is free, so
a future reviewer hat only needs to judge domain semantics. Worker hats default
to the cheap non-thinking path; set `DEEPSEEK_THINKING=enabled` for harder
artifacts.

## What the CEO produces

```
erp/docs/company/
  organization.md        hats/roles & responsibilities
  roadmap.md             phases + exit criteria
  contribution-model.md  how work goes from idea to merged
  first-phase-tasks.md   the first phase's task breakdown
  plan.json              the raw structured plan
```

## Layout

```
forge/src/
  main.rs        CLI (clap): forge ceo | sync | run [TASK]
  config.rs      env config (DeepSeek endpoint/model)
  llm.rs         OpenAI-compatible chat client (JSON mode)
  schema.rs      platform-spec schema registry + JSON-Schema validator
  orchestrator.rs  DAG-aware phase runner (forge run)
  git.rs         local git CLI wrappers (branch/commit/push for --pr)
  pr.rs          publish an artifact as a GitHub PR (--pr)
  agents/mod.rs  hat dispatch + has_hat + shared helpers
  agents/ceo.rs  CEO system prompt + plan schema + run
  agents/architect.rs  entity-authoring hat (schema-validated)
  agents/tech_lead.rs  workflow-authoring hat (schema-validated)
  agents/domain_modeler.rs  D365 reference-digest hat (schema-validated + markdown)
  agents/qa.rs   QA test-plan hat + check_plan runner (forge check)
  state.rs      resumable per-task state (SQLite) under <repo>/.forge/state.db
  plan.rs        CompanyPlan serde model
  render.rs      render the plan to markdown
  github.rs      GitHub REST client (issues, labels, milestones)
  issues.rs      sync plan.json -> GitHub Issues
  util.rs        shared HTTP/string helpers
```

## Roadmap for `forge` itself

- [x] CEO hat produces the company plan
- [x] GitHub integration: turn the first-phase tasks into Issues (+ labels/milestones) (`forge sync`)
- [x] Worker hats: Architect (entities), Tech Lead (workflows), Domain Modeler (reference digests), QA (test-plans) — all schema-validated
- [x] Orchestrator: DAG-aware phase runner (`forge run`)
- [x] Mechanical QA gate: test-plan artifacts + `forge check` (executes assertions)
- [ ] More hats: DevOps, docs — each consumes a task
- [x] PR write-back (`--pr`): branch → commit artifact → open PR
- [x] Resumable state store (SQLite): `run` resumes done tasks, `forge status`, `run --force`
- [x] Phase gate: `forge gate <phase> [--force --note]` — human approval between phases (ADR-0005)

See the [company brief](../erp/docs/company-brief.md) for the goal and
constraints, and [ADR-0005](../erp/docs/adr/0005-gated-delivery-and-weak-model-strategy.md)
for the weak-model strategy it implements.
