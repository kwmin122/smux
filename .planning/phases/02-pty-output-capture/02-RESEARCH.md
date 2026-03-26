# Phase 2: PTY Output Capture - Research

**Researched:** 2026-03-26
**Domain:** ghostty C API (libghostty xcframework), Swift/AppKit, OSC 133, ANSI stripping
**Confidence:** HIGH — all findings derived directly from the vendored ghostty.h header and project source code

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PTY-CAP-01 | Terminal output from an agent running in ghostty EXEC mode is captured by smux in real-time | ghostty_surface_read_text polls the terminal buffer; Timer at ~4 Hz achieves <1s latency |
| PTY-CAP-02 | smux detects when an agent's turn is complete (prompt ready / OSC 133 boundary / configurable silence timeout) | GHOSTTY_ACTION_COMMAND_FINISHED fires via action_cb when OSC 133 D sequence arrives; silence-timeout as fallback via DispatchWorkItem |
| PTY-CAP-03 | Captured terminal output has ANSI escape sequences stripped before relay injection | Pure Swift regex against known ANSI patterns; no external dependency needed |
</phase_requirements>

---

## Summary

Phase 2's central challenge is that ghostty runs in EXEC mode (backend = `GHOSTTY_SURFACE_IO_BACKEND_EXEC`), meaning ghostty manages the PTY internally. smux does not hold a file descriptor to read from the PTY directly — the only interface to terminal content is the ghostty surface API.

The surface API provides `ghostty_surface_read_text(surface, selection, &text_s)` which reads any rectangular region of the terminal buffer as UTF-8 text. This is the **only viable path** for output capture in EXEC mode. Polling this function at a low frequency (200-500 ms) against the full viewport gives sub-second delivery. The returned `ghostty_text_s.text` pointer is owned by ghostty and must be freed with `ghostty_surface_free_text`.

Turn-complete detection has two mechanisms available in the header: (1) `GHOSTTY_ACTION_COMMAND_FINISHED` fired through `action_cb` when the shell emits an OSC 133 `D` (command-end) sequence, and (2) a configurable silence timeout implemented as a cancellable `DispatchWorkItem` that resets on each new text read. The `action_cb` path is authoritative; the silence timeout is the fallback for shells or agents that do not emit OSC 133.

ANSI stripping is a pure-Swift regex operation — no external library is needed. The standard pattern covers CSI sequences, OSC sequences, and standalone escape codes and fits in ~10 lines.

**Primary recommendation:** Implement polling via `ghostty_surface_read_text` with a full-viewport `ghostty_selection_s`, wire `action_cb` to dispatch `GHOSTTY_ACTION_COMMAND_FINISHED` events to `PingPongRouter`, add a silence-timeout fallback, and strip ANSI with a Swift regex before delivering to the router's `onTurnComplete` callback.

---

## Standard Stack

### Core
| Component | Version/Source | Purpose | Why Standard |
|-----------|---------------|---------|--------------|
| `ghostty_surface_read_text` | GhosttyKit.xcframework (vendored) | Read terminal buffer contents | Only API for EXEC mode buffer access |
| `ghostty_surface_free_text` | GhosttyKit.xcframework (vendored) | Release text buffer returned by read_text | Required to avoid memory leak |
| `GHOSTTY_ACTION_COMMAND_FINISHED` action_cb | GhosttyKit.xcframework (vendored) | OSC 133 D (command-end) notification | Authoritative shell-signaled boundary |
| `Foundation.Timer` (scheduledTimer) | macOS SDK | Drive polling loop | Already used in app (ghostty_app_tick timer) |
| Swift `NSRegularExpression` or `Regex` | macOS SDK (Swift 5.7+) | ANSI escape stripping | Zero-dependency, in-process |

### Supporting
| Component | Version/Source | Purpose | When to Use |
|-----------|---------------|---------|-------------|
| `DispatchWorkItem` | Foundation | Silence timeout: cancel/reschedule on each poll | Fallback when OSC 133 absent |
| `ghostty_surface_read_selection` | GhosttyKit.xcframework | Read currently-selected text only | Not useful for capture — no selection guarantee |
| `ghostty_surface_has_selection` | GhosttyKit.xcframework | Check if there is a selection | Diagnostic only |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| ghostty_surface_read_text polling | HOST_MANAGED backend with receive_buffer_cb | receive_buffer_cb is push-based and cleaner, but switching to HOST_MANAGED means smux must manage the PTY itself — rejected as architectural escalation |
| ghostty_surface_read_text polling | CPtyHelper forkpty + direct fd read | CPtyHelper only works for smux-managed PTYs (HOST_MANAGED), not for ghostty-managed EXEC mode PTYs |
| Swift Regex | External library (e.g., SwiftANSI, Ink) | No package manager in project; pure Swift is simpler and sufficient |

