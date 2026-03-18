#!/usr/bin/env python3
"""generate mdBook reference pages from rust source.

run before `mdbook build docs/` to produce up-to-date reference tables.
reads enum variants, struct names, and doc comments from source files.
"""

import re
from pathlib import Path

ROOT = Path(__file__).parent.parent
DOCS_SRC = Path(__file__).parent / "src"


def extract_enum_variants(path: Path, enum_name: str) -> list[dict]:
    """extract variants from a rust enum, capturing doc comments."""
    text = path.read_text()
    # find the enum block
    pattern = rf"pub enum {enum_name}\s*\{{(.*?)\n\}}"
    m = re.search(pattern, text, re.DOTALL)
    if not m:
        return []

    body = m.group(1)
    variants = []
    doc_lines = []

    for line in body.split("\n"):
        stripped = line.strip()
        if stripped.startswith("///"):
            doc_lines.append(stripped[3:].strip())
        elif stripped.startswith("#["):
            continue  # skip attributes
        elif stripped and not stripped.startswith("//") and not stripped.startswith("{"):
            # variant line
            name = stripped.split("{")[0].split("(")[0].split(",")[0].strip()
            if name and name[0].isupper():
                doc = " ".join(doc_lines) if doc_lines else ""
                variants.append({"name": name, "doc": doc})
            doc_lines = []
        elif stripped == "":
            if not doc_lines:
                doc_lines = []

    return variants


def extract_action_structs(path: Path) -> list[dict]:
    """extract pub struct *Action names with their doc comments."""
    text = path.read_text()
    actions = []
    lines = text.split("\n")

    for i, line in enumerate(lines):
        if "pub struct " in line and "Action" in line:
            name = line.split("pub struct ")[1].split("{")[0].split(";")[0].strip()
            if not name.endswith("Action"):
                continue
            # look backwards for doc comment
            doc_lines = []
            j = i - 1
            while j >= 0 and lines[j].strip().startswith("///"):
                doc_lines.insert(0, lines[j].strip()[3:].strip())
                j -= 1
            doc = " ".join(doc_lines) if doc_lines else ""
            actions.append({"name": name, "doc": doc})

    return actions


def extract_methods(path: Path, impl_type: str) -> list[dict]:
    """extract pub async fn methods from an impl block."""
    text = path.read_text()
    methods = []
    lines = text.split("\n")
    in_impl = False

    for i, line in enumerate(lines):
        if f"impl {impl_type}" in line:
            in_impl = True
            continue
        if in_impl and line.strip() == "}" and not any(
            c in line for c in ["{", "//"]
        ):
            # might be end of impl - rough heuristic
            pass

        if in_impl and "pub async fn " in line:
            m = re.search(r"pub async fn (\w+)", line)
            if m:
                name = m.group(1)
                doc_lines = []
                j = i - 1
                while j >= 0 and lines[j].strip().startswith("///"):
                    doc_lines.insert(0, lines[j].strip()[3:].strip())
                    j -= 1
                doc = " ".join(doc_lines) if doc_lines else ""
                methods.append({"name": name, "doc": doc})

    return methods


def extract_command_constants(path: Path) -> list[dict]:
    """extract pub const *: &str command constants."""
    text = path.read_text()
    commands = []
    lines = text.split("\n")

    for i, line in enumerate(lines):
        m = re.match(r'\s*pub const (\w+): &str = "(.+)";', line)
        if m:
            name = m.group(1)
            value = m.group(2)
            doc_lines = []
            j = i - 1
            while j >= 0 and lines[j].strip().startswith("///"):
                doc_lines.insert(0, lines[j].strip()[3:].strip())
                j -= 1
            doc = " ".join(doc_lines) if doc_lines else ""
            commands.append({"name": name, "value": value, "doc": doc})

    return commands


def extract_domain_types(path: Path) -> list[dict]:
    """extract enums from the types module with their variants."""
    text = path.read_text()
    types = []
    enum_pattern = re.compile(
        r"/// (.+?)\n(?:#\[.*?\]\n)*pub enum (\w+)\s*\{(.*?)\n\}", re.DOTALL
    )

    for m in enum_pattern.finditer(text):
        doc = m.group(1)
        name = m.group(2)
        body = m.group(3)
        variant_count = len(
            [
                l
                for l in body.split("\n")
                if l.strip() and l.strip()[0].isupper() and "=" in l or l.strip().endswith(",")
                if l.strip()[0].isupper()
            ]
        )
        types.append({"name": name, "doc": doc, "variants": variant_count})

    return types


def generate_ami_reference():
    """generate docs/src/ami/reference.md."""
    events = extract_enum_variants(
        ROOT / "crates/asterisk-rs-ami/src/event.rs", "AmiEvent"
    )
    actions = extract_action_structs(ROOT / "crates/asterisk-rs-ami/src/action.rs")

    # filter out internal actions
    skip = {"LoginAction", "ChallengeAction", "ChallengeLoginAction"}
    actions = [a for a in actions if a["name"] not in skip]

    out = []
    out.append("<!-- auto-generated by docs/generate.py \u2014 do not edit -->\n")
    out.append("# AMI Reference\n")

    out.append(f"\n## Events ({len(events)} typed variants)\n")
    out.append("| Variant | Description |")
    out.append("|---------|-------------|")
    for e in events:
        out.append(f"| `{e['name']}` | {e['doc']} |")

    out.append(f"\n## Actions ({len(actions)} typed structs)\n")
    out.append("| Action | Description |")
    out.append("|--------|-------------|")
    for a in actions:
        display = a["name"].replace("Action", "")
        out.append(f"| `{a['name']}` | {a['doc']} |")

    return "\n".join(out) + "\n"


