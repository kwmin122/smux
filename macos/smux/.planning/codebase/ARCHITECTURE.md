# Architecture

**Analysis Date:** 2026-03-26

## Pattern Overview

**Overall:** Monolithic AppKit application with libghostty embedded terminal, relay-style inter-pane routing, and optional daemon IPC.

**Key Characteristics:**
- Single-process macOS app using `libghostty` in EXEC mode (ghostty owns the PTY)
- Ping-pong relay captures terminal viewport text via polling, injects into sibling panes via `ghostty_surface_text`
- Optional smux-daemon connectivity over Unix socket IPC for session management
- No SwiftUI -- pure AppKit with manual Auto Layout constraints
- Metal rendering via CAMetalLayer (ghostty handles all GPU rendering internally)

## Layers

**PTY/Terminal Layer (libghostty):**
- Purpose: Terminal emulation, PTY management, rendering
- Location: `Frameworks/GhosttyKit.xcframework/`, consumed by `Sources/SmuxApp/GhosttyTerminalView.swift`
- Contains: Opaque C library (Zig-compiled) providing full terminal emulation
- Depends on: Metal, AppKit, Carbon frameworks
- Used by: `GhosttyTerminalView`, `main.swift` (app/runtime setup), `WorkspaceWindowController` (surface lifecycle)

**Relay Layer (PingPong Routing):**
- Purpose: Capture output from one terminal pane and inject into another
- Location: `Sources/SmuxApp/PingPongRouter.swift`, `Sources/SmuxApp/ANSIStripper.swift`
- Contains: Capture polling, turn-complete detection (OSC 133 + silence timeout), relay injection
- Depends on: `GhosttyTerminalView` (capture + inject APIs), `NotificationCenter` (command finished events)
- Used by: `WorkspaceWindowController.togglePingPong()`

**UI Layer (Workspace):**
- Purpose: Window management, split panes, sidebar, controls
- Location: `Sources/SmuxApp/WorkspaceWindowController.swift`, `Sources/SmuxApp/SidebarView.swift`, `Sources/SmuxApp/MissionControlBar.swift`
- Contains: NSWindow management, NSSplitView-based pane splitting, mission control bar, sidebar navigation
- Depends on: PTY/Terminal Layer (`GhosttyTerminalView`), Relay Layer (`PingPongRouter`), IPC Layer
- Used by: `AppDelegate` in `main.swift`

**IPC/Session Layer:**
- Purpose: Communicate with smux-daemon for session management, detach/reattach
- Location: `Sources/SmuxApp/IpcClient.swift`, `Sources/SmuxApp/SessionDetachReattach.swift`, `Sources/SmuxApp/SessionModel.swift`
- Contains: Unix socket client, length-prefixed JSON protocol, session attach/detach, workspace detection
- Depends on: Darwin sockets, FileManager
- Used by: `WorkspaceWindowController`, `MissionControlState`

**Support Layer:**
- Purpose: Cross-cutting utilities and secondary features
- Location: `Sources/SmuxApp/PolicyEngine.swift`, `Sources/SmuxApp/KeybindingSystem.swift`, `Sources/SmuxApp/SessionRestore.swift`, `Sources/SmuxApp/AppleScriptSupport.swift`, `Sources/SmuxApp/BrowserAutomation.swift`, `Sources/SmuxApp/BrowserPanelView.swift`
- Contains: Command policy enforcement, keybinding registry, state persistence, AppleScript handlers, embedded browser
- Depends on: WebKit (browser), AppKit, Foundation
- Used by: `WorkspaceWindowController`, `AppDelegate`

## Data Flow

**Ghostty EXEC Mode Terminal Flow:**

