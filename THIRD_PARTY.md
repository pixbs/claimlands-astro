# Dependency licenses

Original project work is CC BY-NC-SA 4.0. This does not relicense dependencies.
Cargo.lock and package-lock.json identify the exact dependency versions.
`cargo deny check` validates the Rust dependency policy. Every web build runs the
pinned `cargo-about` generator and ships full notices in `THIRD_PARTY.txt`, including
the original-work license and bundled font licenses. The browser host contains only
generated Rust bindings; development tools are not shipped in the game.

Before a release, generate and review a complete attribution bundle from the locked
dependency graph, including platform libraries. No release is authorized by this
foundation scaffold. The external visual reference and its JavaScript libraries are
excluded from product builds.
