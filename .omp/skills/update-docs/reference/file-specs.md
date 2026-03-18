# Documentation file specifications

Structural requirements and examples for each documentation file type.

## Root README.md

```markdown
# {crate_name}

{badges}

{one_line_pitch — what users can DO, not what the library contains}

- **AMI** -- {one sentence}
- **AGI** -- {one sentence}
- **ARI** -- {one sentence}

## Example

```rust,ignore
{real use case, not just ping — show subscribing to events, originating, or similar}
```

## Install

```toml
[dependencies]
{crate_name} = "{version}"
```

## Capabilities

- {capability described as user-facing ability}
- {not "161 typed events" but "typed events covering the full Asterisk 23 surface"}

## Protocols

| Protocol | Default Port | Transport | Use Case |
|----------|-------------|-----------|----------|

## Documentation

- [API Reference](https://docs.rs/{crate_name})
- [User Guide]({pages_url})

## MSRV / License
```

## Per-crate README.md

```markdown
# {crate_name}

{badges: crates.io, docs.rs}

{one_line_pitch}

{2-3 sentence description of what this protocol does}

```rust,ignore
{code example showing primary use case}
```

## Features

- {capability, not count}

Part of [asterisk-rs]({repo_url}). MSRV {msrv}. MIT/Apache-2.0.
```

## AGENTS.md

The most important file. Drives AI agent behavior.

Structure:
- Project Overview (one paragraph)
- Architecture tree (module names + one-line descriptions, no counts)
- Key Directories table
- Development Commands
- Code Conventions (lints, formatting, comment style, error handling)
- Pattern descriptions (builder, event system, handle, action trait, handler trait)
- Important Files table
- Runtime & Tooling
- CI Matrix
- Testing section (total count is fine here since it's for development, not users)
- Examples table

**Critical**: every path, type name, and pattern description must match source.
Prefer describing patterns over enumerating items.

## CHANGELOG.md

```markdown
## [Unreleased]

### Added

- `{crate}`: {what users can now do, not implementation detail}
```

Describe capability changes: "event-collecting actions for multi-event responses"
not "added EventListResponse type and PendingEventList tracking".

## mdBook guide pages

Guide pages explain HOW. Reference pages (auto-generated) explain WHAT.

Guide page structure:
```markdown
# {Topic}

{1-2 paragraphs explaining the concept}

## {Usage Pattern}

```rust,ignore
{code showing the pattern}
```

{explanation of what the code does}

See [Reference](./reference.md) for the complete list.
```

Never duplicate reference tables in guide pages. Link to reference.md.

## Rules (.omp/rules/)

Rules describe domain knowledge and conventions for AI agents.

- `asterisk`: protocol wire format details, auth mechanisms, message types
- `rust`: error handling, async patterns, type conventions, build commands

Rules should be stable knowledge that rarely changes. Don't enumerate
items that change with every commit (event counts, action lists).
