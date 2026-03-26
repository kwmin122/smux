---
phase: 02-pty-output-capture
verified: 2026-03-26T05:30:00Z
status: passed
score: 3/3 must-haves verified
re_verification: false
---

# Phase 2: PTY Output Capture Verification Report

**Phase Goal:** smux can read and clean terminal output from agents running in ghostty EXEC mode in real-time
**Verified:** 2026-03-26T05:30:00Z
**Status:** passed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | When an agent CLI prints output in a ghostty pane, smux captures the text within one second | VERIFIED | `captureViewportText()` calls `ghostty_surface_read_text` with `GHOSTTY_POINT_VIEWPORT` (GhosttyTerminalView.swift:253). `startCapturing()` polls at 4 Hz (0.25s interval) via `Timer.scheduledTimer` (line 275). PingPongRouter calls `pane?.startCapturing { onChange }` (PingPongRouter.swift:133). Max latency 250ms, well within 1 second. |
| 2 | smux correctly identifies when the agent's turn is complete (prompt reappears, OSC 133 boundary fires, or configurable silence timeout expires) | VERIFIED | OSC 133 primary path: `NotificationCenter.default.addObserver(forName: .ghosttyCommandFinished)` in PingPongRouter (line 76). `actionCb` in main.swift dispatches `GHOSTTY_ACTION_COMMAND_FINISHED` via `NotificationCenter.default.post` (lines 25, 43). Silence timeout fallback: `silenceThreshold = 2.0` with cancellable `DispatchWorkItem` on `DispatchQueue.global().asyncAfter` (lines 43, 176). Double-fire prevention: `silenceWorkItem?.cancel()` before `processTurnComplete()` in `handleCommandFinished` (lines 159-160). |
| 3 | Text delivered to the relay layer contains no ANSI escape sequences -- only plain readable content | VERIFIED | `ANSIStripper.strip()` regex covers CSI (`\e[...X`), OSC (`\e]...\a`/`\e]...\e\\`), and standalone ESC sequences (ANSIStripper.swift:6). Called inside `startCapturing` timer callback: `ANSIStripper.strip(raw)` before `onChange(clean)` (GhosttyTerminalView.swift:278). Data flow: `captureViewportText()` -> `ANSIStripper.strip(raw)` -> `onChange(clean)` -> `handleNewOutput(newText)` -> `currentTurnText = newText` -> `onTurnComplete?(label, output)`. All text reaching the callback is already stripped. |