1. `main.swift:9` calls `ghostty_init()` to initialize the library
2. `main.swift:70-89` creates `ghostty_config_t`, `ghostty_runtime_config_s`, and `ghostty_app_t`
3. `WorkspaceWindowController.setupLayout()` creates `GhosttyTerminalView` instances
4. `GhosttyTerminalView.viewDidMoveToWindow()` calls `ghostty_surface_new(app, &cfg)` to create a surface
5. Ghostty internally forks a PTY child process (shell) -- smux has NO access to the PTY master fd
6. Keyboard input flows: `NSEvent` -> `keyDown` -> `interpretKeyEvents` -> `ghostty_surface_key(surface, key)`
7. Text injection flows: `sendText(text)` -> `ghostty_surface_text(surface, ptr, len)` -- writes to PTY stdin
8. Ghostty renders terminal output via Metal to the `CAMetalLayer` backing the NSView
9. `ghostty_app_tick(gApp)` is called 30 times/second via timer in `main.swift:126-129`

**Ping-Pong Relay Flow:**

1. User triggers `togglePingPong()` in `WorkspaceWindowController` (requires 2+ split panes)
2. `PingPongRouter` is created with references to two `GhosttyTerminalView` instances
3. Router subscribes to `Notification.Name.ghosttyCommandFinished` for OSC 133 D events
4. Router calls `paneA.startCapturing(onChange:)` which polls viewport at 4 Hz (250ms intervals)
5. `GhosttyTerminalView.captureViewportText()` uses `ghostty_surface_read_text(surface, selection, &txt)` to read the full viewport
6. `ANSIStripper.strip()` removes ANSI escape sequences from captured text
7. Turn-complete detected by either: (a) `GHOSTTY_ACTION_COMMAND_FINISHED` from `action_cb` in `main.swift:22-52`, or (b) 2-second silence timeout
8. `PingPongRouter.processTurnComplete()` extracts delta (new output only) by comparing against baseline snapshot
9. Delta is injected into the target pane: `targetPane.sendText(delta + "\n")` which calls `ghostty_surface_text()`
10. Router switches speaker (A->B or B->A), resets baseline, starts capturing the other pane

**Action Callback Flow:**

1. Ghostty internally detects OSC 133 D (command finished) from shell integration
2. Ghostty calls `action_cb` defined in `main.swift:22` with `GHOSTTY_ACTION_COMMAND_FINISHED` tag
3. Callback extracts `exit_code` and `surface` pointer from the action payload
4. Posts `NSNotification` on main queue with name `.ghosttyCommandFinished`
5. `PingPongRouter` receives notification and triggers `processTurnComplete()`

**IPC/Daemon Session Flow:**

1. `SmuxIpcClient` connects to `~/.smux/smux.sock` Unix domain socket
2. Wire format: 4-byte big-endian length prefix + JSON payload
3. Commands: `StartSession`, `ListSessions`, `AttachSession`, `DetachSession`, `ListDetachedSessions`
4. Responses: `SessionCreated`, `SessionList`, `AttachResult`, `DetachResult`, `DetachedSessionList`, `Error`
5. If daemon is not running, app falls back to local mode (terminal works, relay displays work, but no persistent sessions)

**State Management:**
- `MissionControlState`: In-memory session list, auto-mode toggle, approval state
- `SessionRestore`: JSON-based persistence of window frames and split layouts to `~/.smux/workspace-state.json`
- `SessionDetachReattach`: Attach state persistence to `~/.smux/attach-state.json`

## Key Abstractions

**ghostty_surface_t (Opaque Terminal Surface):**
- Purpose: Represents a single terminal pane with its own PTY, renderer, and state
- Created: `GhosttyTerminalView.viewDidMoveToWindow()` via `ghostty_surface_new()`
- Lifecycle: Created when view enters window hierarchy, freed in `destroySurface()` (async on MainActor)
- Pattern: Opaque pointer -- all interaction through `ghostty_surface_*` C API functions

**GhosttyTerminalView (NSView + Terminal):**
- Purpose: Bridge between AppKit and libghostty; hosts a single terminal surface
- Examples: `Sources/SmuxApp/GhosttyTerminalView.swift`
- Pattern: NSView subclass with CAMetalLayer backing, NSTextInputClient for IME, manual keyboard/mouse event routing

**PingPongRouter (Relay State Machine):**
- Purpose: Orchestrates bidirectional text relay between two terminal panes
- Examples: `Sources/SmuxApp/PingPongRouter.swift`
- Pattern: State machine with states: idle -> waitingForOutput -> paneASpeaking/paneBSpeaking -> (repeat) -> idle
- States: `idle`, `waitingForOutput`, `paneASpeaking`, `paneBSpeaking`, `paused`

