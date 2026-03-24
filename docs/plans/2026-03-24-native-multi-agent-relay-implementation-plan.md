# Native Multi-Agent Relay Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the current Tauri-first shell direction with a macOS-native libghostty shell while upgrading smux from a fixed planner/verifier loop into a stage-based multi-agent relay pipeline over real PTYs.

**Architecture:** Keep the Rust orchestration core as the durable source of truth. Add a native macOS project outside the Cargo workspace, extend the daemon and core to support stage pipelines, ownership lanes, and consensus, and keep the old Tauri app available until the native shell reaches parity.

**Tech Stack:** Rust workspace crates, Swift/AppKit or SwiftUI macOS shell, libghostty, PTY integration, Unix socket IPC, TOML config, xcodebuild, swift build, cargo test, cargo fmt, cargo clippy.

---

### Task 0: Prepare disk space and choose the initial toolchain path

**Files:**
- Modify: `docs/plans/2026-03-24-native-multi-agent-relay-design.md`
- Modify: `docs/plans/2026-03-24-native-multi-agent-relay-implementation-plan.md`
- Create: `docs/adr/002-native-toolchain-gate.md`

**Step 1: Record the preconditions**

Write down:
- minimum free disk target: 20 GB or more
- preferred product toolchain: full Xcode installed
- allowed PoC shortcut: pinned prebuilt `xcframework` if disk is constrained

**Step 2: Verify the machine state**

Run:
```bash
df -h .
xcode-select -p
swift --version
zig version
```

Expected:
- enough free disk for the chosen path
- clear visibility into whether Xcode, Swift, and Zig are installed

**Step 3: Decide the day-one toolchain**

Choose one:
- prebuilt `xcframework` PoC for fastest viability test
- full Xcode path for immediate product-grade setup

Pin the decision in the ADR with the exact Ghostty artifact source and version or commit.

**Step 4: Re-run the checklist**

Run:
```bash
df -h .
xcode-select -p
```

Expected: the selected path is feasible before implementation starts.

**Step 5: Commit**

Run:
```bash
git add docs/plans/2026-03-24-native-multi-agent-relay-design.md docs/plans/2026-03-24-native-multi-agent-relay-implementation-plan.md docs/adr/002-native-toolchain-gate.md
git commit -m "docs: add native toolchain gate"
```

### Task 1: Build a libghostty proof of concept and validate Korean IME

**Files:**
- Create: `macos/smux/Package.swift`
- Create: `macos/smux/Sources/SmuxApp/main.swift`
- Create: `macos/smux/Sources/SmuxApp/GhosttyHostView.swift`
- Create: `macos/smux/README.md`
- Create: `docs/plans/2026-03-24-libghostty-poc-report.md`

**Step 1: Write the failing integration target**

Create a tiny native shell target that links a pinned libghostty artifact and attempts to render one terminal surface.

**Step 2: Run the build to verify it fails before integration**

Run:
```bash
swift build --package-path macos/smux
```

Expected: FAIL because the project and dependency wiring do not exist yet.

**Step 3: Implement the minimal PoC**

Build a tiny macOS app that proves:
- libghostty view can initialize
- a shell can attach to a PTY
- Korean IME composition works inside the terminal surface

Pin the Ghostty dependency to an exact commit or exact binary artifact version.

**Step 4: Run the kill-or-go verification**

Run:
```bash
swift build --package-path macos/smux
```

Then manually verify:
- terminal surface renders
- shell prompt appears
- Korean IME composition works
- pasted Korean text renders correctly

Expected: PASS on all four checks. If Korean IME fails, stop the native-shell rollout and reassess before continuing.

**Step 5: Commit**

Run:
```bash
git add macos/smux docs/plans/2026-03-24-libghostty-poc-report.md
git commit -m "feat: prove libghostty native shell viability"
```

### Task 2: Add native-shell project scaffolding

**Files:**
- Create: `macos/smux/SmuxApp.xcodeproj/project.pbxproj`
- Create: `macos/smux/Sources/SmuxApp/AppDelegate.swift`
- Create: `macos/smux/Sources/SmuxApp/WorkspaceWindowController.swift`
- Modify: `macos/smux/README.md`

**Step 1: Write the failing build target**

Add a minimal native shell target that opens a window and exposes a shell configuration model used by the app.

**Step 2: Run build to verify it fails**

