# `modules/` — ERP modules as metadata

*(Phase 4)* The ERP itself, expressed entirely as metadata packages. Each module
is a folder of JSON validated against `platform-spec/` schemas, applied to the
DB by the loader. **No code lives here.**

```
modules/
  financials/        general-ledger/ accounts-payable/ accounts-receivable/
                     cash-and-bank/  financial-reporting/
  supply-chain/      product-information/ procurement/ inventory/
                     sales-orders/         master-planning/
```

Each module folder contains: `entities/*.json`, `forms/*.json`, `lists/*.json`,
`workflows/*.json`, `rules/*.json`, `permissions/*.json`, and `seed/*.json`.

These are the `MOD` (module) and `SEED` (seed-data) artifacts. See
[ADR-0001](../docs/adr/0001-model-driven-architecture.md); the CEO-authored
roadmap (v1 scope, phased) is written to `docs/company/` by `forge`.
