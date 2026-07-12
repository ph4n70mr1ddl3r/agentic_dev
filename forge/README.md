# `forge/` — the Rust harness (the "company")

`forge` is the executive office of the virtual software company. It is **not**
part of the ERP product — it operates *on* the [`erp/`](../erp/) repository
through GitHub. It instantiates the company's hats as AI agents and drives the
artifact-driven, gated workflow defined in the company plan.

> **Status:** MVP — the **CEO hat** runs: it reads the company brief and produces
> the company plan (organization, roadmap, contribution model, first-phase tasks)
> as structured JSON, written into `erp/docs/company/`. More hats and GitHub
> issue/PR automation come next.

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
  main.rs        CLI (clap): forge ceo [--write]
  config.rs      env config (DeepSeek endpoint/model)
  llm.rs         OpenAI-compatible chat client (JSON mode)
  agents/ceo.rs  CEO system prompt + plan schema + run
  plan.rs        CompanyPlan serde model
  render.rs      render the plan to markdown
```

## Roadmap for `forge` itself

- [x] CEO hat produces the company plan
- [ ] GitHub integration: turn the first-phase tasks into Issues (+ labels/milestones)
- [ ] More hats: architect, domain modeler, engineer, QA — each consumes an issue
- [ ] Orchestrator: per-phase DAG + gated transitions
- [ ] Resumable state store (SQLite): `run` / `resume` / `status`

See the [company brief](../erp/docs/company-brief.md) for the goal and
constraints, and [ADR-0005](../erp/docs/adr/0005-gated-delivery-and-weak-model-strategy.md)
for the weak-model strategy it implements.
