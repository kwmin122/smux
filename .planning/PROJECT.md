# smux

## What This Is

smux is a macOS-native terminal multiplexer built on Swift/AppKit + libghostty (Metal GPU rendering) with a Rust orchestration core. It enables two AI agent CLIs (claude, codex, gemini) to run in real PTY panes and automatically relay output between them — a real-time ping-pong loop that turns multi-agent coding into an unattended workflow. Primary market: Korean developers.

## Core Value

Two AI agents ping-pong in real visible PTYs — user sleeps, wakes up to idea→plan→impl→review all done.

## Current Milestone: v0.8 — Ping-Pong Core

**Goal:** Ship the real PTY ping-pong relay between AI agents — smux's only true differentiator — while fixing the last critical stability bugs.

**Target features:**
- PTY output capture in ghostty EXEC mode (core technical blocker)
- End-to-end ping-pong relay loop (the product vision)
- ⌘W window close fix (Metal layer zombie / surface lifecycle ordering)
- E2E verification of browser panel, AppleScript hooks, detach/reattach

## Requirements

### Validated

- ✓ libghostty terminal rendering (Metal GPU) — v0.6
- ✓ PTY attach + keyboard input — v0.6
- ✓ Korean IME (NSTextInputClient, macOS native) — v0.6
- ✓ Tabs (⌘T, macOS native tabbing) — v0.6
- ✓ Split panes (⌘D/⌘⇧D, NSSplitView, Auto Layout) — v0.7
- ✓ Sidebar (workspace list, git branch, ports, notification bell) — v0.7
- ✓ Stage timeline (Ideate→Plan→Execute→Harden) — v0.7
- ✓ Mission control bar (Ping-pong/Approve/Pause/Retry) — v0.7
- ✓ Inspector drawer (Transcript/Findings/Diffs/Files) — v0.7
- ✓ Command palette (⌘P) — v0.7
- ✓ Guide panel (⌘/ or ? button) — v0.7
- ✓ Rust pipeline (orchestrator, consensus, ownership lanes) — v0.6
- ✓ Rust IPC (daemon ↔ CLI ↔ Swift client) — v0.6
- ✓ Auto Layout unified (constraint-based, no autoresizingMask mixing) — v0.7

### Active

- [ ] PTY-CAP-01: Ghostty EXEC mode terminal output captured by smux in real-time
- [ ] PTY-CAP-02: Turn completion detected (OSC 133 / prompt pattern / silence timeout)
- [ ] PTY-CAP-03: Captured ANSI output cleaned to plain text before relay
- [ ] PING-01: User activates/deactivates ping-pong mode via ⌘⇧P or mission control button
- [ ] PING-02: smux auto-relays captured output from Pane A to Pane B stdin
- [ ] PING-03: Relay continues in loop until user pauses/stops
- [ ] PING-04: Mission control bar shows active pane, turn count, relay status
- [ ] STAB-01: ⌘W closes window cleanly without Metal layer zombie
- [ ] STAB-02: Ghostty surface freed in correct order (window.contentView=nil → Task.detached surface_free → app_free)
- [ ] E2E-01: Browser panel opens alongside terminal and renders localhost URLs
- [ ] E2E-02: Browser automation (DOM snapshot) returns page content
- [ ] E2E-03: Detach/reattach preserves and restores session state
- [ ] E2E-04: AppleScript hooks execute from external scripts

### Out of Scope

- Windows shell — macOS-native first, Windows shell later via Rust IPC layer
- Headless daemon agent execution — real PTY ping-pong is the direction, not headless
- Fixed planner/verifier naming — any two agents work (claude+codex, codex+gemini, etc.)
- Web/Electron shell — rejected; Swift+libghostty is the chosen stack
- OAuth/SSO — individual user first, team controls later
- Mobile app — macOS desktop only

## Context

**Architecture:**
- Swift/AppKit app shell + libghostty (prebuilt xcframework from libghostty-spm, ghostty 1.3.1 pinned)
- Rust workspace: smux-core + smux-daemon + smux-cli (cargo test: 25 suites, 230+ tests)
- IPC: Unix socket, length-prefixed JSON (~/.smux/smux.sock)
- Build: `cd macos/smux && swift build && swift run`

**Critical technical research (ping-pong capture):**
- In EXEC mode: ghostty owns the PTY → receive_buffer callback is NOT called
- In HOST_MANAGED mode: smux owns PTY → rendering breaks (write_buffer thread safety)
- ghostty_surface_read_text: polls rendered terminal content (works in EXEC mode)
- action_cb COMMAND_FINISHED (tag=58): detects OSC 133 prompt boundary
- cmux approach: socket API read-screen + send polling

**⌘W fix research:**
- Ghostty source: BaseTerminalController.windowWillClose sets window.contentView = nil
- Surface.deinit: `Task.detached { @MainActor in ghostty_surface_free(surface) }` pattern
- Metal layer goes zombie if surface_free called before contentView = nil

**Korean IME:**
- Working via NSTextInputClient (macOS native) — maintain as hard acceptance gate
- Any change to terminal rendering must verify Korean IME still works

**Competitive context:**
- cmux: libghostty-based, Unix socket API, headless browser automation
- tmux: PTY direct ownership, send-keys, capture-pane, pipe-pane
- smux differentiator: VISIBLE real-time PTY ping-pong (others are headless)

## Constraints

- **Tech Stack**: Swift/AppKit + libghostty + Rust — no changes to core stack
- **Korean IME**: Must work at all times — non-negotiable acceptance gate
- **Real PTY**: Agents run in real visible PTY panes, not headless — core product identity
- **Build**: `swift build` must pass before any commit
- **Cargo tests**: 230+ tests must pass before any commit

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Swift/AppKit + libghostty (not Tauri/Electron) | Best macOS input quality, Korean IME native, cmux-class terminal feel | ✓ Good |
| EXEC mode (ghostty owns PTY) over HOST_MANAGED | Better rendering; HOST_MANAGED breaks rendering | — Pending (capture problem unsolved) |
| receive_buffer removed from EXEC mode approach | Confirmed: callback not called in EXEC mode | ✓ Correct decision |
| ghostty_surface_read_text as capture strategy | Works in EXEC mode; polling acceptable for AI relay | — Pending verification |
| Auto Layout 100% (no autoresizingMask mixing) | Root cause of all UI layout bugs | ✓ Good |
| Real visible PTY over headless daemon | Core product differentiator | ✓ Good |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-26 — Milestone v0.8 started*
