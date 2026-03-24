# Native Multi-Agent Relay Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the current Tauri-first shell direction with a macOS-native libghostty shell while upgrading smux from a fixed planner/verifier loop into a role-based multi-agent relay engine over real PTYs.

**Architecture:** Keep the Rust orchestration core as the durable source of truth. Add a new native shell crate for macOS, extend the daemon and core to support session graphs, routing, ownership lanes, and consensus, and keep the old Tauri app available until the native shell reaches parity.

**Tech Stack:** Rust workspace crates, Swift/AppKit or SwiftUI macOS shell, libghostty, PTY integration, Unix socket IPC, TOML config, cargo test, cargo fmt, cargo clippy.

---

### Task 1: Add native-shell workspace scaffolding

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/smux-native/Cargo.toml`
- Create: `crates/smux-native/build.rs`
- Create: `crates/smux-native/src/lib.rs`
- Create: `crates/smux-native/src/bin/smux-native.rs`
- Create: `crates/smux-native/README.md`

**Step 1: Write the failing workspace test**

Add a minimal crate smoke test in `crates/smux-native/src/lib.rs` that compiles and exports a `NativeShellConfig` type.

**Step 2: Run test to verify the crate is missing**

Run: `cargo test -p smux-native`
Expected: FAIL because the crate does not exist yet.

**Step 3: Add the crate to the workspace**

Update `Cargo.toml` to include `crates/smux-native` in `members`, then create the new crate files with a compile-only baseline.

**Step 4: Run test to verify the crate compiles**

Run: `cargo test -p smux-native`
Expected: PASS with one smoke test.

**Step 5: Commit**

Run:
```bash
git add Cargo.toml crates/smux-native
git commit -m "feat: scaffold native shell crate"
```

### Task 2: Define shell-agnostic session graph types in core

**Files:**
- Modify: `crates/smux-core/src/types.rs`
- Modify: `crates/smux-core/src/lib.rs`
- Create: `crates/smux-core/tests/session_graph.rs`

**Step 1: Write the failing test**

Add tests for:
- agent roles
- stage definitions
- relay edges
- ownership lanes
- session graph validation

**Step 2: Run test to verify it fails**

Run: `cargo test -p smux-core session_graph -- --nocapture`
Expected: FAIL because the new types and validators do not exist.

**Step 3: Implement minimal graph types**

Add:
- `AgentRole`
- `SessionStage`
- `OwnershipLane`
- `RelayEdge`
- `SessionGraph`
- `GraphValidationError`

Require at least one planner, at least one verifier in gated stages, and no duplicate agent identifiers.

**Step 4: Run test to verify it passes**

Run: `cargo test -p smux-core session_graph -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add crates/smux-core/src/types.rs crates/smux-core/src/lib.rs crates/smux-core/tests/session_graph.rs
git commit -m "feat: add multi-agent session graph types"
```

### Task 3: Extend orchestrator from fixed pair to role-based relay engine

**Files:**
- Modify: `crates/smux-core/src/orchestrator.rs`
- Modify: `crates/smux-core/src/consensus.rs`
- Modify: `crates/smux-core/src/context.rs`
- Modify: `crates/smux-core/src/session_store.rs`
- Modify: `crates/smux-core/tests/orchestrator_fake.rs`
- Create: `crates/smux-core/tests/relay_routing.rs`

**Step 1: Write the failing tests**

Cover:
- planner plus two verifier routing
- planner to frontend/backend worker dispatch
- verifier majority consensus
- stage advancement rules

**Step 2: Run tests to verify they fail**

Run: `cargo test -p smux-core relay_routing orchestrator_fake -- --nocapture`
Expected: FAIL because the orchestrator still assumes one planner and one verifier.

**Step 3: Implement the relay engine**

Replace the singular verifier assumption with:
- graph-driven participant lookup
- stage-aware recipients
- ownership-aware worker dispatch
- verifier consensus per checkpoint

Keep single-verifier behavior backward compatible by expressing it as a trivial graph preset.

**Step 4: Run the updated tests**

Run: `cargo test -p smux-core relay_routing orchestrator_fake -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add crates/smux-core/src/orchestrator.rs crates/smux-core/src/consensus.rs crates/smux-core/src/context.rs crates/smux-core/src/session_store.rs crates/smux-core/tests/orchestrator_fake.rs crates/smux-core/tests/relay_routing.rs
git commit -m "feat: upgrade orchestrator to role-based relay engine"
```

### Task 4: Add graph, routing, and consensus fields to IPC and daemon

**Files:**
- Modify: `crates/smux-core/src/ipc.rs`
- Modify: `crates/smux-daemon/src/main.rs`
- Modify: `crates/smux-daemon/tests/daemon_smoke.rs`
- Create: `crates/smux-core/tests/ipc_session_graph.rs`

**Step 1: Write the failing tests**

Add serialization tests for:
- multi-agent session graph payloads
- routing presets
- per-stage approval mode
- consensus settings

**Step 2: Run tests to verify they fail**

Run: `cargo test -p smux-core ipc_session_graph && cargo test -p smux-daemon daemon_smoke -- --nocapture`
Expected: FAIL because IPC payloads do not yet carry graph metadata.

**Step 3: Implement the IPC migration**

Add optional graph-aware fields while preserving current planner/verifier flags:
- `agents`
- `edges`
- `stages`
- `approval_mode`
- `consensus`

Teach the daemon to build a default graph from old requests.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p smux-core ipc_session_graph && cargo test -p smux-daemon daemon_smoke -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add crates/smux-core/src/ipc.rs crates/smux-daemon/src/main.rs crates/smux-daemon/tests/daemon_smoke.rs crates/smux-core/tests/ipc_session_graph.rs
git commit -m "feat: add session graph support to ipc and daemon"
```

