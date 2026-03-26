---
status: investigating
trigger: "Two critical bugs in ping-pong relay — viewport-based capture is fundamentally wrong for TUI agent relay"
created: 2026-03-26T00:00:00Z
updated: 2026-03-26T01:00:00Z
---

## Current Focus

hypothesis: CONFIRMED — Viewport-based capture is architecturally wrong. Root cause is using ghostty_surface_read_text(viewport) + string-prefix-diff, which cannot extract just the response from a full-screen TUI, and sendText() injection triggers the same capture pipeline causing infinite feedback loop.
test: Code trace completed for both bugs. Research completed on alternative architectures.
expecting: N/A — root cause confirmed, now documenting alternative architectures
next_action: Document root cause and research findings for viable replacement architectures

## Symptoms

expected: When Claude in Pane A responds with "ㅎㅇ", only that text gets relayed to Pane B. Pane B's agent processes it and responds. Relay alternates cleanly A->B->A->B.
actual: TWO BUGS — (1) FULL VIEWPORT CAPTURE: Instead of extracting just the response text, relay captures ENTIRE terminal viewport including TUI chrome (logo, greeting, rewind section, prompt symbols). This blob gets pasted into Pane B as "[Pasted text #1 +39 lines][Pasted text #2 +37 lines]..." (2) INFINITE COPY LOOP: After injecting text into Pane B, the silence timeout (2s) fires on SAME pane's viewport changes caused by injection itself, treating self-injected text as "new output" and triggering another copy cycle. Never waits for actual agent response.
errors: No crashes — system works exactly as coded but architecture is wrong
reproduction: Launch SmuxApp, split Cmd+D, run "claude" in both panes, activate ping-pong Cmd+Shift+P, type anything in left pane
started: First time testing with real TUI agents. Viewport capture designed before understanding claude is full-screen TUI.

## Eliminated

- hypothesis: Bug can be fixed by tuning silence threshold or delta detection
  evidence: TUI redraws entire screen — baseline is never a prefix of current viewport. Even with perfect diff, viewport contains TUI chrome mixed with response text. And silence timer fires on self-injected viewport changes regardless of timeout value.
  timestamp: 2026-03-26T00:30:00Z

