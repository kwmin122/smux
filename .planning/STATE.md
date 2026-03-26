---
gsd_state_version: 1.0
milestone: v0.9
milestone_name: PTY Stream Relay
status: planning
stopped_at: "v0.9 planning docs created — ready for Phase 1 discuss/plan"
last_updated: "2026-03-26T07:00:00Z"
last_activity: 2026-03-26
progress:
  total_phases: 3
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-26)

**Core value:** Two AI agents ping-pong in real visible PTYs — user sleeps, wakes up to idea→plan→impl→review all done.
**Current focus:** v0.9 Phase 1 — HOST_MANAGED PTY foundation

## Current Position

Phase: 01 (host-managed-pty) — NOT STARTED
Plan: 0 of TBD
Status: Planning docs complete, ready for Phase 1 research → plan → execute
Last activity: 2026-03-26

Progress: [░░░░░░░░░░] 0%

## v0.8 Completion

All 4 phases of v0.8 completed (13/13 requirements), but Phase 3 relay proved architecturally broken for TUI agents. Two critical bugs:
1. Viewport capture includes full TUI chrome (not just response)
2. Self-injection feedback loop (silence timeout fires on own injected text)

v0.9 replaces the entire capture/relay architecture: EXEC → HOST_MANAGED, viewport polling → raw PTY stream.

## Accumulated Context

### Decisions

- v0.8: EXEC mode chosen — FAILED (viewport capture broken for TUI)
- v0.9: HOST_MANAGED mode chosen — smux owns PTY, ghostty renders only
- Silence timeout (3s) as primary turn signal — OSC 133 doesn't fire in TUI agents
- CPtyHelper (smux_forkpty) already exists but not wired to Package.swift
- receive_buffer callback: ghostty calls this when it wants to write keyboard input to PTY child
- ghostty_surface_write_buffer(): host sends raw PTY output bytes to ghostty for rendering

### Pending Todos

- Wire CPtyHelper into Package.swift as target dependency
- Empirically verify HOST_MANAGED mode with current ghostty 1.3.1 xcframework
- Verify Korean IME works in HOST_MANAGED mode
- Test ghostty_surface_write_buffer() thread safety

### Blockers/Concerns

- HOST_MANAGED mode is UNTESTED with current ghostty version — Phase 1 starts with PoC verification
- ghostty_surface_write_buffer() likely must dispatch to main thread (Metal constraint)
- receive_buffer semantics may differ from expected — needs empirical testing

## Session Continuity

Last session: 2026-03-26
Stopped at: v0.9 planning docs created (PROJECT.md, REQUIREMENTS.md, ROADMAP.md, STATE.md)
Resume: Start Phase 1 with /gsd:discuss-phase or /gsd:plan-phase
