# Repository knowledge

This index is the versioned knowledge system of record for asterisk-rs. The root instruction chain
currently contains only [`AGENTS.md`](../AGENTS.md); a future nested guide must be listed here with
its scope and refine rather than duplicate the root rules. Every canonical document has one purpose,
authority, owner, status, and freshness trigger.

## Current documents

| Document | Purpose and authority | Owner | Status | Revisit when |
|---|---|---|---|---|
| [`docs/README.md`](README.md) | canonical knowledge, scope, ownership, and freshness index | maintainers | active | any canonical owner is added, moved, retired, or changes owner |
| [`AGENTS.md`](../AGENTS.md) | repository-wide routing, execution contract, and non-negotiable boundaries | maintainers | active | command, instruction scope, autonomy, recovery, or authority changes |
| [`ARCHITECTURE.md`](../ARCHITECTURE.md) | stable useful paths, code/state ownership, dependency direction, toolchain/evidence authority, and deliberate absences | protocol maintainers | active | ownership, runtime flow, toolchain, target, evidence, or delivery changes |
| [`README.md`](../README.md) | public project entry point and supported capability overview | maintainers | public | user-visible capability, support, or installation changes |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md) | contributor commands, proof, documentation, and release-note expectations | maintainers | active | contribution, command, or review policy changes |
| [`PLANS.md`](PLANS.md) | normative complex-work, unattended-cycle, and context-handoff contract | maintainers | active | planning, execution, recovery, or completion evidence changes |
| [`design-docs/project-decision-brief.md`](design-docs/project-decision-brief.md) | accepted harness/toolchain outcome, constraints, risks, authority, and acceptance | repository owner | approved | product intent, constraint, material tool choice, or authority changes |
| [`design-docs/core-beliefs.md`](design-docs/core-beliefs.md) | stable engineering principles | maintainers | active | a principle is contradicted or retired |
| [`PRODUCT_SENSE.md`](PRODUCT_SENSE.md) | users, product principles, and non-goals | product maintainer | active | audience or product boundary changes |
| [`product-specs/index.md`](product-specs/index.md) | user-visible behavior ownership and acceptance boundaries | protocol maintainers | active | protocol surface or support promise changes |
| [`src/`](src/) | public mdBook concepts, how-to guidance, and generated protocol references | protocol maintainers | public | public API, wire behavior, or generator input changes |
| [`QUALITY_SCORE.md`](QUALITY_SCORE.md) | current evidence quality and known gaps | maintainers | current assessment | full review, evidence, or gate changes |
| [`RELIABILITY.md`](RELIABILITY.md) | failure, bounds, recovery, and shutdown model | protocol maintainers | active | lifecycle or concurrency behavior changes |
| [`SECURITY.md`](SECURITY.md) | code, dependency, automation, and evidence trust boundaries | security maintainer | active | threat, dependency, workflow, or authority boundaries change |
| [`references/index.md`](references/index.md) | pinned external knowledge required offline or repeatedly | maintainers | active | upstream contract or pinned reference changes |
| [`exec-plans/completed/2026-08-modernization.md`](exec-plans/completed/2026-08-modernization.md) | completed modernization decisions, evidence, and retrospective | plan owner | completed | immutable historical record |
| [`exec-plans/tech-debt-tracker.md`](exec-plans/tech-debt-tracker.md) | accepted deferred debt with owner and reconsideration trigger | maintainers | active | debt is accepted, resolved, or reclassified |
| [`../SECURITY.md`](../SECURITY.md) | public vulnerability-reporting policy | security maintainer | public | reporting channel or support policy changes |
| `crates/*/CHANGELOG.md` | released package history generated and updated by release-plz | release maintainer | release output | release PR generation or correction |

## Create a category with its first fact

Do not create empty indexes, placeholder directories, or copies of facts already owned by code,
schemas, configuration, generated output, or another document. Add a product spec, decision,
reference, runbook, scorecard, reliability/security guide, or execution plan only with its first
verified fact, an accountable owner, a status, a freshness trigger, and a row in this index.

Generated facts need a named generator and freshness check. Pinned external references need a reason
they must remain available offline or repeatedly; ordinary links do not require a local mirror.

## Documentation and changelog workflow

Code, rustdoc, and protocol guides own behavior. Rustdoc is the exhaustive API inventory; curated
reference pages under `docs/src/` explain where behavior lives and link to rustdoc. Do not maintain
a competing handwritten or regex-generated symbol table. Run `just docs-check` to compile the
representative snippets, enforce the missing-doc ratchet, and build both documentation surfaces.

Ordinary changes update affected guides, rustdoc, examples, and migration notes in the same slice.
They do not hand-edit speculative `Unreleased` sections. Accurate conventional commits feed
release-plz, which owns package changelogs and the umbrella release body. A maintainer reviews the
release PR for audience-facing wording, package scope, breaking markers, links, and duplicates.
Changelogs summarize released changes; they never replace canonical documentation or semver proof.

## Maintenance

Delete stale guidance rather than retaining competing truths. `just harness` checks indexed paths,
local links, exact active-plan structure, instruction size, and stable repository boundaries; add
another gardening check only after observed drift warrants its cost. Age alone is neither proof of
staleness nor a reason to keep a document.
