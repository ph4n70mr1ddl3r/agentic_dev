# `tools/schema-validator/`

The CI gate for the model-driven contract (ADR-0005: schema-validation is a hard
gate). A tiny Node script that:

1. loads every `platform-spec/schemas/*.schema.json` into a draft-2020-12
   [ajv](https://ajv.js.org/) registry (so cross-file `$ref`s must resolve), and
2. validates every `platform-spec/examples/*.json` against its same-named schema,
   plus every `$kind`-tagged artifact under `modules/generated/` against its
   kind's schema (`domain-reference`, `test-plan`).

This is the precursor of the full `metadata-loader` (Phase 2/3), which will add
referential-integrity checks (lookup targets, workflow state refs, form/list
field refs) and apply module metadata as versioned migrations.

## Run

```bash
cd erp
npm install           # or: npm ci
npm run schema-validate
```

Exits non-zero on any failure, so it drops straight into CI
(see `/.github/workflows/ci.yml`).
