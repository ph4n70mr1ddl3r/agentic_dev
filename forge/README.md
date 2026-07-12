# `forge/` — the Rust harness (the "company")

`forge` is the executive office of the virtual software company. It is **not**
part of the ERP product — it operates *on* the [`erp/`](../erp/) repository
through GitHub. It instantiates the company's hats as AI agents and drives the
artifact-driven, gated workflow defined in the company plan.

> **Status:** MVP — the **CEO hat** runs (produces the company plan), and the
> **`sync`** command turns the first-phase tasks into GitHub Issues. More hats
> and PR/orchestration automation come next.

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
  main.rs        CLI (clap): forge ceo | sync [--write]
  config.rs      env config (DeepSeek endpoint/model)
  llm.rs         OpenAI-compatible chat client (JSON mode)
  agents/ceo.rs  CEO system prompt + plan schema + run
  plan.rs        CompanyPlan serde model
  render.rs      render the plan to markdown
  github.rs      GitHub REST client (issues, labels, milestones)
  issues.rs      sync plan.json -> GitHub Issues
  util.rs        shared HTTP/string helpers
```

## Roadmap for `forge` itself

- [x] CEO hat produces the company plan
- [x] GitHub integration: turn the first-phase tasks into Issues (+ labels/milestones) (`forge sync`)
- [ ] More hats: architect, domain modeler, engineer, QA — each consumes an issue
- [ ] Orchestrator: per-phase DAG + gated transitions
- [ ] Resumable state store (SQLite): `run` / `resume` / `status`

See the [company brief](../erp/docs/company-brief.md) for the goal and
constraints, and [ADR-0005](../erp/docs/adr/0005-gated-delivery-and-weak-model-strategy.md)
for the weak-model strategy it implements.
