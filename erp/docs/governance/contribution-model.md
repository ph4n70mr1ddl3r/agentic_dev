# Contribution Model

**Status:** Phase 0 · **Owner:** Tech Lead · **Stability:** foundational

How an artifact moves from idea to merged. This is the workflow the `forge`
harness automates. It mirrors a real software company and is intentionally
strict — structure is our substitute for model strength
(see [ADR-0005](../adr/0005-gated-delivery-and-weak-model-strategy.md)).

## 1. The unit of work: an issue

Every artifact starts as a **GitHub issue**.

- **Title:** `[<ROLE>] <artifact-id>: <short description>`, e.g.
  `[DM] MOD-FIN-GL: General Ledger entities & forms`.
- **Labels:** `role:<hat>` (exactly one), `phase:<n>`, `type:<code>`
  (from the [artifact catalog](artifact-catalog.md)), and a `module:<…>` where
  relevant.
- **Body must contain:** the artifact ID, inputs (links to upstream artifacts it
  depends on), the **definition of ready**, and the **definition of done**.

## 2. Lifecycle (the PR loop)

```
issue (DoR met)
  └─ branch:  <type>/<artifact-id>-<slug>      e.g. mod/MOD-FIN-GL-general-ledger
      └─ author artifact on the branch
          └─ open PR  ──► CI (lint/test/schema-validate)
              ├─ pass ─► Reviewer hat review ─► (fix loop) ─► approve ─► merge ─► close issue
              └─ fail ─► feedback to author agent ─► loop
```

- **One artifact per PR.** Atomic, reviewable.
- **Branch base:** `main` for docs; per-phase integration branches (`phase/3`)
  where the harness groups work.
- **Squash-merge** with a conventional commit:
  `<type>(<scope>): <summary>` e.g. `mod(fin-gl): add General Ledger entities`.

## 3. Definition of Ready (DoR)

An issue may be picked up only when:
- [ ] It has a `type:` and `role:` label.
- [ ] All **input** artifacts are `accepted`.
- [ ] DoD is explicit and checkable.
- [ ] For `MOD`: the relevant `platform-spec` schema is `accepted`.

## 4. Definition of Done (DoD)

A PR merges only when:
- [ ] Artifact follows its [template](../templates/).
- [ ] Header carries `Status:` and an owner hat.
- [ ] All links/references resolve.
- [ ] CI is green (lint, tests, schema validation where applicable).
- [ ] **Reviewer hat** (≠ author hat — segregation) approves.
- [ ] For code: unit tests + the service's contract test pass.
- [ ] For `MOD`: loader/validator dry-run passes (referential integrity OK).

## 5. Segregation of authorship

The agent that authors (R) an artifact **must not** be the agent that approves
(A) it. The harness enforces this by assigning the review to a different hat.
For financial artifacts, the Security/BA hat additionally checks the SoD matrix.

## 6. Gates between phases

At each phase gate the Product Owner reviews the phase's exit-gate artifacts
(see [project plan](project-plan.md)) and tags a milestone (`M0`…`v1.0`). No
issue from a later phase may start until the prior gate is approved. This is the
primary safety control for a weak model on a financial system.

## 7. Resumability

Because the entire project state lives in GitHub + this repo, the `forge`
harness can resume at any time: open issues = remaining work, merged PRs =
done work, tags = milestones. No out-of-band state.

## 8. Conventions cheat-sheet

| Thing | Convention |
|---|---|
| Issue title | `[ROLE] ID: description` |
| Branch | `<type>/<id>-<slug>` |
| Commit | `<type>(<scope>): summary` |
| PR | one artifact, links the issue (`Closes #N`) |
| Labels | `role:` `phase:` `type:` `module:` |
| Milestones | `M0`…`M5`, `v1.0` |
