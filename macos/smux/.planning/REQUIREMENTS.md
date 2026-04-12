# Requirements: SUN

## Requirement Format

Each requirement has:
- **ID**: Category prefix + sequential number (e.g., WF-01)
- **Priority**: v1 (must ship) or v2 (deferred)
- **Description**: What the user can do or what must be true

## Categories

| Prefix | Category | Pillar |
|--------|----------|--------|
| WF | Workflow Engine | 1. Workflow Engine |
| CX | Context Engine | 2. Context Engine |
| VF | Verification Engine | 3. Verification Engine |
| AP | Approval Engine | 4. Approval Engine |
| RA | Replay/Audit Engine | 5. Replay/Audit Engine |
| SB | Reproducible Execution | 6. Sandbox Model |
| TX | Operator Terminal UX | 7. Terminal UX |
| WP | Workflow Pack System | 8. Workflow Packs |

---

## Pillar 1: Workflow Engine

The stage graph runtime that drives all structured work.

| ID | Priority | Requirement |
|----|----------|-------------|
| WF-01 | v1 | User can define a workflow as a directed graph of named stages with typed transitions (sequential, conditional, parallel fork/join) |
| WF-02 | v1 | Each stage has explicit entry criteria, execution logic, and completion criteria that the engine enforces |
| WF-03 | v1 | The engine supports retry semantics: a failed stage can be retried without re-running the entire workflow |
| WF-04 | v1 | The engine supports resume: a workflow interrupted by crash or user action can resume from the last completed stage |
| WF-05 | v1 | The engine supports pause/unpause: user can halt a running workflow and resume it later |
| WF-06 | v1 | Stage transitions are atomic: either a stage completes fully and the next begins, or the transition fails and state rolls back |
| WF-07 | v1 | The engine emits structured events for every state change (stage entered, completed, failed, retried, paused) |
| WF-08 | v1 | Workflows can be parameterized: the same workflow definition can run with different inputs (project path, provider, config overrides) |
| WF-09 | v1 | The engine provides a CLI interface for starting, pausing, resuming, and inspecting workflows |
| WF-10 | v1 | Concurrent workflows: user can run multiple independent workflows simultaneously (e.g., feature branch A and feature branch B) |

## Pillar 2: Context Engine

Durable memory and structured handoff between stages and sessions.

| ID | Priority | Requirement |
|----|----------|-------------|
| CX-01 | v1 | Each workflow execution maintains a context store that accumulates structured data (decisions, artifacts, findings) across stages |
| CX-02 | v1 | Context is scoped: stage-local context (scratch) vs workflow-level context (persistent) vs project-level context (cross-workflow) |
| CX-03 | v1 | Stage handoff includes a structured summary: what was done, what was decided, what the next stage needs to know |
| CX-04 | v1 | Context survives process crashes: all context writes are durable (fsync or equivalent) before stage completion is acknowledged |
| CX-05 | v1 | User can inspect the full context of any workflow execution at any point (current or historical) |
| CX-06 | v1 | Context includes token-budget-aware summarization: large context is compressed to fit provider token limits while preserving decision-critical information |
| CX-07 | v1 | Session continuity: when a new terminal session connects to a running or paused workflow, it receives the full context needed to continue |

## Pillar 3: Verification Engine

Evidence-based gates that ensure quality before stage transitions.

| ID | Priority | Requirement |
|----|----------|-------------|
| VF-01 | v1 | Verification gates can be attached to any stage transition in the workflow graph |
| VF-02 | v1 | A verification gate specifies required evidence types (test results, review approval, eval scores, file existence, custom checks) |
| VF-03 | v1 | Gates block stage transitions until all required evidence is provided and passes acceptance criteria |
| VF-04 | v1 | Verification evidence is collected and stored as structured data (not just pass/fail, but the actual test output, review comments, eval results) |
| VF-05 | v1 | User can configure gate strictness per workflow: strict (all evidence required), normal (critical evidence required), relaxed (advisory only) |
| VF-06 | v1 | Built-in verification types: test suite pass, file diff review, build success, linter pass, custom script exit code |
| VF-07 | v1 | Verification results are included in the audit trail with timestamps and evidence payloads |

