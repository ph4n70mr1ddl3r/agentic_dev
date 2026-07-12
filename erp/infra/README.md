# `infra/` — Infrastructure, Docker, CI/CD, observability

*(Phase 3+)*

- `docker/` — `docker-compose.yml` for local dev (all services + Postgres),
  per-service `Dockerfile`s, seed/init volumes.
- `ci/` — CI pipelines (lint, test, schema-validate, loader dry-run, build).
- `observability/` — logging, metrics, tracing config.

See [ADR-0005](../docs/adr/0005-gated-delivery-and-weak-model-strategy.md)
(CI as a hard gate) and the CEO-authored roadmap in `docs/company/`
(written by `forge`).
