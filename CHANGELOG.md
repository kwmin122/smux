# Changelog

All notable changes to smux are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.5.0] — 2026-03-23

### Added

- **Shell Integration (OSC 633)**: Zsh hook script auto-injected via ZDOTDIR. Tracks command boundaries, exit codes, and working directory in real-time.
- **Multi-tab terminals**: Create (⌘T), close (⌘W), rename, color-code, drag-reorder tabs in the sidebar. CWD-based auto-naming via shell integration.
- **Split panes**: Vertical (⌘D) and horizontal (⌘⇧D) recursive splits with drag-to-resize handles and pane zoom (⌘⇧Enter).
- **Terminal search (⌘F)**: Powered by `@xterm/addon-search`. Match count, regex toggle, case-sensitive toggle, Shift+Enter for previous match.
- **Clickable links**: URLs open in browser via `@xterm/addon-web-links`. File paths (`file:line:col`) detected for Rust, TypeScript, Python error formats.
- **Command decorations**: Left gutter with colored dots per command (green=success, red=error, yellow=running). Based on shell integration exit codes.
- **Sticky scroll**: Pinned header shows which command produced the visible output when scrolling.
- **WebGL rendering**: GPU-accelerated terminal via `@xterm/addon-webgl` with canvas fallback on context loss.
- **Unicode 11 support**: Correct CJK/Korean character widths via `@xterm/addon-unicode11`.
- **Config system**: `~/.smux/config.toml` with sections for general, appearance, and AI settings. Hot-reload on save.
- **Full settings view**: 5-category settings page (General, Appearance, Terminal, AI, Keybindings) replacing the old modal.
- **Keybinding presets**: Default, tmux, and vim presets with per-action custom overrides stored in localStorage.
- **AI ping-pong orchestration**: 3-phase terminal-to-terminal auto-loop (Ideation → Planning → Execution). Left PTY output captured and piped to right PTY. Auto-advance on APPROVED, re-iterate on REJECTED.
- **4-tier AI execution levels**: Disabled / Allowlist / Auto / Turbo safety model (inspired by Windsurf).
- **Failed command auto-analysis**: Non-zero exit code detected via shell integration triggers "Fix with AI" overlay.
- **Selection-to-AI (⌘L)**: Select terminal text and send to AI agent as context.
- **N-panel presets**: Dual, Code Review (3-panel), Full Pipeline (4-panel) configurations for AI workflows.
- **Git integration**: Branch name and changed file count in sidebar. Secret redaction utility function.
- **Launch configurations**: Saved workspace presets with multi-pane layouts and auto-commands. Shown on Welcome screen.
- **macOS notifications**: Phase transitions and errors trigger native Notification API alerts.
- **Style guide**: Design reference document at `docs/design/STYLE_GUIDE.md`.
- **Enhanced competitive plan**: 27-task roadmap based on 12-product competitive analysis.

### Changed

- **Layout**: Narrower sidebar (192px), reduced header (36px), minimal panel padding for maximum terminal space.
- **Welcome screen**: Added version badge, feature badges, launch configurations section.
- **Settings**: Upgraded from small modal to full-screen categorized view.
- **AI session UX**: Replaced task-input modal with inline goal input bar in terminal header. Two real PTY panels instead of chat UI.
- **Tab key**: Changed from bare Tab (broke shell tab-completion) to Ctrl+Tab for mode toggle.
- **Cmd+F**: No longer conflicts with fullscreen toggle (moved to Cmd+Shift+F).

### Fixed

- **Korean/CJK input**: Loaded Unicode11 addon, set `activeVersion='11'`, added CSS fix for IME textarea padding.
- **Home navigation**: Properly resets splitRoot, activeLeafId, and projectDir when returning to Welcome screen.
- **Shell integration guard**: Fixed inverted guard variable that prevented hooks from loading (`SMUX_SHELL_INTEGRATION` → `__SMUX_INTEGRATION_LOADED`).
- **lineHeight calculation**: Fixed operator precedence bug (was returning 1.4 instead of 18px).
- **Timer leak**: `waitForCompletion` now clears both interval and timeout on resolve.
- **Stale closure**: `onPtyOutput` callback stored in ref to avoid stale capture in PTY listener.
- **Race condition**: Temp file write and agent command now chained with `&&` instead of hardcoded 300ms delay.
- **Scroll performance**: Throttled with `requestAnimationFrame` instead of firing setState per scroll event.
- **Config read performance**: Deny-list cached in PtyManager, refreshed only on config save (was reading disk per keystroke).

### Security

- **Shell injection prevention**: AI prompts written to temp files and piped via stdin. No user content interpolated into shell commands.
- **Deny-list enforcement**: `write_pty` and `api_exec(pane.write)` check cached deny-list before writing to PTY.
- **Browser WebView isolation**: `create_pty`, `write_pty`, and `api_exec` restricted to main window via `window.label()` check.
- **Shell allowlist**: Strict exact-path matching only (removed `ends_with` bypass).
- **Dangerous defaults removed**: `--dangerously-skip-permissions` and `--full-auto` no longer hardcoded. Agents run in safe mode by default.
- **ZDOTDIR hardening**: 0700 permissions on `~/.smux/zdotdir/` directory.
- **Config file permissions**: 0700 on `~/.smux/`, 0600 on `config.toml`.
- **Temp file security**: Crypto-random names in `~/.smux/tmp/` (not `/tmp/`), 0600 permissions, cleanup on abort.
- **OSC 633 sanitization**: ESC and BEL characters stripped from command text before embedding in escape sequences.
- **Secret redaction**: 15 patterns (OpenAI, GitHub, AWS, Anthropic, Slack, npm, PyPI, HuggingFace, SSH keys, connection strings) applied to all terminal output.
- **File link validation**: Rejects absolute paths, path traversal (`..`), and unsafe characters.

### Removed

- `useAiPingPong.ts` — replaced by `usePingPongOrchestrator.ts`.
- `useSelectionToAi.ts` — never integrated, removed as dead code.
- Old settings modal — replaced by full `SettingsView` component.
- AI task prompt modal — replaced by inline goal input in terminal header.

### Accessibility

- Added `aria-label` to 17+ icon-only buttons across all components.
- TabBar: `role="tablist"`, `role="tab"`, `aria-selected` for proper screen reader support.
- SettingsView ToggleSwitch: `role="switch"`, `aria-checked`.
- SearchOverlay: `aria-label` on all toggle buttons, `aria-pressed` state.
- Design tokens: Replaced hardcoded Tailwind colors with semantic tokens (`secondary`, `error`, `tertiary`) for theme adaptability.