### Task 5: Add ownership lanes and isolated worktree execution

**Files:**
- Modify: `crates/smux-core/src/git_worktree.rs`
- Modify: `crates/smux-core/src/config.rs`
- Modify: `crates/smux-core/src/safety.rs`
- Create: `crates/smux-core/tests/ownership_lanes.rs`
- Create: `crates/smux-core/tests/worktree_assignment.rs`

**Step 1: Write the failing tests**

Cover:
- lane to file-glob mapping
- frontend and backend lane assignment
- collision detection
- worktree naming and reuse

**Step 2: Run tests to verify they fail**

Run: `cargo test -p smux-core ownership_lanes worktree_assignment -- --nocapture`
Expected: FAIL because ownership-lane behavior does not exist.

**Step 3: Implement minimal ownership support**

Add:
- lane definitions in config
- collision checks
- worker to lane assignment
- optional dedicated worktree per worker

**Step 4: Run tests to verify they pass**

Run: `cargo test -p smux-core ownership_lanes worktree_assignment -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add crates/smux-core/src/git_worktree.rs crates/smux-core/src/config.rs crates/smux-core/src/safety.rs crates/smux-core/tests/ownership_lanes.rs crates/smux-core/tests/worktree_assignment.rs
git commit -m "feat: add ownership lanes and worker worktree assignment"
```

### Task 6: Add policy, audit, and transcript redaction primitives

**Files:**
- Modify: `crates/smux-core/src/config.rs`
- Modify: `crates/smux-core/src/safety.rs`
- Create: `crates/smux-core/src/audit.rs`
- Create: `crates/smux-core/src/redaction.rs`
- Modify: `crates/smux-core/src/lib.rs`
- Create: `crates/smux-core/tests/audit.rs`
- Create: `crates/smux-core/tests/redaction.rs`

**Step 1: Write the failing tests**

Add tests for:
- command allow and deny policy
- redaction of tokens and secrets
- stage approval event logging
- verifier finding audit records

**Step 2: Run tests to verify they fail**

Run: `cargo test -p smux-core audit redaction -- --nocapture`
Expected: FAIL because the modules do not exist.

**Step 3: Implement the primitives**

Create:
- `AuditRecord`
- `AuditSink`
- `RedactionRule`
- `redact_transcript()`

