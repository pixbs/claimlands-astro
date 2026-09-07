# Roadmap

GitHub Issues are the source of work status. This page defines milestone outcomes and dependencies; it does not assert that an issue, check or deployment already exists.

## Milestones

| Milestone | Deliverable and acceptance |
| --- | --- |
| 1. Foundation | Original procedural world and terrain viewer, RON contracts, mobile/web entrypoints, architecture boundaries, task prompts, automated gates, repository protections and actual game previews. Complete only after builds/checks and published previews are verified. |
| 2. Visual fidelity | Approved reference captures; terrain/coasts/foam; connected fields; forest and house models; atmosphere/clouds/stars. Each stage compares fixed reference scenes and has CPU asset checks. New unit/capital/action-indicator designs require separate approval. |
| 3. Simulation | Resolve D1–D5; implement connected territories/capitals, local economy, units/combat, faction turns and exact undo. Commands, invariants and deterministic native/Wasm fixtures cover feature interactions. |
| 4. Single-player MVP | Resolve D6; difficulty-controlled AI, internal level editor, campaign, victory/statistics and persistence. Complete representative full-game journeys on browser and mobile. |
| Later capabilities | Replay viewer, science tree, public level editing/sharing and multiplayer. Reuse versioned commands and level contracts; do not prebuild these systems inside foundation work. |

Current source includes initial world/RON/CPU assets, rendering/input and contributor tools. It is not a finished strategy game and has not established visual identity with the reference. Remote checks, deployments and mobile runtime results must be reported from actual runs.

## Initial issue breakdown

Create separately owned issues with these outcomes, then assign actual numbers and dependencies in GitHub:

1. Verify remote protections, publication rejection and the task-prompt workflow using positive and negative cases.
2. Verify PR/main game artifacts, immutable preview links, revision checks and trusted deployment credential separation.
3. Verify Android package and unsigned iOS builds; add emulator/simulator input and suspend/resume smoke tests.
4. Establish coverage/mutation gates, RON fuzz targets, persisted failure artifacts and target-device performance baselines.
5. Capture and approve prototype reference scenes; document camera, seed, time and pixel settings.
6. Port each visual group through separate PRs: terrain/coasts, fields, forest/houses, atmosphere/clouds/stars.
7. Resolve D1–D5 with examples, then implement territory/capital transactions, economy, units and turns in dependency order.
8. Add generated command-sequence tests for combined rules, exact undo and cross-platform determinism.
9. Resolve D6, then implement AI, the internal RON editor, campaign content, victory/statistics and versioned saves.

## Issue contract

Each issue contains:

- **Problem and outcome:** concrete current behavior and the expected result.
- **Owner and area:** one subsystem owner; name any shared-contract changes.
- **Prerequisites:** blocking issue/PR links and unresolved rule IDs.
- **Acceptance:** observable behavior, required tests and the actual-game preview expectation.

Use labels for area, priority and type; milestones express delivery goals. Project status is `Ready`, `In progress`, `In review`, `Blocked` or `Done`. An issue is ready only when its material rule decisions and prerequisites are resolved. Split work where separate agents can own independent contracts; do not split one atomic gameplay transition across uncoordinated PRs.

Reference the issue from code TODOs and PRs. State `Closes #N` only when the PR meets the issue's complete acceptance criteria; a stacked prerequisite should link dependent work without prematurely closing it. Scheduled test failures become defects with their failing revision, seed/input and reproducer. Close an implementation issue after the accepted revision merges with its required evidence.
