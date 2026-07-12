# First Phase: Phase 1: Foundation - Reference & Platform Spec

_Authored by the CEO hat via `forge`._

| ID | Title | Role | Type | Depends on |
|---|---|---|---|---|
| T1 | Author D365 Financials Reference Digest | Domain Modeler (Financials) | spec | — |
| T2 | Author D365 Supply Chain Reference Digest | Domain Modeler (Supply Chain) | spec | — |
| T3 | Define Core Entity Metadata Schema | Solution Architect | spec | — |
| T4 | Define Workflow and Action Vocabulary Schema | Tech Lead | spec | — |
| T5 | Set Up Mono Repo and CI Pipeline | DevOps Engineer | code | — |
| T6 | Create Contribution Model Document | CEO | doc | T5 |
| T7 | Validate Entity Schema Against Financials Reference | QA Engineer | test | T1, T3 |
| T8 | Validate Workflow Schema Against Supply Chain Reference | QA Engineer | test | T2, T4 |

## Details

### T1 — Author D365 Financials Reference Digest
- **Role:** Domain Modeler (Financials)
- **Type:** spec

Compile a structured digest of Dynamics 365 Financials domain entities, fields, workflows, and business rules from public documentation. Output as a markdown file in docs/reference/financials/. This will be the authoritative source for all Financials metadata authoring in later phases.

### T2 — Author D365 Supply Chain Reference Digest
- **Role:** Domain Modeler (Supply Chain)
- **Type:** spec

Compile a structured digest of Dynamics 365 Supply Chain Management domain entities, fields, workflows, and business rules from public documentation. Output as a markdown file in docs/reference/supply-chain/. This will be the authoritative source for all Supply Chain metadata authoring in later phases.

### T3 — Define Core Entity Metadata Schema
- **Role:** Solution Architect
- **Type:** spec

Author the JSON Schema for core entity metadata: entity, field, form, list view, and relationship definitions. This schema lives in platform-spec/schemas/ and validates all downstream metadata packages. Must support typed fields, mandatory company_id, and extensibility via extras JSONB.

### T4 — Define Workflow and Action Vocabulary Schema
- **Role:** Tech Lead
- **Type:** spec

Define JSON schemas for workflow conditions (JSON-logic subset) and the curated action vocabulary (set-field, update-record, etc.). These schemas in platform-spec/schemas/ will be used to validate all rule and workflow metadata. Include examples and refer to ADR-0003.

### T5 — Set Up Mono Repo and CI Pipeline
- **Role:** DevOps Engineer
- **Type:** code

Initialize the monorepo structure with folders for platform-spec, engines, frontend, metadata, docs, and tests. Configure GitHub Actions for CI: linting (ESLint, Prettier), unit tests (Jest), and schema validation (ajv). Include Dockerfile for local development. Ensure harness (Rust) can run locally and trigger CI.

### T6 — Create Contribution Model Document
- **Role:** CEO
- **Type:** doc
- **Depends on:** T5

Write the contribution model (as described in the company plan) into CONTRIBUTING.md. Include GitHub issue/branch/PR workflow, labeling conventions, and review requirements. This document is the authoritative guide for all agents.

### T7 — Validate Entity Schema Against Financials Reference
- **Role:** QA Engineer
- **Type:** test
- **Depends on:** T1, T3

Create automated tests that validate the core entity metadata schema (T3) against sample entities from the Financials reference digest (T1). Ensure the schema can represent GL Account, Journal, Invoice, etc. Report any gaps.

### T8 — Validate Workflow Schema Against Supply Chain Reference
- **Role:** QA Engineer
- **Type:** test
- **Depends on:** T2, T4

Create automated tests that validate the workflow schema (T4) against sample workflows from the Supply Chain reference digest (T2). Ensure the action vocabulary covers typical procurement and inventory actions.