Log stage transitions, approvals, retries, routed messages, and verifier outcomes.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p smux-core audit redaction -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add crates/smux-core/src/config.rs crates/smux-core/src/safety.rs crates/smux-core/src/audit.rs crates/smux-core/src/redaction.rs crates/smux-core/src/lib.rs crates/smux-core/tests/audit.rs crates/smux-core/tests/redaction.rs
git commit -m "feat: add policy audit and transcript redaction"
```

### Task 7: Build the macOS native shell process and IPC client

**Files:**
- Modify: `crates/smux-native/Cargo.toml`
- Modify: `crates/smux-native/build.rs`
- Modify: `crates/smux-native/src/lib.rs`
- Modify: `crates/smux-native/src/bin/smux-native.rs`
- Create: `crates/smux-native/src/ipc_client.rs`
- Create: `crates/smux-native/src/session_model.rs`
- Create: `crates/smux-native/tests/ipc_client.rs`

**Step 1: Write the failing tests**

Add tests for:
- daemon connection bootstrap
- session list parsing
- session event subscription

**Step 2: Run tests to verify they fail**

Run: `cargo test -p smux-native ipc_client -- --nocapture`
Expected: FAIL because the IPC client is missing.

**Step 3: Implement the client layer**

Add a shell-side adapter that:
- talks to `smux-daemon` over Unix socket
- subscribes to session updates
- exposes a session summary model for the native UI

**Step 4: Run tests to verify they pass**

Run: `cargo test -p smux-native ipc_client -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add crates/smux-native/Cargo.toml crates/smux-native/build.rs crates/smux-native/src/lib.rs crates/smux-native/src/bin/smux-native.rs crates/smux-native/src/ipc_client.rs crates/smux-native/src/session_model.rs crates/smux-native/tests/ipc_client.rs
git commit -m "feat: add native shell daemon client"
```

### Task 8: Integrate libghostty-backed PTY panes and workspace state

**Files:**
- Create: `crates/smux-native/src/terminal_bridge.rs`
- Create: `crates/smux-native/src/workspace_state.rs`
- Create: `crates/smux-native/src/layout.rs`
- Create: `crates/smux-native/tests/workspace_state.rs`
- Modify: `crates/smux-native/src/bin/smux-native.rs`

**Step 1: Write the failing tests**

Add tests for:
- pane split state
- tab restore model
- terminal session attachment bookkeeping

**Step 2: Run tests to verify they fail**

Run: `cargo test -p smux-native workspace_state -- --nocapture`
Expected: FAIL because the workspace state modules are missing.

**Step 3: Implement the shell state layer**

Create models for:
- tabs
- splits
- pane focus
- terminal attachment handles
- session restore metadata

Wire those models into the native entrypoint even if the first libghostty rendering pass is minimal.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p smux-native workspace_state -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add crates/smux-native/src/terminal_bridge.rs crates/smux-native/src/workspace_state.rs crates/smux-native/src/layout.rs crates/smux-native/tests/workspace_state.rs crates/smux-native/src/bin/smux-native.rs
git commit -m "feat: add native terminal workspace state"
```

### Task 9: Build mission control, approvals, and summary-first operator views

**Files:**
- Create: `crates/smux-native/src/mission_control.rs`
- Create: `crates/smux-native/src/stage_timeline.rs`
- Create: `crates/smux-native/src/inspector.rs`
- Create: `crates/smux-native/tests/mission_control.rs`
- Modify: `crates/smux-native/src/session_model.rs`
- Modify: `crates/smux-native/src/bin/smux-native.rs`

**Step 1: Write the failing tests**

Cover:
- full-auto vs gated session mode rendering state
- approval prompt state transitions
- unread finding counts
- "raw transcript collapsed by default" behavior

**Step 2: Run tests to verify they fail**

Run: `cargo test -p smux-native mission_control -- --nocapture`
Expected: FAIL because the operator UI state models do not exist.

**Step 3: Implement the mission-control state**

Add models and actions for:
- pause or resume
- approve and continue
- retry stage
- escalate to user
- open raw transcript

Treat the PTY pane as primary, with mission control acting as a control overlay.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p smux-native mission_control -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add crates/smux-native/src/mission_control.rs crates/smux-native/src/stage_timeline.rs crates/smux-native/src/inspector.rs crates/smux-native/tests/mission_control.rs crates/smux-native/src/session_model.rs crates/smux-native/src/bin/smux-native.rs
git commit -m "feat: add mission control and approval state"
```

### Task 10: Replace Tauri-first docs, add native-shell launch path, and verify the end-to-end baseline

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Create: `docs/adr/002-native-shell-direction.md`
- Modify: `docs/plans/2026-03-24-native-multi-agent-relay-design.md`
- Modify: `docs/plans/2026-03-24-native-multi-agent-relay-implementation-plan.md`

**Step 1: Write the failing verification checklist**

Create a checklist in `docs/adr/002-native-shell-direction.md` for:
- native shell launches
- single-verifier backward compatibility
- planner plus two verifier flow
- frontend plus backend worker split
- session pause and restore

**Step 2: Run verification commands before doc updates**

Run:
```bash
cargo test -p smux-core
cargo test -p smux-daemon
cargo test -p smux-native
```

Expected: PASS on all targeted crates before the docs claim readiness.

**Step 3: Update the top-level docs**

Document:
- native shell direction
- migration path away from Tauri-first UI
- supported topologies
- enterprise controls

**Step 4: Run final validation**

Run:
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add README.md CHANGELOG.md docs/adr/002-native-shell-direction.md docs/plans/2026-03-24-native-multi-agent-relay-design.md docs/plans/2026-03-24-native-multi-agent-relay-implementation-plan.md
git commit -m "docs: define native multi-agent relay direction"
```

## Execution Notes

- Use `@test-driven-development` for each implementation task before production code changes.
- Use `@verification-before-completion` before claiming a task is done.
- Use `@verification-gate` after each major milestone and before merging worker outputs.
- Keep the current Tauri app buildable until the native shell reaches feature parity for the core mission flow.