## Pillar 4: Approval Engine

Human-in-the-loop controls with policy-based routing.

| ID | Priority | Requirement |
|----|----------|-------------|
| AP-01 | v1 | User can define approval policies: which stage transitions require human approval and which can auto-proceed |
| AP-02 | v1 | Approval requests present the user with a structured summary of what happened, what will happen next, and what evidence was collected |
| AP-03 | v1 | User can approve, reject, or request-changes at any approval gate |
| AP-04 | v1 | Rejection triggers a configurable response: retry stage, skip stage, or abort workflow |
| AP-05 | v1 | Policy-based auto-approval: low-risk transitions (e.g., lint pass to format) can be configured to auto-approve while high-risk transitions (e.g., deploy) always require human review |

## Pillar 5: Replay/Audit Engine

Full traceability from idea to shipped artifact.

| ID | Priority | Requirement |
|----|----------|-------------|
| RA-01 | v1 | Every workflow execution produces a complete audit trail: sequence of stages, transitions, evidence, approvals, context snapshots |
| RA-02 | v1 | Audit trails are stored as structured data (not just logs) that can be queried and filtered |
| RA-03 | v1 | User can replay a workflow execution step-by-step to understand how a result was produced |
| RA-04 | v1 | Audit trails include timing data: when each stage started, how long it ran, where time was spent waiting for approval vs executing |
| RA-05 | v1 | User can export audit trails in a portable format (JSON lines) for external analysis |

## Pillar 6: Reproducible Execution

Sandbox model ensuring deterministic workflow behavior.

| ID | Priority | Requirement |
|----|----------|-------------|
| SB-01 | v1 | Each workflow execution operates in an isolated working directory with explicit input/output declarations |
| SB-02 | v1 | Workflow execution captures the environment state at start (tool versions, provider versions, relevant config) for reproducibility records |
| SB-03 | v1 | Side effects (file writes, git operations, external calls) are declared in stage definitions and tracked by the engine |
| SB-04 | v1 | User can dry-run a workflow to see what stages would execute, what evidence would be required, and what side effects would occur, without actually executing |

## Pillar 7: Operator Terminal UX

Visible agent sessions with manual takeover capability.

| ID | Priority | Requirement |
|----|----------|-------------|
| TX-01 | v1 | Workflow execution is visible in the terminal: user can see which stage is running, what the agent is doing, and what output is being produced |
| TX-02 | v1 | User can manually take over an agent session mid-execution: pause the workflow, type commands directly into the agent's PTY, then resume |
| TX-03 | v1 | Multiple workflow sessions are visible simultaneously in split terminal panes |
| TX-04 | v1 | Terminal shows a persistent status bar with current workflow state, stage progress, and pending approvals |
| TX-05 | v1 | Agent output is streamed in real-time, not buffered until stage completion |
| TX-06 | v1 | User can scroll back through agent output history within a session |

## Pillar 8: Workflow Pack System

GSD as one pack among many, user-authored packs, community ecosystem.

| ID | Priority | Requirement |
|----|----------|-------------|
| WP-01 | v1 | Workflow packs are declarative definitions (YAML/TOML) that specify stage graphs, verification gates, approval policies, and provider configurations |
| WP-02 | v1 | SUN ships with a built-in "default" workflow pack that covers the spec-plan-implement-verify-ship cycle |
| WP-03 | v1 | A GSD compatibility pack maps GSD's phase/plan/verify model onto SUN's workflow engine |
| WP-04 | v1 | User can create custom workflow packs by authoring pack definition files |
| WP-05 | v1 | Packs can be composed: a stage in one pack can invoke another pack as a sub-workflow |
| WP-06 | v1 | Pack definitions support provider-specific adapter configuration (which provider to use for which stage, with fallback chains) |

