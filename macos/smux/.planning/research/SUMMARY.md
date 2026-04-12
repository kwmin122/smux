# Research Summary: smux Ping-Pong Relay Architecture

**Domain:** Terminal multiplexer with AI agent inter-pane text relay
**Researched:** 2026-03-26
**Overall confidence:** HIGH

## Executive Summary

The current smux ping-pong implementation uses viewport polling at 4Hz via `ghostty_surface_read_text()` to capture terminal output, then diffs against a baseline snapshot. This approach is fundamentally broken for TUI agents like Claude Code because TUI applications redraw the entire screen (including logos, menus, previous conversation history, and status bars), making it impossible to extract only the "new response text" from a viewport dump. Additionally, injecting text via `sendText()` into the PTY stdin changes the viewport, creating a feedback loop.

The research reveals a superior approach already available in the ghostty C API that smux is not using: **HOST_MANAGED IO backend with `receive_buffer` callback**. The `ghostty_surface_config_s` struct has a `backend` field that can be set to `GHOSTTY_SURFACE_IO_BACKEND_HOST_MANAGED` (value 1), along with a `receive_buffer` callback of type `void (*)(void*, const uint8_t*, size_t)`. In this mode, smux manages the PTY directly (via `smux_forkpty()` which already exists in `CPtyHelper`), reads raw bytes from the PTY master fd, and tees them to both: (1) `ghostty_surface_write_buffer()` for rendering and (2) a capture buffer for the ping-pong router. This completely eliminates viewport polling and gives access to the raw PTY byte stream.

For turn detection, the research shows that OSC 133 D (COMMAND_FINISHED) is unreliable for TUI agents because Claude Code and similar tools do not emit shell integration sequences from within their TUI interface. The proven pattern used by Claude Code Agent Teams and tmux-based orchestration tools is a multi-signal approach: silence timeout (primary), regex prompt detection on raw PTY output (secondary), and optional OSC 133 support for non-TUI commands. The silence timeout (2-4 seconds of no PTY output) is the most reliable universal signal.

The competitive landscape confirms this is the right architecture. tmux `pipe-pane` works by forking a child process connected to the pane's output stream via a socket pair -- essentially tapping the raw PTY output. iTerm2 coprocesses use the same principle: "a byte-for-byte copy of the input from the session's pty." Claude Code Agent Teams use `tmux capture-pane -p` and file-based mailbox polling. None of these tools attempt viewport screenshot diffing.

## Key Findings

**Stack:** Switch from EXEC mode to HOST_MANAGED mode; smux owns PTY via `smux_forkpty()`, tees raw bytes to ghostty renderer and capture buffer.

**Architecture:** Three-layer capture: PTY master fd reader thread -> ring buffer with tee -> ghostty renderer + output accumulator for the router.

**Critical pitfall:** OSC 133 will NOT fire for TUI agents. Silence timeout is the only universal turn-complete signal. Must handle TUI alternate screen mode transitions.

## Implications for Roadmap

Based on research, suggested phase structure:

1. **Phase 1: PTY Ownership** - Switch from EXEC to HOST_MANAGED ghostty mode
   - Addresses: PTY-CAP-01 (real-time capture), the core viewport-polling problem
   - Avoids: Infinite feedback loop from viewport read -> sendText -> viewport change
   - Rationale: This is the foundation everything else depends on

2. **Phase 2: Raw Stream Capture** - Implement tee reader thread with ring buffer
   - Addresses: PTY-CAP-01 continuation, clean ANSI stripping on raw stream
   - Avoids: TUI chrome contamination (the #1 problem with viewport approach)

3. **Phase 3: Turn Detection** - Multi-signal turn-complete detection on raw stream
   - Addresses: PTY-CAP-02 (turn detection)
   - Avoids: False positives from OSC 133 (TUI agents don't emit it)

4. **Phase 4: Relay Injection** - Clean text extraction and stdin injection
   - Addresses: PING-02 (relay), PING-03 (loop)
   - Avoids: Injecting ANSI codes, injecting too much/too little text

**Phase ordering rationale:**
- Phase 1 must come first because HOST_MANAGED mode changes the fundamental data flow
- Phase 2 depends on Phase 1's PTY ownership for raw stream access
- Phase 3 depends on Phase 2's raw stream for accurate turn detection
- Phase 4 depends on Phase 3 for knowing when to relay

**Research flags for phases:**
- Phase 1: Needs careful testing -- switching IO backend mode may affect surface lifecycle
- Phase 3: Needs prompt regex tuning per agent (Claude Code vs Codex vs others)
- Phase 4: Standard pattern, unlikely to need deeper research

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack (HOST_MANAGED API) | HIGH | Verified in ghostty.h C header, API exists in xcframework |
| Features (relay flow) | HIGH | tmux pipe-pane, iTerm2 coprocess confirm this pattern |
| Architecture (tee reader) | HIGH | Standard POSIX PTY pattern, proven in every terminal |
| Turn detection | MEDIUM | Silence timeout proven, but TUI prompt regex needs empirical tuning |
| OSC 133 for TUI agents | HIGH | Confirmed Claude Code does NOT emit OSC 133 (GitHub issues #22528, #32635 are feature requests) |

## Gaps to Address

- Exact behavior of `GHOSTTY_SURFACE_IO_BACKEND_HOST_MANAGED` with the current ghostty 1.3.1 xcframework -- needs empirical testing
- Thread safety of `ghostty_surface_write_buffer()` -- likely needs main-thread dispatch (Metal constraint)
- Claude Code's exact prompt pattern for regex detection needs empirical capture
- How alternate screen buffer (used by Claude Code's Ink/React TUI) affects raw stream parsing
- Whether `ghostty_surface_text()` (used for injection) works correctly in HOST_MANAGED mode
