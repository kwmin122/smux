# Cross-Verify Design Spec

> N-way multi-model cross-verification for smux v0.3

## Overview

Extend smux's Planner-Verifier architecture from 2 agents to N agents, enabling parallel cross-verification across Claude, Codex, and Gemini models with configurable consensus strategies.

## Architecture

```
                    ┌─────────────┐
                    │   Planner   │  (any adapter)
                    └──────┬──────┘
                           │ output
              ┌────────────┼────────────┐
              ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ Verifier │ │ Verifier │ │ Verifier │
        │ (Claude) │ │ (Codex)  │ │ (Gemini) │
        └────┬─────┘ └────┬─────┘ └────┬─────┘
             │             │             │
             ▼             ▼             ▼
        ┌─────────────────────────────────────┐
        │         Consensus Engine            │
        └──────────────┬──────────────────────┘
                       ▼
                 Final Verdict
```

## Existing Types Reference

The codebase uses these types (in `types.rs`):
- `VerifyResult` enum: `Approved { reason, confidence }`, `Rejected { category, reason, confidence }`, `NeedsInfo { question }`
- `SessionMeta`: `planner: String`, `verifier: String` (singular)
- `RoundSnapshot`: `round: u32`, `commit_sha: String`, `verdict: VerifyResult`, `files_changed: Vec<String>`

## Components

### 1. GeminiHeadlessAdapter (`crates/smux-core/src/adapter/gemini.rs`)

Same pattern as Claude/Codex adapters:
- Spawns `npx @google/gemini-cli` per turn (consistent with Claude/Codex npx pattern)
- For globally installed: falls back to `gemini` binary
- Streams stdout line-by-line as `Chunk` events
- Parses verdict JSON from output
- Implements full `AgentAdapter` trait
- Headless flags: `--non-interactive` or equivalent (verify actual flags at implementation time)

Discovery: check `npx @google/gemini-cli --version` at startup. If not available, mark adapter as unavailable but don't fail.

Must also: add `"gemini"` arm to `create_adapter()` and `create_adapter_with_safety()` factory functions in `adapter/mod.rs`, and add `pub mod gemini;` to module declarations.

### 2. Orchestrator Refactor (`crates/smux-core/src/orchestrator.rs`)

**Structural change** (not just extension):

Current:
```rust
pub struct Orchestrator {
    planner: Box<dyn AgentAdapter>,
    verifier: Box<dyn AgentAdapter>,    // SINGULAR
    ...
}
```

New:
```rust
pub struct Orchestrator {
    planner: Box<dyn AgentAdapter>,
    verifiers: Vec<Box<dyn AgentAdapter>>,  // 1..=3
    consensus_engine: ConsensusEngine,
    ...
}

impl Orchestrator {
    pub fn new(
        planner: Box<dyn AgentAdapter>,
        verifiers: Vec<Box<dyn AgentAdapter>>,   // at least 1
        consensus: ConsensusStrategy,
        // ... existing params
    ) -> Self { ... }
}
```

Round flow:
1. Planner produces output (unchanged)
2. All verifiers receive planner output **in parallel** (`tokio::join!`)
3. Collect `Vec<VerifierVerdict>` from all verifiers
4. Pass to ConsensusEngine for final `VerifyResult`
5. If rejected, build combined feedback from all rejecting verifiers

**Backward compatibility:** `verifiers: vec![single_adapter]` with `ConsensusStrategy::Majority` behaves identically to current PingPong — one verifier's result passes through consensus unchanged.

**Daemon impact:** `spawn_session()` in `smux-daemon/src/main.rs` must create `Vec<Box<dyn AgentAdapter>>` from the verifier list. Currently creates a single adapter at line ~450.

### 3. ConsensusEngine (`crates/smux-core/src/consensus.rs`)

```rust
pub enum ConsensusStrategy {
    Majority,        // >50% approved → approved (default)
    Weighted,        // confidence-weighted average > 0.5 → approved
    Unanimous,       // all must approve
    LeaderDelegate,  // leader model (first verifier) decides after seeing others' verdicts
}

pub struct VerifierVerdict {
    pub adapter_name: String,
    pub result: VerifyResult,     // uses EXISTING VerifyResult type
    pub duration_ms: u64,
}

pub struct ConsensusResult {
    pub final_result: VerifyResult,           // uses EXISTING VerifyResult type
    pub strategy_used: ConsensusStrategy,
    pub individual_verdicts: Vec<VerifierVerdict>,
    pub agreement_ratio: f64,                 // e.g. 0.67 for 2/3
}

impl ConsensusEngine {
    pub fn new(strategy: ConsensusStrategy) -> Self;
    pub fn decide(&self, verdicts: &[VerifierVerdict]) -> ConsensusResult;
}
```

