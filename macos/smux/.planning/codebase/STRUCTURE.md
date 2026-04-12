# Codebase Structure

**Analysis Date:** 2026-03-26

## Directory Layout

```
macos/smux/
├── Package.swift                    # Swift Package Manager manifest (swift-tools-version: 5.10)
├── Frameworks/
│   └── GhosttyKit.xcframework/     # Pre-built libghostty binary (Zig -> C ABI)
│       ├── macos-arm64_x86_64/
│       │   ├── Headers/ghostty.h   # Complete C API header (1196 lines)
│       │   └── libghostty.a        # Static library
│       ├── ios-arm64/              # iOS variant (unused)
│       └── ios-arm64_x86_64-*/     # Catalyst/simulator variants (unused)
├── Sources/
│   ├── SmuxApp/                    # Main application source (22 Swift files)
│   │   ├── main.swift              # Entry point, ghostty init, AppDelegate, menu bar
│   │   ├── GhosttyTerminalView.swift    # NSView hosting ghostty surface
│   │   ├── WorkspaceWindowController.swift  # Main window, splits, ping-pong, browser
│   │   ├── PingPongRouter.swift    # Relay state machine
│   │   ├── ANSIStripper.swift      # ANSI escape removal utility
│   │   ├── SidebarView.swift       # Left sidebar with workspace/session list
│   │   ├── MissionControlBar.swift # Bottom control bar
│   │   ├── StageTimeline.swift     # Top pipeline stage indicator
│   │   ├── InspectorDrawer.swift   # Right-side transcript/findings drawer
│   │   ├── SearchBar.swift         # Terminal search overlay
│   │   ├── CommandPalette.swift    # Command palette (Cmd+P)
│   │   ├── SessionModel.swift      # Data models: SmuxSession, Workspace, MissionControlState
│   │   ├── IpcClient.swift         # Unix socket IPC client for smux-daemon
│   │   ├── SessionDetachReattach.swift  # tmux-style detach/reattach
│   │   ├── SessionRestore.swift    # Window state persistence
│   │   ├── PolicyEngine.swift      # Command allow/deny policy + audit + templates
│   │   ├── KeybindingSystem.swift  # Keybinding registry + launch configs
│   │   ├── AppleScriptSupport.swift     # AppleScript automation handlers
│   │   ├── BrowserAutomation.swift      # WebKit automation engine
│   │   ├── BrowserPanelView.swift       # Embedded browser panel (WKWebView)
│   │   ├── GuidePanel.swift             # User guide/help panel
│   │   └── NewSessionDialog.swift       # New relay session creation dialog
│   └── CPtyHelper/                 # C module for PTY operations
│       ├── include/
│       │   └── pty_helper.h        # Public header: smux_forkpty(), smux_pty_resize()
│       └── pty_helper.c            # forkpty() wrapper (65 lines)
├── .planning/                      # Planning and analysis documents
│   ├── codebase/                   # Architecture analysis (this directory)
│   └── debug/                      # Debug notes
└── .swiftpm/                       # Swift Package Manager metadata
```

## Directory Purposes

**`Sources/SmuxApp/`:**
- Purpose: All application Swift source code
- Contains: Entry point, views, controllers, models, utilities, automation
- Key files: `main.swift`, `GhosttyTerminalView.swift`, `WorkspaceWindowController.swift`, `PingPongRouter.swift`
- Note: Flat directory structure -- no subdirectories. All 22 files at the same level.

**`Sources/CPtyHelper/`:**
- Purpose: C bridge for PTY operations (forkpty() not callable from Swift)
- Contains: Single C file + header
- Key files: `pty_helper.c`, `include/pty_helper.h`
- Note: Currently not wired into Package.swift as a dependency of SmuxApp -- exists but is unused

**`Frameworks/GhosttyKit.xcframework/`:**
- Purpose: Pre-built libghostty binary framework
- Contains: Static libraries (.a) and C header for each platform
- Key files: `macos-arm64_x86_64/Headers/ghostty.h` (the API contract)
- Note: Universal binary (arm64 + x86_64), includes iOS variants that are not used

**`~/.smux/` (Runtime data, not in repo):**
- Purpose: User configuration and state persistence
- Contains: `smux.sock` (daemon socket), `workspace-state.json`, `attach-state.json`, `templates.json`, `config.toml`, `audits/`, `launch-configs.json`

## Key File Locations

**Entry Points:**
- `Sources/SmuxApp/main.swift`: Application entry point, ghostty initialization, AppDelegate, menu bar setup, tick timer

**Configuration:**
- `Package.swift`: Build configuration, target definitions, framework linkage
- `Frameworks/GhosttyKit.xcframework/macos-arm64_x86_64/Headers/ghostty.h`: Complete ghostty C API

**Core Logic:**
- `Sources/SmuxApp/GhosttyTerminalView.swift`: Terminal surface management, keyboard/mouse input, viewport capture, text injection
- `Sources/SmuxApp/PingPongRouter.swift`: Relay state machine, polling capture, turn detection, output injection
- `Sources/SmuxApp/WorkspaceWindowController.swift`: Window layout, split management, ping-pong orchestration, browser toggle

**Data Models:**
- `Sources/SmuxApp/SessionModel.swift`: `SmuxSession`, `Workspace`, `MissionControlState`, `WorkspaceDetector`

**IPC/Networking:**
- `Sources/SmuxApp/IpcClient.swift`: `SmuxIpcClient` -- Unix socket communication
- `Sources/SmuxApp/SessionDetachReattach.swift`: Session lifecycle management

