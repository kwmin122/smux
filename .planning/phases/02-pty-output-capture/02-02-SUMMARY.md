---
phase: 02-pty-output-capture
plan: 02
subsystem: terminal-capture
tags: [swift, ghostty, pty, pingpong, timer, notificationcenter, ansi]

# Dependency graph
requires:
  - phase: 02-01
    provides: captureViewportText() API, ANSIStripper.strip(), ghosttyCommandFinished notification name

provides:
  - GhosttyTerminalView.startCapturing/stopCapturing timer-driven polling at 4 Hz
  - PingPongRouter real PTY capture integration with OSC 133 + silence timeout turn detection
  - Full PTY output capture pipeline delivering ANSI-stripped text via onTurnComplete callback

affects:
  - phase-03-relay-injection (depends on onTurnComplete delivering clean text)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Timer.scheduledTimer at 0.25s on main RunLoop for ghostty_surface_read_text polling"
    - "DispatchWorkItem on DispatchQueue.global() for cancellable silence timeout"
    - "NotificationCenter observer for OSC 133 COMMAND_FINISHED as primary turn-complete signal"
    - "Double-fire prevention: OSC 133 cancels DispatchWorkItem before processTurnComplete()"

key-files:
  created: []
  modified:
    - macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift
    - macos/smux/Sources/SmuxApp/PingPongRouter.swift

key-decisions:
  - "Timer.scheduledTimer on main RunLoop required because ghostty_surface_read_text has Metal thread-safety constraint"
  - "Delta filter threshold of 4 chars chosen to suppress cursor blink and spinner noise without missing real output"
  - "DispatchQueue.global() for silence timer (not main) to avoid blocking main RunLoop, but processTurnComplete dispatched back to .main"
  - "deinit calls stop() as safety net — critical for cleanup when WorkspaceWindowController releases PingPongRouter"

patterns-established:
  - "Pattern: Timer-driven viewport polling at 4 Hz with delta filtering for ghostty EXEC mode capture"
  - "Pattern: Dual turn-complete detection (OSC 133 primary, silence timeout fallback) with cancellation for double-fire prevention"

requirements-completed: [PTY-CAP-01, PTY-CAP-02]

# Metrics
duration: 9min
completed: 2026-03-26
---

# Phase 02 Plan 02: PTY Capture Integration Summary

**PingPongRouter wired to real ghostty EXEC mode PTY via 4 Hz polling timer + OSC 133 COMMAND_FINISHED notification + 2-second silence timeout fallback, delivering ANSI-stripped output through onTurnComplete callback**

## Performance

- **Duration:** 9 min
- **Started:** 2026-03-26T03:58:09Z
- **Completed:** 2026-03-26T04:07:13Z
- **Tasks:** 2 auto + 1 checkpoint:human-verify (APPROVED 2026-03-26)
- **Files modified:** 2

## Accomplishments

- GhosttyTerminalView gains timer-driven polling wrapper (startCapturing/stopCapturing) at 4 Hz with ANSI stripping and delta-change noise filtering
- PingPongRouter fully rewritten from placeholder stub to real capture integration with dual turn-complete detection (OSC 133 primary, silence timeout fallback)
- Double-fire prevention: handleCommandFinished cancels silence DispatchWorkItem before calling processTurnComplete
- All resources (Timer, DispatchWorkItem, NotificationCenter observer) cleaned up in stop() with deinit as safety net
- Public API preserved — WorkspaceWindowController.togglePingPong() works without modification

## Task Commits

Each task was committed atomically:

1. **Task 1: Add startCapturing/stopCapturing polling wrapper to GhosttyTerminalView** - `c7547cd` (feat)
2. **Task 2: Rewrite PingPongRouter with real capture integration, OSC 133 subscription, and silence timeout** - `85b3211` (feat)

Task 3 `checkpoint:human-verify` — **APPROVED** (2026-03-26). Build succeeds, sendNotification crash fixed (Bundle.main.bundleIdentifier guard), Cmd+Shift+P toggle wiring verified.

## Files Created/Modified

- `macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift` - Added startCapturing/stopCapturing polling methods in MARK: - Text Capture section
- `macos/smux/Sources/SmuxApp/PingPongRouter.swift` - Fully rewritten from stub to real PTY capture integration

## Decisions Made

- Timer runs on main RunLoop (required for ghostty_surface_read_text Metal thread safety)
- Delta filter of > 4 chars suppresses cursor blink without missing meaningful output
- Silence timeout uses DispatchQueue.global() asyncAfter with DispatchWorkItem for cancellability
- processTurnComplete dispatched back to DispatchQueue.main from global queue

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - build succeeded on first attempt for both tasks.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None - startCapturing and processTurnComplete wire to real captureViewportText(). No placeholder data flows to UI. The checkpoint (Task 3) is a human-verify gate to confirm end-to-end behavior at runtime, not a stub.

## Next Phase Readiness

- PTY capture pipeline is complete: polling → ANSI strip → turn detection → onTurnComplete callback
- PingPongRouter is ready to receive Phase 3 relay injection (write captured output to the other pane's stdin)
- WorkspaceWindowController.togglePingPong() wires onTurnComplete — Phase 3 adds the sendText call there
- Human verification (Task 3 checkpoint) APPROVED — build compiles, sendNotification crash fixed, toggle wiring confirmed

---
*Phase: 02-pty-output-capture*
*Completed: 2026-03-26*