---

## v2 (Deferred)

| ID | Category | Requirement |
|----|----------|-------------|
| WF-11 | Workflow | Visual workflow editor (graph UI for designing stage graphs) |
| WF-12 | Workflow | Workflow templates marketplace |
| CX-08 | Context | Cross-project context sharing (learnings from project A inform project B) |
| CX-09 | Context | AI-powered context relevance scoring (auto-prune stale context) |
| VF-08 | Verification | AI-powered code review as verification type (not just human review) |
| VF-09 | Verification | Verification benchmarks (track quality metrics across workflow runs) |
| AP-06 | Approval | Team approval routing (multiple approvers, quorum) |
| AP-07 | Approval | Approval delegation (auto-approve if designated reviewer is unavailable) |
| RA-06 | Replay/Audit | Audit trail comparison (diff two workflow runs) |
| RA-07 | Replay/Audit | Compliance report generation from audit trails |
| SB-05 | Sandbox | Container-based execution isolation (Docker/nsjail) |
| SB-06 | Sandbox | Remote execution (run workflows on cloud VMs) |
| TX-07 | Terminal | Web-based terminal viewer (share workflow progress via URL) |
| TX-08 | Terminal | Mobile notification integration (approval requests on phone) |
| WP-07 | Packs | Community pack registry (publish/discover/install packs) |
| WP-08 | Packs | Pack versioning and dependency resolution |

---

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| WF-01 | Phase 1 | Pending |
| WF-02 | Phase 1 | Pending |
| WF-03 | Phase 1 | Pending |
| WF-04 | Phase 1 | Pending |
| WF-05 | Phase 1 | Pending |
| WF-06 | Phase 1 | Pending |
| WF-07 | Phase 1 | Pending |
| WF-08 | Phase 1 | Pending |
| WF-09 | Phase 1 | Pending |
| WF-10 | Phase 1 | Pending |
| CX-01 | Phase 2 | Pending |
| CX-02 | Phase 2 | Pending |
| CX-03 | Phase 2 | Pending |
| CX-04 | Phase 2 | Pending |
| CX-05 | Phase 2 | Pending |
| CX-06 | Phase 2 | Pending |
| CX-07 | Phase 2 | Pending |
| VF-01 | Phase 3 | Pending |
| VF-02 | Phase 3 | Pending |
| VF-03 | Phase 3 | Pending |
| VF-04 | Phase 3 | Pending |
| VF-05 | Phase 3 | Pending |
| VF-06 | Phase 3 | Pending |
| VF-07 | Phase 3 | Pending |
| AP-01 | Phase 3 | Pending |
| AP-02 | Phase 3 | Pending |
| AP-03 | Phase 3 | Pending |
| AP-04 | Phase 3 | Pending |
| AP-05 | Phase 3 | Pending |
| RA-01 | Phase 3 | Pending |
| RA-02 | Phase 3 | Pending |
| RA-03 | Phase 3 | Pending |
| RA-04 | Phase 3 | Pending |
| RA-05 | Phase 3 | Pending |
| SB-01 | Phase 1 | Pending |
| SB-02 | Phase 1 | Pending |
| SB-03 | Phase 1 | Pending |
| SB-04 | Phase 1 | Pending |
| TX-01 | Phase 4 | Pending |
| TX-02 | Phase 4 | Pending |
| TX-03 | Phase 4 | Pending |
| TX-04 | Phase 4 | Pending |
| TX-05 | Phase 4 | Pending |
| TX-06 | Phase 4 | Pending |
| WP-01 | Phase 5 | Pending |
| WP-02 | Phase 5 | Pending |
| WP-03 | Phase 5 | Pending |
| WP-04 | Phase 5 | Pending |
| WP-05 | Phase 5 | Pending |
| WP-06 | Phase 5 | Pending |