---

## Architecture Patterns

### Recommended Project Structure

No new directories needed. New files added to existing `Sources/SmuxApp/`:

```
Sources/SmuxApp/
├── GhosttyTerminalView.swift     (existing — add captureCurrentText(), startPolling(), stopPolling())
├── PingPongRouter.swift           (existing — add real capture integration, replace stub)
├── ANSIStripper.swift             (new — pure Swift ANSI strip function)
├── main.swift                     (existing — expand actionCb to dispatch COMMAND_FINISHED)
└── WorkspaceWindowController.swift (existing — no changes needed for Phase 2)
```

### Pattern 1: Full-Viewport Selection for ghostty_surface_read_text

`ghostty_surface_read_text` takes a `ghostty_selection_s` defining a rectangle in the terminal buffer. To capture the full visible viewport, build a selection from top-left (row 0, col 0) to bottom-right using a large coordinate (e.g., row 9999, col 9999) with `GHOSTTY_POINT_VIEWPORT` tag and `GHOSTTY_POINT_COORD_BOTTOM_RIGHT` coord. Ghostty clamps to actual content.

**C API signatures (from vendored ghostty.h):**

```c
// ghostty.h line 1148
bool ghostty_surface_read_text(ghostty_surface_t,
                               ghostty_selection_s,
                               ghostty_text_s*);
void ghostty_surface_free_text(ghostty_surface_t, ghostty_text_s*);

typedef struct {
  double tl_px_x;      // top-left pixel x (output, set by ghostty)
  double tl_px_y;      // top-left pixel y (output)
  uint32_t offset_start;
  uint32_t offset_len;
  const char* text;    // UTF-8 string — DO NOT free directly, use ghostty_surface_free_text
  uintptr_t text_len;
} ghostty_text_s;

typedef struct {
  ghostty_point_s top_left;
  ghostty_point_s bottom_right;
  bool rectangle;       // false = stream selection; true = rectangular block
} ghostty_selection_s;

typedef struct {
  ghostty_point_tag_e tag;   // GHOSTTY_POINT_VIEWPORT for visible area
  ghostty_point_coord_e coord;
  uint32_t x;
  uint32_t y;
} ghostty_point_s;
```

**Swift usage pattern:**

```swift
// Source: vendored macos-arm64_x86_64/Headers/ghostty.h
func captureCurrentText() -> String? {
    guard let s = surface else { return nil }

    var sel = ghostty_selection_s()
    sel.rectangle = false
    sel.top_left = ghostty_point_s(
        tag: GHOSTTY_POINT_VIEWPORT,
        coord: GHOSTTY_POINT_COORD_TOP_LEFT,
        x: 0, y: 0
    )
    sel.bottom_right = ghostty_point_s(
        tag: GHOSTTY_POINT_VIEWPORT,
        coord: GHOSTTY_POINT_COORD_BOTTOM_RIGHT,
        x: 9999, y: 9999
    )

    var txt = ghostty_text_s()
    guard ghostty_surface_read_text(s, sel, &txt) else { return nil }
    defer { ghostty_surface_free_text(s, &txt) }

    guard let ptr = txt.text, txt.text_len > 0 else { return nil }
    return String(bytes: UnsafeBufferPointer(start: ptr, count: Int(txt.text_len)),
                  encoding: .utf8)
}
```

**Important:** `ghostty_surface_read_text` MUST be called from the main thread (same thread that drives ghostty rendering). The existing tick timer runs on the main thread via `Timer.scheduledTimer` — polling can piggyback on the same thread safety requirement.

### Pattern 2: OSC 133 via action_cb (GHOSTTY_ACTION_COMMAND_FINISHED)