## Entry Points

**Application Entry:**
- Location: `Sources/SmuxApp/main.swift:267-270`
- Triggers: Process launch
- Responsibilities: Initialize ghostty, create NSApplication, set delegate, run event loop

**AppDelegate.applicationDidFinishLaunching:**
- Location: `Sources/SmuxApp/main.swift:69-129`
- Triggers: NSApplication launch
- Responsibilities: Create ghostty config/app, create workspace controller, setup menu bar, start tick timer, init session manager and AppleScript support

**Action Callback:**
- Location: `Sources/SmuxApp/main.swift:22-52` (the `actionCb` closure)
- Triggers: Any ghostty action event (currently only handles `GHOSTTY_ACTION_COMMAND_FINISHED`)
- Responsibilities: Bridge ghostty events to NSNotificationCenter for Swift consumption

## Ghostty API Surface -- Complete Inventory

**Currently Used APIs:**
| API | Location | Purpose |
|-----|----------|---------|
| `ghostty_init()` | `main.swift:9` | Library initialization |
| `ghostty_config_new()` | `main.swift:71` | Create config |
| `ghostty_config_load_default_files()` | `main.swift:72` | Load user config |
| `ghostty_config_finalize()` | `main.swift:73` | Finalize config |
| `ghostty_config_free()` | `main.swift:88` | Free config |
| `ghostty_app_new()` | `main.swift:86` | Create app instance |
| `ghostty_app_free()` | `main.swift:215` | Free app instance |
| `ghostty_app_tick()` | `main.swift:128` | Advance event loop (30 Hz) |
| `ghostty_surface_config_new()` | `GhosttyTerminalView.swift:63` | Create surface config |
| `ghostty_surface_new()` | `GhosttyTerminalView.swift:74` | Create terminal surface |
| `ghostty_surface_free()` | `GhosttyTerminalView.swift:35` | Free surface (async) |
| `ghostty_surface_draw()` | `GhosttyTerminalView.swift:53` | Render frame |
| `ghostty_surface_set_content_scale()` | `GhosttyTerminalView.swift:78` | Set Retina scale |
| `ghostty_surface_set_size()` | `GhosttyTerminalView.swift:81` | Set pixel dimensions |
| `ghostty_surface_key()` | `GhosttyTerminalView.swift:160` | Send key event |
| `ghostty_surface_text()` | `GhosttyTerminalView.swift:221` | Write text to PTY stdin |
| `ghostty_surface_preedit()` | `GhosttyTerminalView.swift:207` | IME preedit state |
| `ghostty_surface_ime_point()` | `GhosttyTerminalView.swift:416` | Get IME cursor position |
| `ghostty_surface_read_text()` | `GhosttyTerminalView.swift:247` | Read viewport text (capture) |
| `ghostty_surface_free_text()` | `GhosttyTerminalView.swift:248` | Free captured text |
| `ghostty_surface_mouse_button()` | `GhosttyTerminalView.swift:296` | Mouse click |
| `ghostty_surface_mouse_pos()` | `GhosttyTerminalView.swift:297` | Mouse position |
| `ghostty_surface_mouse_scroll()` | `GhosttyTerminalView.swift:318` | Scroll events |

