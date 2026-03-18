#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""scan a rust workspace and emit structured JSON metadata for doc generation."""

from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path


# ---------------------------------------------------------------------------
# cargo.toml parsing
# ---------------------------------------------------------------------------


def parse_toml(path: Path) -> dict:
    with open(path, "rb") as f:
        return tomllib.load(f)


def resolve_members(root: Path, patterns: list[str]) -> list[str]:
    """resolve workspace member globs to actual crate directories (relative to root)."""
    members: list[str] = []
    for pat in patterns:
        if "*" in pat or "?" in pat:
            for p in sorted(root.glob(pat)):
                if (p / "Cargo.toml").is_file():
                    members.append(str(p.relative_to(root)))
        else:
            candidate = root / pat
            if (candidate / "Cargo.toml").is_file():
                members.append(pat)
    return members


def extract_workspace(root: Path) -> dict:
    cargo = parse_toml(root / "Cargo.toml")
    ws = cargo.get("workspace", {})
    pkg = ws.get("package", {})
    lints = ws.get("lints", {})
    deps = ws.get("dependencies", {})

    dep_versions: dict[str, str] = {}
    for name, spec in deps.items():
        if isinstance(spec, str):
            dep_versions[name] = spec
        elif isinstance(spec, dict):
            dep_versions[name] = spec.get("version", "")

    members = resolve_members(root, ws.get("members", []))

    return {
        "root": str(root.resolve()),
        "members": members,
        "package": {
            "edition": pkg.get("edition", ""),
            "rust_version": pkg.get("rust-version", ""),
            "license": pkg.get("license", ""),
            "repository": pkg.get("repository", ""),
            "homepage": pkg.get("homepage", ""),
            "keywords": pkg.get("keywords", []),
            "categories": pkg.get("categories", []),
        },
        "lints": {
            section: {k: v for k, v in rules.items()}
            for section, rules in lints.items()
        },
        "dependencies": dep_versions,
    }


# ---------------------------------------------------------------------------
# per-crate metadata
# ---------------------------------------------------------------------------


def extract_crate(root: Path, member: str, ws_pkg: dict) -> dict:
    crate_dir = root / member
    cargo = parse_toml(crate_dir / "Cargo.toml")
    pkg = cargo.get("package", {})

    name = pkg.get("name", "")
    version = pkg.get("version", "")
    description = pkg.get("description", "")

    # resolve inherited edition
    edition_val = pkg.get("edition", {})
    if isinstance(edition_val, dict) and edition_val.get("workspace"):
        edition = ws_pkg.get("edition", "")
    elif isinstance(edition_val, str):
        edition = edition_val
    else:
        edition = ""

    # dependencies
    deps_section = cargo.get("dependencies", {})
    workspace_deps: list[str] = []
    external_deps: list[str] = []
    for dep_name, spec in deps_section.items():
        if isinstance(spec, dict) and (spec.get("workspace") or spec.get("path")):
            workspace_deps.append(dep_name)
        else:
            external_deps.append(dep_name)

    features = cargo.get("features", {})
    dev_deps = list(cargo.get("dev-dependencies", {}).keys())

    src_dir = crate_dir / "src"
    api = scan_api_surface(src_dir) if src_dir.is_dir() else empty_api()
    tests = count_tests(src_dir) if src_dir.is_dir() else {"total": 0, "files": {}}

    examples = sorted(
        p.name for p in (crate_dir / "examples").glob("*.rs")
    ) if (crate_dir / "examples").is_dir() else []

    lib_rs = src_dir / "lib.rs"
    modules, reexports = parse_lib_rs(lib_rs) if lib_rs.is_file() else ([], [])

    return {
        "name": name,
        "version": version,
        "description": description,
        "edition": edition,
        "workspace_deps": sorted(workspace_deps),
        "external_deps": sorted(external_deps),
        "dependencies": sorted(workspace_deps + external_deps),
        "features": features,
        "dev_dependencies": sorted(dev_deps),
        "api": api,
        "tests": tests,
        "examples": examples,
        "modules": modules,
        "reexports": reexports,
    }


# ---------------------------------------------------------------------------
# public api surface scanning
# ---------------------------------------------------------------------------

# patterns for pub items (not pub(crate), pub(super), etc.)
RE_PUB_STRUCT = re.compile(r"^pub\s+struct\s+(\w+)")
RE_PUB_ENUM = re.compile(r"^pub\s+enum\s+(\w+)")
RE_PUB_FN = re.compile(r"^pub\s+(?:async\s+)?fn\s+(\w+)")
RE_PUB_TRAIT = re.compile(r"^pub\s+trait\s+(\w+)")
RE_PUB_CONST = re.compile(r"^pub\s+const\s+(\w+)")
RE_PUB_TYPE = re.compile(r"^pub\s+type\s+(\w+)")

# restricted visibility — skip these
RE_PUB_RESTRICTED = re.compile(r"^pub\s*\(")

# cfg(test) module detection
RE_CFG_TEST = re.compile(r"#\[cfg\(test\)]")