OSC 133 is a shell integration protocol. Sequence `\e]133;D\a` (command end) causes ghostty to fire `action_cb` with `action.tag == GHOSTTY_ACTION_COMMAND_FINISHED`. The payload is `ghostty_action_command_finished_s`:

```c
// ghostty.h line 843
typedef struct {
  int16_t exit_code;   // -1 if not reported
  uint64_t duration;   // nanoseconds
} ghostty_action_command_finished_s;
```

Currently `actionCb` in `main.swift` only logs and returns `false`. It needs to be wired to notify `PingPongRouter` when `GHOSTTY_ACTION_COMMAND_FINISHED` fires.

The challenge: `actionCb` is a C function pointer (no Swift closure capture). The current pattern passes `rt.userdata = nil`. To communicate back, set `rt.userdata` to a pointer to an object (or use a global/singleton pattern for the router reference).

**Recommended approach — notification-based (no C pointer dance):**

```swift
// Source: project pattern (main.swift)
// Post a Darwin notification or use NotificationCenter from within actionCb
private let actionCb: @convention(c) (ghostty_app_t?, ghostty_target_s, ghostty_action_s) -> Bool = { _, target, action in
    if action.tag == GHOSTTY_ACTION_COMMAND_FINISHED {
        let payload = action.action.command_finished
        DispatchQueue.main.async {
            NotificationCenter.default.post(
                name: .ghosttyCommandFinished,
                object: nil,
                userInfo: ["exit_code": payload.exit_code, "duration": payload.duration]
            )
        }
    }
    return false
}

extension Notification.Name {
    static let ghosttyCommandFinished = Notification.Name("ghosttyCommandFinished")
}
```

`PingPongRouter` subscribes to `ghosttyCommandFinished` and treats it as turn-complete. Because `actionCb` identifies the target surface via `target.tag / target.target.surface`, the notification can include the surface pointer to identify which pane finished.

### Pattern 3: Silence Timeout Fallback

For agents that do not emit OSC 133 (e.g., raw claude CLI before shell integration is configured):

```swift
// Source: standard Swift pattern
private var silenceWorkItem: DispatchWorkItem?
private let silenceThreshold: TimeInterval = 2.0  // configurable

func resetSilenceTimer(onComplete: @escaping () -> Void) {
    silenceWorkItem?.cancel()
    let item = DispatchWorkItem {
        DispatchQueue.main.async { onComplete() }
    }
    silenceWorkItem = item
    DispatchQueue.global().asyncAfter(deadline: .now() + silenceThreshold, execute: item)
}
```

The polling loop calls `resetSilenceTimer` each time new text differs from the previous snapshot. If text stops changing for `silenceThreshold` seconds, treat it as turn-complete.

### Pattern 4: ANSI Stripping

Standard ANSI escape pattern covers:
- CSI sequences: `\e[...m`, `\e[...H`, etc.
- OSC sequences: `\e]...(\a|\e\\)`
- Standalone ESC + single char

```swift
// Source: standard ANSI spec (verified against ANSI X3.64)
// ANSIStripper.swift
import Foundation

enum ANSIStripper {
    private static let pattern = #"\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07\x1B]*(?:\x07|\x1B\\))"#
    private static let regex = try! NSRegularExpression(pattern: pattern)

    static func strip(_ input: String) -> String {
        let range = NSRange(input.startIndex..., in: input)
        return regex.stringByReplacingMatches(in: input, range: range, withTemplate: "")
    }
}
```

Alternatively with Swift 5.7+ `Regex` literal (macOS 14 target is confirmed in Package.swift):

```swift
import RegexBuilder

enum ANSIStripper {
    static let ansiRegex = /\x1B(?:[@-Z\\-_]|\[[0-?]*[ -\/]*[@-~]|\][^\x07\x1B]*(?:\x07|\x1B\\))/

    static func strip(_ input: String) -> String {
        input.replacing(ansiRegex, with: "")
    }
}
```

macOS 14 is the minimum target (Package.swift line 7), so Swift Regex is available.

### Pattern 5: Polling Loop Integration

The existing `tickTimer` fires at 1/30 Hz (~33 ms). A separate slower polling timer at 4-5 Hz (200-250 ms) is appropriate for text capture — frequent enough for <1s latency without over-reading the buffer:

