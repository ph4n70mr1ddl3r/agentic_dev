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

See [artifact catalog](../docs/governance/artifact-catalog.md) (`MOD`, `SEED`)
and the [project plan](../docs/governance/project-plan.md) for the v1 scope.
