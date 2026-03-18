---
name: update-docs
description: "Updates all documentation and rules in the asterisk-rs workspace. Run when code changes affect public API, versions, crate structure, or features."
---

# update-docs

Updates project documentation to reflect current codebase state. Covers READMEs, AGENTS.md, CHANGELOG, mdBook, and omp rules.

## Philosophy

Documentation describes what users can **do**, not implementation metrics. Never put raw counts ("161 events", "47 commands") in user-facing docs. Describe capabilities: "typed events covering the full Asterisk 23 surface", "all AGI commands with typed async methods". Counts belong in auto-generated reference pages only.

## Prerequisites

- Python 3.11+ (for docs/generate.py)
- The workspace must have a root `Cargo.toml` with `[workspace]`

## Step 1: Generate reference pages

```bash
python3 docs/generate.py
```

This parses Rust source files and generates:
- `docs/src/ami/reference.md` — all AMI events and actions
- `docs/src/agi/reference.md` — all AGI commands and channel methods
- `docs/src/ari/reference.md` — all ARI events and resource operations
- `docs/src/types.md` — all domain type enums with variants
- `docs/src/SUMMARY.md` — table of contents

These files are fully auto-generated. Never hand-edit them.

## Step 2: Update documentation files

For each file: read current version, apply targeted edits. Preserve accurate prose. Fix what's wrong.

### 2.1 Root README.md

Structure (see current file for reference):
- One-line pitch: what users can DO with the library
- Three protocol bullets: AMI, AGI, ARI with one-sentence descriptions
- Code example showing a real use case (not just ping)
- Install section with cargo add
- Capabilities list: features described as user-facing abilities, not implementation details
- Protocol table with links to docs.rs
- Links to documentation

**Never put counts in README.** Say "typed events covering the full Asterisk 23 surface" not "161 typed events".

### 2.2 Per-crate READMEs (`crates/*/README.md`)

Structure:
- Badges (crates.io, docs.rs)
- One-line pitch
- Code example showing the primary use case for that protocol
- Features list (capabilities, not counts)
- One-liner: "Part of asterisk-rs. MSRV X. MIT/Apache-2.0."

### 2.3 AGENTS.md

Drives AI agent behavior. Accuracy is critical — wrong info here means wrong code generated.

Update against source:
- **Architecture tree** — module names, file descriptions, type names
- **Key Directories** — verify paths exist
- **Code Conventions** — workspace lints, patterns
- **Event System** — current types (AmiEvent, AriMessage wrapping AriEvent)
- **Important Files** — add/remove as needed
- **Testing section** — total test count, coverage gaps
- **CI Matrix** — match workflow files

**No counts in descriptions.** Say "typed variants + Unknown" not "161 typed variants + Unknown".

### 2.4 CHANGELOG.md

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

- Describe what changed in terms of capability, not implementation detail
- Each item starts with crate name in backticks
- Released sections are immutable history
- Say "typed events covering all Asterisk 23 events" not "161 typed event variants"

### 2.5 mdBook guide pages (`docs/src/`)

Guide pages (overview, connection, events, fastagi, stasis, resources) are manually maintained.

Rules:
- Focus on HOW to use the library, not what it contains
- Code examples use `rust,ignore` fences
- Link to reference.md for complete lists: "see [Reference](./reference.md)"
- No manually-maintained event/action/command tables — those are auto-generated
- Use current API shapes (check source before writing examples)
- Import paths use actual crate names: `asterisk_rs_ami`, `asterisk_rs_agi`, `asterisk_rs_ari`

### 2.6 Rules (`.omp/rules/`)

Update the `asterisk` and `rust` rules when:
- Protocol knowledge changes (new wire format details, auth mechanisms)
- Code conventions change (new derives, new patterns)
- Architecture changes (new modules, new types)

Rules should describe the domain and conventions, not enumerate items.

## Step 3: Verify

1. `python3 docs/generate.py` runs without error
2. `mdbook build docs/` succeeds (if mdbook installed)
3. No stale references to removed types or modules
4. Code examples use current API (check imports against source)
5. MSRV, license text consistent across all files
6. AGENTS.md architecture matches actual directory structure
7. No raw counts in user-facing documentation (README, CHANGELOG, guide pages)

## Anti-patterns

- Putting counts like "161 events" or "47 commands" in READMEs or CHANGELOGs
- Manually maintaining event/action/command tables in mdBook pages
- Writing code examples without verifying against current source
- Duplicating reference content that's auto-generated
- Using `rust,no_run` fences (still compiles; use `rust,ignore` for examples needing a live server)
