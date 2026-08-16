# Repository harness and modernization

## Purpose

Replace stale repository guidance and fragmented automation, resolve current GitHub reports, review
the full Rust/API/CI surface, modernize dependencies and tooling, and leave `main` with reproducible
local and GitHub evidence.

## Progress

- [x] 2026-08-16: inventoried Git, repository structure, package graph, tests, workflows, issues,
  pull requests, dependency state, installed tools, and baseline gates.
- [x] 2026-08-16: measured native feedback and selected Cargo plus a thin just facade; rejected
  Bazel until a repository-specific scaling problem exists.
- [x] 2026-08-16: scaffolded the progressive-disclosure harness and began replacing generic text
  with repository facts.
- [x] 2026-08-16: proved the harness structure, formatting, full-target Clippy, 915 unit tests, and
  211 mock integration tests through `just check`.
- [ ] Resolve open code and dependency findings, then complete Rust review passes.
- [ ] Replace duplicated/insecure workflow policy and validate GitHub YAML.
- [ ] Run frozen-candidate local and live-available gates, fresh diff review, and fixes.
- [ ] Push, monitor GitHub checks, and close only superseded or evidenced issues/PRs.

## Surprises and discoveries

- Root Cargo.toml declares Rust 1.86 while old AGENTS.md, clippy.toml, and README text said 1.83.
- The lockfile contains vulnerable quinn-proto 0.11.14 even though the current dependency feature
  graph does not use it on the host; a normal lock update selects a patched release.
- Current unit, mock, workspace, minimal-feature, policy, and typo gates pass, but cargo-deny reports
  two unused license allowances and Clippy reports the conflicting MSRV configuration.
- Issue 60 identifies a real ARI wire-name mismatch: `caller_id` serializes with the Rust field name
  while Asterisk expects `callerId`.

## Decision log

- Keep Cargo authoritative because crates.io/release-plz require it and no Bazel trigger is measured.
- Keep harness documentation public because it benefits every contributor and automation client.
- Use one active plan for this cross-cutting run and coherent commits for harness, product fixes, and
  automation rather than one unreviewable rewrite.

## Context and orientation

The root workspace contains five publishable crates under `crates/` and one external test crate.
GitHub has two open Dependabot PRs and two open issues. `main` began clean at commit 7597555. See
AGENTS.md and ARCHITECTURE.md for the stable map.

## Plan of work

1. Establish the harness, just recipes, mechanical structure checks, and canonical docs.
2. Update and audit dependencies, fix issue 60 with behavior tests, align edition/MSRV/lints, and
   inspect protocol lifecycle/security/error paths for concrete defects.
3. Rebuild workflows around just recipes, immutable action revisions, minimal permissions,
   non-duplicated triggers, required-job aggregation, and explicit release/live boundaries.
4. Run focused and full gates, live Asterisk tests when Docker is usable, inspect the exact diff,
   and repeat review/fix until clean.
5. Push commits, monitor Actions, resolve surfaced failures, close superseded dependency PRs and
   fixed issues with precise comments.

## Concrete steps

Use `just check` during implementation. Use `just ci`, `just msrv`, and `just live` on the candidate.
Use `gh` to inspect checks, logs, issues, and PRs; do not close external records before pushed
evidence exists.

## Validation and acceptance

Acceptance is the decision brief plus: clean Git status after commits, all available just gates
passing, GitHub required checks green on pushed main, issue 60 covered by a wire-format assertion,
the vulnerable lock entry removed or patched, and open dependency PRs either integrated or closed as
superseded.

## Idempotence and recovery

All checks are repeatable. Generated docs are regenerated before comparison. Dependency updates are
captured only in Cargo.lock. If a push exposes a platform-only failure, fix forward in a new commit
and preserve earlier coherent commits.

## Outcomes and retrospective

Complete when the pushed candidate and external records are settled. Move this file to `completed/`
only after that evidence exists.
