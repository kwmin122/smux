# ADR-002: Native Toolchain Gate

## Status: Accepted

## Context

smux is transitioning from Tauri + xterm.js to a macOS-native Swift + libghostty shell. This requires establishing the toolchain before implementation begins.

## Decision

### Required toolchain (day zero)
- **Full Xcode** (not just Command Line Tools) — for app build, debug, Instruments, signing, packaging
- **Swift 6.1+** — already installed
- **Minimum 20GB free disk** — for Xcode + build artifacts

### Dependency strategy
- **libghostty**: pinned to a specific Ghostty commit or prebuilt xcframework version
- **Prebuilt xcframework**: allowed as dependency bootstrap only, NOT as replacement for Xcode-first development
- **Upgrades**: deliberate and tested, never floating on main

### Development model
- Xcode-first from day one
- Native shell project at `macos/smux/` (NOT in Cargo workspace)
- Rust core stays in `crates/` (smux-core, smux-daemon, smux-cli)
- IPC over Unix socket (JSON protocol) between Swift app and Rust daemon

### Current machine state (2026-03-24)
- macOS 15.5, Apple Silicon (arm64)
- Swift 6.1.2 ✅
- Metal 3 ✅
- Xcode: ❌ pending installation (macOS update required first)
- Zig: ❌ pending (`brew install zig`)
- Disk: 20GB free ✅ (after cleanup)

### Acceptance gate
Before any Swift implementation begins:
1. Xcode installed and `xcode-select -p` points to Xcode.app
2. libghostty artifact available (prebuilt or source-built)
3. Minimal Swift app renders a libghostty terminal surface
4. Korean IME composition works in that surface

If gate #4 fails, stop and reassess.

## Consequences

- Rust tasks (3-7) can proceed without Xcode
- Swift tasks (1-2, 8-10) blocked until Xcode installed
- Two parallel tracks of work