def empty_api() -> dict:
    return {
        "structs": 0,
        "enums": [],
        "traits": 0,
        "functions": 0,
        "constants": 0,
        "type_aliases": 0,
    }


def scan_api_surface(src_dir: Path) -> dict:
    structs = 0
    enums: list[dict] = []
    traits = 0
    functions = 0
    constants = 0
    type_aliases = 0

    for rs_file in sorted(src_dir.rglob("*.rs")):
        result = scan_file_api(rs_file)
        structs += result["structs"]
        enums.extend(result["enums"])
        traits += result["traits"]
        functions += result["functions"]
        constants += result["constants"]
        type_aliases += result["type_aliases"]

    return {
        "structs": structs,
        "enums": enums,
        "traits": traits,
        "functions": functions,
        "constants": constants,
        "type_aliases": type_aliases,
    }


def _extract_variant_name(stripped: str) -> str | None:
    """extract enum variant name from a line known to be at enum top-level depth."""
    # skip noise
    if not stripped or stripped.startswith("//") or stripped.startswith("#[") or stripped == "}":
        return None
    # first word before any punctuation is the variant name
    word = stripped.split("(")[0].split("{")[0].split(",")[0].split("<")[0].strip()
    if word and word[0].isupper() and word.isidentifier():
        return word
    return None


def scan_file_api(path: Path) -> dict:
    """scan a single .rs file for pub declarations, skipping test modules."""
    lines = path.read_text(errors="replace").splitlines()

    structs = 0
    enums: list[dict] = []
    traits = 0
    functions = 0
    constants = 0
    type_aliases = 0

    in_test_module = False
    test_brace_depth = 0
    in_block_comment = False
    in_enum_body = False
    enum_name = ""
    enum_brace_depth = 0
    enum_variants: list[str] = []
    cfg_test_next = False

    for line in lines:
        stripped = line.strip()

        # track block comments
        if in_block_comment:
            if "*/" in stripped:
                in_block_comment = False
            continue
        if "/*" in stripped and "*/" not in stripped:
            in_block_comment = True
            continue

        # skip line comments
        if stripped.startswith("//"):
            continue

        # detect #[cfg(test)]
        if RE_CFG_TEST.search(stripped):
            cfg_test_next = True
            continue

        # entering a test module
        if cfg_test_next:
            if "mod " in stripped:
                in_test_module = True
                test_brace_depth = stripped.count("{") - stripped.count("}")
                cfg_test_next = False
                continue
            elif stripped == "" or stripped.startswith("#[") or stripped.startswith("///"):
                # attribute or doc comment between cfg(test) and mod — keep flag
                continue
            else:
                cfg_test_next = False

        # track test module brace depth
        if in_test_module:
            test_brace_depth += stripped.count("{") - stripped.count("}")
            if test_brace_depth <= 0:
                in_test_module = False
            continue

        # track enum body for variant extraction
        if in_enum_body:
            depth_before = enum_brace_depth
            enum_brace_depth += stripped.count("{") - stripped.count("}")

            if enum_brace_depth <= 0:
                # enum closed — check if closing line also started a variant
                if depth_before == 1:
                    name = _extract_variant_name(stripped)
                    if name:
                        enum_variants.append(name)
                enums.append({"name": enum_name, "variants": len(enum_variants)})
                in_enum_body = False
                continue

            # count variants at the top level of the enum (depth was 1 before this line)
            if depth_before == 1:
                name = _extract_variant_name(stripped)
                if name:
                    enum_variants.append(name)
            continue

        # skip restricted visibility
        if RE_PUB_RESTRICTED.match(stripped):
            continue

        # pub struct
        m = RE_PUB_STRUCT.match(stripped)
        if m:
            structs += 1
            continue

        # pub enum — start variant tracking
        m = RE_PUB_ENUM.match(stripped)
        if m:
            enum_name = m.group(1)
            enum_variants = []
            if "{" in stripped:
                in_enum_body = True
                enum_brace_depth = stripped.count("{") - stripped.count("}")
                if enum_brace_depth <= 0:
                    enums.append({"name": enum_name, "variants": 0})
                    in_enum_body = False
            continue

        # pub fn / pub async fn
        m = RE_PUB_FN.match(stripped)
        if m:
            functions += 1
            continue

        # pub trait
        m = RE_PUB_TRAIT.match(stripped)
        if m:
            traits += 1
            continue

        # pub const
        m = RE_PUB_CONST.match(stripped)
        if m:
            constants += 1
            continue

        # pub type
        m = RE_PUB_TYPE.match(stripped)
        if m:
            type_aliases += 1
            continue

    return {
        "structs": structs,
        "enums": enums,
        "traits": traits,
        "functions": functions,
        "constants": constants,
        "type_aliases": type_aliases,
    }


# ---------------------------------------------------------------------------
# test inventory
# ---------------------------------------------------------------------------

RE_TEST_ATTR = re.compile(r"#\[(tokio::)?test")


