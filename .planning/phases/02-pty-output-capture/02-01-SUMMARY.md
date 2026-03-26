---
phase: 02-pty-output-capture
plan: 01
subsystem: terminal-capture
tags: [ghostty, swift, pty-capture, ansi, notificationcenter, exec-mode]

# Dependency graph
requires:
  - 01-surface-lifecycle-fix (stable ghostty surface lifecycle)
provides:
  - ANSIStripper.strip() pure function (CSI/OSC/standalone ESC removal)
  - GhosttyTerminalView.captureViewportText() using ghostty_surface_read_text C API
  - Notification.Name.ghosttyCommandFinished dispatched from actionCb on COMMAND_FINISHED
affects:
  - 02-02-ping-pong-router (wires these primitives together)
  - 03-ping-pong-relay

# Tech tracking
tech-stack:
  added:
    - "Swift Regex (macOS 14 / Swift 5.10) for ANSI escape stripping"
  patterns:
    - "ghostty_surface_read_text with GHOSTTY_POINT_VIEWPORT + large coord clamping for full viewport reads"
    - "defer { ghostty_surface_free_text } immediately after successful read_text for memory safety"
    - "UInt(bitPattern:) for opaque surface pointer passing in @convention(c) context"
    - "DispatchQueue.main.async for NotificationCenter.post from @convention(c) C callback"

key-files:
  created:
    - macos/smux/Sources/SmuxApp/ANSIStripper.swift
  modified:
    - macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift
    - macos/smux/Sources/SmuxApp/main.swift

key-decisions:
  - "Use GHOSTTY_POINT_VIEWPORT (not SCREEN) to read only the visible terminal area; large x/y coords (9999) are clamped by ghostty internally"
  - "UnsafePointer<CChar>.withMemoryRebound(to: UInt8.self) to bridge C const char* to Swift String(bytes:encoding:)"
  - "UInt(bitPattern: target.target.surface) as opaque key — surface pointer must not be dereferenced in async DispatchQueue block (may be freed by then)"
  - "NotificationCenter.default is a global, accessible without captures from @convention(c) closures"

patterns-established:
  - "captureViewportText() is main-thread-only (Metal thread safety) — callers must ensure main thread"
  - "ANSIStripper has no AppKit dependency (Foundation only) — safe to use from any context"
  - "ghosttyCommandFinished notification carries exit_code (Int) and surface_ptr (UInt) in userInfo"

requirements-completed: [PTY-CAP-01, PTY-CAP-03]

# Metrics
duration: 4min
completed: 2026-03-26
---

# Phase 02 Plan 01: PTY Output Capture Primitives Summary

**Three foundational PTY capture primitives built: ANSIStripper pure function, captureViewportText() using ghostty_surface_read_text, and COMMAND_FINISHED dispatch via NotificationCenter**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-03-26T03:51:39Z
- **Completed:** 2026-03-26T03:55:56Z
- **Tasks:** 2
- **Files created:** 1
- **Files modified:** 2

## Accomplishments

- Created `ANSIStripper.swift` with pure-function ANSI escape stripping using Swift Regex covering CSI (`\e[...X`), OSC (`\e]...\a` / `\e]...\e\\`), and standalone ESC sequences
- Added `captureViewportText() -> String?` to `GhosttyTerminalView` using `ghostty_surface_read_text` with `GHOSTTY_POINT_VIEWPORT` selection and `defer { ghostty_surface_free_text }` for correct memory management
- Expanded `actionCb` in `main.swift` to detect `GHOSTTY_ACTION_COMMAND_FINISHED` and post `Notification.Name.ghosttyCommandFinished` via `NotificationCenter.default` on the main thread with `exit_code` and `surface_ptr` in `userInfo`
- Fixed `CChar`-to-`UInt8` type bridging via `withMemoryRebound` for `String(bytes:encoding:)` initializer compatibility
- Build verified: `swift build` exits 0 with zero errors on both tasks

## Task Commits

Each task was committed atomically:

1. **Task 1: ANSIStripper.swift + captureViewportText()** - `e6e8db9` (feat)
2. **Task 2: actionCb expansion + Notification.Name extension** - `d9aca3c` (feat)

## Files Created/Modified

- `macos/smux/Sources/SmuxApp/ANSIStripper.swift` (new) — Pure ANSI escape stripping with Swift Regex, Foundation only
- `macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift` (modified) — Added `captureViewportText()` in `// MARK: - Text Capture (EXEC mode polling)` section
- `macos/smux/Sources/SmuxApp/main.swift` (modified) — Expanded `actionCb` + `Notification.Name.ghosttyCommandFinished` extension

## Decisions Made

- Used `GHOSTTY_POINT_VIEWPORT` over `GHOSTTY_POINT_SCREEN` to limit reads to the visible area (avoids scrollback buffer content which is irrelevant to ping-pong relay)
- `x: 9999, y: 9999` for bottom_right coordinates — ghostty clamps to actual content dimensions internally
- `withMemoryRebound(to: UInt8.self)` over `String(cString:)` — the latter stops at null bytes and would truncate output with embedded nulls; buffer-based init is correct for terminal output
- Surface pointer passed as `UInt(bitPattern:)` opaque key — the `DispatchQueue.main.async` block runs after the C callback returns; the pointer may be freed by then, so it's only used as a dictionary key for routing, never dereferenced

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed CChar/UInt8 type mismatch in captureViewportText()**
- **Found during:** Task 1 — swift build
- **Issue:** `String(bytes: UnsafeBufferPointer(start: ptr, count:), encoding: .utf8)` failed because `ptr` is `UnsafePointer<CChar>` (Int8) but `String.init(bytes:encoding:)` requires `UInt8` elements
- **Fix:** Added `ptr.withMemoryRebound(to: UInt8.self, capacity:)` wrapper
- **Files modified:** `macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift`
- **Commit:** `e6e8db9`

## Known Stubs

None — all methods are wired to real ghostty C API calls. `captureViewportText()` returns real terminal viewport content. `ANSIStripper.strip()` applies real regex processing. The `ghosttyCommandFinished` notification fires from real OSC 133 events.

## Next Phase Readiness

- Plan 02-02 (PingPongRouter) can proceed — all three primitives are available and build-verified
- `captureViewportText()` is callable from any Swift code on the main thread
- `ANSIStripper.strip()` is callable from any context (no AppKit dependency)
- `.ghosttyCommandFinished` notification is observable via `NotificationCenter.default.addObserver`

---
*Phase: 02-pty-output-capture*
*Plan: 01*
*Completed: 2026-03-26*