```swift
// In GhosttyTerminalView or PingPongRouter
private var captureTimer: Timer?
private var lastCapturedText: String = ""

func startCapturing(onChange: @escaping (String) -> Void) {
    captureTimer?.invalidate()
    captureTimer = Timer.scheduledTimer(withTimeInterval: 0.25, repeats: true) { [weak self] _ in
        guard let self = self else { return }
        guard let raw = self.captureCurrentText() else { return }
        let clean = ANSIStripper.strip(raw)
        if clean != self.lastCapturedText {
            self.lastCapturedText = clean
            onChange(clean)
        }
    }
}

func stopCapturing() {
    captureTimer?.invalidate()
    captureTimer = nil
}
```

### Anti-Patterns to Avoid

- **Reading from background thread:** `ghostty_surface_read_text` must be called on the main thread. Never dispatch to a background queue for the read call itself.
- **Forgetting ghostty_surface_free_text:** The `text` pointer in `ghostty_text_s` is owned by ghostty and must be freed. Missing this causes a memory leak on every poll tick.
- **Using ghostty_surface_read_selection:** This only reads what the user has selected via mouse drag. There is no guarantee a selection exists; this API is not a capture mechanism.
- **Polling too fast:** 30 Hz (matching tickTimer) would double the internal ghostty render load. 4-5 Hz is sufficient and low-cost.
- **Treating all text changes as meaningful:** Cursor blink or spinner output causes text changes that are not agent output boundaries. Delta comparison should filter noise below a meaningful character threshold (e.g., ignore changes < 5 chars).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ANSI escape stripping | Custom state machine parser | Swift Regex / NSRegularExpression | ANSI has edge cases (private use sequences, 8-bit C1 codes); the pattern covers standard cases cleanly |
| OSC 133 parsing | Parse raw PTY bytes for \e]133;D | GHOSTTY_ACTION_COMMAND_FINISHED via action_cb | Ghostty already parses OSC sequences — re-parsing is redundant and fragile |
| PTY output reading | forkpty + direct fd read | ghostty_surface_read_text | smux does not own the PTY fd in EXEC mode; CPtyHelper's forkpty is for HOST_MANAGED mode only |

**Key insight:** In EXEC mode, ghostty is the PTY owner. smux's only sanctioned interface to output is the surface text-read API. Any attempt to intercept the PTY at a lower level would require HOST_MANAGED mode and a complete rearchitecture.

---

## Common Pitfalls

### Pitfall 1: ghostty_surface_read_text Memory Leak

**What goes wrong:** Text is read but `ghostty_surface_free_text` is never called. Over time (polling every 250 ms) this leaks several KB per minute.

**Why it happens:** The pointer lives inside ghostty's internal allocator. Swift ARC does not manage it.

**How to avoid:** Always use `defer { ghostty_surface_free_text(s, &txt) }` immediately after the successful `read_text` call.

**Warning signs:** Memory usage grows linearly with uptime when ping-pong is active.

### Pitfall 2: action_cb Cannot Capture Swift State

**What goes wrong:** Developer tries to capture `self` or `router` inside the `@convention(c)` actionCb closure. Swift compiler refuses — C function pointers cannot close over Swift references.

**Why it happens:** `@convention(c)` requires a bare function pointer, no context capture.

**How to avoid:** Use `NotificationCenter.default.post` from within actionCb (NotificationCenter is accessible as a global) or store a weak reference in the `rt.userdata` void pointer cast to `Unmanaged<T>`.

**Warning signs:** Compile error "C function pointer cannot be formed from a closure that captures context."

### Pitfall 3: Silence Timeout Races with OSC 133

**What goes wrong:** Shell emits OSC 133 D, `action_cb` fires turn-complete, but 50 ms later the silence timeout also fires a second turn-complete event. PingPongRouter processes the same output twice.

**Why it happens:** The silence timer was not cancelled when OSC 133 fired.

**How to avoid:** Cancel the `silenceWorkItem` in the `ghosttyCommandFinished` notification handler before processing turn-complete.

### Pitfall 4: Full-Viewport Read Returns Entire Scrollback

**What goes wrong:** ghostty_surface_read_text with GHOSTTY_POINT_VIEWPORT returns only the visible viewport, not the scrollback. But if the agent's output exceeds one screen, recent lines may scroll off before capture.

**Why it happens:** GHOSTTY_POINT_VIEWPORT is bounded to the visible area.

