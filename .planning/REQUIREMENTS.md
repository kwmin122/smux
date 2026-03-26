# Requirements: smux

**Defined:** 2026-03-26
**Milestone:** v0.8 — Ping-Pong Core
**Core Value:** Two AI agents ping-pong in real visible PTYs — user sleeps, wakes up to idea→plan→impl→review all done.

## v0.8 Requirements

### Stability

- [ ] **STAB-01**: User can close the app window with ⌘W without Metal layer zombie or crash
- [ ] **STAB-02**: Ghostty surface is freed in correct order: window.contentView=nil → Task.detached ghostty_surface_free → ghostty_app_free

### PTY Output Capture

- [ ] **PTY-CAP-01**: Terminal output from an agent running in ghostty EXEC mode is captured by smux in real-time
- [ ] **PTY-CAP-02**: smux detects when an agent's turn is complete (prompt ready / OSC 133 boundary / configurable silence timeout)
- [ ] **PTY-CAP-03**: Captured terminal output has ANSI escape sequences stripped before relay injection

### Ping-Pong Relay

- [ ] **PING-01**: User can activate ping-pong mode via ⌘⇧P or mission control Ping-pong button (requires two split panes)
- [ ] **PING-02**: When ping-pong is active, smux automatically injects Pane A's completed output into Pane B's stdin
- [ ] **PING-03**: Relay continues in a loop (A→B→A→B…) until user pauses or stops
- [ ] **PING-04**: Mission control bar shows relay status: active pane indicator, turn count, running/paused state

### E2E Verification

- [ ] **E2E-01**: Browser panel (⌘⇧B) opens alongside terminal pane and renders a localhost URL
- [ ] **E2E-02**: Browser automation DOM snapshot returns actual page content (not empty)
- [ ] **E2E-03**: Session detach saves state; reattach restores the session with same pane layout
- [ ] **E2E-04**: AppleScript hook executes a test script that targets smux and confirms response

## v2 Requirements (Deferred)

### Multi-Agent Pipeline

- **PIPE-01**: User can configure a 3-agent pipeline (planner + 2 verifiers) via session template
- **PIPE-02**: Verifier consensus (majority/unanimous) gates stage advancement
- **PIPE-03**: Ownership lanes prevent two workers from touching same file glob

### Team Controls

- **TEAM-01**: Session audit log exports to .jsonl
- **TEAM-02**: Command allow/deny policy per session
- **TEAM-03**: Secret/token redaction in stored transcripts

### Platform

- **PLAT-01**: Windows native shell (separate project, shares Rust IPC core)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Headless daemon agent execution | Contradicts core product identity (real visible PTY) |
| Fixed planner/verifier roles | Any two agents should work — no naming enforced |
| Web/Electron shell | Rejected in architecture review — Swift+libghostty chosen |
| OAuth/SSO login | Individual user first, team controls in future milestone |
| Mobile app | macOS desktop only for now |
| Windows shell (v0.8) | Deferred — macOS-native first, Rust IPC is Windows-ready |
| AI provider integration (built-in) | Agents run as external CLI processes in PTY — provider-agnostic |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| STAB-01 | Phase 1 | Pending |
| STAB-02 | Phase 1 | Pending |
| PTY-CAP-01 | Phase 2 | Pending |
| PTY-CAP-02 | Phase 2 | Pending |
| PTY-CAP-03 | Phase 2 | Pending |
| PING-01 | Phase 3 | Pending |
| PING-02 | Phase 3 | Pending |
| PING-03 | Phase 3 | Pending |
| PING-04 | Phase 3 | Pending |
| E2E-01 | Phase 4 | Pending |
| E2E-02 | Phase 4 | Pending |
| E2E-03 | Phase 4 | Pending |
| E2E-04 | Phase 4 | Pending |

**Coverage:**
- v0.8 requirements: 13 total
- Mapped to phases: 13
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-26*
*Last updated: 2026-03-26 — Phase mappings added after roadmap creation*
