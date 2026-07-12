# Metadata Spec Template — Entity / Form / Workflow

- **ID:** SPEC-META-<kind>
- **Status:** draft | in-review | accepted
- **Owner:** Tech Lead
- **Related ADRs:** [ADR-0001](../adr/0001-model-driven-architecture.md),
  [ADR-0002](../adr/0002-storage-strategy-postgres-hybrid.md),
  [ADR-0003](../adr/0003-expression-and-action-language.md),
  [ADR-0004](../adr/0004-multi-company-tenancy.md)

This template sketches the shape of the JSON schemas that will live in
`platform-spec/` (Phase 2). It is illustrative, not final.

## Entity (`platform-spec/entity.schema.json`)

```json
{
  "id": "string (kebab-case, unique)",
  "label": "string",
  "pluralLabel": "string",
  "module": "string (module id)",
  "masterData": false,
  "companyScoped": true,
  "numberSequence": "string | null (id of number sequence)",
  "fields": [
    {
      "id": "string",
      "label": "string",
      "type": "string|integer|decimal|boolean|date|datetime|enum|lookup|json",
      "required": false,
      "unique": false,
      "indexed": false,
      "lookup": { "entity": "string", "displayField": "string" },
      "enum": { "values": ["..."] },
      "default": "any",
      "validation": { "jsonLogic": "/* JSON-logic predicate */" }
    }
  ],
  "relationships": [
    { "kind": "one-to-many|many-to-many", "to": "entity id", "via": "field|table" }
  ]
}
```

## Form (`platform-spec/form.schema.json`)

```json
{
  "id": "string",
  "entity": "entity id",
  "label": "string",
  "sections": [
    {
      "id": "string",
      "label": "string",
      "columns": 2,
      "fields": [
        { "field": "field id", "widget": "text|number|date|select|lookup|checkbox", "readOnly": false, "visible": { "jsonLogic": "/* optional */" } }
      ]
    }
  ],
  "actions": ["save", "cancel", "/* custom action ids */"]
}
```

## List view (`platform-spec/list.schema.json`)

```json
{
  "id": "string",
  "entity": "entity id",
  "columns": ["field ids"],
  "defaultSort": { "field": "id", "dir": "asc" },
  "filters": [ { "field": "id", "op": "eq|like|gt|…", "value": "any" } ],
  "quickFilters": ["field ids"]
}
```

## Workflow (`platform-spec/workflow.schema.json`)

```json
{
  "id": "string",
  "entity": "entity id",
  "initialState": "string",
  "states": [
    { "id": "string", "label": "string", "terminal": false }
  ],
  "transitions": [
    {
      "id": "string",
      "from": "state id | '*'",
      "to": "state id",
      "label": "string",
      "guard": { "jsonLogic": "/* optional condition */" },
      "requiresRole": ["role ids"],
      "onExecute": [
        { "action": "set-field|send-notification|post-to-ledger|…", "args": { } }
      ]
    }
  ]
}
```

## Rule (`platform-spec/rule.schema.json`)

```json
{
  "id": "string",
  "entity": "entity id",
  "trigger": "before-save|after-save|before-delete|on-field-change",
  "condition": { "jsonLogic": "/* predicate */" },
  "actions": [ { "action": "string", "args": { } } ],
  "priority": 100
}
```

## Notes
- All `jsonLogic` values must validate against `platform-spec/jsonlogic.subschema.json`.
- All `action` values must be in the curated vocabulary (see
  [ADR-0003](../adr/0003-expression-and-action-language.md)).
- `companyScoped: true` entities get a mandatory `company_id` + RLS policy
  (see [ADR-0004](../adr/0004-multi-company-tenancy.md)).
