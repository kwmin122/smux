# smux native shell (macOS)

Native macOS terminal powered by libghostty with Korean IME support.

## Quick Start

```bash
# Build
cd macos/smux && swift build

# Run
swift run

# Or via xcodebuild
xcodebuild -scheme SmuxApp -destination 'platform=macOS' build
```

## Features

- **libghostty Metal rendering** — GPU-accelerated terminal (120fps)
- **Korean IME** — NSTextInputClient with ghostty_surface_preedit/text
- **Tabs** — ⌘T new tab, ⌘W close (macOS native tab groups)
- **Splits** — ⌘D vertical, ⌘⇧D horizontal (NSSplitView)
- **Daemon IPC** — connects to smux-daemon over Unix socket
- **Session display** — shows daemon status in title bar

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| ⌘T | New tab |
| ⌘W | Close tab |
| ⌘D | Split vertical |
| ⌘⇧D | Split horizontal |
| ⌘Q | Quit |

## Architecture

```
Swift/AppKit (this app)
├── GhosttyTerminalView — NSView + CAMetalLayer + NSTextInputClient
├── WorkspaceWindowController — tabs + splits
├── SmuxIpcClient — Unix socket → smux-daemon
└── SessionModel + MissionControlState

libghostty (prebuilt xcframework)
├── ghostty_surface_t — terminal rendering + PTY
├── ghostty_surface_key — keyboard input
├── ghostty_surface_text/preedit — IME text
└── ghostty_app_t — configuration + tick loop

Rust daemon (smux-daemon)
├── Orchestrator — pipeline stage execution
├── Consensus — multi-verifier voting
├── IPC — length-prefixed JSON over Unix socket
└── Session store — persistence
```

## Prerequisites

- macOS 14+
- Xcode 16+
- GhosttyKit.xcframework in `Frameworks/` (prebuilt from libghostty-spm)
