#!/usr/bin/env python3

from __future__ import annotations

import os
import re
import sys
import tomllib
from collections.abc import Iterator
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
REQUIRED = (
    "AGENTS.md",
    "ARCHITECTURE.md",
    "CONTRIBUTING.md",
    "justfile",
    "docs/README.md",
    "docs/PLANS.md",
    "docs/PRODUCT_SENSE.md",
    "docs/QUALITY_SCORE.md",
    "docs/RELIABILITY.md",
    "docs/SECURITY.md",
    "docs/design-docs/project-decision-brief.md",
    "docs/product-specs/index.md",
    "docs/references/index.md",
)
EXPECTED_INDEX_TARGETS = {
    "docs/README.md",
    "AGENTS.md",
    "ARCHITECTURE.md",
    "README.md",
    "CONTRIBUTING.md",
    "docs/PLANS.md",
    "docs/design-docs/project-decision-brief.md",
    "docs/design-docs/core-beliefs.md",
    "docs/PRODUCT_SENSE.md",
    "docs/product-specs/index.md",
    "docs/src",
    "docs/QUALITY_SCORE.md",
    "docs/RELIABILITY.md",
    "docs/SECURITY.md",
    "docs/references/index.md",
    "docs/exec-plans/tech-debt-tracker.md",
    "SECURITY.md",
    "crates/*/CHANGELOG.md",
}
PLAN_HEADINGS = {
    "Purpose and scope",
    "Progress",
    "Surprises and discoveries",
    "Decision log",
    "Context and orientation",
    "Plan of work",
    "Concrete steps",
    "Validation and acceptance",
    "Idempotence and recovery",
    "Outcomes and retrospective",
}
DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")
REFERENCE_DEFINITION = re.compile(
    r'^\s{0,3}\[([^\]]+)\]:\s*(?:<([^>]+)>|([^\s]+))(?:\s+(?:"[^"]*"|\'[^\']*\'|\([^)]*\)))?\s*$'
)
TABLE_LINK = re.compile(r"^\[([^\]]+)\]\((.+)\)$")


def fail(message: str) -> None:
    print(f"harness error: {message}", file=sys.stderr)


def load_toml(path: Path) -> dict:
    with path.open("rb") as source:
        return tomllib.load(source)


def text_if_file(relative: str) -> str | None:
    path = ROOT / relative
    if not path.is_file():
        return None
    text = path.read_text(encoding="utf-8")
    return text if text.strip() else None


def without_fenced_code(text: str) -> str:
    output: list[str] = []
    fence: str | None = None
    for line in text.splitlines(keepends=True):
        marker = re.match(r"^\s*(`{3,}|~{3,})", line)
        if marker and fence is None:
            fence = marker.group(1)[0]
            output.append("\n" if line.endswith("\n") else "")
        elif marker and fence == marker.group(1)[0]:
            fence = None
            output.append("\n" if line.endswith("\n") else "")
        elif fence is None:
            output.append(line)
        else:
            output.append("\n" if line.endswith("\n") else "")
    return "".join(output)


def split_link_destination(raw: str) -> str:
    value = raw.strip()
    if value.startswith("<"):
        closing = value.find(">")
        value = value[1:closing] if closing >= 0 else value[1:]
        return re.sub(r"\\([!\"#$%&'()*+,./:;<=>?@\[\\\]^_`{|}~-])", r"\1", value)

    depth = 0
    escaped = False
    for index, character in enumerate(value):
        if escaped:
            escaped = False
            continue
        if character == "\\":
            escaped = True
        elif character == "(":
            depth += 1
        elif character == ")" and depth:
            depth -= 1
        elif character.isspace() and depth == 0:
            value = value[:index]
            break
    return re.sub(r"\\([!\"#$%&'()*+,./:;<=>?@\[\\\]^_`{|}~-])", r"\1", value)


def normalize_reference_label(label: str) -> str:
    return " ".join(label.split()).casefold()