- hypothesis: OSC 133 COMMAND_FINISHED can serve as reliable turn-complete signal for TUI agents
  evidence: Claude Code does not currently emit OSC 133 sequences (github.com/anthropics/claude-code/issues/22528 and #32635 are open feature requests). Even if it did, COMMAND_FINISHED fires when the shell command exits, not when the agent produces a response within its TUI. Claude Code is a single long-running process — it never "finishes" between turns.
  timestamp: 2026-03-26T00:40:00Z

## Evidence

- timestamp: 2026-03-26T00:10:00Z
  checked: PingPongRouter.swift line 137 + GhosttyTerminalView.swift lines 230-257
  found: captureViewportText() uses ghostty_surface_read_text() with GHOSTTY_POINT_VIEWPORT from (0,0) to (9999,9999) — reads the ENTIRE visible viewport. For a TUI like Claude Code, this includes logo, greeting, rewind section, conversation history, prompt symbols, and the actual response text all mixed together.
  implication: There is no way to extract "just the response" from a viewport dump of a full-screen TUI.

- timestamp: 2026-03-26T00:15:00Z
  checked: PingPongRouter.swift extractDelta() lines 238-246
  found: Delta extraction uses string prefix matching: full.hasPrefix(baseline). For a TUI that redraws the entire screen on each update, the new viewport is never a prefix-extension of the old viewport. The function falls through to line 245 and returns the FULL viewport text every time.
  implication: Bug #1 confirmed — full viewport dump is inevitable with this approach.

- timestamp: 2026-03-26T00:20:00Z
  checked: PingPongRouter.swift processTurnComplete() lines 186-233 + startCapturingCurrentPane() lines 131-142
  found: After injecting text (line 208: targetPane?.sendText(delta + "\n")), the router immediately switches speaker (line 228) and starts polling the OTHER pane (line 233: startCapturingCurrentPane()). But sendText() writes to PTY stdin, which the TUI renders, changing the viewport of the target pane. The 4Hz polling timer in startCapturing (GhosttyTerminalView.swift line 269) detects this viewport change, calls handleNewOutput(), which resets the silence timer. After 2 seconds of "silence" (no further changes from the injection rendering), processTurnComplete fires again, captures the viewport (now containing injected text), and injects it back.
  implication: Bug #2 confirmed — self-injection feedback loop. The architecture cannot distinguish "viewport changed because we injected text" from "viewport changed because the agent produced new output."

- timestamp: 2026-03-26T00:30:00Z
  checked: ghostty.h surface_config_s (lines 452-469 of header)
  found: CRITICAL DISCOVERY — ghostty_surface_config_s has: (1) backend field with GHOSTTY_SURFACE_IO_BACKEND_EXEC (default, ghostty manages PTY) and GHOSTTY_SURFACE_IO_BACKEND_HOST_MANAGED (host manages PTY), (2) receive_buffer callback (void*, const uint8_t*, size_t) that receives raw PTY output bytes, (3) ghostty_surface_write_buffer() to inject bytes into the terminal as if from PTY output. Current smux code does NOT set receive_buffer — it defaults to nil.
  implication: libghostty already provides the API needed for PTY output stream monitoring. receive_buffer gives raw bytes BEFORE terminal rendering, which is the correct tap point for output capture.

- timestamp: 2026-03-26T00:35:00Z
  checked: CPtyHelper/pty_helper.c already in project
  found: smux_forkpty() creates a PTY pair via forkpty() and returns the master fd. This is the infrastructure needed for HOST_MANAGED mode where smux controls the PTY directly.
  implication: The project already has the building blocks for a PTY-stream-based approach.

- timestamp: 2026-03-26T00:40:00Z
  checked: tmux pipe-pane architecture (github.com/tmux/tmux cmd-pipe-pane.c)
  found: tmux's pipe-pane reads from the PTY master fd output stream (not the rendered viewport). It uses libevent to monitor the PTY fd for read events, and pipes raw output bytes to external commands. capture-pane reads from the terminal's internal buffer (similar to viewport capture, but tmux has semantic zone awareness via OSC 133). Key insight: tmux NEVER polls the rendered viewport for inter-pane relay — it always works at the PTY byte stream level.
  implication: Real terminal multiplexers monitor the PTY output stream, not the rendered screen. This is the correct architecture.

- timestamp: 2026-03-26T00:45:00Z
  checked: Claude Code OSC 133 support status
  found: github.com/anthropics/claude-code/issues/22528 and #32635 are open feature requests for OSC 133 support. A recent PR added OSC 133 for click_events but full semantic prompt marking (A/B/C/D zones) is not yet complete. Claude Code is a long-running TUI process — even with OSC 133, COMMAND_FINISHED would only fire when claude exits, not between conversation turns.
  implication: Cannot rely on OSC 133 for turn-complete detection with Claude Code. Need agent-specific prompt pattern detection.

## Resolution

root_cause: The ping-pong relay uses viewport-based capture (ghostty_surface_read_text on GHOSTTY_POINT_VIEWPORT) which reads the ENTIRE rendered terminal screen. This is fundamentally incompatible with full-screen TUI agents like Claude Code because: (1) TUIs redraw the entire screen, so there is no prefix-appendable stream to diff against — extractDelta() always returns the full viewport including TUI chrome. (2) sendText() injects into the PTY stdin, which the TUI renders, changing the viewport of the pane being polled — creating an infinite feedback loop where the router copies its own injected text.

fix: ARCHITECTURAL REDESIGN NEEDED — see Research Findings below for 3 viable approaches ranked by feasibility.

verification:
files_changed: []

## Research Findings: Alternative Architectures

### Approach A: PTY Output Stream Tap via receive_buffer (RECOMMENDED)

**How it works:**
- Set `receive_buffer` callback in `ghostty_surface_config_s` during surface creation
- This callback receives raw PTY output bytes (the stream from the program's stdout/stderr through the PTY master fd) BEFORE terminal rendering
- Parse the raw byte stream for agent response boundaries (prompt patterns, OSC sequences)
- Use `ghostty_surface_text()` (existing sendText) for injection into the other pane

**Why this is correct:**
- Works at the byte stream level, like tmux pipe-pane
- No viewport rendering involved — immune to TUI redraw noise
- Can detect prompt patterns in raw output (e.g., Claude's ">" prompt)
- Naturally avoids feedback loop: injection goes via sendText() to PTY stdin, but receive_buffer monitors PTY OUTPUT — the two paths are separate (stdin != stdout)

**Requirements:**
- May require EXEC mode surfaces to support receive_buffer (needs verification — the callback exists in surface_config but may only fire in HOST_MANAGED mode)
- If EXEC mode doesn't support receive_buffer, use HOST_MANAGED mode with CPtyHelper to fork the PTY ourselves, read from master fd, and pipe to ghostty via ghostty_surface_write_buffer()
- Need a VT100/ANSI parser to strip escape sequences from raw output (ANSIStripper already exists)
- Need prompt pattern detection for Claude Code (e.g., regex for ">" or similar)

**Complexity:** Medium — mostly wiring existing APIs differently

### Approach B: HOST_MANAGED Mode with Direct PTY fd Monitoring

**How it works:**
- Switch from EXEC mode to HOST_MANAGED mode for ping-pong panes
- Use CPtyHelper's smux_forkpty() to create PTY, get master fd
- Read from master fd using DispatchSource or kqueue for output monitoring
- Write to master fd for injection (instead of sendText)
- Feed output bytes to ghostty via ghostty_surface_write_buffer() for rendering

**Why this is correct:**
- Full control over PTY I/O — can tap output stream at the source
- Same architecture used by iTerm2, Terminal.app for their capture features
- Complete separation of output monitoring (read from master fd) and injection (write to master fd)
- Feedback loop impossible: we read what the PROGRAM writes, not what WE injected through the same fd

**Wait — feedback loop concern:** Actually, writing to the master fd IS the same as the program reading from stdin, and the program's response comes back on the master fd output. So injection text goes to program stdin, program processes it, writes response to stdout, which comes back on master fd. The relay would only capture the RESPONSE, not the injected text itself (the injected text appears on the slave side's stdin, not on the master's read side, unless the terminal has echo enabled — which it typically does for line-editing shells but NOT for TUI programs that set raw mode).

**Complexity:** Medium-High — requires managing PTY lifecycle, but CPtyHelper already exists

### Approach C: Hybrid — EXEC Mode with Prompt Pattern Detection on Viewport

**How it works:**
- Keep current EXEC mode (ghostty manages PTY)
- Instead of capturing full viewport and diffing, poll for specific prompt patterns
- Wait for Claude's prompt indicator (e.g., ">") to appear at expected screen position
- When prompt detected: extract text between previous prompt and current prompt
- Use a "cooldown" period after injection to prevent feedback loop

**Why this is inferior but simpler:**
- Still uses viewport capture, but with smarter extraction logic
- Fragile: depends on knowing Claude Code's exact prompt format
- Cooldown period is a hack — doesn't truly solve feedback loop
- Different agents (codex, gemini) would need different prompt patterns

**Complexity:** Low — but fragile and agent-specific

### Recommendation

**Approach A is the clear winner** if receive_buffer works in EXEC mode. It's the simplest change (just set a callback in surface config), gives clean byte-stream access, and avoids the feedback loop naturally.

If receive_buffer requires HOST_MANAGED mode, **Approach B** is the correct path, and the project already has CPtyHelper for PTY creation. This is more work but architecturally sound and future-proof.

**Approach C should be avoided** — it patches the symptoms without fixing the root cause.

### Key Verification Needed

Before implementing, verify whether `receive_buffer` fires in EXEC mode:
1. Set receive_buffer in ghostty_surface_config_s before calling ghostty_surface_new()
2. Log any callbacks received
3. If it fires, Approach A is viable with minimal code changes
4. If it doesn't fire, Approach B (HOST_MANAGED) is required