def generate_agi_reference():
    """generate docs/src/agi/reference.md."""
    commands = extract_command_constants(
        ROOT / "crates/asterisk-rs-agi/src/command.rs"
    )
    methods = extract_methods(
        ROOT / "crates/asterisk-rs-agi/src/channel.rs", "AgiChannel"
    )

    out = []
    out.append("<!-- auto-generated by docs/generate.py \u2014 do not edit -->\n")
    out.append("# AGI Reference\n")

    out.append(f"\n## Commands ({len(commands)} total)\n")
    out.append("| Constant | AGI Command | Description |")
    out.append("|----------|-------------|-------------|")
    for c in commands:
        out.append(f"| `{c['name']}` | `{c['value']}` | {c['doc']} |")

    out.append(f"\n## Channel Methods ({len(methods)} total)\n")
    out.append("| Method | Description |")
    out.append("|--------|-------------|")
    for m in methods:
        out.append(f"| `{m['name']}()` | {m['doc']} |")

    return "\n".join(out) + "\n"


def generate_ari_reference():
    """generate docs/src/ari/reference.md."""
    events = extract_enum_variants(
        ROOT / "crates/asterisk-rs-ari/src/event.rs", "AriEvent"
    )

    # collect resource methods from all resource files
    resources_dir = ROOT / "crates/asterisk-rs-ari/src/resources"
    resource_methods = {}
    for rs_file in sorted(resources_dir.glob("*.rs")):
        if rs_file.name == "mod.rs":
            continue
        module_name = rs_file.stem
        # find handle type
        text = rs_file.read_text()
        handle_match = re.search(r"pub struct (\w+Handle)", text)
        handle_name = handle_match.group(1) if handle_match else None

        methods = []
        if handle_name:
            methods.extend(extract_methods(rs_file, handle_name))

        # also get module-level async fns
        for m in re.finditer(
            r"/// (.+?)\npub async fn (\w+)", text
        ):
            methods.append({"name": m.group(2), "doc": m.group(1)})

        if methods:
            resource_methods[module_name] = {
                "handle": handle_name,
                "methods": methods,
            }

    out = []
    out.append("<!-- auto-generated by docs/generate.py \u2014 do not edit -->\n")
    out.append("# ARI Reference\n")

    out.append(f"\n## Events ({len(events)} typed variants)\n")
    out.append("| Variant | Description |")
    out.append("|---------|-------------|")
    for e in events:
        out.append(f"| `{e['name']}` | {e['doc']} |")

    out.append(f"\n## Resources ({len(resource_methods)} modules)\n")
    for mod_name, data in resource_methods.items():
        handle = data["handle"]
        methods = data["methods"]
        title = mod_name.replace("_", " ").title()
        if handle:
            out.append(f"\n### {title} (`{handle}`, {len(methods)} operations)\n")
        else:
            out.append(f"\n### {title} ({len(methods)} operations)\n")
        out.append("| Method | Description |")
        out.append("|--------|-------------|")
        for m in methods:
            out.append(f"| `{m['name']}()` | {m['doc']} |")

    return "\n".join(out) + "\n"


def generate_types_reference():
    """generate docs/src/types.md."""
    path = ROOT / "crates/asterisk-rs-core/src/types.rs"
    if not path.exists():
        return ""

    types = extract_domain_types(path)

    out = []
    out.append("<!-- auto-generated by docs/generate.py \u2014 do not edit -->\n")
    out.append("# Domain Types\n")
    out.append(
        "Typed enums for Asterisk domain constants, available in `asterisk_rs_core::types`.\n"
    )

    for t in types:
        out.append(f"\n## `{t['name']}`\n")
        out.append(f"{t['doc']}\n")

        # extract actual variants for this type
        variants = extract_enum_variants(path, t["name"])
        if variants:
            out.append("| Variant | Description |")
            out.append("|---------|-------------|")
            for v in variants:
                out.append(f"| `{v['name']}` | {v['doc']} |")

    return "\n".join(out) + "\n"


def main():
    # generate reference pages
    ami_ref = generate_ami_reference()
    agi_ref = generate_agi_reference()
    ari_ref = generate_ari_reference()
    types_ref = generate_types_reference()

    (DOCS_SRC / "ami" / "reference.md").write_text(ami_ref)
    (DOCS_SRC / "agi" / "reference.md").write_text(agi_ref)
    (DOCS_SRC / "ari" / "reference.md").write_text(ari_ref)
    (DOCS_SRC / "types.md").write_text(types_ref)

    print(f"ami reference: {ami_ref.count('|') // 3} entries")
    print(f"agi reference: {agi_ref.count('|') // 3} entries")
    print(f"ari reference: {ari_ref.count('|') // 3} entries")
    print(f"types reference: generated")

    # update SUMMARY.md
    summary = """# Summary

- [Getting Started](./getting-started.md)
- [Domain Types](./types.md)

# Protocols

- [AMI (Asterisk Manager Interface)](./ami/overview.md)
  - [Connection & Authentication](./ami/connection.md)
  - [Events](./ami/events.md)
  - [Reference](./ami/reference.md)
- [AGI (Asterisk Gateway Interface)](./agi/overview.md)
  - [FastAGI Server](./agi/fastagi.md)
  - [Reference](./agi/reference.md)
- [ARI (Asterisk REST Interface)](./ari/overview.md)
  - [Stasis Applications](./ari/stasis.md)
  - [Resources](./ari/resources.md)
  - [Reference](./ari/reference.md)
"""
    (DOCS_SRC / "SUMMARY.md").write_text(summary)
    print("updated SUMMARY.md")


if __name__ == "__main__":
    main()
