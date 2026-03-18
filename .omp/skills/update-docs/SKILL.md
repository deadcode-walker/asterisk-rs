---
name: update-docs
description: "Updates all documentation files in a Rust workspace: AGENTS.md, READMEs, CHANGELOGs, CONTRIBUTING.md, and mdBook content. Run when code changes affect public API surface, versions, crate structure, CI config, or test coverage."
---

# update-docs

Updates project documentation to reflect current codebase state. Extracts metadata from source, then updates each documentation file.

## Prerequisites

- Python 3.11+ and [uv](https://docs.astral.sh/uv/)
- The workspace must have a root `Cargo.toml` with `[workspace]`

## Step 1: Extract metadata

Run the extraction script to get ground-truth data:

```bash
uv run skill://update-docs/scripts/extract-workspace-meta.py /path/to/workspace
```

This outputs JSON with: workspace metadata, per-crate info (name, version, description, edition, MSRV, dependencies, features), API surface counts (actions, events, commands, traits), test inventory (per-file counts, total), examples list, documentation file inventory, and CI workflow configuration. Save the output to a variable for use in subsequent steps.

## Step 2: Update each documentation file

For each file: read the current version first, then apply targeted edits. Do not blindly regenerate — preserve existing prose, examples, and formatting where the content is still accurate.

### 2.1 Root README.md

Update:
- Badge URLs (version, docs.rs, CI, MSRV, license) — derive from crate name and repo URL in metadata
- Crates table — one row per crate with name, description, key API counts
- Features list — reflect actual API surface counts from extraction
- Protocol table — ports and transports
- MSRV value
- Quick start example — verify imports match current re-exports

Format: standard crate README. No emojis. Badges at top.

### 2.2 Per-crate README.md (`crates/*/README.md`)

For each crate, update:
- Crate-specific badges (crates.io version, docs.rs, license)
- Description from `Cargo.toml`
- Feature highlights with accurate counts from extraction
- Quick start example — must use the crate's own import path, not the umbrella crate
- MSRV, license

### 2.3 AGENTS.md

This is the most important file — it drives AI agent behavior. Wrong information here means wrong code generated. Accuracy is critical.

Update each section against extraction output:
- **Architecture tree** — reflect current module structure, file names, type names
- **Key Directories table** — verify every listed path exists; remove stale entries, add missing ones
- **Development Commands** — verify each command works
- **Code Conventions** — reflect current workspace lints from root `Cargo.toml`
- **Error Handling** — verify error enum variants match source
- **Builder Pattern** — verify builder methods match current API
- **Event System** — update event counts and serde strategy
- **Action Trait** — verify trait signature matches source
- **Handler Trait** — verify trait signature matches source
- **Handle Pattern** — verify handle types match source
- **Important Files table** — verify all paths exist, add new important files
- **Runtime & Tooling** — verify MSRV, dependencies, tools
- **CI Matrix** — update from CI workflow metadata
- **Testing section** — update test counts per file, identify coverage gaps
- **Examples table** — update from examples inventory
- **Dependency Policy** — verify license list matches `deny.toml`

### 2.4 Root CHANGELOG.md

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

- Verify `## [Unreleased]` section exists at the top
- When preparing a release: move Unreleased items to a new `## [version] - YYYY-MM-DD` section
- Subsections: Added, Changed, Deprecated, Removed, Fixed, Security
- Each item starts with crate name in backticks when the changelog is workspace-wide
- Do not remove or rewrite released sections — they are immutable history

### 2.5 Per-crate CHANGELOG.md (`crates/*/CHANGELOG.md`)

Same Keep a Changelog format. Only items relevant to that specific crate. Do not duplicate workspace-wide items unless they directly affect the crate.

### 2.6 CONTRIBUTING.md

Update:
- Build/test/lint commands — verify they match current workspace setup
- PR requirements — verify they match CI checks
- Tooling references (MSRV, formatter, linter settings)

### 2.7 mdBook (`docs/src/`)

Update:
- `SUMMARY.md` — verify all referenced `.md` files exist; add entries for new pages
- Per-page content — verify code examples compile, API references match current signatures
- Add new pages if significant new features were added

## Step 3: Verify

After updating, confirm:
1. All markdown files are syntactically valid (no broken internal links)
2. Code examples in READMEs use current API imports and compile
3. Counts in documentation match extraction script output exactly
4. No stale references to removed files, types, or modules
5. MSRV, version numbers, and license text are consistent across all files
6. AGENTS.md architecture tree matches actual directory structure

## Rules

- **Ground truth first**: never guess counts, versions, or API signatures — always verify against source or extraction output
- **Preserve prose**: keep existing descriptions that are still accurate — only change what is wrong
- **No emojis** in any documentation file
- **Comment style**: all lowercase, no trailing period in code comments; normal sentence case in documentation prose
- **Keep a Changelog**: use the standard format — do not invent custom changelog formats
- **Consistency**: same MSRV, license text, and repo URL across all files
- **Crate README quick starts**: must use the crate's own import path, not the umbrella crate
- **AGENTS.md accuracy**: this file drives AI agent behavior — every path, count, signature, and type name must match source
