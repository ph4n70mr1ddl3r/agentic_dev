# ERP — Model-Driven Cloud ERP (Financials + Supply Chain)

A **metadata-driven** cloud ERP, built end-to-end as if by a virtual software
company of AI agents. v1 scope: **Financials** and **Supply Chain Management**,
modeled on **Microsoft Dynamics 365** (public docs = ground truth for the agents).

> **Status:** Phase 0 — Foundation **seed** complete. The company plan
> (organization, roadmap, artifact set, contribution model, task breakdown) is
> **authored by the CEO hat** via the `forge` harness — not hand-written here.

## Seed

The founders' seed is two things:

- **[Company brief](docs/company-brief.md)** — the goal + the hard constraints.
- **[Decision records (ADRs)](docs/adr/)** — the 5 immutable architectural decisions.

Everything else is produced by the CEO hat and the hats it delegates to, and
written under `docs/company/`.

## Architecture in one breath

Model-driven: engines read metadata to render UI and execute logic; business
modules are metadata, not code. Postgres hybrid storage. JSON-logic + curated
action vocabulary (no code execution anywhere). Multi-company from day one. See
the [ADRs](docs/adr/) and the [brief](docs/company-brief.md).

## Repository layout

| Path | Purpose |
|---|---|
| `docs/company-brief.md` | Founders' seed: goal + constraints |
| `docs/adr/` | The 5 immutable architecture decision records |
| `docs/company/` | CEO-authored company plan (org, roadmap, contribution model) — produced by `forge` |
| `platform-spec/` | Metadata JSON schemas — the contract (Architect-authored) |
| `services/` | The engines (microservices) |
| `frontend/` | Generic ERP shell + Studio |
| `modules/` | ERP modules as metadata (`financials/`, `supply-chain/`) |
| `infra/` | Docker, CI/CD, observability |
| `tools/` | CLI helpers (metadata-loader) |

---

_Built by AI agents coordinated by the `forge` harness. See
[docs/company-brief.md](docs/company-brief.md) for the goal and constraints._
