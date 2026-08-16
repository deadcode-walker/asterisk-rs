#!/usr/bin/env python3

from __future__ import annotations

import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
REQUIRED = (
    "AGENTS.md",
    "ARCHITECTURE.md",
    "justfile",
    "docs/README.md",
    "docs/PLANS.md",
    "docs/PRODUCT_SENSE.md",
    "docs/QUALITY_SCORE.md",
    "docs/RELIABILITY.md",
    "docs/SECURITY.md",
    "docs/design-docs/index.md",
    "docs/design-docs/project-decision-brief.md",
    "docs/product-specs/index.md",
)


def fail(message: str) -> None:
    print(f"harness error: {message}", file=sys.stderr)


def load_toml(path: Path) -> dict:
    with path.open("rb") as source:
        return tomllib.load(source)


def main() -> int:
    errors = 0
    for relative in REQUIRED:
        path = ROOT / relative
        if not path.is_file() or not path.read_text(encoding="utf-8").strip():
            fail(f"required knowledge file is missing or empty: {relative}")
            errors += 1

    agents_lines = (ROOT / "AGENTS.md").read_text(encoding="utf-8").count("\n") + 1
    if agents_lines > 120:
        fail(f"AGENTS.md has {agents_lines} lines; move detail into linked documents")
        errors += 1

    if (ROOT / "CLAUDE.md").exists():
        fail("CLAUDE.md duplicates the repository knowledge system; remove it")
        errors += 1

    workspace = load_toml(ROOT / "Cargo.toml")
    msrv = workspace["workspace"]["package"]["rust-version"]
    clippy_msrv = load_toml(ROOT / "clippy.toml")["msrv"]
    if msrv != clippy_msrv:
        fail(f"Cargo MSRV {msrv} differs from clippy MSRV {clippy_msrv}")
        errors += 1

    protocols = ("ami", "agi", "ari")
    for protocol in protocols:
        manifest = load_toml(ROOT / f"crates/asterisk-rs-{protocol}/Cargo.toml")
        dependencies = manifest.get("dependencies", {})
        for other in protocols:
            forbidden = f"asterisk-rs-{other}"
            if other != protocol and forbidden in dependencies:
                fail(f"{protocol} depends on protocol peer {forbidden}; move composition upward")
                errors += 1

    if errors:
        return 1
    print("harness structure and dependency boundaries are valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
