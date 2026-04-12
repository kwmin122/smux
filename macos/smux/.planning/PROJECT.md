# Project: SUN

## Core Value

SUN is a terminal-first harness OS that turns one builder into an operator-grade software organization. It replaces ad-hoc AI assistance with a structured workflow runtime that delivers enterprise-grade engineering discipline -- spec, plan, implement, verify, ship -- without enterprise headcount or process overhead.

## Target User

Solo power user (developer, founder, technical operator) who:
- Wants to go from vague idea to shipped result with maximum leverage
- Values reproducibility, verification, and auditability over speed alone
- Uses terminal as primary workspace (not web UI)
- Works with AI agents (Claude, Codex, others) as implementation tools
- Needs the discipline of a well-run engineering org without the org

## Problem Statement

Today's AI-assisted development is structurally fragile:
- Context evaporates between sessions -- work is lost, decisions are forgotten
- No verification discipline -- AI output is trusted on vibes, not evidence
- No workflow closure -- stages blur together, nothing formally completes
- No auditability -- you can't replay how a decision was made or why code changed
- No operator control -- the human is either micromanaging or fully hands-off with no middle ground

These are not UX problems. They are runtime problems. No amount of prompt engineering or slash-command ergonomics fixes them. The fix is a harness runtime that enforces workflow stages, preserves context, gates on evidence, and gives the operator real control.

## Product Identity

**Terminal-First Harness OS for Solo Builders**

- Terminal UX is mandatory: real visible agent activity in real PTY sessions
- The moat is the harness runtime underneath the terminal, not the terminal itself
- SUN is not a "nicer Claude wrapper" or a "terminal multiplexer with AI features"
- SUN is an operating system for workflows that happen to render in a terminal

## Differentiation

| Dimension | Claude Code / Codex | GSD | SUN |
|-----------|-------------------|-----|-----|
| Workflow structure | None (chat) | Slash-command phases | Explicit stage graph engine |
| Context continuity | Session-scoped | STATE.md (fragile) | Durable context engine with handoff/recovery |
| Verification | Trust the output | Optional verification gate | Mandatory evidence-based gates |
| Approval control | None or full-auto | User discipline | Policy-based routing engine |
| Auditability | Chat log | Summaries | Full replay/audit trail |
| Reproducibility | None | Git commits | Sandboxed execution model |
| Provider model | Single provider | Claude-centric | Provider-agnostic workflow engine |

## Product Pillars

1. **Workflow Engine** -- Explicit stage graph with typed transitions, completion criteria, retry/resume semantics
2. **Context Engine** -- Durable memory, structured summaries, session handoff, crash recovery
3. **Verification Engine** -- Test gates, review gates, eval gates, acceptance criteria with evidence collection
4. **Approval Engine** -- Human-in-the-loop controls, policy-based auto-approve/manual-review routing
5. **Replay/Audit Engine** -- Full traceability from idea to shipped artifact, debugging time-travel
6. **Reproducible Execution** -- Sandbox model ensuring same inputs produce same outputs
7. **Operator Terminal UX** -- Visible agent sessions, manual takeover, real PTY interaction
8. **Workflow Pack System** -- GSD as one importable pack, user-authored packs, community packs

## Constraints

- **Terminal-first**: Core interactions happen in terminal. Web/GUI is supplementary, never primary.
- **Solo-user optimized**: No team coordination, no multi-tenant, no access control in v1.
- **Provider-agnostic**: Must work with Claude, Codex, and future providers. Provider commands are adapters, not core.
- **Local-first**: All state lives on the user's machine. No cloud dependency for core workflow.
- **No code in planning phase**: This project is being designed before implementation begins.

## Non-Goals

