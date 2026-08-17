set shell := ["bash", "-euo", "pipefail", "-c"]
set windows-shell := ["powershell.exe", "-NoLogo", "-NonInteractive", "-Command"]

default:
    @just --list

# compile every workspace target and feature
build:
    cargo build --locked --workspace --all-targets --all-features

# format Rust sources
fmt:
    cargo fmt --all

# verify formatting without writing
fmt-check:
    cargo fmt --all -- --check

# lint every workspace target and feature
lint:
    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

# run unit and mock integration tests, optionally filtered
test filter="":
    cargo test --locked -p asterisk-rs-tests --test unit --test mock_integration -- {{ filter }}

# report informative protocol throughput and admission measurements
bench:
    cargo bench --locked -p asterisk-rs-tests --bench protocol

# test publishable crates with all features
test-workspace:
    cargo test --locked --workspace --all-features --exclude asterisk-rs-tests

# prove feature gates without defaults
test-minimal:
    cargo test --locked --workspace --no-default-features --exclude asterisk-rs-tests

# prove each public umbrella feature independently
test-features:
    cargo check --locked -p asterisk-rs --all-targets --no-default-features
    cargo check --locked -p asterisk-rs --all-targets --no-default-features --features ami
    cargo check --locked -p asterisk-rs --all-targets --no-default-features --features agi
    cargo check --locked -p asterisk-rs --all-targets --no-default-features --features ari
    cargo check --locked -p asterisk-rs-core --all-targets --no-default-features

# compile the intended public 0.8 API from an external crate boundary
downstream:
    cargo check --locked -p asterisk-rs-tests --test downstream_api

# run the behavior and publishable-crate proof used on every supported runner
platform: test test-workspace

# bounded development gate
check: fmt-check lint test

# verify the declared minimum Rust version
msrv:
    cargo +1.86.0 test --locked --workspace --all-features --no-run
    cargo +1.86.0 test --locked -p asterisk-rs-tests --test unit --test mock_integration

# check advisories, licenses, duplicate versions, and sources
supply-chain:
    cargo deny check
    cargo shear --deny-warnings

# check public API compatibility against the latest published releases
semver:
    cargo semver-checks --workspace --exclude asterisk-rs-tests

# reject new undocumented public API
missing-docs:
    python3 scripts/check_missing_docs.py --self-test

# compile the representative public snippets used by the documentation
docs-snippets:
    cargo test --locked -p asterisk-rs-tests --test documentation_snippets

# build and verify public documentation
docs-check: missing-docs docs-snippets
    cargo doc --locked --workspace --all-features --no-deps
    mdbook build docs/

# build rustdoc plus mdBook
docs:
    cargo doc --locked --workspace --all-features --no-deps
    mdbook build docs/

# validate repository knowledge and dependency boundaries
harness:
    python3 scripts/check_harness.py
    python3 scripts/check_protocol_contracts.py
    python3 scripts/check_live_runner.py

# verify pinned protocol artifacts against the exact upstream Asterisk commit
protocol-contracts-upstream:
    python3 scripts/check_protocol_contracts.py --verify-upstream

# validate GitHub Actions syntax and security policy
workflows:
    actionlint
    zizmor --persona=pedantic .github/workflows

# frozen-candidate local gate
ci: check test-workspace test-minimal test-features downstream supply-chain docs-check harness workflows
    typos

# run the representative live boundary against an explicitly selected owned instance
live-smoke:
    scripts/run-live-tests.sh smoke attach

# run every exhaustive live test against an explicitly selected owned instance
live-full:
    scripts/run-live-tests.sh full attach

# compatibility name for the exhaustive live boundary
live: live-full

# own the isolated Asterisk lifecycle and run the representative live boundary
live-smoke-ci:
    scripts/run-live-tests.sh smoke compose

# own the isolated Asterisk lifecycle and run the exhaustive live boundary
live-full-ci:
    scripts/run-live-tests.sh full compose

# compatibility name for the exhaustive Compose-owned live boundary
live-ci: live-full-ci

# show compatible lockfile updates without writing
outdated:
    cargo update --dry-run