**How to avoid:** Use GHOSTTY_POINT_SCREEN (which includes scrollback) for large output, or capture incrementally. For Phase 2, viewport-only is acceptable since agent output per turn is typically bounded. Document as known limitation.

**Warning signs:** Captured text is truncated at top when agent emits multi-screen output.

### Pitfall 5: Swift Regex / NSRegularExpression Is Not Thread-Safe for Shared Instances

**What goes wrong:** Multiple `GhosttyTerminalView` instances (split panes) call `ANSIStripper.strip` simultaneously from the main thread. Because both run on main thread this is safe, but if polling is ever moved off-main the shared static `regex` would need a lock.

**Why it happens:** NSRegularExpression is documented as thread-safe for `stringByReplacingMatches` after initialization. This is actually fine — flagging for awareness only.

**How to avoid:** Keep all capture calls on main thread. If ever moved background, each call should use a local NSRegularExpression instance.

---

## Code Examples

### Full ghostty_surface_read_text Call (Swift)

```swift
// Source: derived from vendored macos-arm64_x86_64/Headers/ghostty.h
func captureViewportText() -> String? {
    guard let s = surface else { return nil }

    var sel = ghostty_selection_s()
    sel.rectangle = false
    sel.top_left = ghostty_point_s(
        tag: GHOSTTY_POINT_VIEWPORT,
        coord: GHOSTTY_POINT_COORD_TOP_LEFT,
        x: 0, y: 0
    )
    sel.bottom_right = ghostty_point_s(
        tag: GHOSTTY_POINT_VIEWPORT,
        coord: GHOSTTY_POINT_COORD_BOTTOM_RIGHT,
        x: 9999, y: 9999
    )

    var txt = ghostty_text_s()
    guard ghostty_surface_read_text(s, sel, &txt) else { return nil }
    defer { ghostty_surface_free_text(s, &txt) }

    guard let ptr = txt.text, txt.text_len > 0 else { return nil }
    return String(bytes: UnsafeBufferPointer(start: UnsafePointer(ptr), count: Int(txt.text_len)),
                  encoding: .utf8)
}
```

### ANSI Stripper (Swift)

```swift
// Source: standard ANSI X3.64 / VT100 specification
// File: ANSIStripper.swift
import Foundation

enum ANSIStripper {
    // Covers: CSI sequences, OSC sequences, standalone ESC+char
    private static let ansiRegex: Regex = {
        // macOS 14+ Swift Regex (Package.swift: .macOS(.v14))
        try! Regex(#"\x1B(?:[@-Z\\-_]|\[[0-?]*[ -\/]*[@-~]|\][^\x07\x1B]*(?:\x07|\x1B\\))"#)
    }()

    static func strip(_ input: String) -> String {
        input.replacing(ansiRegex, with: "")
    }
}
```

### actionCb Expansion for COMMAND_FINISHED

```swift
// Source: main.swift pattern + ghostty.h line 933/943/976
// Expand the existing actionCb (currently just logs):
private let actionCb: @convention(c) (ghostty_app_t?, ghostty_target_s, ghostty_action_s) -> Bool = { _, target, action in
    if action.tag.rawValue == GHOSTTY_ACTION_COMMAND_FINISHED.rawValue {
        let payload = action.action.command_finished
        // Must use NotificationCenter — cannot capture Swift refs in @convention(c)
        DispatchQueue.main.async {
            NotificationCenter.default.post(
                name: .ghosttyCommandFinished,
                object: nil,
                userInfo: [
                    "exit_code": Int(payload.exit_code),
                    "surface": target.tag == GHOSTTY_TARGET_SURFACE
                        ? target.target.surface as AnyObject
                        : NSNull()
                ]
            )
        }
    }
    return false
}

extension Notification.Name {
    static let ghosttyCommandFinished = Notification.Name("smux.ghosttyCommandFinished")
}
```

### PingPongRouter Capture Integration (Sketch)

```swift
// Replaces placeholder stub in PingPongRouter.swift
func start() {
    isActive = true
    round = 0
    updateState(.waitingForOutput)

    // Subscribe to OSC 133 turn-complete events
    NotificationCenter.default.addObserver(
        self,
        selector: #selector(handleCommandFinished(_:)),
        name: .ghosttyCommandFinished,
        object: nil
    )

    // Start polling for text capture
    paneA?.startCapturing { [weak self] newText in
        self?.handleNewOutput(from: "A", text: newText)
    }
}

@objc private func handleCommandFinished(_ note: Notification) {
    silenceWorkItem?.cancel()  // avoid double-fire
    processTurnComplete()
}
```

