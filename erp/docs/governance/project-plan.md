# Project Plan

**Status:** Phase 0 complete · **Owner:** Product Owner · **Stability:** living

The phased delivery plan with gates, exit criteria, and milestones. Each phase
ends in a **human approval gate** (see
[ADR-0005](../adr/0005-gated-delivery-and-weak-model-strategy.md)) before the
next begins.

## Phases

| # | Phase | Goal | Exit-gate artifacts | Status |
|---|---|---|---|---|
| 0 | **Foundation** | Stand up the company itself | Org charter, artifact catalog, project plan, contribution model, ADR-0001..0005, templates | ✅ complete |
| 1 | **Discovery** | Know *what* to build & why | Vision/roadmap, PRDs, BPMN processes (P2P, O2C, R2R), domain model + glossary, **D365 reference base**, regulatory & NFRs | ⬜ next |
| 2 | **Architecture & Design** | Know *how* | C4 model, ADRs (ongoing), data models + dictionary, threat/security model + SoD/RBAC matrices, design system, wireframes, OpenAPI contracts, service template, test strategy | ⬜ |
| 3 | **Platform Build** | The engines (hard code, once) | All 8 services + tests, generic frontend shell + Studio, CI/CD, Docker compose, observability, `platform-spec` metadata schemas + loader/validator | ⬜ |
| 4 | **Module Authoring** *(parallel)* | The ERP itself, as metadata | Financials + SCM module packages (see scope below) + seed/demo data | ⬜ |
| 5 | **Integration & QA** | Prove it works end-to-end | Procure-to-pay, order-to-cash, record-to-report scenarios; financial-reconciliation tests; perf & security tests | ⬜ |
| 6 | **Harden & Ship** | Make it deliverable | Full docs, release notes, deploy/rollback runbooks, demo environment, **v1.0 tag** | ⬜ |

## v1 module scope (Phase 4 detail)

**Financials — modeled on D365 Finance**
- General Ledger — chart of accounts, ledger, fiscal calendars/periods, financial dimensions, journals & posting, trial balance
- Accounts Payable — vendors, vendor invoices, payments, posting profiles, 3-way match
- Accounts Receivable — customers, free-text & product invoices, receipts, credit/collections
- Cash & Bank — bank accounts, deposits, bank reconciliation
- Financial Reporting — trial balance, P&L, balance sheet (dimension-driven)
- _Fixed Assets → v1.1_

**Supply Chain — modeled on D365 SCM**
- Product Information Management — products, variants, UoM, categories, BOMs
- Procurement — purchase orders, receiving, invoice matching
- Inventory Management — on-hand, reservations, transfers, costing (FIFO/weighted/standard/moving), inventory dimensions (site/warehouse/location/batch/serial)
- Sales & Order Fulfillment — sales orders, pricing/discounts, pick/pack/ship
- Master Planning — MRP supply/demand, planned orders _(simplified)_
- _Warehouse Management → simplified/v1.1_

## Milestones

| Tag | Milestone | Phase exit |
|---|---|---|
| M0 | Company foundation accepted | Phase 0 gate |
| M1 | Discovery & requirements approved | Phase 1 gate |
| M2 | Architecture & design approved | Phase 2 gate |
| M3 | Platform operational (engines + shell + Studio, demo entity CRUD through full stack) | Phase 3 gate |
| M4 | All v1 modules authored & validated | Phase 4 gate |
| M5 | End-to-end scenarios pass QA | Phase 5 gate |
| **v1.0** | Financials + Supply Chain shipped | Phase 6 gate |

## Cross-cutting platform capabilities (designed in Phase 2, built in Phase 3)

These are first-class in the metadata schema, not modules: Legal Entities ·
Ledger & Fiscal Calendars · Financial Dimensions · Number Sequences ·
Posting Profiles/Rules · Workflow Engine · Security (role→duty→privilege) ·
Data Entities / OData · Batch framework · Electronic Reporting ·
Segregation of Duties enforcement.

## Phase 1 entry checklist (next)

- [ ] GitHub repository created & remote configured
- [ ] `forge` harness able to read issues / open branches / open PRs (stub OK)
- [ ] D365 reference sources shortlisted (Microsoft Learn areas)
- [ ] Discovery issue set created (one per Phase-1 artifact)

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| Weak model produces inconsistent domain models | D365 reference base as ground truth; BA reviews every `MOD`/`DOM` |
| Workflow/dynamic-schema engines are hard | Phase 3 is gated; invest in tests + a reference implementation of one entity through the full stack before scaling |
| Metadata drift across modules | `platform-spec` JSON schemas + loader/validator as a hard gate (Phase 2/3) |
| Financial correctness | Segregation-of-duties matrix + reconciliation tests in Phase 5; GAAP/IFRS requirements in Phase 1 |
