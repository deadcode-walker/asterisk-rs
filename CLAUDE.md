# asterisk-rs

See [AGENTS.md](./AGENTS.md) for full project guidelines, architecture, conventions, and commands.

## Agent Instructions

- after editing any `.rs` file, run `cargo test -p asterisk-rs-tests --test unit` to catch regressions fast
- after editing codec, connection, or transport modules, also run `cargo test -p asterisk-rs-tests --test mock_integration`
- use `cargo clippy --workspace --all-targets --all-features -- -D warnings` before committing
- all tests live in the external `tests/` crate — never add `#[cfg(test)]` to production code
- breaking API changes require a version bump (CI runs cargo-semver-checks on PRs)