**Available But Unused APIs (from `ghostty.h`):**
| API | Potential Use |
|-----|--------------|
| `ghostty_surface_write_buffer(surface, data, len)` | **CRITICAL: Write raw bytes to terminal input (bypasses keyboard processing)** |
| `ghostty_surface_process_exited(surface)` | Check if child process has exited |
| `ghostty_surface_process_exit(surface, code, time)` | Notify surface of process exit |
| `ghostty_surface_refresh(surface)` | Force redraw |
| `ghostty_surface_set_focus(surface, bool)` | Notify surface of focus state |
| `ghostty_surface_set_occlusion(surface, bool)` | Notify of window occlusion |
| `ghostty_surface_size(surface)` | Get current size (cols, rows, px) |
| `ghostty_surface_set_color_scheme(surface, scheme)` | Light/dark mode |
| `ghostty_surface_key_translation_mods(surface, mods)` | Modifier translation |
| `ghostty_surface_key_is_binding(surface, key, flags)` | Check if key is bound |
| `ghostty_surface_mouse_captured(surface)` | Check if app captures mouse |
| `ghostty_surface_has_selection(surface)` | Check for active selection |
| `ghostty_surface_read_selection(surface, text)` | Read selected text |
| `ghostty_surface_request_close(surface)` | Request surface close |
| `ghostty_surface_split(surface, direction)` | Ghostty-native split (unused -- smux uses own split) |
| `ghostty_surface_split_focus(surface, direction)` | Focus split (unused) |
| `ghostty_surface_split_resize(surface, direction, amount)` | Resize split (unused) |
| `ghostty_surface_binding_action(surface, action, len)` | Execute named binding action |
| `ghostty_surface_needs_confirm_quit(surface)` | Check if quit confirmation needed |
| `ghostty_surface_inherited_config(surface, context)` | Get inherited config for new surface |
| `ghostty_surface_update_config(surface, config)` | Update surface config at runtime |
| `ghostty_surface_complete_clipboard_request(surface, ...)` | Complete pending clipboard read |
| `ghostty_app_set_focus(app, bool)` | Set app-level focus |
| `ghostty_app_key(app, key)` | App-level key event |
| `ghostty_app_key_is_binding(app, key)` | Check app-level binding |
| `ghostty_app_keyboard_changed(app)` | Notify keyboard layout change |
| `ghostty_app_update_config(app, config)` | Update app config |
| `ghostty_app_needs_confirm_quit(app)` | Check app-level quit confirmation |
| `ghostty_app_set_color_scheme(app, scheme)` | Set color scheme |
| `ghostty_surface_inspector(surface)` | Get inspector for surface |

**HOST_MANAGED Backend (Alternative to EXEC):**
The header defines `ghostty_surface_io_backend_e` with two values:
- `GHOSTTY_SURFACE_IO_BACKEND_EXEC = 0` -- ghostty forks and manages PTY (current mode)
- `GHOSTTY_SURFACE_IO_BACKEND_HOST_MANAGED = 1` -- **host app manages PTY, ghostty only renders**

When using HOST_MANAGED, the `ghostty_surface_config_s` provides:
- `receive_buffer`: callback `(void*, const uint8_t*, size_t)` -- ghostty calls this with PTY output bytes to send to the child
- `receive_resize`: callback `(void*, uint16_t, uint16_t, uint32_t, uint32_t)` -- ghostty calls this when terminal resizes
- `receive_userdata`: context pointer for the callbacks
- `ghostty_surface_write_buffer(surface, data, len)`: host writes PTY output bytes TO ghostty for rendering

This means in HOST_MANAGED mode, smux could:
1. Create its own PTY via `smux_forkpty()` (CPtyHelper already exists)
2. Read from PTY master fd to get raw output bytes
3. Send those bytes to ghostty via `ghostty_surface_write_buffer()` for rendering
4. Intercept the bytes in transit (full stream capture, not polling)
5. Use `receive_buffer` callback to get the bytes ghostty wants to write to PTY
6. Inject/modify data before writing to the actual PTY

