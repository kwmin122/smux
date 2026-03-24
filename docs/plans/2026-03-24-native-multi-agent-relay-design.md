# smux Native Multi-Agent Relay Design

## Status

Proposed and approved for planning on 2026-03-24.

## Goal

Build a macOS-first terminal product that feels as fast and correct as cmux/tmux, fixes Korean IME/input quality at the root, and turns multi-agent coding into a real operator workflow instead of a brittle demo.

## Product Position

smux should stop thinking of itself as "two agents ping-ponging" and instead become a:

- native terminal workspace for real AI CLIs
- role-based multi-agent relay engine
- gated execution and verification system
- policy-aware operator console for teams

The user experience stays terminal-native. Users still run `claude`, `codex`, `gemini`, and future CLIs in real PTYs. smux adds orchestration, routing, state, approval gates, auditability, and recovery on top.

## Why This Direction

### Primary requirement

Korean IME and terminal quality are not optional. The current Tauri plus xterm.js stack can be patched around, but not trusted as the long-term base for a product whose first serious users will type Korean inside terminal panes all day.

### Architecture conclusion

The best fit is:

- app shell: native macOS shell built with Swift and AppKit/SwiftUI
- terminal renderer: libghostty-backed native terminal views
- orchestration and session core: existing Rust crates
- PTY, session persistence, routing, policy, consensus: existing Rust layer extended

This preserves the current Rust investment while replacing only the weakest layer: the webview terminal shell.

## Alternatives Compared

### A. Keep Tauri

Pros:
- highest UI code reuse
- Windows path stays open through WebView2

Cons:
- macOS still depends on WKWebView
- IME and input behavior remain bounded by web terminal quality
- long-term trust in terminal feel stays weak

Verdict: reject for the primary product line.

### B. Move to Electron

Pros:
- Chromium behavior is more predictable than WKWebView
- easier short-term migration than native Swift

Cons:
- heavier app
- weaker terminal feel than native libghostty
- solves packaging convenience better than it solves terminal quality

Verdict: reject.

### C. tmux-style TUI only

Pros:
- extremely fast
- terminal-native by definition
- great session durability

Cons:
- does not meet the desired GUI product shape
- weak browser, inspector, policy, and operator affordances

Verdict: keep as a capability source, not as the product shell.

### D. Tauri plus native terminal hybrid

Pros:
- partial reuse of current UI
- can embed native terminal view in theory

Cons:
- mixed view hierarchies add complexity in the most failure-prone layer
- still keeps product identity split between web shell and native terminal

Verdict: reject.

### E. Pure Swift shell plus libghostty

Pros:
- best macOS input quality
- best terminal rendering and interaction quality
- closest to cmux/Ghostty-class product feel
- clean separation between native shell and Rust orchestration core

Cons:
- highest short-term UI rewrite cost
- Windows shell must be built separately later

Verdict: choose this.

## Product Principles

1. Real terminals first. No fake chat console. Every agent runs in a real PTY pane.
2. Summary first, raw transcript second. Operators should see state and blockers before walls of text.
3. Gated automation. Automation is the default value, not the default risk.
4. Ownership before concurrency. Parallel workers only run with explicit work ownership.
5. Restore is mandatory. Sessions must survive app close, daemon restarts, and operator context switching.
6. Policy is a product feature. Team controls are part of the core design, not a late enterprise add-on.

## User Experience Model

### Main entry point

The user types one line:

`I want a blog automation tool.`

smux expands this into a staged relay:

1. ideation
2. planning
3. execution
4. hardening

The user does not need to invoke internal skills or think about prompt protocol. Those are internal orchestration details.

### Automation modes

Each session supports both:

- full auto: keep advancing until blocked by policy, ambiguity, or explicit failure
- gated: require user approval between stages or after configured checkpoints

The mode can be changed per session.

### Default screen model

The default workspace contains:

- left rail: workspaces, alerts, pinned sessions
- center: operator timeline and stage state
- right or bottom: live PTY panes for agents
- inspector drawer: raw transcript, findings, diffs, commands, files, tests

The user can always type directly into any agent terminal. Buttons and shortcuts are accelerators, not replacements.

## Multi-Agent Session Graph

smux should model sessions as a directed graph of agents and lanes rather than a fixed planner/verifier pair.

### Roles

- ideator: proposes options during idea exploration
- planner: writes the plan and decomposes work
- worker: implements an assigned slice
- verifier: reviews plans, diffs, tests, and regressions
- integrator: merges approved work into the canonical branch or worktree
- auditor: optional policy or compliance checker

### Supported topologies

- planner + verifier
- planner + verifier + verifier
- planner + frontend worker + backend worker + verifier
- planner + multiple workers + integrator + multiple verifiers

### Constraints

Free-form all-to-all messaging should not be allowed by default. Each stage should use routing policies so the system stays legible, efficient, and auditable.

Examples:

- ideation: `planner -> verifiers`
- planning: `planner -> verifiers -> planner`
- execution: `planner -> workers`
- worker completion: `worker -> verifiers -> planner`
- integration: `integrator <- approved workers`

## Consensus and Routing

### Verifier consensus

Configurable strategies:

