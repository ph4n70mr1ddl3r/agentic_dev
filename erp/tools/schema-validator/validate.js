#!/usr/bin/env node
// CI gate for the model-driven contract (ADR-0005: schema-validation is a hard
// gate). Two checks:
//   1. every platform-spec schema compiles in the registry (cross-file $ref OK);
//   2. every example, and every $kind-tagged generated artifact, validates
//      against its schema.
// Exits non-zero on any failure.
const Ajv2020 = require('ajv/dist/2020');
const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..', '..');
const SCHEMA_DIR = path.join(root, 'platform-spec', 'schemas');
const EX_DIR = path.join(root, 'platform-spec', 'examples');
const GEN_DIR = path.join(root, 'modules', 'generated');
const ID = 'https://agentic.dev/platform-spec/schemas/';

const ajv = new Ajv2020({ allErrors: true, strict: false });

let failures = 0;

// (1) register every schema; addSchema compiles and resolves cross-file $refs.
for (const f of fs.readdirSync(SCHEMA_DIR).filter((f) => f.endsWith('.schema.json'))) {
  const schema = JSON.parse(fs.readFileSync(path.join(SCHEMA_DIR, f), 'utf8'));
  try {
    ajv.addSchema(schema);
  } catch (e) {
    console.error(`COMPILE FAIL  ${f}: ${e.message}`);
    failures++;
  }
}

function check(label, schemaFile, data) {
  const validate = ajv.getSchema(ID + schemaFile);
  if (!validate) {
    console.error(`NO SCHEMA     ${label} -> ${schemaFile}`);
    failures++;
    return;
  }
  if (validate(data)) {
    console.log(`PASS          ${label}`);
  } else {
    console.log(`FAIL          ${label}`);
    console.log('  ' + ajv.errorsText(validate.errors, { separator: '\n  ' }));
    failures++;
  }
}

// (2a) examples validate against their same-named schema.
for (const ex of fs.readdirSync(EX_DIR).filter((f) => f.endsWith('.json'))) {
  const data = JSON.parse(fs.readFileSync(path.join(EX_DIR, ex), 'utf8'));
  check(`examples/${ex}`, ex.replace(/\.json$/, '.schema.json'), data);
}

// (2b) $kind-tagged generated artifacts validate against their kind's schema.
if (fs.existsSync(GEN_DIR)) {
  for (const f of fs.readdirSync(GEN_DIR).filter((f) => f.endsWith('.json'))) {
    const data = JSON.parse(fs.readFileSync(path.join(GEN_DIR, f), 'utf8'));
    const kind = data['$kind'];
    if (!kind) continue; // entities/workflows carry no $kind tag
    check(`modules/generated/${f}`, `${kind}.schema.json`, data);
  }
}

console.log(failures ? `\n${failures} FAILURE(S)` : '\nall green');
process.exit(failures ? 1 : 0);