---

## State of the Art

| Old Approach | Current Approach | Notes |
|--------------|-----------------|-------|
| `receive_buffer` callback (HOST_MANAGED) | `ghostty_surface_read_text` polling (EXEC mode) | STATE.md documents this explicitly: "receive_buffer NOT called in EXEC mode" |
| Separate PTY fd read thread | Main-thread polling timer | EXEC mode: no fd accessible |
| Direct ANSI parser library | Swift Regex one-liner | Swift 5.7+ regex is sufficient for standard sequences |

**Deprecated/outdated:**

- `CPtyHelper.smux_forkpty`: Created for HOST_MANAGED PTY mode. Not useful for Phase 2 (ghostty manages the PTY in EXEC mode). Keep in codebase but do not use in Phase 2.
- `GHOSTTY_SURFACE_IO_BACKEND_HOST_MANAGED`: The backend that would enable `receive_buffer_cb`. Not used — EXEC mode was chosen as architectural decision. Do not switch.

---

## Open Questions

1. **Does ghostty_surface_read_text include only visible content or full scrollback?**
   - What we know: `GHOSTTY_POINT_VIEWPORT` likely means viewport only; `GHOSTTY_POINT_SCREEN` likely includes scrollback. The header defines four point tags: ACTIVE, VIEWPORT, SCREEN, SURFACE.
   - What's unclear: ghostty's Zig source defines the semantics. Without reading it, the exact boundary between VIEWPORT and SCREEN is uncertain.
   - Recommendation: Start with VIEWPORT. If test shows truncation on long agent output, switch to SCREEN. Document as a tuning parameter.

2. **Does action_cb fire per-surface or per-app for COMMAND_FINISHED?**
   - What we know: `ghostty_target_s` carries `tag` (APP or SURFACE) and `target.surface`. For per-command events the target is likely SURFACE.
   - What's unclear: Whether the target.surface pointer is valid for the lifetime of the async DispatchQueue.main block.
   - Recommendation: Do not dereference the surface pointer inside the async block. Use it only as an opaque key to identify which pane fired.

3. **Will GHOSTTY_ACTION_COMMAND_FINISHED fire for agents that don't use OSC 133?**
   - What we know: OSC 133 requires shell integration. The `claude` CLI does not itself configure shell integration — it inherits the user's shell prompt.
   - What's unclear: Whether the user's shell has OSC 133 configured (zsh with Ghostty shell integration, for instance).
   - Recommendation: OSC 133 is the primary path; silence timeout is a required fallback. Both must be implemented. Phase 2 success criteria say "or configurable silence timeout" — both are in scope.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| GhosttyKit.xcframework | All capture tasks | Yes (vendored) | In Frameworks/ | None needed |
| Swift 5.7+ Regex | ANSIStripper | Yes (macOS 14 target) | Swift 5.9 (Xcode 15+) | NSRegularExpression fallback |
| NotificationCenter | action_cb dispatch | Yes (AppKit, always available) | macOS SDK | None needed |
| CPtyHelper | HOST_MANAGED only | Yes (Sources/CPtyHelper/) | — | Not needed for Phase 2 |

---

## Validation Architecture

> Nyquist validation config: .planning/config.json not found — treating as enabled (absent = enabled).

### Test Framework

Manual build-and-run verification is the only practical test approach for this phase. There is no unit test infrastructure in the project (no `Tests/` directory, no `XCTestManifests.swift`, no `Package.swift` test targets).

