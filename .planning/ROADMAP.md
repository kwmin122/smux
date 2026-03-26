# Roadmap: smux — v0.8 Ping-Pong Core

## Overview

v0.8 ships smux's core differentiator: real PTY ping-pong between two AI agent CLIs running in visible ghostty panes. The path runs through four natural delivery boundaries — fix the crash that blocks stable testing, solve the hard capture problem (ghostty EXEC mode), wire the relay loop end-to-end, then verify the surrounding features (browser panel, AppleScript, detach/reattach) that complete the product story.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Surface Lifecycle Fix** - Fix ⌘W Metal zombie and ghostty surface teardown order
- [ ] **Phase 2: PTY Output Capture** - Capture terminal output from ghostty EXEC mode; detect turn boundaries; strip ANSI
- [ ] **Phase 3: Ping-Pong Relay** - Wire captured output into live relay loop with mission control status
- [ ] **Phase 4: E2E Feature Verification** - Verify browser panel, browser automation, detach/reattach, and AppleScript hooks end-to-end

## Phase Details

### Phase 1: Surface Lifecycle Fix
**Goal**: Users can close the app window cleanly without Metal layer artifacts or crashes
**Depends on**: Nothing (first phase)
**Requirements**: STAB-01, STAB-02
**Success Criteria** (what must be TRUE):
  1. User presses ⌘W and the window closes immediately with no visible Metal layer artifact left on screen
  2. User can open a new window after closing the previous one without any crash or error
  3. App can be quit and relaunched repeatedly without accumulating zombie surfaces or memory errors in the console
**Plans:** 1 plan

Plans:
- [x] 01-01-PLAN.md — Fix ghostty surface teardown order (contentView=nil FIRST, Task.detached surface_free, windowWillClose delegate)

### Phase 2: PTY Output Capture
**Goal**: smux can read and clean terminal output from agents running in ghostty EXEC mode in real-time
**Depends on**: Phase 1
**Requirements**: PTY-CAP-01, PTY-CAP-02, PTY-CAP-03
**Success Criteria** (what must be TRUE):
  1. When an agent CLI (e.g., `claude`) prints output in a ghostty pane, smux captures the text within one second
  2. smux correctly identifies when the agent's turn is complete (prompt reappears, OSC 133 boundary fires, or configurable silence timeout expires)
  3. Text delivered to the relay layer contains no ANSI escape sequences — only plain readable content
**Plans:** 2 plans

Plans:
- [ ] 02-01-PLAN.md — Create capture primitives: ANSIStripper, captureViewportText(), actionCb COMMAND_FINISHED dispatch
- [ ] 02-02-PLAN.md — Wire PingPongRouter to real capture: polling, OSC 133 turn-complete, silence timeout fallback

### Phase 3: Ping-Pong Relay
**Goal**: Two AI agents relay responses between each other automatically in a live visible loop until the user stops it
**Depends on**: Phase 2
**Requirements**: PING-01, PING-02, PING-03, PING-04
**Success Criteria** (what must be TRUE):
  1. User activates ping-pong mode via ⌘⇧P or the mission control Ping-pong button and sees the mode become active (requires two split panes to be open)
  2. After Pane A's agent finishes a turn, smux injects the cleaned output into Pane B's stdin automatically without user intervention
  3. The relay continues A→B→A→B in a self-sustaining loop until user presses Pause or Stop
  4. Mission control bar displays which pane is currently active, the running turn count, and whether the relay is running or paused
**Plans**: TBD

Plans:
- [ ] 03-01: Wire PingPongRouter to EXEC mode capture output (replace receive_buffer pattern)
- [ ] 03-02: Implement relay injection (sendText to target pane stdin on turn-complete)
- [ ] 03-03: Implement loop state machine (running/paused/stopped) with ⌘⇧P toggle
- [ ] 03-04: Update mission control bar to show pane indicator, turn count, relay state
**UI hint**: yes

### Phase 4: E2E Feature Verification
**Goal**: Browser panel, browser automation, session detach/reattach, and AppleScript hooks all work end-to-end
**Depends on**: Phase 3
**Requirements**: E2E-01, E2E-02, E2E-03, E2E-04
**Success Criteria** (what must be TRUE):
  1. User presses ⌘⇧B and a browser panel opens alongside the terminal pane and renders a localhost URL correctly
  2. Browser automation DOM snapshot call returns actual page content (non-empty, structurally valid HTML/text)
  3. User detaches a session, relaunches the app, reattaches, and sees the same pane layout restored
  4. An external AppleScript targeting smux executes successfully and receives a confirmed response
**Plans**: TBD

Plans:
- [ ] 04-01: Verify BrowserPanelView (⌘⇧B) opens and renders localhost URLs
- [ ] 04-02: Verify BrowserAutomation DOM snapshot returns real content
- [ ] 04-03: Verify SessionDetachReattach preserves and restores pane layout
- [ ] 04-04: Verify AppleScriptSupport hook responds to external script execution
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Surface Lifecycle Fix | 1/1 | Complete | 2026-03-26 |
| 2. PTY Output Capture | 0/2 | Not started | - |
| 3. Ping-Pong Relay | 0/4 | Not started | - |
| 4. E2E Feature Verification | 0/4 | Not started | - |
