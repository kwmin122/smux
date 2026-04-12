# CODEX GSD Seeds

**Author:** Codex
**Date:** 2026-03-26
**Purpose:** Give Claude Code concrete wording for `gsd-health`, `gsd-new-milestone`, and follow-up `gsd-plan-phase` so the next planning cycle starts from the right thesis.

## Recommended Sequence

1. `gsd-health`
2. `gsd-new-milestone`
3. `gsd-plan-phase` for the first workflow-core phase

## Step 1: `gsd-health`

### Intent

Confirm that the current `.planning` state reflects an old milestone thesis and that a milestone reset is the right move.

### What Claude Code Should Look For

- `.planning/PROJECT.md`, `.planning/ROADMAP.md`, and `.planning/STATE.md` still center `v0.9 — PTY Stream Relay`
- handoff docs added by Codex center `Verified Multi-Agent Workflow`
- the repo now contains two competing planning narratives

### Expected Outcome

Claude Code should conclude:

- the planning tree is internally coherent for the old thesis
- but the thesis itself should be replaced before more phase planning happens

## Step 2: `gsd-new-milestone`

### Recommended milestone name

`Verified Multi-Agent Workflow`

### One-line milestone thesis

Turn smux from a PTY relay demo into a trustworthy planner/verifier coding workflow with structured adapters, explicit stage semantics, canonical evidence, and live operator visibility.

### Recommended milestone framing

#### Problem

The current PTY relay demonstrates visible agent ping-pong, but it does not provide a trustworthy workflow closure model for planning, verification, revision, implementation, and code review.

#### Why now

Continuing to optimize relay heuristics risks polishing the wrong layer. The next milestone should move source-of-truth logic to structured adapters and canonical workflow state while preserving the native PTY shell as product UX.

#### User promise

Users should be able to watch Claude and Codex collaborate in real panes, but the workflow should stay correct even when pane rendering, viewport contents, or TUI behavior change.

#### Non-goals

- shipping viewport diff relay as a public feature
- equating round count with progress
- scaling to N-agent consensus before the 2-agent path is trustworthy
- replacing native panes with a web-only shell

### Recommended requirements themes

Claude Code should bias requirements toward these themes:

- workflow correctness
- structured provider contracts
- transcript and evidence integrity
- operator intervention safety
- eval-driven release confidence
- staged rollout readiness

### Recommended milestone deliverables

- workflow state model
- provider adapter contract
- verdict schema
- canonical transcript/evidence model
- eval matrix
- operator control model
- release readiness checklist

## Step 3: `gsd-plan-phase`

Codex recommendation: start with the workflow core, not with more PTY relay mechanics.

### Phase 1 recommendation

**Suggested phase name:** `Workflow Core Reset`

**Goal:**
Define the authoritative session state machine, role model, and verifier verdict contract for the planner/verifier workflow.

**Why this phase comes first:**

- without a state model, structured adapters have no contract
- without a verdict contract, plan verification is still just conversation
- without explicit stage semantics, PTY UI cannot project meaningful state

**Suggested scope:**

- define stages and transitions
- define role ownership by stage
- define verdict vocabulary and schema
- define retry, blocked, and escalation behavior
- define canonical session event model

**Out of scope:**

- wiring live provider APIs
- PTY UI polish
- rollout mechanics

**Success criteria:**

- stage graph is explicit
- every transition has entry and exit criteria
- verifier verdicts are machine-readable
- fixture transcripts can be mapped to state transitions

### Phase 2 recommendation

**Suggested phase name:** `Structured Provider Adapters`

**Goal:**
Make Claude and Codex produce and consume structured artifacts as the canonical automation path.

**Suggested scope:**

- adapter boundary for Claude
- adapter boundary for Codex
- structured request/response envelopes
- transcript and evidence persistence
- resume and continuation semantics

**Success criteria:**

- workflow can run without trusting pane viewport text
- adapters can resume a session from canonical state
- stage artifacts validate against schema

### Phase 3 recommendation

**Suggested phase name:** `Operator Shell Projection`

**Goal:**
Reframe the native PTY UI as an operator shell backed by canonical workflow state.

**Suggested scope:**

- pane-role binding
- current stage and verdict visibility
- transcript and evidence inspector
- take over / retry / approve / reject controls

**Success criteria:**

- operator actions mutate canonical session state
- panes reflect workflow state instead of defining it
- manual takeover and resume are demonstrably safe

### Phase 4 recommendation

**Suggested phase name:** `Release Gate And Rollout`

**Goal:**
Prove the workflow is trustworthy enough for dogfood and then private beta.

**Suggested scope:**

- eval matrix
- dogfood protocol
- runbook
- rollout gates
- halt / rollback policy

**Success criteria:**

- release gates are written and testable
- internal dogfood passes with real provider runs
- known failure classes are documented with operator guidance

## Suggested Phase Planning Questions

Claude Code should force these questions during phase planning:

1. What becomes the source of truth after this phase?
2. What exact artifact proves that truth?
3. What nondeterminism enters here?
4. What eval catches it?
5. What operator action is possible if the phase fails in production?

## Anti-Patterns To Reject

- phases named around relay mechanics rather than workflow outcomes
- "works in demo" used as a completion argument
- stage progress inferred from alternating turns
- release claims without dogfood or eval evidence
- UI-first planning before workflow semantics exist

## Copy-Paste Prompt Seed

If Claude Code wants a short prompt for milestone creation:

> Replace the current PTY-relay-as-core milestone with a new milestone called Verified Multi-Agent Workflow. Keep native PTY panes as operator UX, but move workflow authority to structured Claude/Codex adapters, explicit stage semantics, canonical transcripts, verifier verdicts, evals, and release gates.

If Claude Code wants a short prompt for the first phase:

> Plan the first phase as Workflow Core Reset. Define the stage machine, role ownership, verdict schema, canonical session events, and blocked/retry/escalation paths before any further PTY relay work.