def markdown_links(text: str) -> tuple[list[tuple[int, str]], list[tuple[int, str]]]:
    text = without_fenced_code(text)
    links: list[tuple[int, str]] = []
    errors: list[tuple[int, str]] = []
    definitions: dict[str, str] = {}
    definition_lines: set[int] = set()
    for line_number, line in enumerate(text.splitlines(), start=1):
        match = REFERENCE_DEFINITION.match(line)
        if match:
            label = normalize_reference_label(match.group(1))
            destination = split_link_destination(match.group(2) or match.group(3))
            definitions.setdefault(label, destination)
            definition_lines.add(line_number)

    index = 0
    while index < len(text):
        if text[index] == "!" and index + 1 < len(text) and text[index + 1] == "[":
            index += 1
        if text[index] != "[" or (index and text[index - 1] == "\\"):
            index += 1
            continue
        line_number = text.count("\n", 0, index) + 1
        if line_number in definition_lines:
            newline = text.find("\n", index)
            index = len(text) if newline < 0 else newline + 1
            continue
        closing = text.find("]", index + 1)
        if closing < 0:
            break
        label = text[index + 1 : closing]
        next_index = closing + 1
        target: str | None = None
        if next_index < len(text) and text[next_index] == "(":
            destination_start = next_index + 1
            while destination_start < len(text) and text[destination_start].isspace():
                destination_start += 1
            cursor = destination_start
            depth = 1
            if cursor < len(text) and text[cursor] == "<":
                angle_end = text.find(">", cursor + 1)
                cursor = len(text) if angle_end < 0 else text.find(")", angle_end + 1)
                if cursor >= 0 and cursor < len(text):
                    cursor += 1
                    depth = 0
            else:
                escaped = False
                while cursor < len(text) and depth:
                    character = text[cursor]
                    if escaped:
                        escaped = False
                    elif character == "\\":
                        escaped = True
                    elif character == "(":
                        depth += 1
                    elif character == ")":
                        depth -= 1
                    cursor += 1
            if depth == 0:
                target = split_link_destination(text[next_index + 1 : cursor - 1])
                index = cursor
        elif next_index < len(text) and text[next_index] == "[":
            reference_end = text.find("]", next_index + 1)
            if reference_end >= 0:
                reference = text[next_index + 1 : reference_end] or label
                target = definitions.get(normalize_reference_label(reference))
                if target is None:
                    errors.append(
                        (line_number, f"unresolved Markdown reference: {reference}")
                    )
                index = reference_end + 1
        else:
            target = definitions.get(normalize_reference_label(label))
            index = next_index
        if target is not None:
            links.append((line_number, target))
        elif index <= closing:
            index = closing + 1
    return links, errors


def markdown_h2_section(text: str, heading: str) -> str:
    lines = without_fenced_code(text).splitlines(keepends=True)
    start: int | None = None
    for index, line in enumerate(lines):
        match = re.match(r"^##\s+(.+?)\s*$", line)
        if match and match.group(1).strip() == heading:
            start = index + 1
            continue
        if start is not None and match:
            return "".join(lines[start:index])
    return "" if start is None else "".join(lines[start:])


def normalized_repository_target(source: Path, target: str) -> str | None:
    if target.startswith(("#", "http://", "https://", "mailto:")):
        return None
    path_only = target.split("#", 1)[0]
    if not path_only:
        return None
    resolved = (source.parent / path_only).resolve()
    try:
        return resolved.relative_to(ROOT.resolve()).as_posix()
    except ValueError:
        return f"../{resolved}"