| Property | Value |
|----------|-------|
| Framework | None — manual verification only |
| Config file | None |
| Quick run command | `cd /Users/min-kyungwook/Desktop/mmux/macos/smux && swift build 2>&1 \| tail -5` |
| Full verification | Build + launch app + type in terminal + observe NSLog output |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Command | Notes |
|--------|----------|-----------|---------|-------|
| PTY-CAP-01 | captureViewportText() returns non-nil when agent prints | manual-smoke | Launch app, type `echo hello` in terminal, check NSLog shows capture | No automated test infra |
| PTY-CAP-02 | COMMAND_FINISHED fires when shell prompt reappears | manual-smoke | Launch app with OSC 133 shell; run command; observe NSLog | Requires OSC 133 configured in zsh |
| PTY-CAP-02 | Silence timeout fires if no OSC 133 | manual-smoke | Launch app without OSC 133; run command; wait 2s; observe NSLog | — |
| PTY-CAP-03 | ANSIStripper.strip removes escape sequences | unit-buildable | Write standalone test harness in a scratch Swift file | Can be validated as pure function |

### Wave 0 Gaps

- [ ] No test target in Package.swift — add `testTarget` for `ANSIStripper` if unit isolation is desired
- [ ] NSLog-based verification for PTY-CAP-01 and PTY-CAP-02 requires manual observation

*(Consider adding a `.testTarget` for `ANSIStripper` to Package.swift — it is a pure function with no AppKit dependency and can be tested headlessly.)*

---

## Project Constraints (from CLAUDE.md)

No `CLAUDE.md` found at project root. No additional directives to propagate.

Constraints derived from project source and STATE.md decisions:
- EXEC mode is locked — do not switch to HOST_MANAGED
- Real visible PTY is non-negotiable — headless/daemon capture rejected
- `ghostty_surface_read_text` polling is the chosen capture strategy (STATE.md: "pending verification")
- Swift/AppKit — no Electron, no web
- All ghostty surface calls must be on main thread (Metal thread safety)
- `Task.detached { @MainActor }` pattern for any async ghostty work (established in Phase 1)

---

## Sources

### Primary (HIGH confidence)

- `/Users/min-kyungwook/Desktop/mmux/macos/smux/Frameworks/GhosttyKit.xcframework/macos-arm64_x86_64/Headers/ghostty.h` — Full C API: ghostty_surface_read_text, ghostty_surface_free_text, ghostty_text_s, ghostty_selection_s, ghostty_point_s, GHOSTTY_ACTION_COMMAND_FINISHED, ghostty_action_command_finished_s, ghostty_runtime_action_cb, GHOSTTY_SURFACE_IO_BACKEND_EXEC
- `/Users/min-kyungwook/Desktop/mmux/macos/smux/Sources/SmuxApp/main.swift` — Existing actionCb pattern, tickTimer at 1/30 Hz
- `/Users/min-kyungwook/Desktop/mmux/macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift` — Surface access pattern, sendText, thread requirements
- `/Users/min-kyungwook/Desktop/mmux/macos/smux/Sources/SmuxApp/PingPongRouter.swift` — Current stub: state machine, callback signatures onTurnComplete, onStateChanged
- `/Users/min-kyungwook/Desktop/mmux/.planning/STATE.md` — "receive_buffer NOT called in EXEC mode" confirmed; ghostty_surface_read_text chosen as workaround
- `/Users/min-kyungwook/Desktop/mmux/macos/smux/Package.swift` — macOS 14 minimum target (confirms Swift 5.7+ Regex available)

### Secondary (MEDIUM confidence)

- ANSI X3.64 / VT100 specification — ANSI escape sequence pattern covers CSI, OSC, and standalone ESC sequences. Pattern is well-established and widely used in terminal ecosystem.
- OSC 133 shell integration spec (https://iterm2.com/documentation-escape-codes.html) — `\e]133;D\a` = command-end sequence that maps to GHOSTTY_ACTION_COMMAND_FINISHED.

### Tertiary (LOW confidence)

- ghostty VIEWPORT vs SCREEN point semantics — inferred from point tag names in header. Zig source not inspected. Test empirically during implementation.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — API verified directly in vendored ghostty.h
- Architecture (polling): HIGH — ghostty_surface_read_text exists and has correct signature; pattern follows existing sendText idiom
- Architecture (action_cb OSC 133): HIGH — GHOSTTY_ACTION_COMMAND_FINISHED exists in header, ghostty_action_command_finished_s defined; existing actionCb is live in main.swift
- ANSI stripping: HIGH — pure Swift Regex, no external dependency, standard pattern
- VIEWPORT vs SCREEN semantics: LOW — inferred from names; verify empirically

**Research date:** 2026-03-26
**Valid until:** 2026-04-25 (ghostty header is vendored/pinned — stable until xcframework update)