Run:
```bash
xcodebuild -project macos/smux/SmuxApp.xcodeproj -scheme SmuxApp -configuration Debug build
```

Expected: FAIL because the project scaffolding does not exist yet.

**Step 3: Create the baseline project**

Create the native project and compile-only baseline. Do not add the native shell to the Cargo workspace.

**Step 4: Run build to verify it passes**

Run:
```bash
xcodebuild -project macos/smux/SmuxApp.xcodeproj -scheme SmuxApp -configuration Debug build
```

Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add macos/smux
git commit -m "feat: scaffold native shell project"
```

### Task 3: Define shell-agnostic stage pipeline types in core

**Files:**
- Modify: `crates/smux-core/src/types.rs`
- Modify: `crates/smux-core/src/lib.rs`
- Create: `crates/smux-core/tests/session_pipeline.rs`

**Step 1: Write the failing test**

Add tests for:
- agent roles
- stage definitions
- pipeline stage slots
- ownership lanes
- session pipeline validation

**Step 2: Run test to verify it fails**

Run: `cargo test -p smux-core session_pipeline -- --nocapture`
Expected: FAIL because the new types and validators do not exist.

**Step 3: Implement minimal pipeline types**

Add:
- `AgentRole`
- `SessionStage`
- `OwnershipLane`
- `StageParticipants`
- `SessionPipeline`
- `PipelineValidationError`

Require at least one planner, explicit verifier slots where a stage is gated, and no duplicate agent identifiers.

**Step 4: Run test to verify it passes**

Run: `cargo test -p smux-core session_pipeline -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add crates/smux-core/src/types.rs crates/smux-core/src/lib.rs crates/smux-core/tests/session_pipeline.rs
git commit -m "feat: add multi-agent session pipeline types"
```

### Task 4: Extend orchestrator from fixed pair to stage-based relay engine

**Files:**
- Modify: `crates/smux-core/src/orchestrator.rs`
- Modify: `crates/smux-core/src/consensus.rs`
- Modify: `crates/smux-core/src/context.rs`
- Modify: `crates/smux-core/src/session_store.rs`
- Modify: `crates/smux-core/tests/orchestrator_fake.rs`
- Create: `crates/smux-core/tests/pipeline_routing.rs`

**Step 1: Write the failing tests**

Cover:
- planner plus two verifier routing
- planner to frontend/backend worker dispatch
- verifier majority consensus
- stage advancement rules

**Step 2: Run tests to verify they fail**

Run: `cargo test -p smux-core pipeline_routing orchestrator_fake -- --nocapture`
Expected: FAIL because the orchestrator still assumes one planner and one verifier.

**Step 3: Implement the relay engine**

Replace the singular verifier assumption with:
- pipeline-driven participant lookup
- stage-aware recipients
- ownership-aware worker dispatch
- verifier consensus per checkpoint

Keep single-verifier behavior backward compatible by expressing it as a trivial pipeline preset.

**Step 4: Run the updated tests**

Run: `cargo test -p smux-core pipeline_routing orchestrator_fake -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add crates/smux-core/src/orchestrator.rs crates/smux-core/src/consensus.rs crates/smux-core/src/context.rs crates/smux-core/src/session_store.rs crates/smux-core/tests/orchestrator_fake.rs crates/smux-core/tests/pipeline_routing.rs
git commit -m "feat: upgrade orchestrator to stage-based relay engine"
```

### Task 5: Add pipeline, routing, and consensus fields to IPC and daemon

**Files:**
- Modify: `crates/smux-core/src/ipc.rs`
- Modify: `crates/smux-daemon/src/main.rs`
- Modify: `crates/smux-daemon/tests/daemon_smoke.rs`
- Create: `crates/smux-core/tests/ipc_session_pipeline.rs`

**Step 1: Write the failing tests**

Add serialization tests for:
- multi-agent session pipeline payloads
- routing presets
- per-stage approval mode
- consensus settings

**Step 2: Run tests to verify they fail**

Run: `cargo test -p smux-core ipc_session_pipeline && cargo test -p smux-daemon daemon_smoke -- --nocapture`
Expected: FAIL because IPC payloads do not yet carry pipeline metadata.

**Step 3: Implement the IPC migration**

Add optional pipeline-aware fields while preserving current planner/verifier flags:
- `agents`
- `stages`
- `stage_participants`
- `approval_mode`
- `consensus`

Teach the daemon to build a default pipeline from old requests.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p smux-core ipc_session_pipeline && cargo test -p smux-daemon daemon_smoke -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add crates/smux-core/src/ipc.rs crates/smux-daemon/src/main.rs crates/smux-daemon/tests/daemon_smoke.rs crates/smux-core/tests/ipc_session_pipeline.rs
git commit -m "feat: add session pipeline support to ipc and daemon"
```

