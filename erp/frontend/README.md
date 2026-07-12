# `frontend/` — Generic ERP shell + Studio

*(Phase 3)* A single Next.js (React) application:

- **Shell** — fetches metadata and renders any form/list/dashboard/workflow from
  it. There is exactly one renderer for all modules.
- **Studio** — admin app to author metadata (entities, fields, forms,
  workflows, rules, permissions, number sequences, posting profiles).

Plus a shared component library driven by the design system (Phase 2). The
frontend contains **no module-specific code** — all module behavior comes from
metadata under `modules/`.