**UI Components:**
- `Sources/SmuxApp/SidebarView.swift`: Left sidebar (200px fixed width) with workspace/session list, notification bell
- `Sources/SmuxApp/MissionControlBar.swift`: Bottom bar (32px) with Approve/Pause/Retry/PingPong buttons
- `Sources/SmuxApp/StageTimeline.swift`: Top bar (28px) showing Ideate -> Plan -> Execute -> Harden pipeline
- `Sources/SmuxApp/InspectorDrawer.swift`: Right drawer (250px) with tabs: Transcript, Findings, Diffs, Files
- `Sources/SmuxApp/SearchBar.swift`: Terminal search overlay (Cmd+F)
- `Sources/SmuxApp/CommandPalette.swift`: Floating command palette (Cmd+P)
- `Sources/SmuxApp/NewSessionDialog.swift`: Relay session creation dialog
- `Sources/SmuxApp/GuidePanel.swift`: Help/guide floating panel

**Automation:**
- `Sources/SmuxApp/AppleScriptSupport.swift`: AppleScript command handlers
- `Sources/SmuxApp/BrowserAutomation.swift`: WebKit automation engine
- `Sources/SmuxApp/BrowserPanelView.swift`: Embedded WKWebView browser

**Utilities:**
- `Sources/SmuxApp/ANSIStripper.swift`: Regex-based ANSI escape sequence removal
- `Sources/SmuxApp/PolicyEngine.swift`: Command policy (allow/deny), audit export, session templates
- `Sources/SmuxApp/KeybindingSystem.swift`: Keybinding registry, launch configurations
- `Sources/SmuxApp/SessionRestore.swift`: Window/split state JSON persistence

**Testing:**
- No test files exist. No test target in Package.swift.

## Naming Conventions

**Files:**
- PascalCase matching the primary class/struct name: `GhosttyTerminalView.swift`, `PingPongRouter.swift`
- Single file per major class (with minor supporting types allowed in same file)

**Directories:**
- PascalCase for Swift module directories: `SmuxApp/`, `CPtyHelper/`
- Lowercase for non-code directories: `.planning/`, `.swiftpm/`

## Where to Add New Code

**New Terminal Feature (e.g., new capture method):**
- Primary code: `Sources/SmuxApp/GhosttyTerminalView.swift` (add as method on the view)
- If it involves new ghostty API calls, check `Frameworks/GhosttyKit.xcframework/macos-arm64_x86_64/Headers/ghostty.h`

**New Relay/Router Feature:**
- Primary code: `Sources/SmuxApp/PingPongRouter.swift`
- Wire to UI in: `Sources/SmuxApp/WorkspaceWindowController.swift` (togglePingPong method area, line 387+)

**New UI Component:**
- Create new file: `Sources/SmuxApp/MyComponent.swift` (PascalCase, one class per file)
- Add to window layout in: `Sources/SmuxApp/WorkspaceWindowController.swift` (setupLayout method)
- Add menu item in: `Sources/SmuxApp/main.swift` (setupMenuBar method, line 132+)

**New IPC Command:**
- Client-side: `Sources/SmuxApp/IpcClient.swift` (add method like `listSessions()`)
- Session handling: `Sources/SmuxApp/SessionDetachReattach.swift` (if session-related)
- Data model: `Sources/SmuxApp/SessionModel.swift`

**New Data Model:**
- Add to: `Sources/SmuxApp/SessionModel.swift` (contains all domain models)

**New Utility:**
- Create new file: `Sources/SmuxApp/MyUtility.swift`
- Follow pattern: use `enum` for stateless utilities (like `ANSIStripper`), `class` for stateful ones

**New C Bridge Code:**
- Add to: `Sources/CPtyHelper/pty_helper.c` and `Sources/CPtyHelper/include/pty_helper.h`
- Note: Must also add CPtyHelper as a dependency in `Package.swift` (currently missing)

**New Ghostty Action Handler:**
- Add case to `actionCb` in `Sources/SmuxApp/main.swift:22-52`
- Post NSNotification for Swift-side consumption
- Subscribe in relevant component

## Special Directories

**`Frameworks/GhosttyKit.xcframework/`:**
- Purpose: Pre-built libghostty binary framework
- Generated: Yes (built externally from ghostty Zig source)
- Committed: Yes (checked into repo as binary)

**`.build/`:**
- Purpose: Swift Package Manager build artifacts
- Generated: Yes
- Committed: No (should be in .gitignore)

**`.swiftpm/`:**
- Purpose: SPM workspace and configuration metadata
- Generated: Yes
- Committed: Partially (configuration files committed)

**`~/.smux/`:**
- Purpose: Runtime user data (config, state, audit logs)
- Generated: Yes (created at runtime)
- Committed: No (user-local directory)

## Architecture Constraints

**Single Package Target:**
The entire app is a single `executableTarget` named `SmuxApp`. There are no library targets, no test targets, and no modular boundaries. All 22 Swift files compile together.

**CPtyHelper Not Wired:**
`Sources/CPtyHelper/` exists with a valid C module but is NOT listed as a target or dependency in `Package.swift`. To use it, add:
```swift
.target(name: "CPtyHelper", path: "Sources/CPtyHelper"),
```
and add `"CPtyHelper"` to SmuxApp's dependencies array.

**No Test Infrastructure:**
There is no test target, no test files, and no test framework configured. To add tests, create a `Tests/` directory and add a `.testTarget` in `Package.swift`.

---

*Structure analysis: 2026-03-26*