### Task 6: Add ownership lanes and isolated worktree execution

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

### Task 7: Add policy, audit, and transcript redaction primitives

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

### Task 8: Build the macOS native shell process and IPC client

**Files:**
- Modify: `macos/smux/Package.swift`
- Modify: `macos/smux/Sources/SmuxApp/main.swift`
- Create: `macos/smux/Sources/SmuxApp/IpcClient.swift`
- Create: `macos/smux/Sources/SmuxApp/SessionModel.swift`
- Create: `macos/smux/Tests/SmuxAppTests/IpcClientTests.swift`

**Step 1: Write the failing tests**

Add tests for:
- daemon connection bootstrap
- session list parsing
- session event subscription

**Step 2: Run tests to verify they fail**

Run:
```bash
swift test --package-path macos/smux
```

Expected: FAIL because the IPC client is missing.

**Step 3: Implement the client layer**

Add a shell-side adapter that:
- talks to `smux-daemon` over Unix socket
- subscribes to session updates
- exposes a session summary model for the native UI

**Step 4: Run tests to verify they pass**

Run:
```bash
swift test --package-path macos/smux
```

Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add macos/smux
git commit -m "feat: add native shell daemon client"
```

### Task 9: Integrate libghostty-backed PTY panes and workspace state

**Files:**
- Create: `macos/smux/Sources/SmuxApp/TerminalBridge.swift`
- Create: `macos/smux/Sources/SmuxApp/WorkspaceState.swift`
- Create: `macos/smux/Sources/SmuxApp/LayoutState.swift`
- Create: `macos/smux/Tests/SmuxAppTests/WorkspaceStateTests.swift`
- Modify: `macos/smux/Sources/SmuxApp/main.swift`

**Step 1: Write the failing tests**

Add tests for:
- pane split state
- tab restore model
- terminal session attachment bookkeeping

**Step 2: Run tests to verify they fail**

Run:
```bash
swift test --package-path macos/smux
```

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

Run:
```bash
swift test --package-path macos/smux
```

Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add macos/smux
git commit -m "feat: add native terminal workspace state"
```

### Task 10: Build mission control, approvals, and summary-first operator views

**Files:**
- Create: `macos/smux/Sources/SmuxApp/MissionControlState.swift`
- Create: `macos/smux/Sources/SmuxApp/StageTimelineState.swift`
- Create: `macos/smux/Sources/SmuxApp/InspectorState.swift`
- Create: `macos/smux/Tests/SmuxAppTests/MissionControlTests.swift`
- Modify: `macos/smux/Sources/SmuxApp/SessionModel.swift`
- Modify: `macos/smux/Sources/SmuxApp/main.swift`

**Step 1: Write the failing tests**

Cover:
- full-auto vs gated session mode rendering state
- approval prompt state transitions
- unread finding counts
- "raw transcript collapsed by default" behavior

**Step 2: Run tests to verify they fail**

Run:
```bash
swift test --package-path macos/smux
```

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

Run:
```bash
swift test --package-path macos/smux
```

Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add macos/smux
git commit -m "feat: add mission control and approval state"
```

### Task 11: Replace Tauri-first docs, add native-shell launch path, and verify the end-to-end baseline

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
swift test --package-path macos/smux
```

Expected: PASS on all targeted modules before the docs claim readiness.

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
swift test --package-path macos/smux
```

Expected: PASS.

**Step 5: Commit**

Run:
```bash
git add README.md CHANGELOG.md docs/adr/002-native-shell-direction.md docs/plans/2026-03-24-native-multi-agent-relay-design.md docs/plans/2026-03-24-native-multi-agent-relay-implementation-plan.md
git commit -m "docs: revise native multi-agent relay plan"
```

## Execution Notes

- Use `@test-driven-development` for each implementation task before production code changes.
- Use `@verification-before-completion` before claiming a task is done.
- Use `@verification-gate` after each major milestone and before merging worker outputs.
- Keep the current Tauri app buildable until the native shell reaches feature parity for the core mission flow.
