# Agent map

This root guide applies repository-wide; there are currently no nested instruction files. Keep
durable detail in its canonical owner and enforce stable rules mechanically.

## Read first

1. Read [the project decision brief](docs/design-docs/project-decision-brief.md) for outcomes,
   authority, target support, and deliberate tooling choices.
2. Read [ARCHITECTURE.md](ARCHITECTURE.md) for code ownership, useful paths, and dependency direction.
3. Read the relevant public guide under `docs/src/` before changing protocol behavior.
4. For complex, risky, or multi-session work, follow [docs/PLANS.md](docs/PLANS.md) and maintain the
   single applicable plan under `docs/exec-plans/active/`.

## Commands

Run `just --list` for the facade. Cargo manifests and `Cargo.lock` are the only build and dependency
graph; recipes delegate to them.

- `just test <filter>`: focused external unit or mock test.
- `just check`: formatting, strict Clippy, unit tests, and mock integration tests.
- `just ci`: frozen-candidate local gate, including features, docs, supply chain, and harness checks.
- `just msrv`: compile and test on the declared minimum Rust version.
- `just semver`: compare public APIs with the latest published releases.
- `just live`: mutation-capable Asterisk proof against an explicitly selected isolated test PBX.
- `just docs`: regenerate references and build rustdoc plus mdBook.

## Work contract

For a planned slice, record its outcome, non-goals, authority, baseline, assumptions, and acceptance
evidence. Preserve unrelated work, trace the complete useful path, change the smallest causal owner,
and delete the replaced path. Iterate with focused proof, then run `just check`. Freeze one exact tree
for `just ci`, applicable `just live` evidence, diff inspection, and fresh read-only review. Later
edits invalidate affected evidence. Update canonical documentation in the same slice.

Do not infer authority to push, close issues or pull requests, merge, publish, release, deploy, or
change repository settings from permission to edit and test locally.

## Non-negotiable boundaries

- The umbrella crate may compose protocol crates; protocol crates depend only on core, never peers.
- All I/O is asynchronous on Tokio. Do not add a sync client or runtime abstraction casually.
- `unsafe` is forbidden. Do not weaken workspace lints to land a change.
- Keep credentials redacted; reject CR/LF injection; percent-encode user-controlled URL components.
- Tests live in the external `tests` crate, not production `#[cfg(test)]` modules.
- Public API changes require compatibility review and semver evidence.
- Mock evidence is developmental. Asterisk behavior requires `just live` or an explicit unavailable gate.

The canonical knowledge, documentation, changelog, and ownership map is [docs/README.md](docs/README.md).
