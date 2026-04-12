# CODEX Release Readiness

**Author:** Codex
**Date:** 2026-03-26
**Purpose:** Define what "ship-ready" means for the proposed verified multi-agent workflow so Claude Code does not confuse a successful PTY demo with a releasable product.

## Executive Judgment

If Claude Code follows only the direction in `.planning/CODEX-MILESTONE-CONTEXT.md`, the likely outcome is:

- **prototype** if only the architecture reset is done
- **internal dogfood** if structured adapters and workflow state machine are working
- **private beta** if evals, operator recovery, and rollout controls are also working
- **public release** only after explicit release gates, PRR-style checks, and staged rollout evidence pass

The important distinction is:

- `visible PTY relay demo` is not the same as
- `trustworthy multi-agent product`

## What Must Be True Before smux Is Public-Release Ready

### 1. PTY is not the workflow source of truth

The visible panes can remain a core part of the product experience, but public shipment should not depend on viewport text extraction or turn-complete heuristics as the authoritative workflow state.

Public shipment requires:

- structured Claude and Codex execution paths
- canonical message/event storage
- explicit stage transitions
- persisted verdict and evidence artifacts

### 2. The workflow closes by stage, not by round count

The product should be able to prove where a session is inside a fixed stage graph such as:

`Ideate -> Plan -> Verify Plan -> Revise -> Execute -> Verify Code -> Harden -> Done`

Each transition should have:

- entry criteria
- exit criteria
- machine-readable verdicts
- retry / blocked / escalation paths

### 3. The system can be evaluated continuously

Release decisions must be based on repeatable evidence, not on "it looked good in a live demo".

At minimum, the workflow needs automated and human-reviewed evals for:

- stage handoff accuracy
- verifier verdict correctness
- tool selection correctness
- structured output schema validity
- transcript completeness
- timeout / stuck recovery
- human takeover and resume behavior

### 4. The operator can intervene safely

Because this is a multi-agent coding tool, the operator path is part of the product, not a debug-only escape hatch.

Release-ready operator behavior requires:

- clear current stage and current owner
- explicit pause / take over / retry / approve / reject actions
- replayable evidence
- resume without transcript corruption

### 5. Deployment is progressive and reversible

A public release should be gated by internal dogfood and private beta waves, each with health checks and stop conditions.

## Recommended Methodology

This section is the practical method Claude Code should follow when reframing the next milestone.

### Method 1: Contract-first

Define machine-readable schemas before building workflow automation.

Suggested contract objects:

- `plan_artifact`
- `verifier_verdict`
- `revision_request`
- `execution_result`
- `code_review_result`
- `session_state_snapshot`
- `release_evidence`

Why this matters:

- Codex officially supports non-interactive automation through `codex exec`, JSONL events, and `--output-schema`.
- Claude Code officially supports programmatic use through `claude -p`, `--bare`, `--output-format json|stream-json`, and `--json-schema`.

That means the product has a realistic path to structured orchestration without relying on TUI scraping as the system of record.

### Method 2: Eval-first

Define eval datasets and pass criteria before claiming milestone success.

Suggested eval buckets for smux:

- `workflow_closure`
  - Does the session reach the correct next stage?
- `verdict_accuracy`
  - Does the verifier choose `APPROVED`, `REJECTED`, `NEEDS_INFO`, or `BLOCKED` correctly?
- `handoff_accuracy`
  - Does the system pass control to the right role at the right time?
- `tool_and_argument_correctness`
  - Are tool calls and arguments correct?
- `transcript_integrity`
  - Are messages lost, duplicated, truncated, or reordered?
- `recovery_behavior`
  - Do retry, timeout, and stuck handling work?
- `human_takeover`
  - After manual intervention, can the session resume correctly?

Suggested rule:

- no phase is "done" until its eval pack exists and runs

### Method 3: PRR-first

Treat production readiness as a separate review, not as a side effect of implementation.

Suggested PRR checklist for smux:

- availability and latency expectations defined
- failure domains documented
- operator runbook exists
- rollback / roll-forward procedure exists
- logs, metrics, and traces exist
- secret handling and key storage are reviewed
- staging and production isolation are defined
- release halt conditions are defined

### Method 4: Safe deployment

Use wave-based rollout, not a one-shot public push.

Suggested deployment ladder:

1. local developer sessions
2. internal dogfood
3. limited private beta
4. broader private beta
5. public release

Each step should require:

- health gates
- error budget check
- explicit human approval
- rollback or halt path

