# Agent map

This file routes work in asterisk-rs. Keep durable detail in the linked source of truth and enforce
stable rules mechanically.

## Read first

1. Read [the project decision brief](docs/design-docs/project-decision-brief.md) for outcomes,
   constraints, and deliberate tooling choices.
2. Read [ARCHITECTURE.md](ARCHITECTURE.md) for code ownership, runtime flows, and dependency
   direction.
3. Read the relevant mdBook protocol guide under `docs/src/` before changing public behavior.
4. For complex or multi-session work, follow [docs/PLANS.md](docs/PLANS.md) and maintain one plan
   under `docs/exec-plans/active/`.

## Command surface

Run `just --list` for the complete facade.

- `just check`: bounded development gate: format, Clippy, unit tests, and mock integration tests.
- `just test unit::ari::test_name`: run a focused external test by filter.
- `just ci`: frozen-candidate local gate including workspace/features, docs, supply chain, and harness.
- `just msrv`: compile and test on the declared minimum Rust version.
- `just live`: Asterisk-backed protocol tests; requires `tests/docker-compose.yml` services.
- `just docs`: regenerate references and build rustdoc plus mdBook.

Cargo.toml and Cargo.lock are the only dependency/build graph. The justfile is a thin facade, not a
second policy implementation.

## Work loop

1. Inspect Git status and preserve unrelated changes.
2. Trace the complete protocol path before editing.
3. Name the observable outcome, side effects, failure behavior, and proof.
4. Make the smallest coherent change and delete the replaced path once proved.
5. Run focused evidence while iterating, then `just check`.
6. On a frozen candidate, run `just ci` and `just live` when Docker/Asterisk is available.
7. Inspect the exact diff and update affected canonical docs.

## Non-negotiable boundaries

- The umbrella crate may compose protocol crates; protocol crates depend only on core and never on
  one another.
- All I/O is asynchronous on Tokio. Do not introduce a sync client or runtime abstraction casually.
- `unsafe` is forbidden. Do not weaken workspace lints to land a change.
- Credentials and secrets remain redacted from Debug output and logs.
- Reject CR/LF injection in line protocols and percent-encode user-controlled URL path/query data.
- Tests live in the external `tests` crate; production modules do not contain `#[cfg(test)]` trees.
- Public API changes require compatibility review and semver evidence.
- Mock tests are development evidence. Claims about Asterisk behavior require `just live` or an
  explicitly documented unavailable external gate.

## Documentation map

- `docs/README.md`: repository knowledge and ownership index.
- `docs/src/`: public user guide and generated protocol references.
- `docs/design-docs/`: approved engineering decisions.
- `docs/product-specs/`: user-visible behavior map.
- `docs/exec-plans/`: active/completed complex work and concrete debt.
- `docs/QUALITY_SCORE.md`, `docs/RELIABILITY.md`, `docs/SECURITY.md`: evidence, risks, and gates.