Note: `VerifierVerdict.result` uses the existing `VerifyResult` enum which already contains `confidence: f64` inside `Approved` and `Rejected` variants. No new verdict type needed.

### 4. Type Migration (`crates/smux-core/src/types.rs`)

Evolve singular verifier fields to support multi-verifier:

```rust
pub struct SessionMeta {
    pub planner: String,
    pub verifiers: Vec<String>,      // was: verifier: String
    pub consensus_strategy: ConsensusStrategy,
    // ... rest unchanged
}
```

`RoundSnapshot` gains optional cross-verify data:
```rust
pub struct RoundSnapshot {
    pub round: u32,
    pub commit_sha: String,
    pub verdict: VerifyResult,            // final consensus result
    pub cross_verify: Option<ConsensusResult>,  // None for single-verifier
    pub files_changed: Vec<String>,
}
```

### 5. IPC Protocol Extension (`crates/smux-core/src/ipc.rs`)

**Migration strategy:** Keep `verifier` field, add optional `verifiers` field. Daemon interprets:
- If `verifiers` present → use multi-verifier
- Else if `verifier` present → wrap as `vec![verifier]` (backward compat)

```rust
// Client → Daemon
StartSession {
    task: String,
    planner: String,
    verifier: String,                    // KEPT for backward compat
    verifiers: Option<Vec<String>>,      // NEW: overrides verifier if present
    consensus: Option<String>,           // NEW: "majority"|"weighted"|"unanimous"|"leader"
    // ... existing fields
}

// Daemon → Client (NEW event)
CrossVerifyResult {
    round: u32,
    individual: Vec<VerifierVerdictInfo>,  // serializable version
    final_result: String,                  // "approved" | "rejected"
    strategy: String,
    agreement_ratio: f64,
}
```

Backward compat: v0.2 CLI sends `verifier` only → daemon wraps as single verifier. `CrossVerifyResult` event only emitted for multi-verifier sessions. Single-verifier sessions emit `RoundComplete` as before.

### 6. Config File Extension (`~/.smux/config.toml`)

```toml
[agents]
planner = "claude"

# Single verifier (backward compat)
verifier = "claude"

# Multi-verifier (overrides verifier if present)
# verifiers = ["claude", "codex", "gemini"]

[defaults]
consensus = "majority"    # majority | weighted | unanimous | leader

[safety.gemini]
# Gemini-specific safety config (TBD based on gemini-cli flags)
```

### 7. CLI Extension (`crates/smux-cli/src/main.rs`)

```
smux start "task" --planner claude --verifiers claude,codex,gemini --consensus majority
```

- `--verifiers` accepts comma-separated list (default from config, fallback: `claude`)
- `--consensus` accepts strategy name (default from config, fallback: `majority`)
- `--verifier` (singular) still works for backward compat

### 8. Tauri UI Cross-Verify Panel

In Control Mode, add Cross-Verify section:

```
┌─ Cross-Verify ─────────────────────────────┐
│                                             │
│  Claude    ✓ APPROVED  confidence: 0.92     │
│  Codex     ✓ APPROVED  confidence: 0.87     │
│  Gemini    ✗ REJECTED  confidence: 0.71     │
│                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  Final: APPROVED (majority 2/3)             │
│  Agreement: 67%                             │
└─────────────────────────────────────────────┘
```

Each verifier row: adapter icon, verdict badge, confidence bar, expandable reason text.

## Task Breakdown (added to v0.3)

| Task | Description | Depends |
|------|-------------|---------|
| 25 | GeminiHeadlessAdapter + factory registration | — (independent) |
| 26 | Orchestrator refactor + types migration (singular→plural) | — (independent) |
| 27 | ConsensusEngine (4 strategies) | — (independent) |
| 28 | IPC + CLI + Config extension (--verifiers, --consensus) | 26, 27 |
| 29 | Tauri UI Cross-Verify panel | 28, Task 20 |

Tasks 25, 26, 27 are **parallelizable** — no dependencies between them.

## Constraints

- smux-core changes must not break existing 1-planner-1-verifier behavior
- Gemini adapter is optional — smux works fine without it installed
- ConsensusStrategy is set per-session, not per-round
- Max 3 verifiers (practical limit for parallel CLI processes)
- All verdicts use existing `VerifyResult` type and JSON format
- IPC backward compat: `verifier` (singular) field kept, `verifiers` is additive