## Suggested smux Release Gates

These are proposed product gates for the next milestone. They are not yet implemented. Claude Code should convert them into milestone requirements and phase exit criteria.

### Gate 0: Architecture Gate

Must be true:

- PTY panes are explicitly treated as operator UX
- structured adapters are the canonical execution path
- canonical session state exists outside the pane viewport

Evidence:

- adapter contract document
- workflow state model
- ADR or equivalent architecture note

### Gate 1: Workflow Correctness Gate

Must be true:

- stage graph exists
- verdict contract exists
- blocked / retry / revise paths exist
- workflow does not advance on round count alone

Evidence:

- state machine specification
- structured fixture runs for happy and unhappy paths

### Gate 2: Eval Gate

Must be true:

- eval sets exist for single-stage and multi-stage behavior
- CI or repeatable local runs can execute them
- thresholds are written down

Suggested initial thresholds:

- schema validity: 100%
- transcript integrity on fixture runs: 100%
- stage handoff accuracy on curated set: >= 95%
- verdict accuracy on curated set: >= 90%
- manual takeover/resume success on scripted scenarios: >= 95%

These numbers are product suggestions, not external standards. Claude Code can tune them, but should not remove the gate.

### Gate 3: Operator Gate

Must be true:

- user can see current role, stage, verdict, and evidence
- user can pause, take over, retry, approve, or reject
- resume does not corrupt session state

Evidence:

- operator UX walkthrough
- manual runbook
- scenario test log

### Gate 4: Production Operations Gate

Must be true:

- keys and secrets are handled safely
- staging and production are isolated
- rate/spend monitoring exists
- incidents can be detected and triaged
- rollback or safe halt exists

Evidence:

- deployment/runbook doc
- observability checklist
- secret management note

### Gate 5: Rollout Gate

Must be true:

- internal dogfood passed for real tasks
- private beta passed with external users
- known failure cases are documented
- release halt thresholds are defined

Evidence:

- dogfood report
- beta issue log
- go/no-go review note

## Recommended Artifact Set For The Milestone

Claude Code should try to produce these artifacts over the next milestone:

- milestone context
- requirements doc
- roadmap
- workflow ADR or equivalent design note
- provider adapter contract
- verdict schema
- eval matrix
- operator runbook
- release readiness checklist
- dogfood report template

## How To Use This With GSD

### During `gsd-new-milestone`

Use this document to prevent the new milestone from being phrased as:

- "make PTY ping-pong work better"
- "improve relay accuracy"
- "finish the relay"

Instead phrase it as:

- "ship a verified multi-agent workflow with structured adapters, explicit stage semantics, operator controls, and release evidence"

### During `gsd-plan-phase`

Every planned phase should answer:

1. what contract becomes authoritative?
2. what evals prove this phase works?
3. what operator behavior changes?
4. what release gate does this phase help close?

If a phase cannot answer those questions, it is probably still scoped around relay mechanics instead of shipment.

## Source Notes

These sources informed the readiness model. Claude Code can cite or revisit them when writing milestone requirements.

- OpenAI Codex non-interactive docs:
  - https://developers.openai.com/codex/noninteractive
  - Relevant points: `codex exec`, JSONL events, schema-based output, automation/CI orientation
- Claude Code headless docs:
  - https://code.claude.com/docs/en/headless
  - Relevant points: `claude -p`, `--bare`, structured output, streaming JSON, session resume
- OpenAI eval best practices:
  - https://developers.openai.com/api/docs/guides/evaluation-best-practices
  - Relevant points: eval-driven development, log everything, automate scoring, continuous evaluation, multi-agent handoff evaluation
- OpenAI production best practices:
  - https://developers.openai.com/api/docs/guides/production-best-practices
  - Relevant points: staging/production isolation, key safety, spend/rate monitoring, scaling, MLOps
- Google SRE PRR:
  - https://sre.google/sre-book/evolving-sre-engagement-model/
  - Relevant points: PRR as a prerequisite for production ownership, early engagement is preferable
- Azure Well-Architected safe deployment guidance:
  - https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments
  - Relevant points: small incremental releases, progressive exposure, health checks, immediate halt and recovery

## Final Rule

Do not let the next milestone claim "ship-ready" unless the answer is yes to both questions:

1. Is the workflow trustworthy without trusting the terminal viewport?
2. Is there repeatable evidence that real users can operate, recover, and ship with it safely?

If either answer is no, the correct label is not "released". It is still "prototype", "dogfood", or "private beta".
