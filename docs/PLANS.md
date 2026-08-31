# Execution plans

Use a checked-in ExecPlan only when work is complex, risky, discovery-heavy, or likely to cross
contexts. Small changes use an in-session plan. Keep one applicable plan under `exec-plans/active/`;
move it to `exec-plans/completed/` only after observable acceptance is proved.

An ExecPlan is a self-contained living document for a capable agent that has the current tree but no
private chat history. It defines repository-specific terms, names exact owners and commands, and ends
in demonstrably working behavior rather than source shape or activity counts.

## Artifact and action semantics

A request to “create,” “make,” “write,” “record,” “save,” or “check in” a plan authorizes creation
or update of the durable plan artifact when this contract applies. Use
`$harness-engineering:write-exec-plan` when available. Do not return only plan prose unless the user
explicitly requested an outline, chat-only response, or no writes, or the active product mode
forbids mutation. Plan-file authority does not authorize implementation or external effects.

A request to “resume,” “execute,” “continue,” or “finish” a plan means read the selected active plan
and perform its authorized work through `$harness-engineering:execute-repository-work`; do not merely
summarize or replace it. Fresh plan or candidate review uses
`$harness-engineering:review-repository-work` against a pinned target and stays read-only.

## Context and state contract

Durable state and visible model context are different. A compacted thread continues but may lose
detail; a new session, worker, or subagent receives only its supplied instructions and explicitly
loaded artifacts. Before compaction, delegation, controller restart, deliberate fresh context, or a
material stopping point, update the plan with the objective, exact candidate, decisions, evidence,
risks, authority, blocker status, and next action. On resume, reload the instruction chain and plan,
verify the tree and external assumptions, and continue from evidence.

Use a persistent product Goal only when the user or controlling workflow explicitly requests one.
The checked-in plan remains the portable repository recovery authority.

Before creating or resuming a checked-in ExecPlan, invoke
`$harness-engineering:load-harness-context` when available, then read this contract and the selected
active plan before the first implementation mutation. If unavailable, the same local read order is
mandatory.

### Execution capability selection

At each context boundary, consider the execution surfaces available in the current environment and
keep the current context as integration owner and default sole writer. Use `codex exec` as the
default isolation surface for a bounded read-only plan or candidate review; run
`codex exec --help` before relying on flags and declare its root, sandbox/approval boundary, result
contract, budget, and ownership. Before handing off a checked-in plan or claiming a material
candidate complete, run one such fresh review when honest isolation is available. Record a concrete
fallback otherwise. Do not continue coherent work in nested Codex, launch duplicate reviewers,
bypass authority, or overlap writers.

For a plan executed through the Harness plugin, each independently landable task is an exact-candidate
review checkpoint. The primary trajectory implements and repairs the task. A fresh reviewer stays
read-only; after a material repair, that same reviewer re-reviews the new exact candidate until the
unchanged candidate is approved. Approval makes an effectful task ready for its protected delivery
path; it does not prove the external effect. After delivery, freeze and review the resulting proof and
bookkeeping before marking that task complete.

The final landing review covers cumulative proof, cross-task seams, and plan lifecycle. It does not
repeat every approved task review. Unplanned work retains the repository's ordinary single
frozen-candidate review boundary.

## Required living sections

Every active plan uses these exact H2 sections:

- **Purpose and non-goals:** what becomes possible, for whom, and what remains outside scope.
- **Authority and side effects:** permitted writes/effects, explicit prohibitions, approvals, and rollback.
- **Progress:** timestamped results, exact tree identities, evidence, partial state, and next action.
- **Surprises and discoveries:** unexpected facts with concise evidence.
- **Decision log:** each consequential choice, rationale, date, and owner.
- **Outcomes and retrospective:** achieved acceptance, remaining gaps, and reusable lessons.

## Required execution sections

- **Context and orientation:** architecture, affected owners, baseline, terms, and assumptions.
- **Milestones:** coherent slices that each end in an independently observable result.
- **Concrete steps:** exact commands, working directories, and short expected evidence.
- **Validation and acceptance:** criteria mapped to focused checks, complete gates, or runtime
  journeys; say why a proof is not applicable instead of inventing it.
- **Idempotence and recovery:** safe retry, best candidate, partial-failure cleanup, and rollback.
- **Interfaces and dependencies:** required public contracts and why each dependency exists.

## Execution policy

Work one smallest coherent outcome at a time: prove the baseline, change one causal owner, run the
cheapest representative evidence, inspect the diff, and update the plan. Continue without asking for
routine next steps while the next action remains authorized.

Exit the tactic when a failure recurs without new evidence, an assumption is contradicted, scope
grows, the verifier cannot prove acceptance, or the candidate regresses. Record the first
unrecoverable step, choose a materially different action, preserve the best candidate, and continue.

Each unattended cycle records external wall-clock, tool, retry, and concurrency ceilings plus an
explicit terminal reason. These bounds stop one cycle, not the objective. Complete only when
acceptance is proved. Report blocked only when progress requires new authority, unavailable external
state, or an unresolved outcome-changing decision.
