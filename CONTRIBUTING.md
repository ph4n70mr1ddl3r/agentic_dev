# Contributing to agentic_dev

This repo is built by a **virtual software company of AI agents** coordinated by
the [`forge`](forge/) harness, on a **model-driven cloud ERP** ([`erp/`](erp/)).
Humans direct and gate the work; agents produce artifacts that must pass
mechanical review before they land. See
[ADR-0005](erp/docs/adr/0005-gated-delivery-and-weak-model-strategy.md) for the
why.

## How work flows (the contribution model)

Everything is GitHub-native — the repo *is* the project state.

1. **Tasks = Issues.** Each task in the CEO-authored plan is a GitHub Issue
   (`forge sync --write`), labeled `phase-N`, `type-<t>`, `role-<r>`, `forge`,
   and assigned to the milestone for its phase. The issue body lists the owner
   role, type, phase, and dependencies.
2. **Work = branches.** One branch per unit of work: `forge/<task>-<artifact>`
   (e.g. `forge/T3-company`). `forge run --pr` creates the branch, commits the
   artifact, pushes, and opens the PR automatically.
3. **Review = PRs.** Every artifact is a PR. **Author ≠ reviewer** (segregation
   of duties). Structural correctness is enforced mechanically — see *Gates*
   below; the human/domain reviewer judges content.
4. **Milestones = phases.** A GitHub milestone tags the phase. A phase closes
   behind a **human gate** (`forge gate <phase>`); no later-phase work begins
   before the prior gate is approved (ADR-0005).

## Gates (mechanical review)

These run in [CI](.github/workflows/ci.yml) and locally, and are **hard** gates:

- **Schema validation** — every `platform-spec` schema must compile, every
  example and every `$kind`-tagged artifact must validate (`npm run
  schema-validate` in `erp/`). The agents' JSON outputs are validated against
  these schemas *before* acceptance, and again here.
- **forge (Rust)** — `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test`.

## Repo layout

```
erp/      the PRODUCT — model-driven ERP (engines + metadata modules)
  platform-spec/   the JSON-Schema contract (T3/T4) — gate for all artifacts
  modules/generated/   artifacts the hats produce (entities, workflows, ...)
  docs/            company brief, ADRs, CEO-authored plan
forge/    the COMPANY — Rust harness that runs the agents
```

## Running the harness

```bash
# plan → issues → run the company (resumable)
forge ceo --repo erp --write
forge sync --repo erp --write
forge run --repo erp            # 4 hats produce schema-validated artifacts
forge run --repo erp --pr       # ...each as a PR
forge status --repo erp         # persisted progress + gate state
forge gate --repo erp 1         # approve the phase gate
```

See [`forge/README.md`](forge/README.md) for the full harness reference.

## Local dev setup

- **forge** (Rust): `cargo test --manifest-path forge/Cargo.toml`.
- **erp schemas** (Node): `cd erp && npm install && npm run schema-validate`.
- **Postgres** (ADR-0002): `docker compose -f erp/infra/docker/docker-compose.yml up -d`.

## Authoring an artifact by hand

Prefer letting the hats produce artifacts (`forge run`). If you hand-author one
(e.g. an entity), it must still pass its schema — validate with
`npm run schema-validate` (and `forge check <test-plan.json>` for QA plans)
before opening a PR.

---

_Built by AI agents coordinated by `forge`; gated by humans._
