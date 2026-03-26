# Requirements: smux

**Defined:** 2026-03-26
**Milestone:** v0.9 — PTY Stream Relay
**Core Value:** Two AI agents ping-pong in real visible PTYs — user sleeps, wakes up to idea→plan→impl→review all done.

## v0.9 Requirements

### HOST_MANAGED PTY Foundation

- [ ] **HPTY-01**: GhosttyTerminalView creates surfaces in HOST_MANAGED mode with smux-owned PTY via `smux_forkpty()`
- [ ] **HPTY-02**: Raw PTY output bytes from master fd are captured in real-time and teed to ghostty renderer via `ghostty_surface_write_buffer()`
- [ ] **HPTY-03**: Korean IME (NSTextInputClient) works correctly in HOST_MANAGED mode — preedit, composition, commit all functional
- [ ] **HPTY-04**: `receive_buffer` callback correctly forwards ghostty's keyboard/input bytes to PTY slave via master fd write

### Turn Detection (Raw Stream)

- [ ] **TURN-01**: Turn-complete detected by silence timeout (configurable, default 3s) on raw PTY output stream — no viewport polling
- [ ] **TURN-02**: Optional prompt pattern regex on raw PTY stream as secondary signal (Claude Code ❯, shell $, etc.)
- [ ] **TURN-03**: Turn detection ignores ANSI escape sequences and cursor movement — only counts printable content as "activity"

### Relay Injection

- [ ] **RELAY-01**: On turn-complete, captured output (ANSI-stripped) is injected into target pane's PTY stdin via master fd write
- [ ] **RELAY-02**: Relay injection does NOT create feedback loop — stdin write path is separate from stdout capture path
- [ ] **RELAY-03**: Relay continues A→B→A→B until user pauses or stops (max rounds configurable)
- [ ] **RELAY-04**: Empty or whitespace-only turns are skipped (no injection)

### UI & Control

- [ ] **UI-01**: ⌘⇧P toggles ping-pong mode (existing, verify still works with HOST_MANAGED)
- [ ] **UI-02**: Mission control bar shows relay direction (A→B / B→A), turn count, running/paused state
- [ ] **UI-03**: Inspector transcript logs each relay turn with speaker label and output preview

### Stability

- [ ] **STAB-01**: PTY cleanup on window close — master fd closed, child process terminated, ghostty surface freed
- [ ] **STAB-02**: Thread-safe PTY reader — dispatch raw bytes to main thread for ghostty_surface_write_buffer

## v0.8 Requirements (Completed)

All 13 v0.8 requirements (STAB-01/02, PTY-CAP-01/02/03, PING-01/02/03/04, E2E-01/02/03/04) were implemented but PING-02/03 proved architecturally broken for TUI agents. v0.9 replaces the capture/relay architecture entirely.

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| HPTY-01 | Phase 1 | Pending |
| HPTY-02 | Phase 1 | Pending |
| HPTY-03 | Phase 1 | Pending |
| HPTY-04 | Phase 1 | Pending |
| TURN-01 | Phase 2 | Pending |
| TURN-02 | Phase 2 | Pending |
| TURN-03 | Phase 2 | Pending |
| RELAY-01 | Phase 3 | Pending |
| RELAY-02 | Phase 3 | Pending |
| RELAY-03 | Phase 3 | Pending |
| RELAY-04 | Phase 3 | Pending |
| UI-01 | Phase 3 | Pending |
| UI-02 | Phase 3 | Pending |
| UI-03 | Phase 3 | Pending |
| STAB-01 | Phase 1 | Pending |
| STAB-02 | Phase 1 | Pending |

**Coverage:**
- v0.9 requirements: 16 total
- Mapped to phases: 16
- Unmapped: 0

---
*Requirements defined: 2026-03-26*
