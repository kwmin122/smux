# smux

## What This Is

smux is a macOS-native terminal multiplexer built on Swift/AppKit + libghostty (Metal GPU rendering) with a Rust orchestration core. It enables two AI agent CLIs (claude, codex, gemini) to run in real PTY panes and automatically relay output between them — a real-time ping-pong loop that turns multi-agent coding into an unattended workflow. Primary market: Korean developers.

## Core Value

Two AI agents ping-pong in real visible PTYs — user sleeps, wakes up to idea→plan→impl→review all done.

## Current Milestone: v0.9 — PTY Stream Relay

**Goal:** Replace broken viewport-polling relay with PTY-stream-based architecture using HOST_MANAGED ghostty mode. The v0.8 relay was fundamentally wrong — viewport capture includes TUI chrome and creates infinite feedback loops. v0.9 switches to raw PTY byte stream interception.

**Critical bugs solved:**
1. Full viewport dump (TUI chrome mixed with output) → Raw PTY stream capture (only actual bytes)
2. Infinite copy loop (self-injection feedback) → Separate stdin/stdout paths (no feedback possible)

**Architecture change:** EXEC mode (ghostty owns PTY) → HOST_MANAGED mode (smux owns PTY, ghostty renders only)

## Requirements

### Validated (v0.8 and prior)

- ✓ libghostty terminal rendering (Metal GPU) — v0.6
- ✓ PTY attach + keyboard input — v0.6
- ✓ Korean IME (NSTextInputClient, macOS native) — v0.6
- ✓ Tabs (⌘T, macOS native tabbing) — v0.6
- ✓ Split panes (⌘D/⌘⇧D, NSSplitView, Auto Layout) — v0.7
- ✓ Sidebar, Stage timeline, Mission control, Inspector, Command palette, Guide — v0.7
- ✓ Rust pipeline (orchestrator, consensus, ownership lanes) — v0.6
- ✓ Rust IPC (daemon ↔ CLI ↔ Swift client) — v0.6
- ✓ ⌘W clean close (Metal surface lifecycle) — v0.8
- ✓ ANSI stripping (pure Swift Regex) — v0.8
- ✓ Browser panel, AppleScript hooks, session restore — v0.8

### Active (v0.9)

- [ ] HPTY-01: GhosttyTerminalView supports HOST_MANAGED mode with smux-owned PTY
- [ ] HPTY-02: Raw PTY output bytes captured in real-time via tee reader
- [ ] HPTY-03: Turn-complete detected on raw stream (silence timeout + prompt pattern)
- [ ] HPTY-04: Relay injects clean text into target pane stdin (no feedback loop)
- [ ] HPTY-05: Korean IME works in HOST_MANAGED mode (non-negotiable)
- [ ] HPTY-06: Mission control shows relay status with raw-stream-based state

### Out of Scope

- Windows shell — macOS-native first
- Headless daemon agent execution — real PTY is the direction
- Web/Electron shell — rejected; Swift+libghostty is the stack
- Viewport-based capture — proven fundamentally broken for TUI agents
- OSC 133 as primary turn signal — TUI agents (Claude Code) don't emit it

## Context

**Architecture (v0.9 target):**
- HOST_MANAGED ghostty mode: smux owns PTY via `smux_forkpty()`, ghostty renders only
- PTY data flow: master fd → tee reader → (1) ghostty_surface_write_buffer for render + (2) capture buffer for router
- Injection: write to PTY master fd (stdin side), completely separate from capture (stdout side)
- Turn detection: silence timeout on raw PTY output stream + optional prompt regex

**Critical research findings:**
- `receive_buffer` callback: ghostty calls this in HOST_MANAGED when it wants to write to PTY child
- `ghostty_surface_write_buffer()`: host sends raw bytes to ghostty for rendering
- `smux_forkpty()` in CPtyHelper: creates PTY pair, returns master fd — already exists, not wired to Package.swift
- tmux pipe-pane: works at PTY byte stream level, never polls viewport — confirms architecture direction
- Claude Code does NOT emit OSC 133 between turns (GitHub issues #22528, #32635)
- Silence timeout (2-4s) is proven universal turn-complete signal for TUI agents

**Korean IME:**
- Working via NSTextInputClient — must verify works in HOST_MANAGED mode
- Any change to terminal rendering must verify Korean IME still works

**Competitive context:**
- tmux: pipe-pane reads raw PTY stream, never polls viewport
- iTerm2: coprocesses get "byte-for-byte copy of input from session's pty"
- Warp Blocks: output boundary detection at PTY stream level
- smux differentiator: VISIBLE real-time PTY ping-pong (others are headless)

## Constraints

- **Korean IME**: Must work at all times — non-negotiable acceptance gate
- **Real PTY**: Agents run in real visible PTY panes — core product identity
- **Build**: `swift build` must pass before any commit
- **Thread safety**: `ghostty_surface_write_buffer()` must dispatch to main thread (Metal constraint)
- **HOST_MANAGED verification**: Must empirically verify ghostty behavior before full implementation

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Swift/AppKit + libghostty (not Tauri/Electron) | Best macOS input, Korean IME native | ✓ Good |
| EXEC mode for v0.8 | Simpler; capture workaround via viewport polling | ✗ Failed — viewport capture fundamentally broken for TUI |
| **HOST_MANAGED mode for v0.9** | smux owns PTY, raw stream capture, no feedback loop | NEW — pending verification |
| Silence timeout as primary turn signal | OSC 133 doesn't fire in TUI agents (Claude Code) | ✓ Validated by research |
| Real visible PTY over headless daemon | Core product differentiator | ✓ Good |

## Evolution

This document evolves at phase transitions and milestone boundaries.

---
*Last updated: 2026-03-26 — Milestone v0.9 started after v0.8 relay architecture failure*
