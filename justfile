set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# compile every workspace target and feature
build:
    cargo build --workspace --all-targets --all-features

# format Rust sources
fmt:
    cargo fmt --all

# verify formatting without writing
fmt-check:
    cargo fmt --all -- --check

# lint every workspace target and feature
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# run unit and mock integration tests, optionally filtered
test filter="":
    cargo test -p asterisk-rs-tests --test unit --test mock_integration -- {{filter}}

# test publishable crates with all features
test-workspace:
    cargo test --workspace --all-features --exclude asterisk-rs-tests

# prove feature gates without defaults
test-minimal:
    cargo test --workspace --no-default-features --exclude asterisk-rs-tests

# bounded development gate
check: fmt-check lint test

# verify the declared minimum Rust version
msrv:
    cargo +1.86.0 test --workspace --all-features

# check advisories, licenses, duplicate versions, and sources
supply-chain:
    cargo deny check

# regenerate and reject stale generated documentation
docs-check:
    python3 docs/generate.py
    git diff --exit-code -- docs/src/ami/reference.md docs/src/agi/reference.md docs/src/ari/reference.md docs/src/types.md docs/src/SUMMARY.md
    cargo doc --workspace --all-features --no-deps

# regenerate and build rustdoc plus mdBook
docs:
    python3 docs/generate.py
    cargo doc --workspace --all-features --no-deps
    mdbook build docs/

# validate repository knowledge and dependency boundaries
harness:
    python3 scripts/check_harness.py

# frozen-candidate local gate
ci: check test-workspace test-minimal supply-chain docs-check harness
    typos

# run tests against a caller-managed Asterisk instance
live:
    cargo test-live

# show compatible lockfile updates without writing
outdated:
    cargo update --dry-run
