# `forge/` — the Rust harness (the "company")

> **Status:** not yet started. Built after Phase 1 (Discovery) so the critical
> D365 reference base and domain model can be human-curated first.

`forge` is the executive office of the virtual software company. It is **not**
part of the ERP product — it operates *on* the [`erp/`](../erp/) repository
through GitHub. It instantiates the company's hats as AI agents and drives the
artifact-driven, gated workflow defined in the company plan.

## Responsibilities

- **Orchestrator** — phase gates + per-phase DAG of agent tasks; resumable.
- **LLM client** — DeepSeek (OpenAI-compatible endpoint); structured JSON I/O
  via strict schemas; retries/queuing for rate limits.
- **Agents** — role definitions (system prompts, tools, guardrails) for each hat.
- **GitHub integration** — issues → branches → PRs → review → merge; milestones/tags.
- **State store** — SQLite: run state, agent logs, task status (resumability).
- **CLI** — `forge plan | run | resume | status`.

## Planned layout (when built)

```
forge/
  Cargo.toml
  src/
    main.rs            CLI
    orchestrator/      phase + DAG engine
    agents/            hat definitions
    llm/               DeepSeek client + structured output
    github/            gh wrapper + git ops
    store/             SQLite state
    workspace/         mounted erp/ working dir
```

See the [company brief](../erp/docs/company-brief.md) for the goal and
constraints, and [ADR-0005](../erp/docs/adr/0005-gated-delivery-and-weak-model-strategy.md)
for the weak-model strategy it implements.
