# Artifact Catalog

**Status:** Phase 0 · **Owner:** Tech Lead (process) / Product Owner (content)
· **Stability:** foundational

The authoritative list of artifacts the company produces, who owns each, and in
which phase. Every artifact has a stable **ID** and a **path convention** so it
can be referenced from issues, PRs, and ADRs.

## Naming & path conventions

- **ID scheme:** `<TYPE>-<area>-<n>` for governed artifacts, e.g. `ADR-0007`,
  `PRD-FIN-GL-01`, `SPEC-META-entity`, `BPMN-SCM-PO-01`.
- **Path convention:** artifacts live under `docs/<area>/` or `modules/<…>/`
  per the layout in the [README](../../README.md).
- **Format:** Markdown for prose; JSON for machine-consumed specs/metadata;
  BPMN as XML + a rendered PNG + a Markdown description.
- **Status field:** every artifact header carries `Status:` ∈
  `{draft, in-review, accepted, superseded, deprecated}`.

## Artifact types

| Code | Type | Owner hat | Notes |
|---|---|---|---|
| VIS | Vision & roadmap | PO | strategic |
| PRD | Product requirements document | PM | per release/area |
| BPMN | Business process model | BA | per process |
| DOM | Domain model & glossary | BA | living document |
| REF | Reference knowledge base (D365) | BA | curated ground truth |
| REG | Regulatory/compliance requirements | BA/Sec | GAAP/IFRS, SoD |
| NFR | Non-functional requirements | SA | |
| ADR | Architecture decision record | SA | see `docs/adr/` |
| C4 | C4 architecture diagrams | SA | context/container/component |
| INT | Integration design | SA | |
| DM | Data model (conceptual/logical/physical) | DA | |
| DD | Data dictionary | DA | |
| THREAT | Threat model (STRIDE) | Sec | |
| SECCOMP | Security & compliance matrix | Sec | |
| SOD | Segregation-of-duties matrix | Sec | |
| RBAC | Role/duty/privilege matrix | Sec | |
| DS | Design system & tokens | UX | |
| WF | Wireframe / user flow | UX | per screen/flow |
| STD | Coding standards & conventions | TL | |
| TMPL | Service / artifact templates | TL | |
| OAS | OpenAPI contract | TL | per service |
| DOD/DOR | Definitions of Done/Ready | TL | |
| TD | Tech-debt register | TL | living document |
| ENG | Engine implementation (code) | PE | per service |
| UI | Frontend implementation (code) | FE | |
| MOD | Module metadata package | DM | per D365 module |
| SEED | Seed/demo data | DM | per module |
| TS | Test strategy | QA | |
| TP | Test plan | QA | per area |
| TC | Test case | QA | |
| INFR | Infrastructure / IaC | DO | |
| CICD | CI/CD pipeline | DO | |
| OBS | Observability plan | DO | |
| RB | Runbook | DO/RM | |
| VER | Versioning & release policy | RM | |
| RN | Release notes | RM | per release |
| DOC | User/admin/dev documentation | TW | |

## Phase → artifacts matrix (at-a-glance)

| Phase | Key artifacts produced |
|---|---|
| **0 — Foundation** | VIS, STD, TMPL, ADR-0001..0005, this catalog, project plan, contribution model |
| **1 — Discovery** | PRD (per area), BPMN (P2P, O2C, R2R…), DOM/REF (D365 reference + glossary), REG, NFR |
| **2 — Architecture & Design** | C4, ADRs (ongoing), DM, DD, THREAT, SECCOMP, SOD, RBAC, DS, WF, OAS, TMPL (service), TS |
| **3 — Platform Build** | ENG (×8 engines), UI (shell + Studio), INFR, CICD, OBS, TC (engine), platform-spec/MOD schemas |
| **4 — Module Authoring** | MOD + SEED per Financials & SCM module (see project plan for the list) |
| **5 — Integration & QA** | TP, TC (end-to-end: procure-to-pay, order-to-cash, record-to-report), perf/security tests |
| **6 — Harden & Ship** | DOC (user/admin/dev), RN, RB (deploy/rollback), demo env, v1.0 tag |

## v1 module scope (what `MOD` artifacts will cover)

**Financials:** General Ledger · Accounts Payable · Accounts Receivable ·
Cash & Bank · Financial Reporting. _(Fixed Assets → v1.1)_

**Supply Chain:** Product Information Management · Procurement · Inventory
Management · Sales & Order Fulfillment · Master Planning (simplified).
_(Warehouse Management → simplified/v1.1)_

**Cross-cutting (in platform metadata schema, not modules):** Legal Entities ·
Ledger & Fiscal Calendars · Financial Dimensions · Number Sequences ·
Posting Profiles/Rules · Workflow · Security (role→duty→privilege) ·
Data Entities · Batch · Electronic Reporting.
