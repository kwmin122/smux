# Roadmap: smux — v0.9 PTY Stream Relay

## Overview

v0.9 replaces the broken viewport-polling relay with a PTY-stream-based architecture. The v0.8 approach (ghostty_surface_read_text viewport capture at 4Hz) was fundamentally incompatible with TUI agents — it captured full-screen UI chrome and created infinite feedback loops. The fix is architectural: switch from EXEC mode (ghostty owns PTY) to HOST_MANAGED mode (smux owns PTY), giving direct access to the raw PTY byte stream for capture and injection.

## Architecture Change

```
v0.8 (BROKEN):
  ghostty EXEC → viewport poll → baseline diff → sendText inject
  Problem: TUI redraws entire screen, diff always returns full viewport, inject triggers re-capture

v0.9 (NEW):
  smux_forkpty() → master fd → tee reader → ghostty_surface_write_buffer (render)
                                           → capture buffer (relay router)
  Inject: write to master fd stdin → completely separate from capture path
```

## Phases

- [ ] **Phase 1: HOST_MANAGED PTY** — Switch from EXEC to HOST_MANAGED mode; smux owns PTY via forkpty, tees raw bytes to ghostty renderer
- [ ] **Phase 2: Stream Turn Detection** — Detect turn-complete on raw PTY output stream; silence timeout + prompt regex
- [ ] **Phase 3: Stream Relay + UI** — Wire relay injection via master fd write; update mission control for stream-based state

## Phase Details

### Phase 1: HOST_MANAGED PTY
**Goal**: GhosttyTerminalView creates terminals in HOST_MANAGED mode where smux owns the PTY, reads raw output, and pipes it to ghostty for rendering
**Depends on**: Nothing (foundation phase)
**Requirements**: HPTY-01, HPTY-02, HPTY-03, HPTY-04, STAB-01, STAB-02
**Success Criteria** (what must be TRUE):
  1. Terminal renders and accepts keyboard input in HOST_MANAGED mode (shell prompt visible, commands execute)
  2. Korean IME works — preedit composition and commit in both Korean and English
  3. Raw PTY output bytes are accessible to smux (logged or buffered) before ghostty renders them
  4. `receive_buffer` callback fires when ghostty processes keyboard input, and bytes reach the child process
  5. Window close cleanly frees PTY fd + child process + ghostty surface
**Plans**: TBD (research → plan → execute)
**Risk**: HIGH — HOST_MANAGED mode behavior needs empirical verification with current ghostty xcframework
**Verification gate**: Must pass Korean IME test before proceeding to Phase 2

### Phase 2: Stream Turn Detection
**Goal**: smux detects when an AI agent's turn is complete by monitoring the raw PTY output stream
**Depends on**: Phase 1 (raw stream access)
**Requirements**: TURN-01, TURN-02, TURN-03
**Success Criteria** (what must be TRUE):
  1. Silence timeout (3s default) fires correctly when PTY output stops after agent response
  2. Prompt pattern regex detects Claude Code ❯ prompt in raw stream (optional, secondary signal)
  3. ANSI escape sequences and cursor movement do not trigger false "activity" signals
  4. Turn detection does NOT fire during text injection (stdin write doesn't appear on capture path)
**Plans**: TBD
**Risk**: MEDIUM — silence threshold tuning needed per agent

### Phase 3: Stream Relay + UI
**Goal**: Two AI agents relay responses between each other via PTY stream capture and master fd injection
**Depends on**: Phase 2 (turn detection)
**Requirements**: RELAY-01, RELAY-02, RELAY-03, RELAY-04, UI-01, UI-02, UI-03
**Success Criteria** (what must be TRUE):
  1. When Pane A's agent finishes, ONLY the response text (not TUI chrome) is injected into Pane B
  2. Relay runs A→B→A→B without feedback loop or duplication
  3. Mission control shows correct relay direction and turn count
  4. User can pause/resume/stop relay with ⌘⇧P
**Plans**: TBD
**Risk**: LOW — straightforward once Phase 1+2 work

## Progress

**Execution Order:** 1 → 2 → 3 (strictly sequential — each phase depends on previous)

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. HOST_MANAGED PTY | 0/TBD | Not started | - |
| 2. Stream Turn Detection | 0/TBD | Not started | - |
| 3. Stream Relay + UI | 0/TBD | Not started | - |

## Research References

- `.planning/debug/relay-viewport-capture-architecture.md` — Root cause analysis of v0.8 bugs
- `.planning/research/SUMMARY.md` — Architecture research (HOST_MANAGED, tmux pipe-pane, turn detection)
- `.planning/codebase/ARCHITECTURE.md` — Full ghostty API inventory and integration points
