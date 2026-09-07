# Claim Lands

A procedural planetary strategy game for iOS, Android, and browser WebAssembly.

The foundation contains a Rust hexagonal planet with seed/size controls, camera orbit,
zoom, and tile selection. Gameplay is planned; this is not yet a playable strategy match.
The browser runs the same Rust application used by the mobile adapters.

## Run

Install Rust with rustup, Python 3.12+, Node 22, and the pinned tools:

```sh
rustup show
cargo install wasm-bindgen-cli --version 0.2.128 --locked
cargo install cargo-about --version 0.9.2 --features cli --locked
npm ci
cargo xtask web
npm run serve
```

Open `http://localhost:4173`. Drag to orbit, scroll/pinch to zoom, and select a tile.
The in-game controls change seed and size. Keyboard: `N` next seed, `0` ocean, `+/-` size.
Use `?backend=webgl` or `?backend=webgpu` to require a particular backend during testing.

```sh
cargo xtask check
python -m unittest discover -s tools -p 'test_*.py'
npx playwright install chromium
npm test
```

Mobile build instructions live in [platform](platform/README.md).
CI also measures coverage, tests mutations, executes browser contracts, builds both
mobile packages, and validates the deployed game. Local checks alone do not authorize a merge.

## Work on a feature

[GitHub Issues](https://github.com/pixbs/claimlands-astro/issues) define ownership,
dependencies, acceptance checks, and unresolved decisions.

```sh
cargo xtask task prepare --issue 123 --role implementer
```

Give the resulting brief to the agent explicitly. Read [AGENTS.md](AGENTS.md) and
[contributing](docs/contributing.md) for naming, worktrees, stacked PRs and manual acceptance.

| Reference | Purpose |
| --- | --- |
| [Architecture](docs/architecture.md) | Dependency map and public contracts |
| [Rules](docs/rules.md) | Authoritative game behavior and open mechanics |
| [Decisions](docs/decisions.md) | Stack and format rationale |
| [Testing](docs/testing.md) | Required gates and failure handling |
| [Roadmap](docs/roadmap.md) | Milestones and issue scope |

The external HTML prototype is a visual reference only. Its runtime is never bundled.
Exact appearance is developed and reviewed through separate fidelity milestones.

Original work © 2026 pixbs, licensed [CC BY-NC-SA 4.0](LICENSE).
Dependency licenses remain separate; see [THIRD_PARTY.md](THIRD_PARTY.md).
