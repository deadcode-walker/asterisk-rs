# Documentation file specifications

Reference templates and structural requirements for each documentation file.

## Root README.md

```markdown
# {crate_name}

![crates.io](https://img.shields.io/crates/v/{crate_name}.svg)
![docs.rs](https://img.shields.io/docsrs/{crate_name})
![CI](https://github.com/{repo_owner}/{repo_name}/actions/workflows/ci.yml/badge.svg)
![MSRV](https://img.shields.io/badge/MSRV-{msrv}-blue)
![License](https://img.shields.io/badge/license-{license_badge}-blue)

{short_description}

## Overview
{paragraph_description}

## Crates
| Crate | Description |
|---|---|
| `{name}` | {description} |

## Quick Start
```sh
cargo add {crate_name}
```
```rust,no_run
{example_code}
```

## Features
- {feature_list}

## Protocols (if applicable)
| Protocol | Port | Transport | Crate |
|---|---|---|---|

## MSRV
{msrv}

## License
{license_text}

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
```

## Per-crate README.md

```markdown
# {crate_name}

![crates.io](https://img.shields.io/crates/v/{crate_name}.svg)
![docs.rs](https://img.shields.io/docsrs/{crate_name})
![MSRV](https://img.shields.io/badge/MSRV-{msrv}-blue)
![License](https://img.shields.io/badge/license-{license_badge}-blue)

{description_from_cargo_toml}

## Features
- {feature_highlights_with_counts}

## Quick Start

Add to your `Cargo.toml`:
```toml
[dependencies]
{crate_name} = "{version}"
```

```rust,no_run
{minimal_example}
```

## MSRV
{msrv}

## License
{license_text}
```

## AGENTS.md structure

The AGENTS.md file must contain these sections in order:

1. **Repository Guidelines** (h1)
2. **Project Overview** (h2) — one paragraph summary
3. **Architecture & Data Flow** (h2) — ASCII tree showing crate structure, module listing per crate with file.rs → purpose annotations
4. **Key Directories** (h2) — table: Path | Purpose
5. **Development Commands** (h2) — code block with build, test, lint, format, doc commands
6. **Code Conventions & Common Patterns** (h2) — subsections for:
   - Workspace-Level Lints
   - Formatting
   - Comment Style
   - Error Handling (with error enum hierarchy)
   - Builder Pattern (with examples)
   - Event System (with counts and strategy)
   - Reconnection
   - Handle Pattern (if applicable)
   - Protocol-specific traits (Action, Handler, etc.)
   - Credentials/Security patterns
7. **Important Files** (h2) — table: File | Role
8. **Runtime & Tooling** (h2) — bullet list of runtime deps, MSRV, tools
9. **CI Matrix** (h2) — table: Job | Runs On | Toolchain | What
10. **Testing** (h2) — subsections:
    - Framework
    - Test Location & Count (table: File | Tests | Coverage)
    - Test Patterns
    - Coverage Gaps
    - Running Tests (commands)
    - Examples (table: Example | Crate | Demonstrates)
11. **Dependency Policy** (h2) — allowed licenses, enforcement

## CHANGELOG format

Follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/):

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- {description of new feature}

### Changed
- {description of change}

### Fixed
- {description of fix}

## [{version}] - {YYYY-MM-DD}

### Added
- {items}
```

Rules:
- each entry is a single line starting with `- `
- for workspace changelogs, prefix with crate name in backticks: `` - `asterisk-rs-ami`: added X ``
- released sections are immutable — never modify them
- unreleased section is always present at top
- subsections (Added/Changed/etc.) only appear when they have content

## CONTRIBUTING.md

```markdown
# Contributing

## Development

```sh
# build
{build_command}

# test
{test_command}

# lint
{lint_command}

# format
{format_command}
```

## Pull requests

- One logical change per PR
- All CI checks must pass
- Add tests for new functionality
- Use conventional commits: `feat(scope): description`

## Code of Conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
```

## SECURITY.md

Typically static — only update the supported versions table when versions change.

## mdBook SUMMARY.md

```markdown
# Summary

- [Getting Started](./getting-started.md)

# {Section Name}

- [{Topic}](./{path}/overview.md)
  - [{Subtopic}](./{path}/{page}.md)
```

Rules:
- every entry must point to an existing .md file
- use `#` headers to group related pages
- indent subtopics with two spaces
