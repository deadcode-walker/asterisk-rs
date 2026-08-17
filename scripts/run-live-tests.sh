#!/usr/bin/env bash
set -euo pipefail

suite="${1:?usage: run-live-tests.sh smoke|full attach|compose}"
lifecycle="${2:?usage: run-live-tests.sh smoke|full attach|compose}"
case "$suite" in smoke|full) ;; *) echo "unknown live suite: $suite" >&2; exit 2 ;; esac
case "$lifecycle" in attach|compose) ;; *) echo "unknown live lifecycle: $lifecycle" >&2; exit 2 ;; esac

compose_file="tests/docker-compose.yml"
owned_compose=0
cleanup() {
    status=$?
    if [[ $owned_compose -eq 1 ]]; then
        if [[ $status -ne 0 ]]; then
            docker compose -f "$compose_file" logs --no-color || true
        fi
        docker compose -f "$compose_file" down --volumes --remove-orphans || true
    fi
    exit "$status"
}
trap cleanup EXIT INT TERM

select_repository_fixture() {
    export ASTERISK_TEST_ALLOW_MUTATION=1
    export ASTERISK_TEST_INSTANCE_MARKER=asterisk-rs-owned-test-v1
    export ASTERISK_TEST_BRANCH=22
    export ASTERISK_AMI_HOST=127.0.0.1
    export ASTERISK_AMI_PORT=5038
    export ASTERISK_AMI_USERNAME=testadmin
    export ASTERISK_AMI_SECRET=testsecret
    export ASTERISK_ARI_HOST=127.0.0.1
    export ASTERISK_ARI_PORT=8088
    export ASTERISK_ARI_USERNAME=testuser
    export ASTERISK_ARI_PASSWORD=testpass
    export ASTERISK_ARI_APP=test-app
}

if [[ "$lifecycle" == compose ]]; then
    owned_compose=1
    select_repository_fixture
    docker compose -f "$compose_file" up --build --detach --wait
elif docker compose -f "$compose_file" ps --status running --services 2>/dev/null | grep -qx asterisk; then
    select_repository_fixture
else
    required=(
        ASTERISK_TEST_ALLOW_MUTATION ASTERISK_TEST_INSTANCE_MARKER ASTERISK_TEST_BRANCH
        ASTERISK_AMI_HOST ASTERISK_AMI_PORT ASTERISK_AMI_USERNAME ASTERISK_AMI_SECRET
        ASTERISK_ARI_HOST ASTERISK_ARI_PORT ASTERISK_ARI_USERNAME ASTERISK_ARI_PASSWORD
        ASTERISK_ARI_APP
    )
    for name in "${required[@]}"; do
        if [[ -z "${!name:-}" ]]; then
            echo "$name is required for an externally managed live test instance" >&2
            exit 1
        fi
    done
    if [[ "$ASTERISK_TEST_ALLOW_MUTATION" != 1 ]]; then
        echo "ASTERISK_TEST_ALLOW_MUTATION must be 1" >&2
        exit 1
    fi
fi

export ASTERISK_TEST_RUN_ID="${ASTERISK_TEST_RUN_ID:-run-${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-0}-$$}"

# Refuse mutation before both the durable marker and declared branch match.
python3 - <<'PY'
import base64
import json
import os
import urllib.parse
import urllib.request

host = os.environ["ASTERISK_ARI_HOST"]
port = os.environ["ASTERISK_ARI_PORT"]
user = os.environ["ASTERISK_ARI_USERNAME"]
password = os.environ["ASTERISK_ARI_PASSWORD"]
auth = base64.b64encode(f"{user}:{password}".encode()).decode()

def get(path):
    request = urllib.request.Request(f"http://{host}:{port}/ari/{path}")
    request.add_header("Authorization", f"Basic {auth}")
    with urllib.request.urlopen(request, timeout=10) as response:
        return json.load(response)

marker_name = urllib.parse.quote("ASTERISK_RS_TEST_INSTANCE", safe="")
marker = get(f"asterisk/variable?variable={marker_name}").get("value")
expected_marker = os.environ["ASTERISK_TEST_INSTANCE_MARKER"]
if marker != expected_marker:
    raise SystemExit(f"selected PBX marker mismatch: expected {expected_marker!r}, got {marker!r}")

version = get("asterisk/info").get("system", {}).get("version", "")
expected_branch = os.environ["ASTERISK_TEST_BRANCH"]
if not version.startswith(expected_branch + "."):
    raise SystemExit(f"selected PBX branch mismatch: expected {expected_branch}.x, got {version!r}")
PY

target="live_${suite}"
listed="$(cargo test --locked -p asterisk-rs-tests --test "$target" --features integration -- --list)"
count="$(awk '/tests, 0 benchmarks$/ { print $1 }' <<<"$listed")"
if [[ -z "$count" || "$count" -eq 0 ]]; then
    echo "$suite live test discovery returned zero tests" >&2
    exit 1
fi

cargo "test-live-${suite}"