def count_tests(src_dir: Path) -> dict:
    total = 0
    files: dict[str, int] = {}

    for rs_file in sorted(src_dir.rglob("*.rs")):
        content = rs_file.read_text(errors="replace")
        count = len(RE_TEST_ATTR.findall(content))
        if count > 0:
            rel = str(rs_file.relative_to(src_dir))
            files[rel] = count
            total += count

    return {"total": total, "files": files}


# ---------------------------------------------------------------------------
# lib.rs module structure
# ---------------------------------------------------------------------------

RE_PUB_MOD = re.compile(r"^pub\s+mod\s+(\w+)")
RE_PUB_USE = re.compile(r"^pub\s+use\s+\w+::(?:\{([^}]+)\}|(\w+))")


def parse_lib_rs(path: Path) -> tuple[list[str], list[str]]:
    content = path.read_text(errors="replace")
    modules: list[str] = []
    reexports: list[str] = []

    for line in content.splitlines():
        stripped = line.strip()
        if stripped.startswith("//"):
            continue

        m = RE_PUB_MOD.match(stripped)
        if m:
            modules.append(m.group(1))
            continue

        m = RE_PUB_USE.match(stripped)
        if m:
            if m.group(1):
                for item in m.group(1).split(","):
                    name = item.strip()
                    if name:
                        reexports.append(name)
            elif m.group(2):
                reexports.append(m.group(2))

    return modules, reexports


# ---------------------------------------------------------------------------
# documentation files
# ---------------------------------------------------------------------------


def find_docs(root: Path, members: list[str]) -> dict:
    root_files = sorted(
        p.name for p in root.glob("*.md") if p.is_file()
    )

    crate_readmes: dict[str, str] = {}
    crate_changelogs: dict[str, str] = {}
    for member in members:
        crate_dir = root / member
        cargo = parse_toml(crate_dir / "Cargo.toml")
        crate_name = cargo.get("package", {}).get("name", Path(member).name)

        readme = crate_dir / "README.md"
        if readme.is_file():
            crate_readmes[crate_name] = str(readme.relative_to(root))

        changelog = crate_dir / "CHANGELOG.md"
        if changelog.is_file():
            crate_changelogs[crate_name] = str(changelog.relative_to(root))

    mdbook: dict = {}
    summary = root / "docs" / "src" / "SUMMARY.md"
    if summary.is_file():
        mdbook["summary"] = str(summary.relative_to(root))
        mdbook["pages"] = sorted(
            str(p.relative_to(root))
            for p in (root / "docs" / "src").rglob("*.md")
            if p.is_file()
        )

    return {
        "root_files": root_files,
        "crate_readmes": crate_readmes,
        "crate_changelogs": crate_changelogs,
        "mdbook": mdbook,
    }


# ---------------------------------------------------------------------------
# ci workflows
# ---------------------------------------------------------------------------


def scan_workflows(root: Path) -> list[dict]:
    wf_dir = root / ".github" / "workflows"
    if not wf_dir.is_dir():
        return []

    workflows: list[dict] = []
    for yml in sorted(wf_dir.glob("*.yml")):
        content = yml.read_text(errors="replace")
        name_match = re.search(r"^name:\s*(.+)$", content, re.MULTILINE)
        wf_name = name_match.group(1).strip().strip('"').strip("'") if name_match else yml.stem

        jobs: list[str] = []
        in_jobs = False
        job_indent: int | None = None
        for line in content.splitlines():
            if line.rstrip() == "jobs:" or re.match(r"^jobs:\s*$", line):
                in_jobs = True
                job_indent = None
                continue
            if in_jobs:
                if not line or not line.strip():
                    continue
                if job_indent is None:
                    stripped = line.lstrip()
                    if stripped and not stripped.startswith("#"):
                        job_indent = len(line) - len(stripped)
                if job_indent is not None:
                    if line[0] not in (" ", "\t"):
                        break
                    indent = len(line) - len(line.lstrip())
                    if indent == job_indent:
                        m = re.match(r"\s+(\w[\w-]*):", line)
                        if m:
                            jobs.append(m.group(1))

        workflows.append({
            "file": yml.name,
            "name": wf_name,
            "jobs": jobs,
        })

    return workflows


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def main() -> None:
    if len(sys.argv) > 1:
        root = Path(sys.argv[1]).resolve()
    else:
        root = Path.cwd().resolve()

    root_cargo = root / "Cargo.toml"
    if not root_cargo.is_file():
        print(f"error: no Cargo.toml found at {root}", file=sys.stderr)
        sys.exit(1)

    workspace = extract_workspace(root)
    ws_pkg = workspace["package"]
    members = workspace["members"]

    crates: dict[str, dict] = {}
    for member in members:
        crate_data = extract_crate(root, member, ws_pkg)
        crates[crate_data["name"]] = crate_data

    docs = find_docs(root, members)
    ci = scan_workflows(root)

    output = {
        "workspace": workspace,
        "crates": crates,
        "docs": docs,
        "ci": {"workflows": ci},
    }

    json.dump(output, sys.stdout, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
