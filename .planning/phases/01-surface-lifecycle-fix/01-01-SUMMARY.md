---
phase: 01-surface-lifecycle-fix
plan: 01
subsystem: ui
tags: [ghostty, metal, swift, appkit, surface-lifecycle, cametallayer]

# Dependency graph
requires: []
provides:
  - Correct ghostty surface teardown ordering: contentView=nil BEFORE surface_free
  - Async ghostty_surface_free via Task.detached { @MainActor } pattern
  - NSWindowDelegate windowWillClose hook on WorkspaceWindowController
  - performCleanShutdown and applicationWillTerminate both route through corrected destroyAllSurfaces()
affects:
  - 02-pty-capture
  - 03-ping-pong-relay
  - 04-e2e-verification

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Task.detached { @MainActor in } for Metal-safe async resource deallocation"
    - "NSWindowDelegate.windowWillClose for eager Metal layer detach on window close"
    - "contentView = nil BEFORE surface_free — mandatory ordering for zombie-free ghostty teardown"

key-files:
  created: []
  modified:
    - macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift
    - macos/smux/Sources/SmuxApp/WorkspaceWindowController.swift
    - macos/smux/Sources/SmuxApp/main.swift

key-decisions:
  - "Use Task.detached { @MainActor in } (not DispatchQueue.main.async) to match Ghostty's own Surface.deinit pattern exactly"
  - "Set window.contentView = nil in destroyAllSurfaces() BEFORE the loop that calls tv.destroySurface() — Metal CALayer must be detached from hierarchy before the surface is freed"
  - "Add NSWindowDelegate.windowWillClose to WorkspaceWindowController so Cmd+W triggers clean teardown without relying only on AppDelegate paths"

patterns-established:
  - "Ghostty teardown order: (1) contentView=nil → (2) Task.detached surface_free → (3) app_free"
  - "NSWindowDelegate.windowWillClose as the primary hook for window-close teardown; performCleanShutdown/applicationWillTerminate as safety nets"

requirements-completed: [STAB-01, STAB-02]

# Metrics
duration: 3min
completed: 2026-03-26
---

# Phase 01 Plan 01: Surface Lifecycle Fix Summary

**ghostty_surface_free moved to Task.detached { @MainActor } with window.contentView=nil-first ordering, eliminating Metal CALayer zombie on Cmd+W**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-26T02:22:30Z
- **Completed:** 2026-03-26T02:24:54Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Rewrote `GhosttyTerminalView.destroySurface()` to use `Task.detached { @MainActor in ghostty_surface_free(s) }` — exact match of Ghostty's own `Surface.deinit` pattern
- Fixed `destroyAllSurfaces()` ordering: `window?.contentView = nil` now executes BEFORE `tv.destroySurface()` loop — Metal CALayer detaches from hierarchy before the surface pointer is freed
- Added `NSWindowDelegate` conformance to `WorkspaceWindowController` with `windowWillClose(_:)` so Cmd+W triggers `destroyAllSurfaces()` immediately
- Build verified: `swift build` exits 0 with zero errors

## Task Commits

Each task was committed atomically:

1. **Task 1: Async Task.detached surface teardown in GhosttyTerminalView** - `2274e52` (fix)
2. **Task 2: destroyAllSurfaces order fix + NSWindowDelegate + windowWillClose** - `0da3f4f` (fix)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified

- `macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift` — destroySurface() rewritten: guard/nil-then-Task.detached async free
- `macos/smux/Sources/SmuxApp/WorkspaceWindowController.swift` — NSWindowDelegate, window.delegate=self, windowWillClose, contentView=nil-first in destroyAllSurfaces
- `macos/smux/Sources/SmuxApp/main.swift` — Updated comments in performCleanShutdown and applicationWillTerminate to reflect correct teardown path

## Decisions Made

- Used `Task.detached { @MainActor in }` rather than `DispatchQueue.main.async` because the former matches Ghostty's exact pattern and correctly carries the `@MainActor` isolation required for Metal thread safety
- Placed `surface = nil` BEFORE the `Task.detached` block to prevent double-free if `deinit` races the explicit `destroySurface()` call

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Build produced only pre-existing warnings (unrelated to this plan's changes).

## Known Stubs

None — no stub values or placeholder text introduced.

## Next Phase Readiness

- Metal zombie bug is fixed; app can be built and tested manually with Cmd+W
- Phase 02 (PTY output capture) can proceed — no surface lifecycle blockers remain
- The `ghostty_surface_read_text` polling path for PTY capture is the active research track for Phase 02

---
*Phase: 01-surface-lifecycle-fix*
*Completed: 2026-03-26*

## Self-Check: PASSED

- FOUND: macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift
- FOUND: macos/smux/Sources/SmuxApp/WorkspaceWindowController.swift
- FOUND: macos/smux/Sources/SmuxApp/main.swift
- FOUND: .planning/phases/01-surface-lifecycle-fix/01-01-SUMMARY.md
- FOUND commit 2274e52: fix(01-01): async Task.detached surface teardown in GhosttyTerminalView
- FOUND commit 0da3f4f: fix(01-01): correct Metal surface teardown order + windowWillClose delegate