- NOT a PTY relay product (smux already explored this -- SUN's terminal is a rendering surface, not the product)
- NOT just a nicer Claude/Codex wrapper
- NOT GitHub-star bait (optimize for user outcome, not demo virality)
- NOT team-only enterprise software first
- NOT web UI as core identity
- NOT a prompt library or "AI skill" collection

## GSD Relationship

SUN's relationship to GSD is deliberate and specific:

**INHERIT from GSD:**
- Phase-based planning model (phases, plans, requirements mapping)
- Atomic commit discipline
- Verification gates as a concept
- State tracking as a concept (STATE.md pattern)

**REPLACE in SUN:**
- Ad-hoc slash commands --> structured workflow engine with typed stages
- Implicit state (convention-based files) --> explicit stage graph with runtime enforcement
- Optional verification --> mandatory evidence-based gates (configurable strictness)
- Loose plan/execute/verify cycle --> formal workflow with completion criteria and transitions
- Claude-specific assumptions --> provider-agnostic workflow engine

**KEEP COMPATIBLE:**
- GSD workflow packs can be imported as one workflow type in SUN
- Users familiar with GSD phases/plans will recognize the structure
- Migration path from GSD projects to SUN projects

**REJECT:**
- GSD's "CLI skill" identity (SUN is a runtime, not a skill)
- Slash-command ergonomics as architecture (commands are UI, not structure)
- Implicit conventions as enforcement mechanism (SUN enforces explicitly)

## Technical Direction

SUN is a **runtime** that manages workflow execution. Key architectural bets:

1. **Stage graph as data structure**: Workflows are directed graphs of typed stages, not scripts. The engine traverses the graph, the user observes progress, gates block until evidence is provided.

2. **Context as first-class resource**: Not "save a summary file" but a managed context engine that knows what's relevant, what's stale, and what needs handoff between stages.

3. **Terminal as rendering surface**: The terminal shows real agent activity (PTY sessions with real processes). But the harness runtime is what matters -- the terminal could theoretically be replaced with any rendering surface.

4. **Provider adapters, not provider integration**: Claude, Codex, etc. are adapters that conform to SUN's provider interface. SUN doesn't know or care about provider internals.

5. **Workflow packs as configuration**: GSD-style workflows, custom workflows, community workflows -- all expressed as declarative pack definitions that the engine executes.

## Key Decisions

| # | Decision | Rationale | Date |
|---|----------|-----------|------|
| 1 | SUN-native workflow core first (not GSD compat first) | The structural moat is the workflow engine. GSD compat is a pack that runs on the engine, not a constraint on the engine's design. Building compat first would compromise the engine's generality. | 2026-03-26 |
| 2 | Terminal-first but terminal is not the product | smux R&D proved terminal rendering is solvable. SUN's value is the runtime underneath. Terminal UX is Phase 4, not Phase 1. | 2026-03-26 |
| 3 | Local-first, no cloud dependency | Solo builder's data stays on their machine. Cloud sync/backup is a future layer, not a v1 requirement. | 2026-03-26 |

## v1 Priority Recommendation

**Recommendation: (a) SUN-native workflow core first**

**Rationale:**

The core thesis is that SUN's moat is the harness runtime, not the terminal and not GSD compatibility. Building the workflow engine, context engine, and verification engine first establishes the structural foundation that everything else depends on.

The hybrid approach (c) sounds pragmatic but creates a real risk: GSD's conventions (file-based state, slash-command dispatch, Claude-specific assumptions) would leak into the engine's design, making it harder to build the general-purpose runtime SUN needs to be.

GSD compatibility (b) as a starting point would mean SUN v1 is "GSD but slightly better" -- which is not a product, it's an iteration.

The correct sequence:
1. Build the workflow engine that can express any workflow (including but not limited to GSD)
2. Build the context and verification engines that make workflows durable and trustworthy
3. Build the terminal UX that makes workflows visible and controllable
4. Build the GSD workflow pack as proof that the engine is general enough

This sequence means GSD users don't get compat on day one. That's acceptable because SUN v1 targets users who want the full harness, not users who want "GSD plus." GSD compat comes in Phase 5 as validation that the engine design is correct.
