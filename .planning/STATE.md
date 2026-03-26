---
gsd_state_version: 1.0
milestone: v0.8
milestone_name: milestone
status: executing
stopped_at: Completed 02-pty-output-capture-02-01-PLAN.md
last_updated: "2026-03-26T03:58:09.294Z"
last_activity: 2026-03-26
progress:
  total_phases: 4
  completed_phases: 1
  total_plans: 3
  completed_plans: 2
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-26)

**Core value:** Two AI agents ping-pong in real visible PTYs — user sleeps, wakes up to idea→plan→impl→review all done.
**Current focus:** Phase 02 — pty-output-capture

## Current Position

Phase: 02 (pty-output-capture) — EXECUTING
Plan: 1 of 2
Status: Executing Phase 02
Last activity: 2026-03-26 -- Phase 02 execution started

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
| Phase 01-surface-lifecycle-fix P01 | 3 | 2 tasks | 3 files |
| Phase 02-pty-output-capture P01 | 6 | 2 tasks | 3 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- EXEC mode chosen over HOST_MANAGED (better rendering; capture workaround via ghostty_surface_read_text required)
- Real visible PTY is non-negotiable product identity (headless approach rejected)
- ghostty_surface_read_text polling chosen as PTY capture strategy (pending verification)
- [Phase 01-surface-lifecycle-fix]: Task.detached { @MainActor } for ghostty_surface_free matches Ghostty own pattern; contentView=nil BEFORE surface_free is critical ordering
- [Phase 01-surface-lifecycle-fix]: NSWindowDelegate.windowWillClose added to WorkspaceWindowController as primary Cmd+W teardown hook
- [Phase 02-pty-output-capture]: Use GHOSTTY_POINT_VIEWPORT with large coords (9999,9999) for full visible viewport reads; ghostty clamps to actual content
- [Phase 02-pty-output-capture]: UInt(bitPattern: surface) as opaque key in @convention(c) actionCb — pointer never dereferenced async

### Pending Todos

None yet.

### Blockers/Concerns

- Phase 1: Metal layer zombie on ⌘W — fix is known (contentView=nil → Task.detached surface_free order)
- Phase 2: receive_buffer NOT called in EXEC mode — ghostty_surface_read_text polling is the workaround path
- Phase 4: BrowserPanelView, BrowserAutomation, SessionDetachReattach, AppleScriptSupport code exists but is unverified end-to-end

## Session Continuity

Last session: 2026-03-26T03:58:09.289Z
Stopped at: Completed 02-pty-output-capture-02-01-PLAN.md
Resume file: None
