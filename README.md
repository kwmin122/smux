<div align="center">

# smux

**AI agents that verify each other's work — automatically.**

[![CI](https://github.com/kwmin122/smux/actions/workflows/ci.yml/badge.svg)](https://github.com/kwmin122/smux/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

</div>

---

smux orchestrates two AI coding agents in an adversarial ping-pong loop:
one plans and codes, the other independently verifies. No more "it looks good to me" —
the verifier catches workarounds, weak tests, and regressions before they ship.

## Why smux?

Existing tools (cmux, Claude Squad, AMUX) run agents **in parallel**.
smux makes them **argue with each other** until the code is actually correct.

```
Planner (Claude) → writes code → Verifier (Codex) → "That's a workaround, not a fix"
                  ← revises    ←                    → "Now it's a root fix. APPROVED."
```

## Features

### Terminal Basics
- **Multi-tab terminals** — Create, rename, color-code, reorder tabs
- **Split panes (⌘D / ⌘⇧D)** — Recursive H/V splits with resize handles and pane zoom
- **Terminal search (⌘F)** — Regex, case-sensitive, match count
- **Clickable links** — URLs open in browser, `file:line:col` patterns detected
- **Shell Integration (OSC 633)** — Command boundaries, exit codes, CWD tracking
- **Command decorations** — Exit code gutter dots (green/red/yellow)
- **Sticky scroll** — Pinned command header when scrolling through output
- **WebGL rendering** — GPU-accelerated with canvas fallback
- **CJK/Korean support** — Unicode 11 character widths + IME composition

### AI Ping-Pong
- **3-phase orchestration** — Ideation → Planning → Execution, auto-advance on APPROVED
- **Terminal-to-terminal** — Both panels are real PTYs; user can type in either at any time
- **Auto ping-pong** — Left output captured → piped to right → verdict detected → iterate
- **4-tier safety** — Disabled / Allowlist / Auto / Turbo execution levels
- **Failed command analysis** — Non-zero exit → "Fix with AI" overlay
- **Selection-to-AI (⌘L)** — Select text, send to AI as context
- **Cross-verification consensus** — Multiple verifiers, voting strategies

### Configuration & UX
- **Config file** — `~/.smux/config.toml` with all settings
- **Full settings view** — 5 categories: General, Appearance, Terminal, AI, Keybindings
- **Keybinding presets** — Default / tmux / vim, custom overrides
- **Launch configurations** — Saved workspace presets with auto-commands
- **Secret redaction** — API keys, tokens, credentials auto-masked in output
- **Git integration** — Branch + changed files in sidebar
- **macOS notifications** — Phase transitions, errors, completion

### Security
- **Shell injection prevention** — Prompts via temp files + stdin pipe
- **Deny-list enforcement** — Dangerous commands blocked at PTY write level
- **API access control** — Socket API restricted to main window
- **Path validation** — Shell allowlist, CWD existence check, link path validation
- **ZDOTDIR hardening** — 0700 permissions on shell integration files

## Quick Start

```bash
# Install (macOS)
brew tap kwmin122/tap && brew install smux

# Or build from source
cargo install --path crates/smux-cli

# Start a session
smux start --planner claude --verifier codex --task "fix the rate limit bug"

# Manage sessions
smux list              # active sessions
smux attach <id>       # reconnect
smux detach            # disconnect (session keeps running)
smux rewind <id> 2     # go back to round 2
smux recover           # find orphaned sessions
```

## How It Works

```
Phase 1: Plan
  Planner creates plan → Verifier reviews → ping-pong until APPROVED

Phase 2: Execute (per task)
  Planner implements → Verifier checks (root fix? tests? regression?)
  → REJECTED? → feedback → Planner revises → repeat
  → APPROVED → next task
```

Each round:
1. Planner output → context passer (token budget, prior round summaries)
2. → Verifier → stop detection (JSON verdict → keyword → NeedsInfo)
3. → APPROVED? done. REJECTED? → feedback to planner → loop

## Configuration

```bash
smux init  # creates ~/.smux/config.toml
```

```toml
[agents.planner]
default = "claude"

[agents.verifier]
default = "codex"

[defaults]
max_rounds = 5
```

See [design doc](docs/superpowers/specs/2026-03-21-smux-design.md#configuration) for all options.

## Architecture

```
smux-cli ─── Unix socket ──→ smux-daemon
                                 │
                    ┌────────────┴────────────┐
                    │    Orchestrator Core     │
                    │  ping-pong + stop detect │
                    │  + context + rewind      │
                    └──────┬──────────┬────────┘
                           │          │
                    Claude CLI   Codex CLI
                    (headless)   (headless)
```

## Building

```bash
cargo build --workspace
cargo test --workspace
```

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

Inspired by research on multi-agent debate (Du et al. 2023), adversarial verification (Irving 2018), and the growing ecosystem of AI coding tools.
