# CODEX Milestone Context

**Author:** Codex
**Date:** 2026-03-26
**Purpose:** Explain why the current PTY ping-pong relay direction should not be treated as the product core, and give Claude Code a clear milestone reset brief for `gsd-new-milestone` and follow-up `gsd-plan-phase`.

## Read This First

This file is intentionally named with a `CODEX-` prefix for handoff clarity.

If Claude Code wants GSD to auto-consume this context during milestone creation, either:

1. read this file manually and carry the contents into `$gsd-new-milestone`, or
2. temporarily rename/copy it to `.planning/MILESTONE-CONTEXT.md` before running the command.

## Companion Documents

Read these files together with this one before changing the milestone thesis:

- `.planning/CODEX-RELEASE-READINESS.md`
- `.planning/CODEX-GSD-SEEDS.md`

## Why Codex Is Proposing a New Plan

The current native shell work proves that smux can:

- render real PTY panes,
- capture PTY output,
- inject text into the opposite pane,
- and demo a visible A->B->A->B loop.

That is enough for a demo.

It is not enough for a trustworthy product.

Codex reviewed the current PTY relay flow and found that the implementation is still centered on a **terminal relay heuristic**, not on a **stage-gated multi-agent workflow**.

### Problems that triggered this reset

1. **No stage semantics**
The router alternates panes by round count. It does not model:
`plan -> verify -> revise -> verify -> implement -> code review`.
There is no explicit approval contract, rejection loop, or stage exit criteria.

2. **Viewport text is not a canonical transcript**
The relay still depends on what is visible in the terminal surface at turn boundaries.
That is not reliable enough for long plans, code blocks, or review findings.

3. **Provider identity is not enforced**
The panes are generic login shells. The product does not truly know:
"this is Claude planner" and "this is Codex verifier" as first-class system roles.

4. **Turn detection is heuristic and brittle**
Short verifier responses, redraw-heavy TUIs, paste semantics, and ignore windows can all create false negatives, stalls, or partial relay.

5. **The repo's own design direction already points elsewhere**
The validated design docs say Claude and Codex both support structured headless paths and that PTY should not be the default system-of-record path.

## Codex Conclusion

Do **not** continue treating "real PTY ping-pong relay" as the product core.

Keep PTY panes because they are valuable for:

- operator visibility,
- manual intervention,
- trust,
- and product differentiation.

But move the actual workflow engine to a **hybrid architecture**:

- **Headless structured adapters as the source of truth**
- **Explicit planner/verifier stage machine**
- **PTY panes as operator console / projection / takeover surface**

## Recommended Milestone

**Suggested name:** `Verified Multi-Agent Workflow`

**One-sentence goal:**
Turn smux from a brittle PTY relay demo into a shippable planner/verifier product with explicit stage semantics, canonical transcripts, structured evidence, and live operator visibility.

## Target Features For The New Milestone

- Explicit workflow stages:
  `Ideate`, `Plan`, `Verify Plan`, `Execute`, `Verify Code`, `Harden`
- Provider-aware roles:
  planner, verifier, worker, optional additional verifier/integrator later
- Structured adapters for Claude and Codex as canonical execution path
- Canonical transcript and evidence store
- Verifier verdict contract:
  `APPROVED`, `REJECTED`, `NEEDS_INFO`, `BLOCKED`
- PTY operator console that reflects canonical session state instead of acting as the source of truth
- Release gates based on live provider E2E, not just code-level wiring

## Non-Goals For This Milestone

- Shipping viewport-diff relay as a public product feature
- Treating round count as workflow progress
- Treating shell text capture as authoritative workflow state
- Expanding to N-agent consensus before the 2-agent planner/verifier flow is trustworthy
- Replacing the native macOS shell with a web shell

## Recommended Phase Breakdown

### Phase A: Workflow Core Reset

Define the stage model and the approval contract.

Deliverables:

- session state machine
- stage transition rules
- verdict schema
- failure / retry / blocked handling

### Phase B: Structured Provider Adapters

Make Claude and Codex run through structured headless adapters as the canonical path.

Deliverables:

- adapter contract review
- canonical transcript representation
- evidence artifact persistence
- context handoff rules between stages

### Phase C: PTY As Operator Shell

Reframe the native shell as a projection and takeover layer, not the workflow engine.

Deliverables:

- pane-to-session role binding
- inspector driven by canonical transcript
- explicit "take over", "replay", "retry", "approve" controls
- transcript/evidence visibility in UI

### Phase D: Ship Gate

Prove that the real Claude/Codex flow is trustworthy enough to dogfood.

Deliverables:

- live provider E2E matrix
- short-response handling
- stuck detection and recovery
- release checklist
- human verification protocol

## Constraints Claude Code Should Preserve

- Keep Korean IME and native AppKit/libghostty quality as a non-negotiable constraint.
- Keep the visible native shell experience; do not regress to a purely invisible daemon product.
- Do not claim shipment based on code-level wiring alone.
- Do not let PTY relay heuristics define business logic.
- Use the Rust orchestration core for canonical workflow state where possible.

## What Claude Code Should Do Next

1. Start a new milestone from this context, not by extending the current PTY relay thesis.
2. Write milestone requirements around **trustworthy workflow closure**, not around raw relay mechanics.
3. Roadmap the milestone around the four phase groups above.
4. Plan the first phase as the workflow-core reset before any additional PTY relay polishing.

## Suggested GSD Flow

Recommended sequence:

1. `gsd-health`
2. `gsd-new-milestone`
3. `gsd-plan-phase` for the first workflow-core phase

If Claude Code needs a shorter prompt:

> Replace the current PTY-relay-as-core milestone with a new milestone centered on a verified multi-agent workflow. Use headless structured adapters as system of record, PTY panes as operator shell, and prioritize stage semantics plus evidence over terminal relay heuristics.

## Source Evidence Claude Code Should Review

- `.planning/ROADMAP.md`
- `.planning/STATE.md`
- `docs/superpowers/specs/2026-03-21-smux-design.md`
- `docs/plans/2026-03-24-native-multi-agent-relay-design.md`
- `macos/smux/Sources/SmuxApp/PingPongRouter.swift`
- `macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift`
- `macos/smux/Sources/SmuxApp/PTYManager.swift`

## Final Intent

This is not a request to delete the PTY shell work.

It is a request to **put the PTY shell in the right layer**:

- great UX surface,
- poor workflow source of truth.

That distinction is the core reason for the milestone reset.
