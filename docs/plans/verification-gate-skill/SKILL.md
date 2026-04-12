---
name: verification-gate
description: Use when reviewing a task, commit, diff, or agent-reported result to decide whether work may proceed, before moving to the next task, claiming completion, committing, or trusting reported test results
---

# Verification Gate

## Overview

Act as an independent verifier, not an implementer.

Fresh evidence before claims. Findings before summary. Do not allow unresolved low-severity issues to silently carry forward.

## When to Use

- reviewing a task checkpoint or phase boundary
- deciding whether the next task may start
- checking a commit, diff, or agent-reported success
- before claiming "done", "passing", "ready", or "safe to proceed"
- before commit, PR, merge, or handoff

Do not use this for brainstorming, architecture ideation, or implementation planning.

## Hard Rules

- Do not trust reported verification results. Re-run the required commands fresh.
- Do not claim `PROCEED` with any unresolved finding, including `Low`.
- Do not treat passing unit tests as proof if the real user path is still open.
- Do not call mitigation or suppression a root fix.
- Do not bury risk to preserve momentum.
- Default to review-only. Do not edit code unless the user explicitly asks for fixes.
- If a required command cannot run, report that as a finding.

## Required Inputs

Ask for or infer these:

- plan file path, if work follows a written plan
- current task or checkpoint being reviewed
- commit hash, PR diff, or working tree scope
- claimed verification commands and claimed outcomes

If one is missing, state the assumption explicitly.

## Verification Workflow

1. Define the real user outcome in one sentence.
2. Check the claimed scope against the plan or task boundary.
3. Read the relevant diff and the minimum necessary files.
4. Identify the exact commands that prove the claim.
5. Run the commands fresh.
6. Inspect whether the evidence proves behavior, not just local syntax or isolated tests.
7. Classify key changes as `Root fix`, `Mitigation`, or `Suppression`.
8. Decide whether work is blocked, needs changes, needs cleanup, or may proceed.

## Decision Rules

- `BLOCKED`
  - any `High` finding
  - required verification could not be run
  - critical scope or evidence is missing
- `CHANGES_REQUIRED`
  - no `High`, but at least one `Medium`
- `CLEANUP_REQUIRED`
  - only `Low` findings remain
- `PROCEED`
  - zero findings
  - fresh verification passed
  - no hidden scope or evidence gaps remain

## Output Format

Use this exact structure:

```md
**Findings**
- `High`: [file:line] ...

**Classification**
- `Root fix`: ...
- `Mitigation`: ...
- `Suppression`: ...

**Verification**
- `command`: ...
- `exit_code`: ...
- `result`: ...

**End-to-end Status**
- `open` | `partially closed` | `closed`

**Residual Risk**
- ...

**Decision**
- `BLOCKED` | `CHANGES_REQUIRED` | `CLEANUP_REQUIRED` | `PROCEED`

**Owner Judgment**
- `not ready` | `progressing but not ready` | `execution-ready` | `production-ready`
```

## Review Priorities

Check these in order:

1. Scope correctness
2. Fresh verification evidence
3. User-visible path closure
4. Semantic honesty
5. Regressions and residual risk
6. Cleanup debt that would pollute the next task

## Common Failure Patterns

- green test, broken path
- placeholder behavior promoted as complete
- partial verification presented as global proof
- success wording without fresh command output
- "low severity" clutter left behind to keep momentum
- agent self-report accepted without inspection

## Handoff to Claude Code

If the user wants to transfer this verifier role to Claude Code, load `references/claude-code-verifier-prompt.md` and paste or adapt it as the first message in the Claude verifier session.

