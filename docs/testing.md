# Testing

Automated checks are required evidence for merging. Agent review and owner preview review supplement them; neither replaces a failing or unavailable check. A configured workflow is not evidence that it has passed.

## Strategy

Use many small deterministic behavior tests, subsystem integration tests, and a focused set of critical game journeys. Exercise public contracts and observable state, not private call sequences. Control random seeds and time, isolate fixtures, and avoid live services or fixed sleeps in ordinary tests. Every bug fix includes a reproducer.

This follows Google's guidance on [qualification](https://testing.googleblog.com/2021/06/how-much-testing-is-enough.html), [public behavior](https://abseil.io/resources/swe-book/html/ch12.html), and [self-contained test environments](https://abseil.io/resources/swe-book/html/ch23.html). Coverage identifies untested code; it does not prove correct assertions or absence of regressions.

## Required gates

| Gate | Acceptance |
| --- | --- |
| Static checks | Formatting, Clippy, dependency boundaries, public documentation and dependency/license policy pass. |
| Native behavior | Unit, integration and property tests pass with nonzero discovery. Run documentation examples separately when using nextest. |
| Coverage | At least 90% executable line coverage in each pure `world`, CPU `visuals`, and later `gameplay` crate; at least 90% of changed executable domain lines. Publish uncovered lines. |
| Mutation | For changed domain code, viable mutations are caught by tests. Missed mutations and timeouts fail for investigation; an equivalent mutation needs a narrow owner-reviewed exclusion. |
| Browser | Shared world contracts execute as Wasm; the actual game starts, renders, rotates, zooms, selects and regenerates through WebGL2 and WebGPU. |
| Visual | Fixed scene comparisons pass against owner-approved baselines in the pinned browser/backend environment. |
| Platforms | Android package and unsigned iOS application build. Add emulator/simulator lifecycle smoke tests with the platform packaging. Compilation is not runtime validation. |
| Publication | Metadata policy passes for all introduced commits and current PR metadata; deployed game revision matches the reviewed head and smoke tests pass. |

The coverage percentages are project defaults for the portable core, not a universal rule from Google. Do not impose the same percentage on GPU/platform adapters or generated glue. Existing generated/test-code exclusions are explicit; new exclusions and threshold reductions require owner review. [Google coverage guidance](https://testing.googleblog.com/2020/08/code-coverage-best-practices.html)

Use [`cargo-nextest`](https://nexte.st/) for native execution, plus `cargo test --doc`; [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) exports LCOV to [`diff-cover`](https://github.com/Bachmann1234/diff_cover) for changed-line coverage against the actual PR base. Branch coverage remains an optional pinned-nightly report while its instrumentation is unstable.

Use [`cargo-mutants --in-diff`](https://mutants.rs/in-diff.html) for changed domain lines after a successful baseline run. A timeout is inconclusive, not a caught mutation. Keep full ordinary domain tests even when mutation work is filtered. Run full mutation tests and longer [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) jobs nightly; fuzzing uses supported Linux/nightly tooling. Initial fuzz targets cover RON parsing/validation; later targets cover commands and replays.

## Available local checks

```text
cargo xtask check
cargo xtask boundaries
cargo xtask web
python -m unittest discover -s tools -p test_*.py
npm test
```

`cargo xtask check` currently invokes formatting, boundary validation, Clippy and Cargo tests. It does not stand in for coverage, mutation, mobile builds or a deployed preview. `cargo xtask web` requires the pinned Wasm target and compatible `wasm-bindgen` CLI, then validates the deployment artifact. Browser tests require installed Playwright browsers and the generated `dist` build; `PREVIEW_URL` selects a deployment and `EXPECTED_REVISION` checks its revision.

Existing world tests cover topology sizes, pentagons, adjacency/connectivity, repeatability, RON round trips, invalid inputs and seed-zero overrides. CPU visual tests check bounded buffers, indices, finite data and repeatable pixels. Browser tests exercise both graphics backends and a foundation screenshot. A foundation screenshot is not a prototype-fidelity approval.

## Game regression cases

Add these with their behavior, after unresolved rules are decided:

- Split/merge treasury conservation, exactly one capital per retained territory, and loss of components with no eligible capital tile.
- Income, local build prices, starvation priority, partially affordable town production, and rejected purchases leaving state untouched.
- Legal movement/capture by unit class, no allied stacking, upgrade consuming a move, and faction-wide turn availability.
- Command followed by undo restores exact state, randomness and statistics; committed turns cannot be undone.
- The same initial board and commands produce identical state hashes natively and in browser Wasm.

Use [`proptest`](https://proptest-rs.github.io/proptest/proptest/state-machine.html) for generated command sequences. Persist failure seeds and check important minimized cases in as explicit fixtures; changed strategies can change seed interpretation. Keep future RON migration fixtures alongside the reader that supports them.

## Failure handling

Preserve logs, seeds, screenshots and traces. Reruns diagnose failures; they must not turn a fail-then-pass into a green gate. A temporary quarantine needs an issue, owner and expiry, and cannot silently remove a critical journey. Check thresholds, metadata rejection, missing artifacts and stale previews using intentionally failing fixtures. Scheduled failures become tracked defects with reproducible inputs.

Measure generation time, memory, download size and frame time. Set performance budgets from actual target-device baselines. Review physical iOS/Android appearance, touch input and lifecycle before releases; a software browser renderer cannot establish device parity.