**Complete List of Ghostty Action Tags (from `ghostty_action_tag_e`):**
```
GHOSTTY_ACTION_QUIT, NEW_WINDOW, NEW_TAB, CLOSE_TAB, NEW_SPLIT,
CLOSE_ALL_WINDOWS, TOGGLE_MAXIMIZE, TOGGLE_FULLSCREEN, TOGGLE_TAB_OVERVIEW,
TOGGLE_WINDOW_DECORATIONS, TOGGLE_QUICK_TERMINAL, TOGGLE_COMMAND_PALETTE,
TOGGLE_VISIBILITY, TOGGLE_BACKGROUND_OPACITY, MOVE_TAB, GOTO_TAB,
GOTO_SPLIT, GOTO_WINDOW, RESIZE_SPLIT, EQUALIZE_SPLITS, TOGGLE_SPLIT_ZOOM,
PRESENT_TERMINAL, SIZE_LIMIT, RESET_WINDOW_SIZE, INITIAL_SIZE, CELL_SIZE,
SCROLLBAR, RENDER, INSPECTOR, SHOW_GTK_INSPECTOR, RENDER_INSPECTOR,
DESKTOP_NOTIFICATION, SET_TITLE, SET_TAB_TITLE, PROMPT_TITLE, PWD,
MOUSE_SHAPE, MOUSE_VISIBILITY, MOUSE_OVER_LINK, RENDERER_HEALTH,
OPEN_CONFIG, QUIT_TIMER, FLOAT_WINDOW, SECURE_INPUT, KEY_SEQUENCE,
KEY_TABLE, COLOR_CHANGE, RELOAD_CONFIG, CONFIG_CHANGE, CLOSE_WINDOW,
RING_BELL, UNDO, REDO, CHECK_FOR_UPDATES, OPEN_URL, SHOW_CHILD_EXITED,
PROGRESS_REPORT, SHOW_ON_SCREEN_KEYBOARD, COMMAND_FINISHED,
START_SEARCH, END_SEARCH, SEARCH_TOTAL, SEARCH_SELECTED, READONLY,
COPY_TITLE_TO_CLIPBOARD
```

Only `COMMAND_FINISHED` is currently handled in the action callback. Other useful actions that could be handled:
- `PWD` -- working directory changes (cd detection)
- `SET_TITLE` -- terminal title updates
- `PROGRESS_REPORT` -- OSC progress reporting
- `DESKTOP_NOTIFICATION` -- in-terminal notification requests
- `SCROLLBAR` -- scroll position tracking
- `RING_BELL` -- bell events
- `SHOW_CHILD_EXITED` -- child process exit notification

## Error Handling

**Strategy:** Fail-silent with NSLog warnings; no crashes, no user-visible errors.

**Patterns:**
- Ghostty API calls protected by `guard let s = surface else { return }` in every method
- IPC operations wrapped in do/catch with silent fallback to local mode
- `ghostty_surface_free()` runs asynchronously on MainActor to avoid Metal thread-safety issues
- Surface destruction order is critical: detach contentView FIRST, then free surfaces (documented in `WorkspaceWindowController.destroyAllSurfaces()`)

## Cross-Cutting Concerns

**Logging:** `NSLog()` throughout with `[smux]`, `[pingpong]`, `[ghostty-action]` prefixes
**Validation:** `PolicyEngine` in `Sources/SmuxApp/PolicyEngine.swift` -- configurable allow/deny lists for commands
**Authentication:** None -- local app connecting to local daemon via Unix socket
**State Persistence:** JSON files in `~/.smux/` (workspace-state.json, attach-state.json, templates.json, config.toml)

## Integration Points for New Relay Architecture

**Option A: HOST_MANAGED Backend (Recommended)**
Hook location: `GhosttyTerminalView.viewDidMoveToWindow()` at line 63-83
Change: Set `cfg.backend = GHOSTTY_SURFACE_IO_BACKEND_HOST_MANAGED`, wire `receive_buffer` and `receive_resize` callbacks, use `smux_forkpty()` to create own PTY, intercept all PTY traffic.

**Option B: Enhanced Polling (Current approach, improved)**
Hook location: `PingPongRouter.startCapturingCurrentPane()` at line 131
Improvement: Increase polling frequency, use scrollback-aware selections, handle viewport overflow.

**Option C: Action Callback Expansion**
Hook location: `main.swift:22-52` (the `actionCb`)
Addition: Handle `PWD`, `SET_TITLE`, `PROGRESS_REPORT`, `SCROLLBAR` actions for richer state tracking.

**CPtyHelper (Already Available):**
- Location: `Sources/CPtyHelper/pty_helper.c`
- Provides: `smux_forkpty(pid, rows, cols)` returns master fd, `smux_pty_resize(fd, rows, cols)`
- Note: Currently unused in the build (Package.swift does not include CPtyHelper as a target dependency)
- For HOST_MANAGED mode: Use `smux_forkpty()` to create PTY, read/write master fd, pipe through ghostty for rendering

---

*Architecture analysis: 2026-03-26*