**Score:** 3/3 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `macos/smux/Sources/SmuxApp/ANSIStripper.swift` | Pure ANSI escape sequence stripping | VERIFIED | Contains `enum ANSIStripper` with `static func strip(_ input: String) -> String` using Swift Regex. 12 lines, no stubs, no TODOs. |
| `macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift` | `captureViewportText()` + `startCapturing()/stopCapturing()` | VERIFIED | `captureViewportText()` at line 236 uses `ghostty_surface_read_text` with proper `defer { ghostty_surface_free_text }` memory management (line 254). `startCapturing()` at line 272 polls at 4 Hz with ANSI stripping and delta filtering. `stopCapturing()` at line 289 invalidates timer. |
| `macos/smux/Sources/SmuxApp/PingPongRouter.swift` | Full capture integration -- polling, turn detection, silence timeout | VERIFIED | 226 lines. No stubs. Real polling via `startCapturing` (line 133), OSC 133 subscription (line 76), silence timeout at 2.0s (line 43), double-fire prevention (line 159-160), full cleanup in `stop()` (lines 87-107), `deinit` safety net (line 59-61). |
| `macos/smux/Sources/SmuxApp/main.swift` | OSC 133 COMMAND_FINISHED dispatch via NotificationCenter | VERIFIED | `actionCb` detects `GHOSTTY_ACTION_COMMAND_FINISHED` (line 25), extracts `exit_code` and `surface_ptr`, posts `.ghosttyCommandFinished` notification on main thread (line 42-49). `Notification.Name.ghosttyCommandFinished` extension at line 265. |
| `macos/smux/Sources/SmuxApp/WorkspaceWindowController.swift` | `togglePingPong()` wiring | VERIFIED | Creates PingPongRouter with two terminal panes (line 406), wires `onStateChanged` (line 411), `onTurnComplete` (line 425), `onSessionComplete` (line 430), calls `router.start()` (line 449). Cleanup in `destroyAllSurfaces()` (line 662-663). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| GhosttyTerminalView | ghostty C API | `ghostty_surface_read_text` call in `captureViewportText()` | WIRED | Line 253: `ghostty_surface_read_text(s, sel, &txt)` with `GHOSTTY_POINT_VIEWPORT`. Memory freed via `defer { ghostty_surface_free_text(s, &txt) }` at line 254. |
| GhosttyTerminalView.startCapturing | ANSIStripper | `ANSIStripper.strip(raw)` in timer callback | WIRED | Line 278: `let clean = ANSIStripper.strip(raw)` -- stripping happens before onChange delivery. |
| PingPongRouter | GhosttyTerminalView | `pane?.startCapturing { onChange }` | WIRED | Line 133: `pane?.startCapturing { [weak self] newText in self?.handleNewOutput(newText) }`. Delegates capture to GhosttyTerminalView polling wrapper. |
| PingPongRouter | NotificationCenter (.ghosttyCommandFinished) | `addObserver(forName:)` | WIRED | Line 75-81: `NotificationCenter.default.addObserver(forName: .ghosttyCommandFinished, ...)`. Observer removed in `stop()` at line 101. |
| main.swift actionCb | NotificationCenter | `NotificationCenter.default.post(name: .ghosttyCommandFinished)` | WIRED | Line 42-49: Posts on main thread with exit_code and surface_ptr in userInfo. |
| PingPongRouter | WorkspaceWindowController | `onTurnComplete` callback | WIRED | PingPongRouter calls `onTurnComplete?(label, output)` at line 196. WorkspaceWindowController wires `router.onTurnComplete = { ... }` at line 425. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| PingPongRouter | `currentTurnText` | `GhosttyTerminalView.startCapturing` -> `handleNewOutput(newText)` | Yes -- reads live viewport via `ghostty_surface_read_text` C API. Not static/empty. | FLOWING |
| GhosttyTerminalView | viewport text | `ghostty_surface_read_text` C API | Yes -- reads from ghostty surface's live terminal buffer. `GHOSTTY_POINT_VIEWPORT` with full extent. | FLOWING |
| main.swift actionCb | COMMAND_FINISHED notification | ghostty runtime callback (`action_cb`) | Yes -- fires when ghostty's OSC 133 parser detects command completion. Real ghostty event, not synthetic. | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Project builds successfully | `swift build` | `Build complete! (0.16s)` -- zero errors | PASS |
| Commit e6e8db9 exists (ANSIStripper + captureViewportText) | `git log --oneline -1 e6e8db9` | `feat(02-01): add ANSIStripper and captureViewportText()` | PASS |
| Commit d9aca3c exists (actionCb expansion) | `git log --oneline -1 d9aca3c` | `feat(02-01): expand actionCb for GHOSTTY_ACTION_COMMAND_FINISHED dispatch` | PASS |
| Commit c7547cd exists (startCapturing/stopCapturing) | `git log --oneline -1 c7547cd` | `feat(02-02): add startCapturing/stopCapturing polling wrapper to GhosttyTerminalView` | PASS |
| Commit 85b3211 exists (PingPongRouter rewrite) | `git log --oneline -1 85b3211` | `feat(02-02): rewrite PingPongRouter with real PTY capture integration` | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| PTY-CAP-01 | 02-01, 02-02 | Terminal output from an agent running in ghostty EXEC mode is captured by smux in real-time | SATISFIED | `captureViewportText()` reads live viewport via `ghostty_surface_read_text`. `startCapturing()` polls at 4 Hz (250ms). PingPongRouter wires polling to turn detection. |
| PTY-CAP-02 | 02-02 | smux detects when an agent's turn is complete (prompt ready / OSC 133 boundary / configurable silence timeout) | SATISFIED | OSC 133: `NotificationCenter.default.addObserver(forName: .ghosttyCommandFinished)` fires `handleCommandFinished`. Silence timeout: `DispatchWorkItem` at 2.0s fires `processTurnComplete` when text stops changing. Double-fire prevention via `silenceWorkItem?.cancel()`. |
| PTY-CAP-03 | 02-01 | Captured terminal output has ANSI escape sequences stripped before relay injection | SATISFIED | `ANSIStripper.strip()` regex covers CSI, OSC, and standalone ESC. Called in `startCapturing` timer callback (GhosttyTerminalView.swift:278) before text reaches `onChange` and ultimately `onTurnComplete`. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | -- | -- | -- | No anti-patterns detected in any Phase 2 files |

