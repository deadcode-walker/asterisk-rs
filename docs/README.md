# Repository knowledge

This is the canonical index for durable asterisk-rs knowledge. The root instruction chain currently
contains only [`AGENTS.md`](../AGENTS.md); a future nested guide must be listed here with its scope and
must refine rather than duplicate the root rules.

## Canonical owners

| Location | Purpose and authority | Owner | Status | Freshness trigger |
|---|---|---|---|---|
| [`docs/README.md`](README.md) | canonical knowledge, scope, ownership, and freshness index | maintainers | active | any canonical document is added, moved, retired, or changes owner |
| [`AGENTS.md`](../AGENTS.md) | repository-wide routing and non-negotiable agent boundaries | maintainers | active | command, instruction-scope, or boundary change |
| [`ARCHITECTURE.md`](../ARCHITECTURE.md) | stable crate ownership, useful paths, and dependency direction | protocol maintainers | active | ownership or runtime-flow change |
| [`README.md`](../README.md) | public project entry point and supported capability overview | maintainers | public | user-visible capability or install change |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md) | contributor commands, evidence, documentation, and release-note expectations | maintainers | active | contribution or command policy change |
| [`PLANS.md`](PLANS.md) | contract for recoverable complex execution plans | maintainers | active | planning or recovery policy change |
| [`design-docs/project-decision-brief.md`](design-docs/project-decision-brief.md) | approved harness, toolchain, target, and authority decisions | repository owner | approved | a recorded constraint or material choice changes |
| [`design-docs/core-beliefs.md`](design-docs/core-beliefs.md) | stable engineering principles | maintainers | active | a principle is contradicted or retired |
| [`PRODUCT_SENSE.md`](PRODUCT_SENSE.md) | users, product principles, and non-goals | product maintainer | active | audience or product boundary changes |
| [`product-specs/index.md`](product-specs/index.md) | user-visible behavior ownership and acceptance boundaries | protocol maintainers | active | protocol surface or support promise changes |
| [`src/`](src/) | public mdBook concepts, how-to guidance, and generated protocol references | protocol maintainers | public | public API, wire behavior, or generator input changes |
| [`QUALITY_SCORE.md`](QUALITY_SCORE.md) | current evidence quality and remaining gaps | maintainers | current assessment | full review or gate changes |
| [`RELIABILITY.md`](RELIABILITY.md) | failure, bounds, recovery, and shutdown model | protocol maintainers | active | lifecycle or concurrency behavior changes |
| [`SECURITY.md`](SECURITY.md) | code and automation trust boundaries | security maintainer | active | threat, dependency, or workflow boundary changes |
| [`references/index.md`](references/index.md) | index of pinned external knowledge when offline ownership is necessary | maintainers | active | upstream contract or pinned reference changes |
| [`exec-plans/active/2026-08-modernization.md`](exec-plans/active/2026-08-modernization.md) | current modernization objective, decisions, evidence, risks, and next actions | plan owner | active | every material stopping point |
| [`exec-plans/tech-debt-tracker.md`](exec-plans/tech-debt-tracker.md) | accepted deferred debt with owner and reconsideration trigger | maintainers | active | debt accepted, resolved, or reclassified |
| [`../SECURITY.md`](../SECURITY.md) | public vulnerability-reporting policy | security maintainer | public | reporting channel or support policy changes |
| `crates/*/CHANGELOG.md` | package release history generated and updated by release-plz | release maintainer | release output | release PR generation or correction |

## Documentation and changelog workflow

Code and protocol guides are the source of truth for behavior. Generated reference pages under
`docs/src/` derive from Rust source through `generate.py`; change the source or generator, run
`just docs`, and do not maintain a competing hand-written reference.

Ordinary changes update affected guides, rustdoc, examples, and migration notes in the same slice.
They do not hand-edit a speculative release section. Accurate conventional commits feed release-plz,
which owns each package changelog and the umbrella GitHub release body. The release PR is where a
maintainer checks audience-facing wording, package scope, breaking markers, links, and duplicate
entries before publication. Changelogs summarize released changes; they never replace canonical
documentation or semver evidence.

Delete stale documents instead of retaining competing truths. Add a new document only with its first
verified fact, one accountable owner, and a row in this index.
