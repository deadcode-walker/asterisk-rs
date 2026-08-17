#!/usr/bin/env python3
"""Ratchet rustdoc's missing-documentation diagnostics per published crate."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
BASELINE_PATH = ROOT / "scripts" / "missing-docs-baseline.json"
MISSING_DOC = re.compile(r"^error: missing documentation", re.MULTILINE)
ANSI_ESCAPE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def count_missing_docs(output: str) -> int:
    """Count rustdoc diagnostics without depending on paths or message categories."""
    return len(MISSING_DOC.findall(ANSI_ESCAPE.sub("", output)))


def check_self_test() -> None:
    fixture = """error: missing documentation for a method
  --> src/lib.rs:1:1
error: missing documentation for a struct field
error: could not document `fixture`
"""
    assert count_missing_docs(fixture) == 2
    assert count_missing_docs("\x1b[1m\x1b[91merror\x1b[0m: missing documentation") == 1
    assert count_missing_docs("Documenting fixture\nFinished") == 0


def rustdoc_count(package: str) -> tuple[int, str, int]:
    env = os.environ.copy()
    flags = env.get("RUSTDOCFLAGS", "").strip()
    env["RUSTDOCFLAGS"] = f"{flags} -Dmissing_docs".strip()
    result = subprocess.run(
        [
            "cargo",
            "rustdoc",
            "--locked",
            "-p",
            package,
            "--all-features",
        ],
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    return count_missing_docs(result.stdout), result.stdout, result.returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--self-test", action="store_true", help="exercise the diagnostic parser first"
    )
    args = parser.parse_args()

    if args.self_test:
        check_self_test()

    baseline = json.loads(BASELINE_PATH.read_text())
    failed = False
    for package, allowed in baseline["packages"].items():
        actual, output, returncode = rustdoc_count(package)
        if actual == 0 and returncode != 0:
            print(f"{package}: rustdoc failed without missing-doc diagnostics", file=sys.stderr)
            print(output, file=sys.stderr)
            failed = True
        elif actual > allowed:
            print(f"{package}: missing docs increased: {actual} > {allowed}", file=sys.stderr)
            print(output, file=sys.stderr)
            failed = True
        elif actual < allowed:
            print(
                f"{package}: missing docs improved: {actual} < {allowed}; lower the baseline",
                file=sys.stderr,
            )
            failed = True
        else:
            print(f"{package}: {actual} missing-doc diagnostics (baseline {allowed})")

    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
