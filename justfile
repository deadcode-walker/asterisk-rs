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

# regenerate and reject stale generated documentation
docs-check:
    python3 docs/generate.py
    git diff --exit-code -- docs/src/ami/reference.md docs/src/agi/reference.md docs/src/ari/reference.md docs/src/types.md docs/src/SUMMARY.md
    cargo doc --locked --workspace --all-features --no-deps
    mdbook build docs/

# regenerate and build rustdoc plus mdBook
docs:
    python3 docs/generate.py
    cargo doc --locked --workspace --all-features --no-deps
    mdbook build docs/

# validate repository knowledge and dependency boundaries
harness:
    python3 scripts/check_harness.py

# validate GitHub Actions syntax and security policy
workflows:
    actionlint
    zizmor --persona=pedantic .github/workflows

# frozen-candidate local gate
ci: check test-workspace test-minimal test-features supply-chain docs-check harness workflows
    typos

# run every ignored live test against an explicitly selected Asterisk instance
live:
    #!/usr/bin/env bash
    set -euo pipefail
    if docker compose -f tests/docker-compose.yml ps --status running --services 2>/dev/null | grep -qx asterisk; then
        # the repository service is isolated and publishes only known loopback ports
        # inherited environment variables must not redirect this mutation-capable suite elsewhere
        export ASTERISK_TEST_ALLOW_MUTATION=1
        export ASTERISK_AMI_HOST=127.0.0.1
        export ASTERISK_AMI_PORT=5038
        export ASTERISK_ARI_HOST=127.0.0.1
        export ASTERISK_ARI_PORT=8088
    else
        if [[ "${ASTERISK_TEST_ALLOW_MUTATION:-}" != 1 ]]; then
            echo "live tests mutate PBX state; set ASTERISK_TEST_ALLOW_MUTATION=1 for the selected isolated instance" >&2
            exit 1
        fi
        : "${ASTERISK_AMI_HOST:?ASTERISK_AMI_HOST is required when the repository Asterisk service is not running}"
        : "${ASTERISK_AMI_PORT:?ASTERISK_AMI_PORT is required when the repository Asterisk service is not running}"
        : "${ASTERISK_ARI_HOST:?ASTERISK_ARI_HOST is required when the repository Asterisk service is not running}"
        : "${ASTERISK_ARI_PORT:?ASTERISK_ARI_PORT is required when the repository Asterisk service is not running}"
    fi
    cargo test-live

# own the isolated Asterisk lifecycle and prove that live tests actually execute
live-ci:
    #!/usr/bin/env bash
    set -euo pipefail
    cleanup() {
        status=$?
        if [[ $status -ne 0 ]]; then
            docker compose -f tests/docker-compose.yml logs --no-color || true
        fi
        docker compose -f tests/docker-compose.yml down --volumes --remove-orphans || true
        exit $status
    }
    trap cleanup EXIT INT TERM
    docker compose -f tests/docker-compose.yml up --build --detach --wait
    timeout 30 bash -c 'until nc -z 127.0.0.1 5038 && curl --fail --silent --user testuser:testpass http://127.0.0.1:8088/ari/asterisk/info >/dev/null; do sleep 1; done'
    listed="$(cargo test --locked -p asterisk-rs-tests --test live_integration --features integration -- --list)"
    count="$(awk '/tests, 0 benchmarks$/ { print $1 }' <<<"$listed")"
    if [[ -z "$count" || "$count" -eq 0 ]]; then
        echo "live test discovery returned zero tests" >&2
        exit 1
    fi
    export ASTERISK_TEST_ALLOW_MUTATION=1
    export ASTERISK_AMI_HOST=127.0.0.1
    export ASTERISK_AMI_PORT=5038
    export ASTERISK_ARI_HOST=127.0.0.1
    export ASTERISK_ARI_PORT=8088
    cargo test-live

# show compatible lockfile updates without writing
outdated:
    cargo update --dry-run