def canonical_index_targets(text: str, errors: list[str]) -> set[str]:
    source = ROOT / "docs/README.md"
    targets: set[str] = set()
    section = markdown_h2_section(text, "Canonical owners")
    if not section:
        errors.append("docs/README.md is missing exact H2 section: Canonical owners")
        return targets
    section_start = text.find(section)
    first_line = text.count("\n", 0, section_start) + 1
    for offset, line in enumerate(section.splitlines()):
        line_number = first_line + offset
        if not line.startswith("|") or line.startswith("|---"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if cells[0] == "Location":
            continue
        if len(cells) != 5 or any(not cell for cell in cells):
            errors.append(
                f"docs/README.md:{line_number} canonical-owner row must have five populated cells"
            )
            continue
        first = cells[0]
        match = TABLE_LINK.fullmatch(first)
        if match:
            normalized = normalized_repository_target(
                source, split_link_destination(match.group(2))
            )
            if normalized is None or normalized.startswith("../"):
                errors.append(
                    f"docs/README.md:{line_number} canonical owner must remain in the repository"
                )
            else:
                targets.add(normalized.rstrip("/"))
        elif first.startswith("`") and first.endswith("`"):
            targets.add(first[1:-1])
        else:
            errors.append(
                f"docs/README.md:{line_number} canonical owner is not an exact link or literal"
            )
    return targets


def local_link_errors(relative: str) -> list[str]:
    source = ROOT / relative
    errors: list[str] = []
    if not source.is_file():
        return [f"indexed Markdown owner is missing: {relative}"]
    links, parse_errors = markdown_links(source.read_text(encoding="utf-8"))
    errors.extend(
        f"{relative}:{line_number} {message}" for line_number, message in parse_errors
    )
    for line_number, raw_target in links:
        normalized = normalized_repository_target(source, raw_target)
        if normalized is None:
            continue
        if normalized.startswith("../"):
            errors.append(
                f"{relative}:{line_number} link escapes the repository: {raw_target}"
            )
        elif not (ROOT / normalized).exists():
            errors.append(
                f"{relative}:{line_number} has a broken local link: {raw_target}"
            )
    return errors


def indexed_markdown_files(targets: set[str]) -> set[str]:
    files: set[str] = set()
    for target in targets:
        if "*" in target:
            files.update(
                path.relative_to(ROOT).as_posix()
                for path in ROOT.glob(target)
                if path.suffix == ".md"
            )
            continue
        path = ROOT / target
        if path.is_dir():
            files.update(
                child.relative_to(ROOT).as_posix() for child in path.rglob("*.md")
            )
        elif path.suffix == ".md":
            files.add(target)
    return files


def second_level_headings(text: str) -> set[str]:
    return {
        match.group(1).strip()
        for line in without_fenced_code(text).splitlines()
        if (match := re.match(r"^##\s+(.+?)\s*$", line))
    }


def iter_dependency_tables(manifest: dict) -> Iterator[tuple[str, dict]]:
    for table in DEPENDENCY_TABLES:
        dependencies = manifest.get(table)
        if isinstance(dependencies, dict):
            yield table, dependencies
    targets = manifest.get("target")
    if not isinstance(targets, dict):
        return
    for selector, target in targets.items():
        if not isinstance(target, dict):
            continue
        for table in DEPENDENCY_TABLES:
            dependencies = target.get(table)
            if isinstance(dependencies, dict):
                yield f"target.{selector}.{table}", dependencies


def dependency_packages(
    manifest: dict, workspace_dependencies: dict
) -> Iterator[tuple[str, str]]:
    for table, dependencies in iter_dependency_tables(manifest):
        for alias, specification in dependencies.items():
            resolved = specification
            if (
                isinstance(specification, dict)
                and specification.get("workspace") is True
            ):
                resolved = workspace_dependencies.get(alias, specification)
            package = (
                resolved.get("package", alias) if isinstance(resolved, dict) else alias
            )
            yield package, f"{table}.{alias}"


def sanitize_rust_source(text: str) -> str:
    output = list(text)
    index = 0

    def blank(start: int, end: int) -> None:
        for position in range(start, end):
            if output[position] != "\n":
                output[position] = " "

    while index < len(text):
        if text.startswith("//", index):
            end = text.find("\n", index)
            end = len(text) if end < 0 else end
            blank(index, end)
            index = end
            continue
        if text.startswith("/*", index):
            start = index
            depth = 1
            index += 2
            while index < len(text) and depth:
                if text.startswith("/*", index):
                    depth += 1
                    index += 2
                elif text.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            blank(start, index)
            continue

        raw = re.match(r"(?:br|r)(?P<hashes>#{0,255})\"", text[index:])
        if raw:
            start = index
            hashes = raw.group("hashes")
            terminator = '"' + hashes
            index += raw.end()
            end = text.find(terminator, index)
            index = len(text) if end < 0 else end + len(terminator)
            blank(start, index)
            continue

        prefix = 1 if text.startswith(('b"', "b'"), index) else 0
        quote_index = index + prefix
        if quote_index < len(text) and text[quote_index] == '"':
            start = index
            index = quote_index + 1
            escaped = False
            while index < len(text):
                character = text[index]
                index += 1
                if escaped:
                    escaped = False
                elif character == "\\":
                    escaped = True
                elif character == '"':
                    break
            blank(start, index)
            continue
        character = re.match(
            r"(?:b)?'(?:\\(?:[nrt0\\'\"]|x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]{1,6}\})|[^\\'\n])'",
            text[index:],
        )
        if character:
            end = index + character.end()
            blank(index, end)
            index = end
            continue
        index += 1
    return "".join(output)


def has_positive_test_configuration(content: str) -> bool:
    tokens = re.findall(r"[A-Za-z_][A-Za-z0-9_]*|[(),]", content)

    def visit(position: int, positive: bool) -> tuple[int, bool]:
        if position >= len(tokens):
            return position, False
        name = tokens[position]
        found = name == "test" and positive
        position += 1
        if position >= len(tokens) or tokens[position] != "(":
            return position, found
        position += 1
        child_positive = not positive if name == "not" else positive
        while position < len(tokens) and tokens[position] != ")":
            position, child_found = visit(position, child_positive)
            found = found or child_found
            if position < len(tokens) and tokens[position] == ",":
                position += 1
        if position < len(tokens) and tokens[position] == ")":
            position += 1
        return position, found

    position = 0
    found = False
    while position < len(tokens):
        position, expression_found = visit(position, True)
        found = found or expression_found
        if position < len(tokens) and tokens[position] == ",":
            position += 1
    return found


def test_cfg_attributes(text: str) -> Iterator[tuple[int, str]]:
    source = sanitize_rust_source(text)
    index = 0
    while index < len(source):
        if source[index] != "#":
            index += 1
            continue
        start = index
        index += 1
        if index < len(source) and source[index] == "!":
            index += 1
        while index < len(source) and source[index].isspace():
            index += 1
        if index >= len(source) or source[index] != "[":
            continue
        content_start = index + 1
        depth = 1
        index += 1
        while index < len(source) and depth:
            if source[index] == "[":
                depth += 1
            elif source[index] == "]":
                depth -= 1
            index += 1
        if depth:
            continue
        content = source[content_start : index - 1]
        if re.match(
            r"\s*cfg(?:_attr)?\s*\(", content
        ) and has_positive_test_configuration(content):
            yield source.count("\n", 0, start) + 1, content.strip()


def checker_self_errors() -> list[str]:
    errors: list[str] = []
    rust_cases = {
        "// #[cfg(test)]\npub fn live() {}": False,
        'const TEXT: &str = "#[cfg(test)]";': False,
        'const TEXT: &str = r###"#![cfg(test)]"###;': False,
        "#[cfg(test)]\nmod tests {}": True,
        "#[cfg(all(test, unix))]\nmod tests {}": True,
        "#![cfg(test)]\nmod tests {}": True,
        "#[cfg_attr(test, allow(dead_code))]\nfn helper() {}": True,
        "#[cfg(not(test))]\nfn production() {}": False,
        "#[cfg_attr(not(test), allow(dead_code))]\nfn production() {}": False,
        "#[cfg(not(not(test)))]\nmod tests {}": True,
        "fn f<'a>() { #[cfg(test)] fn hidden() {} let _: &'a str; }": True,
        "fn f() { 'retry: loop { #[cfg(test)] fn hidden() {} break 'retry; } }": True,
    }
    for fixture, expected in rust_cases.items():
        observed = bool(list(test_cfg_attributes(fixture)))
        if observed != expected:
            errors.append(f"internal Rust cfg scanner fixture failed: {fixture!r}")

    valid_markdown_fixture = (
        "[nested](guide(v2).md)\n"
        '[title](guide.md "Guide")\n'
        "[reference][guide]\n"
        "[escaped](guide\\(v3\\).md)\n"
        "[angle](<guide(v4.md>)\n"
        "[spaced][foo bar]\n"
        "[escaped-reference]\n"
        "[angle-reference]\n"
        "[duplicate]\n"
        "[guide]: <reference.md> 'Reference'\n"
        "[foo   bar]: spaced.md\n"
        "[escaped-reference]: guide\\(v5\\).md\n"
        "[angle-reference]: <guide\\(v6\\).md>\n"
        "[duplicate]: first.md\n"
        "[duplicate]: second.md\n"
    )
    observed_links, markdown_errors = markdown_links(valid_markdown_fixture)
    observed_targets = [target for _, target in observed_links]
    if (
        observed_targets
        != [
            "guide(v2).md",
            "guide.md",
            "reference.md",
            "guide(v3).md",
            "guide(v4.md",
            "spaced.md",
            "guide(v5).md",
            "guide(v6).md",
            "first.md",
        ]
        or markdown_errors
    ):
        errors.append(
            "internal Markdown link scanner fixture failed: "
            f"links={observed_targets!r}, errors={markdown_errors!r}"
        )
    _, unresolved_errors = markdown_links("[guide][missing]\n[missing][]\n")
    if unresolved_errors != [
        (1, "unresolved Markdown reference: missing"),
        (2, "unresolved Markdown reference: missing"),
    ]:
        errors.append(
            f"internal unresolved Markdown reference fixture failed: {unresolved_errors!r}"
        )

    dependency_fixture = {
        "dependencies": {
            "peer-alias": {"package": "asterisk-rs-ari", "version": "1"},
            "workspace-alias": {"workspace": True},
        },
        "package": {"metadata": {"dependencies": {"not-a-dependency": "1"}}},
    }
    workspace_fixture = {
        "workspace-alias": {"package": "asterisk-rs-ami", "version": "1"}
    }
    observed_dependencies = list(
        dependency_packages(dependency_fixture, workspace_fixture)
    )
    expected_dependencies = [
        ("asterisk-rs-ari", "dependencies.peer-alias"),
        ("asterisk-rs-ami", "dependencies.workspace-alias"),
    ]
    if observed_dependencies != expected_dependencies:
        errors.append(
            "internal Cargo dependency scanner fixture failed: "
            f"{observed_dependencies!r}"
        )
    return errors


def main() -> int:
    errors = checker_self_errors()
    texts: dict[str, str] = {}
    for relative in REQUIRED:
        text = text_if_file(relative)
        if text is None:
            errors.append(f"required knowledge file is missing or empty: {relative}")
        else:
            texts[relative] = text

    agents = texts.get("AGENTS.md")
    if agents is not None:
        agents_words = len(agents.split())
        if not 200 <= agents_words <= 400:
            errors.append(
                f"AGENTS.md has {agents_words} words; keep the root routing map between 200 and 400"
            )

    guides: list[Path] = []
    for directory, child_directories, files in os.walk(ROOT):
        child_directories[:] = [
            name for name in child_directories if name not in {".git", "target"}
        ]
        if "AGENTS.md" in files:
            guides.append(Path(directory) / "AGENTS.md")

    index_targets: set[str] = set()
    knowledge_index = texts.get("docs/README.md")
    if knowledge_index is not None:
        index_targets = canonical_index_targets(knowledge_index, errors)
        for target in sorted(EXPECTED_INDEX_TARGETS - index_targets):
            errors.append(
                f"docs/README.md does not index canonical owner exactly: {target}"
            )
        for guide in guides:
            relative = guide.relative_to(ROOT).as_posix()
            if relative != "AGENTS.md" and relative not in index_targets:
                errors.append(
                    f"nested instruction scope is not indexed in docs/README.md: {relative}"
                )

    active_directory = ROOT / "docs/exec-plans/active"
    active_plans = (
        sorted(active_directory.glob("*.md")) if active_directory.is_dir() else []
    )
    if len(active_plans) != 1:
        errors.append(
            "expected exactly one active ExecPlan; found "
            f"{len(active_plans)} under docs/exec-plans/active"
        )
    else:
        active_relative = active_plans[0].relative_to(ROOT).as_posix()
        if knowledge_index is not None and active_relative not in index_targets:
            errors.append(
                f"active ExecPlan is not indexed exactly in docs/README.md: {active_relative}"
            )
        headings = second_level_headings(active_plans[0].read_text(encoding="utf-8"))
        for heading in sorted(PLAN_HEADINGS - headings):
            errors.append(f"{active_relative} is missing exact H2 section: {heading}")

    for relative in sorted(indexed_markdown_files(index_targets)):
        errors.extend(local_link_errors(relative))

    if (ROOT / "CLAUDE.md").exists():
        errors.append("CLAUDE.md duplicates the repository knowledge system; remove it")
    if (ROOT / "docs/generated").exists():
        errors.append(
            "docs/generated is an unowned empty scaffold; generated references belong under docs/src"
        )

    cargo_path = ROOT / "Cargo.toml"
    clippy_path = ROOT / "clippy.toml"
    workspace: dict | None = None
    clippy: dict | None = None
    if cargo_path.is_file():
        workspace = load_toml(cargo_path)
    else:
        errors.append("required build authority is missing: Cargo.toml")
    if clippy_path.is_file():
        clippy = load_toml(clippy_path)
    else:
        errors.append("required lint compatibility file is missing: clippy.toml")

    if workspace is not None:
        msrv = workspace.get("workspace", {}).get("package", {}).get("rust-version")
        if clippy is not None and msrv != clippy.get("msrv"):
            errors.append(
                f"Cargo MSRV {msrv} differs from clippy MSRV {clippy.get('msrv')}"
            )
        if (
            workspace.get("workspace", {})
            .get("lints", {})
            .get("rust", {})
            .get("unsafe_code")
            != "forbid"
        ):
            errors.append('workspace Rust lints must keep unsafe_code = "forbid"')

    workspace_dependencies = (
        workspace.get("workspace", {}).get("dependencies", {})
        if workspace is not None
        else {}
    )
    protocols = ("ami", "agi", "ari")
    for protocol in protocols:
        relative = f"crates/asterisk-rs-{protocol}/Cargo.toml"
        manifest_path = ROOT / relative
        if not manifest_path.is_file():
            errors.append(f"protocol manifest is missing: {relative}")
            continue
        manifest = load_toml(manifest_path)
        for package, location in dependency_packages(manifest, workspace_dependencies):
            for other in protocols:
                forbidden = f"asterisk-rs-{other}"
                if other != protocol and package == forbidden:
                    errors.append(
                        f"{relative} [{location}] resolves to protocol peer {forbidden}; "
                        "move composition upward"
                    )

    for source in (ROOT / "crates").glob("*/src/**/*.rs"):
        for line_number, attribute in test_cfg_attributes(
            source.read_text(encoding="utf-8")
        ):
            errors.append(
                f"{source.relative_to(ROOT)}:{line_number} production module contains "
                f"#[{attribute}]; move behavior proof to the external tests crate"
            )

    if errors:
        for message in errors:
            fail(message)
        return 1
    print(
        "harness knowledge, recovery, links, tests, and dependency boundaries are valid"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
