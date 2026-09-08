# Decisions

Keep accepted architecture choices here; use GitHub Issues for implementation status. Change a decision with its reason, compatibility impact and affected tests. Open mechanics are explicit blockers, not permission to infer new game rules.

## Accepted foundation choices

| Choice | Reason and consequence |
| --- | --- |
| Rust workspace, edition 2024 | Shared procedural generation, simulation and application code; private state and crate dependencies enforce boundaries. Toolchain/dependencies are pinned in manifests and lockfiles. |
| wgpu/WGSL | A shared renderer with Metal on iOS, Vulkan/GLES on Android and WebGPU/WebGL2 in browsers. Keep the common rendering path within WebGL2 capabilities. |
| winit | Provides surface/window handles, touch/input events and application lifecycle on mobile and the browser. It is not a Windows-only dependency. There is no Windows product build. |
| egui for development controls | Reuses the Rust renderer; product interface design can evolve without moving game rules into UI code. |
| RON + Serde for levels | Readable Rust-oriented data with comments, enums and a compact serialization. Level types are deliberate versioned contracts, not unrestricted serialization of runtime internals. |
| Immutable resolved initial world | Rendering cannot change occupation or economy. A future simulation owns mutable session state through commands. |
| Modular monolith | Separate real subsystem boundaries now; add gameplay, AI/editor modules when first behavior exists. Avoid empty plugin systems and premature distributed services. |
| GitHub Issues + stacked PRs | One owning issue/worktree per change, dependent contracts merged first, independent review and owner acceptance of each current revision. |
| GitHub Actions + Cloudflare Pages | Build and test the real Wasm game for PR previews and `main` deployments. A workflow file alone does not prove remote setup or deployment. |
| CC BY-NC-SA 4.0 for original work | Attribute original project work to `pixbs`; preserve third-party licenses separately. |

RON v1 currently rejects unknown fields/versions, inputs above 1 MiB, recursion deeper than 64, frequencies outside 2–12, duplicate/out-of-range factions, AI values outside 0–100, invalid tile IDs, owned/improved water and unowned capitals. These are reader contracts; future migrations must add compatibility fixtures. Seed zero means all water before overrides, allowing a campaign editor to start blank and place land.

## Mechanics to settle before their implementation

| ID | Decision needed | Blocks |
| --- | --- | --- |
| D1 | Turn settlement order: when income is credited; upkeep, starvation and town conversion; knight gold shortages; equal-tier starvation order; whether newly bought/upgraded units act immediately | Economy, starvation, turn engine |
| D2 | Capital capture: whether 25% applies to gold only or wheat too; rounding; source/destination treasury; split proportions and remainder allocation; ordering of loot, replacement and splitting | Capture, treasury conservation |
| D3 | Capital relocation center/distance and seeded candidate choice; eligible unit-occupied tiles; merge survivor precedence between proximity and age; equal-age/equal-distance ties | Territory components, capital replacement/merge |
| D4 | Movement route versus final capture distance; passing occupied cells; terrain/passability; higher-tier movement and improvement-capture inheritance; where units may be purchased | Units, pathfinding, legal actions |
| D5 | Forest spreading cadence and probability scope; owned versus unowned targets; simultaneous attempts and chaining; interaction with units | Forest growth |
| D6 | AI difficulty behavior/budget; decisive-advantage threshold; elimination definition; turn/statistic accounting; starting treasuries/units and campaign metadata | AI, campaign, victory, saves |

Resolve each in its prerequisite design issue with concrete examples and expected outcomes. The existing split and town-production examples in [rules.md](rules.md) must remain acceptance fixtures unless the owner explicitly changes those rules.

## Enforcement limits

The uppercase prompt header is a reminder, not a security boundary. Local hooks can be disabled. Required checks and repository rules must run on trusted policy and current metadata. Under direct push, GitHub can receive an offending commit before a check blocks its merge; a check cannot erase that initial arrival. Native branch/tag restrictions can reject prohibited ref creation where available.

Do not claim complete prevention while agents share owner credentials or bypass privileges. Restrict publish credentials, disable routine bypass and automated merging, and verify the actual repository settings. Remote credentials, signing and physical-device access are deployment prerequisites rather than source-code features.

## Sources

[winit platform support](https://docs.rs/winit/latest/winit/), [wgpu](https://docs.rs/wgpu/latest/wgpu/), [RON](https://github.com/ron-rs/ron), [GitHub rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets), [Cloudflare previews](https://developers.cloudflare.com/pages/configuration/preview-deployments/), [license terms](https://creativecommons.org/licenses/by-nc-sa/4.0/).
