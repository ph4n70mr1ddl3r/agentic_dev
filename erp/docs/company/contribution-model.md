# Contribution Model

_Authored by the CEO hat via `forge`._

All work flows through GitHub: tasks are created as issues with labels for phase and type (spec, code, metadata, test, doc). Each issue is assigned to a single hat. Work begins by creating a branch from the issue. All changes must be in a pull request (PR) referencing the issue; PRs must pass automated CI (lint, test, schema validation) and be reviewed by a different hat than the author. Approved PRs are merged to the main branch. Milestones (via GitHub tags) correspond to phases; each PR must be tagged with the phase milestone. The CEO or Product Owner gates phase transitions by verifying phase exit criteria and closing the milestone.
