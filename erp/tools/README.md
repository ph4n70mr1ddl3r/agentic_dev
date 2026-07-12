# `tools/` — CLI helpers

*(Phase 2/3)*

- `metadata-loader/` — loads + validates module JSON against `platform-spec/`
  schemas; performs referential-integrity checks (every field ref exists, every
  workflow transition's states exist, every action is in the curated vocabulary);
  applies as versioned migrations. **This is the QA gate for `MOD` artifacts.**
- misc. dev CLI utilities.

Written in Node.js/TypeScript to stay in the same stack as the platform.