- majority
- unanimous
- weighted
- leader-decides

### Worker ownership

Parallel execution is only enabled when ownership is explicit, for example:

- `frontend`
- `backend`
- `api`
- `infra`
- `docs`

That ownership maps to file globs, worktrees, and verification scope.

### Conflict rule

If two workers overlap on the same ownership lane, the system must force serialization or user approval. "Hope they do not collide" is not a valid design.

## Stage Machine

### Stage 1: Ideate

Purpose:
- explore options
- identify hidden constraints
- propose a recommendation

Exit criteria:
- accepted solution direction
- known constraints and success criteria

### Stage 2: Plan

Purpose:
- produce execution phases and task breakdown
- identify verification strategy and rollback points

Exit criteria:
- approved implementation plan
- ownership and test strategy defined

### Stage 3: Execute

Purpose:
- run assigned tasks through workers
- verify each completed slice before integration

Exit criteria:
- all tasks integrated
- no blocking verifier findings remain

### Stage 4: Harden

Purpose:
- run regression, policy, docs, packaging, and release checks

Exit criteria:
- quality bar met
- session ready for handoff, merge, or release

## Native Shell Architecture

### Layer split

#### Layer 1: Rust core

Keep and extend:

- `crates/smux-core`
- `crates/smux-daemon`
- `crates/smux-cli`

Responsibilities:

- agent adapters
- routing
- stage machine
- consensus
- policy engine
- session store
- audit log
- IPC contracts
- worktree orchestration

#### Layer 2: Native shell

Create a new native crate and macOS app shell.

Responsibilities:

- windowing
- workspace chrome
- native Ghostty terminal panes
- drag, split, tabs, notifications
- command palette
- state timeline and inspector

#### Layer 3: Agent runtime

Continue to treat agent CLIs as external processes in PTYs or headless adapters depending on provider support. The shell should not absorb provider logic.

## Enterprise Requirements

The product should be usable by an individual first, but designed for team controls from the start.

### Required

- session audit log
- command allow and deny policy
- secret redaction in stored transcripts
- role-based automation limits
- exportable evidence for plan approval and verifier findings
- persistent session metadata

### Later but planned now

- SSO and org settings
- team policy bundles
- shared workspace templates
- reviewer assignment
- retention controls

## Competitive Feature Set

### From cmux

- native Ghostty-level terminal quality
- workspace sidebar
- notification center
- socket control API

### From tmux

- durable session model
- detach and reattach
- activity and silence alerts
- scripting and control-mode mindset

### From Ghostty

- native keybinding system
- polished split and tab behavior
- AppleScript and native automation hooks

### From Warp and enterprise terminals

- launch configurations
- policy controls
- operator visibility into commands, failures, and automation state

## Windows Strategy

Windows remains a valid future target, but not through the same UI codebase.

Plan for this now by keeping:

- session graph logic in Rust
- IPC protocol UI-agnostic
- policy engine UI-agnostic
- shell-specific rendering behind an app shell interface

That lets smux ship:

- macOS native shell first
- Windows native shell later

without redoing the orchestration core.

## Proposed Milestones

### Milestone 1: Native terminal foundation

- native app shell created
- libghostty panes rendering
- PTY bridge working
- tabs, splits, search, restore baseline

### Milestone 2: Relay engine upgrade

- session graph replaces fixed planner/verifier pair
- multiple verifiers and worker lanes supported
- routing and consensus configurable

### Milestone 3: Operator workflow

- stage timeline
- summary-first mission control
- approvals, retry, pause, resume, escalate
- diff and findings inspector

### Milestone 4: Team controls

- policy engine
- audit exports
- session templates
- enterprise-grade reliability and recovery

## Risks

### Risk 1: Native shell rewrite cost

Mitigation:
- preserve Rust core
- build native shell beside the existing Tauri app until parity

### Risk 2: Multi-agent complexity explosion

Mitigation:
- constrain topology with role presets
- allow advanced custom graphs only behind explicit configuration

### Risk 3: Parallel worker merge pain

Mitigation:
- require ownership lanes and isolated worktrees
- add integrator gate before merge

### Risk 4: Operator overload

Mitigation:
- default to summaries and stage state
- keep raw transcripts collapsible

## Decision

smux should move from:

- Tauri-first web terminal shell
- fixed planner/verifier ping-pong

to:

- macOS-native Swift plus libghostty shell
- role-based multi-agent relay engine over real PTYs

This is the architecture most likely to produce the terminal quality, Korean input fidelity, and operator-grade workflow the product is aiming for.

## References

- cmux docs: https://www.cmux.dev/docs/getting-started
- Ghostty features: https://ghostty.org/docs/features
- Ghostty AppleScript: https://ghostty.org/docs/features/applescript
- Ghostty keybinding reference: https://ghostty.org/docs/config/keybind/reference
- tmux wiki: https://github.com/tmux/tmux/wiki
- tmux advanced use: https://github.com/tmux/tmux/wiki/Advanced-Use
- Tauri webview versions: https://v2.tauri.app/reference/webview-versions/
- Warp privacy and enterprise docs: https://docs.warp.dev/getting-started/privacy
