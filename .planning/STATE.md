# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-26)

**Core value:** Two AI agents ping-pong in real visible PTYs — user sleeps, wakes up to idea→plan→impl→review all done.
**Current focus:** v0.8 — Ping-Pong Core

## Current Position

Phase: 1 of 4 (Surface Lifecycle Fix)
Plan: 0 of 1 in current phase
Status: Ready to plan
Last activity: 2026-03-26 — Roadmap created for v0.8 Ping-Pong Core

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**
- Total plans completed: 0
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**
- Last 5 plans: —
- Trend: —

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- EXEC mode chosen over HOST_MANAGED (better rendering; capture workaround via ghostty_surface_read_text required)
- Real visible PTY is non-negotiable product identity (headless approach rejected)
- ghostty_surface_read_text polling chosen as PTY capture strategy (pending verification)

### Pending Todos

None yet.

### Blockers/Concerns

- Phase 1: Metal layer zombie on ⌘W — fix is known (contentView=nil → Task.detached surface_free order)
- Phase 2: receive_buffer NOT called in EXEC mode — ghostty_surface_read_text polling is the workaround path
- Phase 4: BrowserPanelView, BrowserAutomation, SessionDetachReattach, AppleScriptSupport code exists but is unverified end-to-end

## Session Continuity

Last session: 2026-03-26
Stopped at: Roadmap created — ready to plan Phase 1
Resume file: None
