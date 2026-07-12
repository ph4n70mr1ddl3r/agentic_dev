# `services/` — The engines (microservices)

*(Phase 3)* The platform code, written once. Each service is a small, focused
Node.js/TypeScript service.

| Service | Responsibility |
|---|---|
| `metadata-service` | Owns the metadata catalogs; backend of the Studio; serves metadata |
| `data-service` | Generic CRUD/query over dynamic entities; enforces schema + `company_id` RLS |
| `workflow-service` | Runs state machines; evaluates JSON-logic guards; dispatches curated actions |
| `auth-service` | Identity, roles/duties/privileges, sessions, token issuance |
| `reporting-service` | Executes report definitions (electronic reporting) |
| `notification-service` | Email / in-app / webhook delivery |
| `audit-service` | Append-only change tracking + record version history |
| `gateway` | API entry point: auth, routing, composes metadata+data for the frontend |

Each service uses the shared service template (Phase 2) and exposes an OpenAPI
contract. See [ADR-0001](../docs/adr/0001-model-driven-architecture.md).
