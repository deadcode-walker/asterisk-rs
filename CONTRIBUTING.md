# Contributing to asterisk-rs

The repository pins its development Rust toolchain in `rust-toolchain.toml`. Install
[`just`](https://just.systems/) and run `just --list`; the justfile is the contributor-facing command
map and delegates build and dependency ownership to Cargo.

## Development evidence

Use the smallest command that proves the change:

```sh
just test unit::ari::build_default_config
just check
```

Before requesting review for a frozen candidate, run:

```sh
just ci
```

Protocol claims that require a real PBX also need `just live` against an explicitly selected isolated
test instance. Mock tests remain development evidence. See `AGENTS.md` and `docs/README.md` for the
proof and ownership maps.

## Changes

- Keep one coherent outcome per pull request and preserve unrelated work.
- Add behavior-level tests for observable changes and failure semantics.
- Public API changes require `just semver` evidence and migration guidance.
- Update affected guides, rustdoc, examples, reliability/security notes, and generated references in
  the same change. Run `just docs`; do not hand-edit generated protocol tables.
- Use conventional commit prefixes: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `perf:`, `test:`,
  `ci:`, or `build:`. Mark breaking changes explicitly.

release-plz turns reviewed commits into the per-crate changelogs and umbrella release notes. Ordinary
changes do not add speculative `Unreleased` entries by hand. Review the generated release PR for
package scope, user-facing wording, breaking markers, links, and duplicates before merging it.

## Pull requests and external effects

Every required CI job must pass on the reviewed commit. Permission to contribute does not imply
authority to change repository settings, merge, publish packages, create a release, or close external
records; those actions follow the documented repository policy and evidence.

Participation is governed by the [Code of Conduct](CODE_OF_CONDUCT.md).