All five Phase 2 files (ANSIStripper.swift, GhosttyTerminalView.swift, PingPongRouter.swift, main.swift, WorkspaceWindowController.swift) were scanned for TODO/FIXME/HACK/PLACEHOLDER/empty returns/hardcoded empty data. Zero hits.

### Human Verification Required

### 1. Live Terminal Output Capture

**Test:** Launch `SmuxApp`, split panes with Cmd+D, activate ping-pong with Cmd+Shift+P, type `echo "hello world"` in left pane.
**Expected:** Console.app shows `[pingpong] started`, then `[pingpong] COMMAND_FINISHED received` or `[pingpong] silence timeout (2.0s)`, then `[pingpong] turn complete -- speaker=Left output=XX chars` with XX > 0.
**Why human:** Requires a running ghostty process with live PTY interaction. Cannot verify terminal rendering and OSC 133 signal firing programmatically.

### 2. ANSI Escape Stripping Quality

**Test:** Run `ls --color=always` in a ghostty pane while ping-pong is active. Check inspector transcript for the captured output.
**Expected:** Inspector transcript shows plain filenames with no `\e[` or similar escape sequences.
**Why human:** Regex correctness for all edge cases requires visual inspection of real terminal output with color codes.

### 3. Silence Timeout Behavior

**Test:** In ping-pong mode, run a command that has no OSC 133 support. Wait 2+ seconds after output stops.
**Expected:** Console shows `[pingpong] silence timeout (2.0s) -- treating as turn-complete` after 2 seconds of no text change.
**Why human:** Requires observing timing behavior of the silence fallback path, which only triggers when OSC 133 is unavailable.

### 4. Clean Shutdown

**Test:** While ping-pong is active, press Cmd+Shift+P to stop. Then close the window with Cmd+W.
**Expected:** Console shows `[pingpong] stopped -- cleanup complete`. No crash, no zombie timers, no orphaned notification observers.
**Why human:** Resource cleanup verification requires runtime observation of timer invalidation and observer removal.

### Gaps Summary

No gaps found. All three success criteria are fully implemented in the codebase:

1. **Real-time capture (PTY-CAP-01):** `ghostty_surface_read_text` called at 4 Hz via Timer -- maximum 250ms latency, well within the 1-second requirement.

2. **Turn-complete detection (PTY-CAP-02):** Dual detection paths: OSC 133 COMMAND_FINISHED (primary, authoritative) and 2-second silence timeout (fallback). Double-fire prevention via DispatchWorkItem cancellation.

3. **ANSI stripping (PTY-CAP-03):** ANSIStripper.strip() applied in the `startCapturing` timer callback before any downstream consumer receives the text. Regex covers CSI, OSC, and standalone ESC sequences.

All artifacts exist, are substantive (no stubs), are wired into the application, and have real data flowing through them. Build succeeds with zero errors. All four commits verified. Human verification checkpoint (Task 3) was APPROVED during development.

---

_Verified: 2026-03-26T05:30:00Z_
_Verifier: Claude (gsd-verifier)_
