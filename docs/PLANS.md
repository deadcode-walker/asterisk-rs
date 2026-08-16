# Execution plans

Use a checked-in ExecPlan for work that is complex, risky, multi-session, or depends on discoveries.
Small work uses an in-session plan. Keep one applicable plan under `exec-plans/active/`; move it to
`completed/` only after its outcome is proved.

Before work begins, record the outcome, non-goals, granted authority, baseline, material assumptions,
side effects, and acceptance-to-evidence map. Every plan remains self-contained for a newcomer with
only the current tree and uses these maintained sections:

- Purpose and scope
- Progress with timestamps and exact tree identities at checkpoints
- Surprises and discoveries
- Decision log, including rejected alternatives
- Context and orientation
- Plan of work
- Concrete steps
- Validation and acceptance
- Idempotence and recovery
- Outcomes and retrospective

At every material stopping point, persist the current objective, authority, exact tree, decisions,
evidence, risks, and next action. A process exit, elapsed time, changed-line count, subcheck green, or
review request is not completion. Freeze the best candidate, run the applicable complete gate once,
and bind fresh review to that exact tree. Later changes receive proportionate fresh proof.

If evidence stagnates, an assumption fails, scope grows, or the verifier cannot prove acceptance,
stop that tactic, identify the first contradicted assumption, update the plan, and continue with a
materially different action. Ask the user only for new authority, unavailable external state, or an
unresolved choice that materially changes the outcome.
