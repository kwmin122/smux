# smux — Multi-AI Agent Ping-Pong Orchestrator

## Overview

smux는 여러 AI 코딩 에이전트(Claude Code, Codex CLI, Gemini CLI 등)를 **자동 핑퐁 검증 루프**로 연결하는 macOS용 도구다. headless orchestrator core 위에 Tauri 네이티브 UI를 씌운 구조.

기존 도구(cmux, Claude Squad, AMUX 등)는 에이전트 **병렬 실행**에 초점을 맞추지만, smux는 **"계획자 ↔ 검증자" 간 자동화된 adversarial debate 루프**를 핵심으로 한다. 유사한 패턴(생성자-검증자 분리, 다중 라운드 비판)이 코드 생성 품질을 높인다는 연구 결과가 있다 (Du et al. 2023, AgentCoder 2023). 다만 CLI orchestration 환경에서의 효과는 smux가 직접 검증해야 할 가설이다.

### Problem

개발자가 두 AI CLI를 열어 한쪽은 계획/실행, 한쪽은 검증 역할을 시키는 워크플로우에서, **매번 수동으로 출력을 복사 → 다른 터미널에 붙여넣기**하는 반복 작업이 발생한다. 이 과정에서:
- 컨텍스트가 누락되거나 잘려나감
- 검증자가 "우회(mitigation)"를 통과시키는 실수 발생
- 핑퐁 라운드가 늘어날수록 수동 관리가 비현실적

### Solution

smux가 두 에이전트 사이에서 **자동으로 컨텍스트를 전달**하고, 검증 결과에 따라 **핑퐁을 계속하거나 다음 단계로 진행**한다. 사용자는 실시간으로 양쪽 대화를 관전하면서, 필요할 때만 개입한다.

---

## Architecture

### Tech Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Orchestrator Core | Rust (headless 라이브러리) | UI 없이 독립 실행 가능, 테스트 가능한 코어 |
| Daemon | Rust (background process) | 세션 소유, PTY 유지, attach/detach 지원 |
| App Shell | Tauri v2 (v0.3+) | Rust 백엔드 + macOS WebKit. v0.1-0.2는 터미널 출력 |
| Frontend | HTML/CSS/TypeScript (v0.3+) | Tauri WebView, xterm.js, 내장 브라우저 |
| Agent I/O | Provider Capability Adapter | provider별 최적 경로 선택 (아래 상세) |
| Platform | macOS only (Apple Silicon + Intel) | 우선 macOS, Tauri로 크로스플랫폼 확장 가능 |

> **설계 결정: UI보다 오케스트레이션 코어가 먼저**
>
> v0.1은 headless prototype. Tauri UI는 v0.3부터.
> 이유: 핵심 리스크(agent I/O, stop detection, rewind)가 UI와 무관하게 검증되어야 한다.
> headless core가 안정되면 UI는 그 위에 얹는 것이다.

### Agent I/O Protocol — Provider Capability Adapter

> **설계 결정: PTY 단일화 대신 provider별 최적 경로**
>
> PTY를 모든 CLI의 기본 I/O로 쓰면 brittle하다. Claude Code SDK, Gemini headless 모드 등
> 구조화된 경로가 이미 존재하는 provider에서 PTY를 쓰면 오히려 불안정해진다.
> provider별 capability를 먼저 조사하고, 최적 경로를 선택하는 adapter 패턴을 쓴다.

**Provider Capability Matrix:**

| Provider | Structured (SDK/API) | Headless CLI | Interactive (PTY) |
|----------|---------------------|--------------|-------------------|
| Claude Code | `claude-code-sdk` (Anthropic 공식) — 프로그래매틱 제어, 스트리밍, 구조화 출력 | `claude -p "prompt"` — 단발 | PTY 가능하나 불필요 |
| Codex CLI | OpenAI API 직접 호출 가능 | `codex -q "prompt"` — 단발 | PTY 필요 (대화형) |
| Gemini CLI | Gemini API 직접 호출 가능 | `gemini -p "prompt"` — 단발 | PTY 가능하나 불필요 |
| Custom CLI | 없음 | 있을 수도 없을 수도 | PTY fallback |

**Adapter 선택 우선순위:**
1. **SDK/API** (가장 안정): 구조화된 입출력, 스트리밍, 에러 처리 내장
2. **Headless CLI** (중간): 단발성이지만 깔끔한 출력, 라운드마다 새 프로세스
3. **PTY** (최후 수단): 대화형 CLI만 가능, ANSI 파싱 필요, idle detection 불안정

```rust
/// Provider별 capability를 선언하고, session lifecycle를 관리하는 adapter
trait AgentAdapter: Send + Sync {
    /// 이 adapter가 지원하는 capability 목록
    fn capabilities(&self) -> AdapterCapabilities;

    /// 세션 시작 (선택적 이전 대화 컨텍스트 주입)
    async fn start_session(&mut self, config: SessionConfig) -> Result<()>;

    /// 하나의 턴을 전송하고, 이벤트 스트림으로 응답을 받는다
    async fn send_turn(&mut self, prompt: &str) -> Result<TurnHandle>;

    /// 진행 중인 응답의 이벤트 스트림 (token-by-token 또는 chunk)
    fn stream_events(&self) -> impl Stream<Item = AgentEvent>;

    /// 현재 대화 상태를 직렬화 (rewind 복원용)
    async fn snapshot_state(&self) -> Result<SessionSnapshot>;

    /// 직렬화된 상태에서 세션 복원 (rewind 후 재시작)
    async fn restore_state(&mut self, snapshot: SessionSnapshot) -> Result<()>;

    /// 세션 종료
    async fn terminate(&mut self) -> Result<()>;
}

struct AdapterCapabilities {
    persistent_session: bool,  // SDK: true, headless: false, PTY: true
    streaming: bool,           // SDK: true, headless: false, PTY: true
    native_snapshot: bool,     // SDK: true (대화 ID), headless: false, PTY: false
}

struct SessionConfig {
    system_prompt: String,
    working_directory: PathBuf,
    prior_transcript: Option<Vec<Turn>>,  // rewind 복원 시 이전 대화
}

enum AgentEvent {
    Chunk { text: String },           // 스트리밍 청크
    TurnComplete { response: String, token_estimate: usize },
    Error { message: String },
    ProcessExited { code: i32 },
}

struct SessionSnapshot {
    adapter_type: String,
    /// SDK adapter: 대화 ID나 내부 상태
    /// PTY adapter: canonical transcript (전체 턴 기록)
    /// Headless adapter: 이전 턴의 prompt/response 목록
    state: Vec<u8>,  // adapter별 opaque 직렬화
}
```

> **Persistent session을 지원하지 않는 adapter (headless CLI)는 canonical transcript를 기반으로 다음 턴을 재구성한다.**
> 즉 restore_state()가 호출되면 snapshot 안의 이전 대화를 시스템 프롬프트에 요약 삽입하여 문맥을 복원한다.
> SDK adapter (Claude)는 네이티브 대화 세션을 유지하므로 대화 ID만 저장/복원하면 된다.
> PTY adapter는 canonical transcript를 새 프로세스에 순차 재전송한다.

```rust
// Provider별 구현
struct ClaudeSdkAdapter { /* claude-code-sdk: persistent session, native snapshot */ }
struct HeadlessCliAdapter { /* -p flag: 매 턴 새 프로세스, transcript 기반 복원 */ }
struct PtyAdapter { /* PTY: persistent session, transcript 기반 복원 */ }

fn create_adapter(provider: &str) -> Box<dyn AgentAdapter> {
    match provider {
        "claude" => Box::new(ClaudeSdkAdapter::new()),
        "codex"  => Box::new(PtyAdapter::new("codex")),
        "gemini" => Box::new(HeadlessCliAdapter::new("gemini")),
        custom   => Box::new(PtyAdapter::new(custom)),
    }
}
```

**Idle Detection (PTY adapter에서만 필요):**
1. 출력 스트림 2초 정지 → idle 후보
2. 프롬프트 패턴 정규식 매칭
3. 두 조건 모두 → "응답 완료"
4. SDK/Headless adapter에서는 불필요 — 함수 반환이 곧 완료

```toml
# ~/.smux/config.toml — PTY adapter용 프롬프트 패턴
[agents.pty_patterns]
codex = "^[❯›>\\$] "
custom = "^\\$ "

# adapter 명시 오버라이드 (자동 감지 대신 수동 지정)
[agents.planner]
default = "claude"
adapter = "sdk"           # sdk | headless | pty

[agents.verifier]
default = "codex"
adapter = "pty"           # codex는 아직 SDK 없음
```

### System Diagram

```
  +---------+     +-----------+     +----------+
  | smux CLI | or | Tauri GUI | or | 3rd party|
  | (v0.1+)  |    | (v0.3+)   |    |          |
  +----+-----+    +-----+-----+    +----+-----+
       |               |                |
       +-------+-------+-------+--------+
               |  Unix Socket (IPC)  |
       +-------+---------------------+-------+
       |          smux-daemon                 |
       |         (background process)         |
       |                                      |
       |  +------------------------------+   |
       |  |     Orchestrator Core        |   |
       |  |  - Ping-Pong Engine          |   |
       |  |  - Phase Manager             |   |
       |  |  - Stop Detector             |   |
       |  |  - Context Passer            |   |
       |  |  - Rewind System             |   |
       |  |  - Health Monitor            |   |
       |  +----+-----------------+-------+   |
       |       |                 |           |
       |  +----+-------+  +-----+--------+  |
       |  | Planner    |  | Verifier     |  |
       |  | Adapter    |  | Adapter      |  |
       |  +----+-------+  +-----+--------+  |
       +-------|-----------------|----------+
               |                 |
       +-------+------+  +------+--------+
       | ClaudeSdk    |  | PtyAdapter    |
       | Adapter      |  | (codex tty)   |
       | (SDK 호출)    |  |               |
       +--------------+  +---------------+
```

---

## Core Concepts

### Ping-Pong Engine

핵심 오케스트레이션 루프:

```
Phase 0: Brainstorm (optional)
  Planner → /brainstorm → 설계 산출물
  Verifier → 설계 리뷰 → 핑퐁 → 확정

Phase 1: Plan
  Planner → /think → /write-plan → 구현 계획
  Verifier → /think (독립) → 계획 검증 → 핑퐁
  Stop condition: Verifier가 구조화된 VERDICT 블록 출력

Phase 2: Execute (per task)
  Planner → /think → 코드 구현 → /test
  Verifier → /review → /debug → 검증
  Browser → /agent-browser → E2E 확인
  Stop condition: 3-way verify 통과 (Code + Test + E2E)

Phase 3: Final Verification
  Verifier → /verification-before-completion → 최종 판정
  Stop condition: 전체 태스크 approved + 회귀 0
```

### Stop Detection

Verifier의 시스템 프롬프트에서 **구조화된 출력 포맷**을 강제한다:

```
## Verdict (REQUIRED — must appear at end of every response)
```json
{
  "verdict": "APPROVED" | "REJECTED",
  "category": "root_fix" | "mitigation" | "weak_test" | "regression" | "incomplete" | "security",
  "reason": "one-line explanation",
  "confidence": 0.0-1.0
}
```​
```

**Detection 방식:**
1. 1차: JSON 블록 파싱 — Verifier 출력 끝에서 `{"verdict":` 패턴 검색
2. 2차 (JSON 없을 때): 키워드 정규식 (`APPROVED`, `REJECTED`, `pass`, `fail`)
3. 3차 (둘 다 실패): 사용자에게 수동 판정 요청 (timeout 30초)
4. Timeout: 자동으로 `NeedsInfo` 처리 → Verifier에게 "판정을 내려주세요" 재요청

```rust
enum VerifyResult {
    Approved { reason: String, confidence: f64 },
    Rejected { reason: String, category: RejectCategory, confidence: f64 },
    NeedsInfo { question: String },
}

enum RejectCategory {
    Mitigation,      // 우회 — 근본 해결 아님
    WeakTest,        // 테스트 커버리지 부족
    Regression,      // 회귀 발생
    IncompleteImpl,  // 구현 미완
    SecurityIssue,   // 보안 문제
}
```

### Context Passing

에이전트 간 컨텍스트 전달 전략:

**크기 관리:**
- 단일 라운드 컨텍스트: 전체 출력 전달 (최대 4,000 토큰)
- 4,000 토큰 초과 시: 마지막 1,000 토큰 + AI 요약 (요약은 Verifier에게 요청)
- 누적 컨텍스트: 라운드별 verdict만 유지, 전체 대화는 저장만 하고 전달하지 않음

**전달 형식:**
```
[smux → Verifier, Round 3]

## Previous Rounds Summary
- R1: REJECTED (mitigation) — 전역 limiter는 근본 해결 아님
- R2: REJECTED (weak_test) — override 복원 테스트 누락

## Current Round Context (from Planner)
{planner의 출력, 최대 4000 토큰}

## Your Task
위 계획/구현을 독립적으로 검증하세요.
반드시 마지막에 JSON verdict 블록을 포함하세요.
```

---

## UI Design

### Mode System

두 가지 모드를 `[Tab]` 키로 토글:

**Focus Mode (기본)**
```
+---------------------------------------------------+
| smux  FOCUS  | pingpong R3/10 Phase 1 | [Tab]     |
+------------------------+--------------------------+
| ● PLANNER     claude   | ● VERIFIER     codex    |
|                        |                          |
| [R3] Revised plan:     | [R3] Verifying...       |
| T1: per-route limit    | ✓ T1: root fix          |
| T2: override test      | ✓ T2: proper test       |
| T3: quality gate       | ✓ T3: correct           |
|                        |                          |
| Plan ready ✓           | APPROVED ✓              |
+------------------------+--------------------------+
| [Tab] Control [i] Intervene [r] Rewind [d] Diff   |
+---------------------------------------------------+
```

**Control Mode**
```
+---------------------------------------------------+
| smux  CONTROL  | pingpong R3/10      | [Tab]      |
+----------+---------------+------------------------+
| ● PLANNER| ■ MISSION     | ● VERIFIER             |
|          | CONTROL       |                        |
| [R3]     |               |                        |
| T1: rate | ROUNDS        | [R3] Verifying...      |
| T2: test | R1 ✗ mitig.   | ✓ T1: root fix         |
| T3: gate | R2 ✗ weak     | ✓ T2: proper           |
|          | R3 ✓ approved  | ✓ T3: correct          |
| Ready ✓  |               |                        |
|          | ⟲ REWIND      | APPROVED ✓             |
|          | ▸ R3 (current)|                        |
|          | ◂ R2 rewind   |                        |
|          | ◂ R1 rewind   |                        |
|          |               |                        |
|          | ♥ HEALTH      |                        |
|          | Plan: ● 62%   |                        |
|          | Veri: ● 84%   |                        |
|          |               |                        |
|          | ⚠ SAFETY      |                        |
|          | Danger: block |                        |
|          | Worktree: iso |                        |
|          |               |                        |
|          | ▣ GIT         |                        |
|          | fix/rate-lim  |                        |
|          | 3 files       |                        |
|          | [d] Diff      |                        |
|          |               |                        |
|          | ☰ LOG         |                        |
|          | 14:23 R1 ✗    |                        |
|          | 14:23 R2 ✗    |                        |
|          | 14:23 R3 ✓    |                        |
+----------+---------------+------------------------+
| [Tab] Focus [i] Intervene [r] Rewind [d] Diff     |
+---------------------------------------------------+
```

### Browser Panel & Flexible Layout

모든 패널(Planner, Verifier, Browser, Control)은 **독립 블록**이다.
Tauri v2의 multi-webview 기능으로 각 패널이 별도 WebView.

**패널 배치:**
- `Cmd+Drag`: 마우스로 패널 위치 변경
- `Cmd+1`: Browser Center (Planner | Browser | Verifier)
- `Cmd+2`: Browser Right (Planner | Verifier | Browser)
- `Cmd+3`: Browser Bottom (Planner + Verifier 위 / Browser 아래)
- `Cmd+B`: 브라우저 패널 토글
- `Cmd+F`: 패널 풀스크린 (집중 모드)
- `Cmd+S`: 현재 레이아웃을 커스텀 프리셋으로 저장
- 경계 드래그: 패널 크기 자유 조절

---

## Features

### 1. Rewind (smux 고유)

라운드별로 **git commit** + 대화 컨텍스트를 저장. 잘못된 방향이면 `[r]` 키로 되감기.

> **설계 결정: commit-per-round on worktree branch**
>
> git tag는 dirty worktree를 저장하지 않는다. stash는 스택 기반이라 불안정하다.
> 대신 **worktree 브랜치에 라운드마다 실제 commit**을 만든다.
> 이렇게 하면:
> - dirty 파일 포함 전체 상태가 정확히 보존됨
> - `git checkout <commit>` 한 줄로 완전 복원
> - worktree 브랜치이므로 main에 영향 없음
> - `git log`로 라운드 히스토리 바로 확인

```rust
struct RoundSnapshot {
    round: u32,
    commit_sha: String,            // worktree 브랜치의 실제 커밋
    planner_context_path: PathBuf, // sessions/<id>/rounds/round-N-planner.json
    verifier_context_path: PathBuf,// sessions/<id>/rounds/round-N-verifier.json
    verdict: VerifyResult,
    files_changed: Vec<String>,
    timestamp: DateTime<Utc>,
}
```

**동작:**
1. 매 라운드 종료 시:
   - `git add -A && git commit -m "smux: round N — {verdict}"` (worktree 브랜치)
   - 대화 컨텍스트를 `~/.smux/sessions/<id>/rounds/` 에 JSON 저장
   - adapter의 `snapshot_state()`로 세션 스냅샷 저장
2. `[r]` 키 → 라운드 목록 → 선택 → 아래 복원 절차 실행
3. 복원된 라운드부터 새 에이전트 프로세스로 핑퐁 재개 (`restore_state()`)
4. 이후 라운드의 커밋은 `git reflog`에 남아 복구 가능

**Rewind 복원 절차 (정확한 순서):**
```bash
# 1. tracked 파일을 지정 커밋으로 복원
git reset --hard <commit>

# 2. untracked 파일 제거 (이후 라운드에서 생긴 산출물)
git clean -fd
# -f: force, -d: 디렉토리 포함
# .gitignore에 매칭되는 파일은 보존 (빌드 캐시 등)
```

> **설계 결정: `git clean -fd` (not `-fdx`)**
>
> `-fdx`는 .gitignore 파일까지 삭제한다 (node_modules, .env 등).
> smux는 `-fd`를 기본값으로 쓴다 — ignored artifact는 보존.
> config에서 `rewind_clean_ignored = true`로 `-fdx` 동작 선택 가능.

**Rewind가 실제로 하는 일:**
- 파일 시스템: tracked 파일 복원 + untracked artifact 제거 = 해당 라운드 시점의 정확한 상태
- 에이전트: 현재 프로세스 종료 → 새 프로세스 시작 → `restore_state(snapshot)` 호출로 대화 상태 복원
- 오케스트레이터: 라운드 카운터, phase 상태 복원

### 2. Safety Guard — Repository Integrity Protection

**Scope:** smux의 safety는 **repository integrity**(저장소 안 파일의 의도치 않은 변경/삭제 방지)에 대한 structural protection이다.

> **Non-goals (smux가 방어하지 않는 것)**
>
> smux는 네트워크 exfiltration, provider 자체 버그, 저장소 밖 부작용(외부 API 호출, secret 유출, 시스템 파일 변경)을 완전히 차단하지 않는다.
> 이는 provider의 permission 시스템(Claude Code hooks, Codex approval mode)과 OS sandbox의 책임 범위다.

**3층 방어:**

```
Layer 1: Worktree Isolation (구조적, fail-closed)
  → 모든 세션은 worktree 안에서만 동작. 비활성화 불가.
  → worktree 생성 실패 시 세션 시작 자체를 거부한다 (worktree 없이 진행하지 않음).
  → 최악의 경우에도 worktree 삭제로 저장소 완전 복원.

Layer 2: Agent Permission (provider별, 위임)
  → Claude Code: CLAUDE.md의 allowedTools + hooks로 명령어 제한
  → Codex: --approval-mode으로 실행 전 승인 요구
  → CLI별 자체 safety 기능을 활용 (smux가 재발명하지 않음)

Layer 3: Post-hoc Audit (감시, best-effort)
  → 라운드 종료 시 git diff를 검사
  → 삭제된 파일 수, 변경 규모 이상 감지
  → 이상 시 자동 rewind 제안 (강제하지 않음)
  → 이 layer는 mitigation이며, 그 한계를 인정한다.
```

```rust
struct SafetyConfig {
    // Layer 1: worktree는 항상 활성, config에 노출하지 않음 (fail-closed)

    /// Layer 2: provider별 permission 설정
    claude_allowed_tools: Vec<String>,
    codex_approval_mode: String,  // "suggest" | "auto-edit" | "full-auto"

    /// Layer 3: post-hoc 감사 기준
    max_files_deleted_per_round: usize,   // 기본 5
    max_lines_changed_per_round: usize,   // 기본 2000
    alert_on_threshold_breach: bool,       // 기본 true
}
```

**동작:**
- Layer 1: 세션 시작 시 자동 활성화. worktree 생성 실패 → 세션 시작 실패 (fallback 없음)
- Layer 2: config에서 provider별 설정, smux가 에이전트 시작 시 옵션 전달
- Layer 3: 라운드 종료마다 `git diff --stat` 검사 → 임계값 초과 시 알림

### 3. Self-Healing (v0.5, AMUX 영감)

```rust
enum AgentState {
    Working,
    WaitingForInput,
    Stuck { since: DateTime<Utc> },
    Dead,
}
```

**동작:**
- 30초 이상 이벤트 없음 → stuck 감지 → 사용자 알림 → kill + `restore_state()` 옵션
- `AgentEvent::ProcessExited` 수신 → 자동 `restore_state(latest_snapshot)` → 새 프로세스
- provider별 context management는 adapter 내부에서 처리 (smux 코어가 `/compact` 같은 provider-specific 명령을 직접 보내지 않음)

### 4. Diff Viewer (Superset 영감)

`[d]` 키로 현재 변경사항을 WebView에서 렌더링:
- Side-by-side syntax-highlighted diff
- 라운드별 diff 비교 가능 (R1 vs R3)
- WebView이므로 rich formatting 자연스러움

### 5. Git Worktree Isolation (Claude Squad 영감)

세션 시작 시 자동 worktree 생성:

```bash
smux start → git worktree add .smux/worktrees/<session-id> -b smux/<session-id>
```

- 모든 에이전트 작업이 격리된 브랜치에서 진행
- 실패 시 worktree 삭제로 완전 롤백
- 성공 시 main에 merge

### 6. Smart Notifications (cmux 영감)

- 에이전트 입력 대기 → 패널 테두리 파란색
- 검증 완료/실패 → macOS 알림 센터
- 전체 세션 완료 → 사운드 + 배너 알림

---

## Failure Modes & Recovery

| Component | Failure | Detection | Recovery |
|-----------|---------|-----------|----------|
| Agent adapter | Process crash/exit | `AgentEvent::ProcessExited` | `restore_state(latest_snapshot)` → 새 프로세스로 재시작 |
| Agent adapter | Hang (no events) | 30초 타임아웃 (config) | 사용자에게 알림 → kill + 재시작 옵션 |
| Agent adapter | SDK API 에러 | HTTP status / SDK exception | 3회 재시도 → 실패 시 사용자 알림 |
| Stop detection | JSON 파싱 실패 | verdict 블록 없음 | 키워드 정규식 fallback → 실패 시 수동 판정 요청 |
| Stop detection | 모호한 응답 | 키워드도 매칭 안 됨 | 30초 내 수동 판정 → timeout 시 NeedsInfo → 재요청 |
| Context passing | 출력 4000 토큰 초과 | 토큰 카운터 | 자동 truncation (마지막 1000토큰) + 이전 라운드 요약 |
| Rewind | commit SHA 없음 | `git cat-file -t` 실패 | 에러 메시지 + 사용 가능한 라운드 목록 표시 |
| Rewind | `git clean -fd` 후 빌드 깨짐 | 사용자 보고 | `rewind_clean_ignored = true` 안내 |
| Browser panel | localhost 연결 실패 | HTTP health check | 재시도 3회 → 실패 시 패널 비활성화 + 알림 |
| Git worktree | worktree 생성 실패 | git exit code | 기존 worktree cleanup → 재시도 → **실패 시 세션 시작 거부** |
| PTY adapter | ANSI 파싱 에러 | malformed escape seq | 원본 바이트 보존 + 깨진 부분 skip |

---

## Data Model & Persistence

### Storage

```
~/.smux/
├── config.toml              # 사용자 설정
├── sessions/
│   └── <session-id>/
│       ├── session.json     # 세션 메타데이터
│       ├── rounds/
│       │   ├── round-001.json  # 라운드별 스냅샷
│       │   ├── round-002.json
│       │   └── round-003.json
│       ├── planner.log      # planner 전체 출력 로그
│       ├── verifier.log     # verifier 전체 출력 로그
│       └── events.jsonl     # 이벤트 로그 (타임스탬프 + 이벤트)
└── layouts/
    └── custom-1.json        # 커스텀 레이아웃 프리셋
```

**session.json:**
```json
{
  "id": "a1b2c3",
  "created_at": "2026-03-21T14:20:00Z",
  "task": "upload-rfp endpoint-specific rate limit",
  "planner": "claude",
  "verifier": "codex",
  "phase": "execute",
  "current_round": 3,
  "status": "in_progress",
  "worktree_path": ".smux/worktrees/a1b2c3",
  "worktree_branch": "smux/a1b2c3"
}
```

### Session Lifecycle — Daemon Architecture

> **설계 결정: daemon이 세션을 소유한다**
>
> `smux attach`가 PTY에 재연결하려면, PTY를 소유하는 프로세스가 UI와 독립적으로 살아있어야 한다.
> tmux가 tmux-server로 이걸 해결하듯, smux도 background daemon이 필요하다.

```
smux start → smux-daemon (background) 생성
  daemon이 PTY + 에이전트 프로세스 + 오케스트레이션 소유
  CLI/UI는 daemon에 IPC(Unix socket)로 연결

smux attach → daemon의 Unix socket에 연결 → 스트림 수신
smux detach → 연결만 끊기, daemon과 에이전트는 계속 실행
smux gui → Tauri 앱이 daemon에 연결 (같은 IPC)
```

```rust
// daemon ↔ client IPC 프로토콜
enum ClientMessage {
    Attach { session_id: String },
    Detach,
    Intervene { target: AgentRole, message: String },
    Rewind { round: u32 },
    GetStatus,
}

enum DaemonMessage {
    AgentOutput { role: AgentRole, content: String },
    RoundComplete { round: u32, verdict: VerifyResult },
    SessionComplete { summary: String },
    Error { message: String },
}
```

**Session Discovery:**
- `smux list`: daemon에 GetStatus 요청 → 활성 세션 목록
- daemon이 죽은 경우: `~/.smux/sessions/` 파일 스캔 → "orphaned" 상태 표시
- Cleanup: 7일 이상 된 completed 세션 자동 삭제 (config로 조절 가능)

---

## Integrations

### Superpowers Skills (Claude Code 전용)

Claude Code가 Planner/Verifier일 때만 활성화. 다른 CLI에서는 graceful degradation.

| Phase | Planner (Claude) | Verifier (Claude) | Verifier (non-Claude) |
|-------|---------|----------|----------|
| 0: Brainstorm | `/brainstorm` | 설계 리뷰 | 일반 프롬프트 |
| 1: Plan | `/write-plan` | 계획 검증 | 일반 프롬프트 |
| 2: Execute | 코드 구현 + `/test` | `/review` + `/debug` | 일반 프롬프트 |
| 3: Final | — | `/verification-before-completion` | 일반 프롬프트 |

### /think Integration (Claude Code 전용)

Claude Code에만 `/think` 존재. 다른 CLI에서는 "Think step by step" 프롬프트로 대체.

### Agent-Browser Integration

Phase 2 E2E 검증 단계에서 자동 실행:
1. Planner가 코드 수정 완료
2. smux가 agent-browser 트리거
3. 내장 WebView에서 실제 앱 테스트 실시간 표시
4. E2E 결과 → Verifier에게 전달
5. 3-way verify: Code Review + Unit Test + E2E (우회 가능성을 크게 줄임)

---

## 3-Way Verification

세 가지 독립된 검증을 모두 통과해야 APPROVED:

```
┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│ Code Review │   │  Unit Test  │   │ E2E Browser │
│  (Verifier) │   │ (pytest/    │   │ (agent-     │
│             │   │  vitest)    │   │  browser)   │
│ 근본해결?    │   │ 테스트통과?  │   │ 실제동작?    │
└──────┬──────┘   └──────┬──────┘   └──────┬──────┘
       │                 │                 │
       └────────┬────────┘                 │
                └──────────┬───────────────┘
                           │
                    ┌──────┴──────┐
                    │  ALL PASS?  │
                    │  → APPROVED │
                    └─────────────┘
```

---

## CLI Interface

```bash
# 기본 사용
smux start \
  --planner "claude" \
  --verifier "codex" \
  --task "upload-rfp endpoint-specific rate limit 근본 수정"

# 옵션
smux start \
  --planner "claude" \
  --verifier "gemini" \
  --task "fix authentication bug" \
  --max-rounds 10 \
  --phase plan \
  --browser on \
  --layout center

# 세션 관리
smux list                    # 활성 세션 목록
smux attach <session-id>     # 세션 재접속
smux rewind <session-id> 2   # 라운드 2로 되감기
smux log <session-id>        # 세션 로그 보기
smux diff <session-id>       # 변경사항 보기
```

---

## Configuration

`~/.smux/config.toml`:

```toml
[defaults]
max_rounds = 10
browser = false
layout = "center"       # center | right | bottom
# worktree는 항상 활성 (fail-closed). config에서 비활성화 불가.

[agents.planner]
default = "claude"
adapter = "sdk"              # sdk | headless | pty (자동 감지 오버라이드)
system_prompt = """
You are the planner/executor. Generate plans and implement code.
Think deeply before major decisions.
"""

[agents.verifier]
default = "codex"
adapter = "pty"              # codex는 SDK 미지원
system_prompt = """
You are the independent verifier. Your job is to catch:
- Workarounds (mitigation) instead of root fixes
- Weak test coverage
- Regressions
- Security issues
Be strict. Only approve when the fix is genuinely root-cause.

REQUIRED: End every response with a JSON verdict block:
{"verdict": "APPROVED"|"REJECTED", "category": "...", "reason": "...", "confidence": 0.0-1.0}
"""

# PTY adapter 전용 설정
[agents.pty_patterns]
codex = '^[❯›>\\$] '
custom = '^\\$ '

[agents.idle]
timeout_secs = 2            # PTY adapter: 출력 멈춤 후 idle 판정까지
max_response_secs = 300     # 모든 adapter: 최대 응답 대기 (5분)

[safety]
# Layer 2: provider별 permission
claude_allowed_tools = ["Read", "Write", "Edit", "Bash", "Grep", "Glob"]
codex_approval_mode = "suggest"  # "suggest" | "auto-edit" | "full-auto"

# Layer 3: post-hoc audit 임계값
max_files_deleted_per_round = 5
max_lines_changed_per_round = 2000

[rewind]
clean_ignored = false        # true면 git clean -fdx (ignored 포함)

[notifications]
desktop = true
sound = true

[health]
stuck_timeout = 30           # seconds
auto_restart = true

[sessions]
cleanup_after_days = 7       # completed 세션 자동 삭제
```

---

## Milestones

> **설계 결정: headless first, UI later**
>
> 핵심 리스크(agent I/O adapter, stop detection, rewind)는 UI와 무관하게 검증되어야 한다.
> v0.1은 headless orchestrator + 터미널 로그 출력. Tauri UI는 코어가 안정된 뒤.

### v0.1 — Headless Prototype (2-3주)

핵심: **Claude SDK ↔ Codex PTY 단일 페어로 자동 핑퐁이 동작한다.**

UI 없음. 터미널에 로그 출력. 성공 기준이 계량화되어 있음.

- [ ] Rust 프로젝트 스캐폴딩 (cargo workspace)
- [ ] `AgentAdapter` trait (session/event 모델) + `ClaudeSdkAdapter` 구현
- [ ] `PtyAdapter` 구현 (Codex용) + idle detection + `snapshot_state`/`restore_state`
- [ ] 기본 핑퐁 루프 (Planner → Verifier → Planner → ...)
- [ ] Stop detection (JSON verdict 파싱 + 키워드 fallback)
- [ ] 기본 context passing (전체 출력, 최대 4000 토큰)
- [ ] Git worktree isolation (세션마다 자동 생성)
- [ ] Rewind (commit-per-round + 컨텍스트 JSON 저장/복원)
- [ ] CLI: `smux start`, `smux list`, `smux rewind`
- [ ] 터미널 출력: 양쪽 에이전트 로그 인터리브

**v0.1 성공 기준 (계량):**
- [ ] 수동 개입 없이 5라운드 핑퐁 완주 (claude ↔ codex)
- [ ] stop detection 정확도: 10회 중 8회 이상 정확한 판정
- [ ] rewind 후 이전 라운드 파일 상태 100% 복원 확인
- [ ] 에이전트 crash 후 자동 재시작 성공

### v0.2 — Daemon + Session (2주)

핵심: **detach/attach가 가능한 persistent session.**

- [ ] smux daemon (background process, PTY 소유)
- [ ] `smux attach` / `smux detach` (tmux처럼)
- [ ] Session persistence (daemon이 세션 상태 유지)
- [ ] Safety Layer 2: provider별 permission 설정 전달
- [ ] Safety Layer 3: post-hoc git diff 감사
- [ ] HeadlessCliAdapter 구현 (Gemini용)
- [ ] 3번째 provider 지원 검증

### v0.3 — Tauri UI (3주)

핵심: **headless core 위에 네이티브 앱 UI를 씌운다.**

- [ ] Tauri v2 앱 스캐폴딩 (macOS)
- [ ] Focus Mode (좌우 분할, xterm.js)
- [ ] Control Mode (가운데 관제 패널)
- [ ] Diff viewer (WebView 렌더링)
- [ ] macOS 알림 센터 연동
- [ ] CLI에서 `smux gui`로 UI 실행

### v0.4 — Browser & E2E (3주)

핵심: **내장 브라우저 + 3-way verification.**

- [ ] 내장 브라우저 패널 (Tauri multi-webview)
- [ ] 패널 자유 배치 (드래그, 프리셋 3종, 커스텀 저장)
- [ ] Agent-browser integration (E2E 트리거)
- [ ] 3-way verification (Code + Test + E2E)
- [ ] Context management (토큰 제한, 요약)

### v0.5 — Integrations & Polish (2주)

- [ ] Self-healing (auto-compact, stuck detection, restart)
- [ ] Superpowers skills integration (Claude Code 전용, graceful degradation)
- [ ] Phase system (0-3 자동 전환)
- [ ] 성능 프로파일링 + 최적화

### v1.0 — Public Release

- [ ] `brew install smux`
- [ ] README + 문서 + 스크린캐스트
- [ ] GitHub Actions CI/CD (macOS universal binary)
- [ ] 라이선스: MIT
- [ ] ProductHunt / HN 런칭

---

## Academic Foundation

smux의 핑퐁 검증 패턴을 뒷받침하는 주요 연구:

| Paper | Key Finding | smux 적용 |
|-------|-------------|----------|
| Du et al. "Multiagent Debate" (2023, MIT) | 여러 LLM이 서로 비판→합의 → 팩트 정확도 향상 | 다중 라운드 핑퐁의 직접적 근거 |
| AgentCoder (2023) | Programmer+Tester 분리 → HumanEval 96.3% | 생성자≠검증자 분리의 효과 |
| CoVe (2023, Meta) | 독립적 검증 → F1 23% 향상 | 별도 세션의 독립 검증이 편향 줄임 |
| Self-Refine (2023, CMU) | 생성→비판→수정 루프 → 20% 향상 | 핑퐁 루프 반복의 정량적 근거 |
| MapCoder (2024, ACL) | Plan→Code→Debug 분리, 디버그시 원래 계획 참조 | Phase 기반 파이프라인 |
| Irving et al. "AI Safety via Debate" (2018) | 두 AI 토론이 단일 AI보다 정확 | adversarial debate의 이론적 기반 (영감) |

---

## Competitive Differentiation

**smux의 핵심 차별점은 자동화된 adversarial 검증 루프다.**
다른 도구는 "여러 에이전트를 **병렬로** 돌리자"이고, smux는 "두 에이전트가 **서로 검증**하게 하자"이다.

| Feature | cmux | Claude Squad | AMUX | Superset | **smux** |
|---------|------|-------------|------|----------|----------|
| Auto ping-pong | ✗ | ✗ | ✗ | ✗ | **✓** |
| Stop detection | ✗ | ✗ | ✗ | ✗ | **✓** |
| Rewind | ✗ | ✗ | ✗ | ✗ | **✓** |
| Embedded browser | **✓** | ✗ | ✗ | ✗ | ✓ (v0.4) |
| Self-healing | ✗ | ✗ | **✓** | ✗ | ✓ (v0.5) |
| Git worktree | ✗ | **✓** | ✗ | **✓** | ✓ |
| Multi-provider | **✓** | **✓** | **✓** | **✓** | ✓ |
| Session mgmt | **✓** (tabs) | **✓** (tmux) | **✓** (web) | **✓** (electron) | ✓ (daemon) |
| Safety/permissions | ✗ | worktree | watchdog | review gate | multi-layer |
| Native macOS | **✓** | ✗ (tmux) | ✗ (python) | ✗ (electron) | ✓ (Tauri) |

경쟁 도구들의 강점을 인정한다. cmux의 GPU 렌더링, Claude Squad의 단순함, AMUX의 self-healing은 각각 우수하다. smux가 차별화되는 건 **핑퐁 자동화**라는 단일 축이다.

---

## Success Criteria (계량화)

### v0.1 Gate (headless prototype)

| Metric | Target | Measurement |
|--------|--------|-------------|
| 핑퐁 완주 | 수동 개입 없이 5라운드 연속 | 10회 시도 중 8회 이상 |
| Stop detection 정확도 | APPROVED/REJECTED 정확 판정 | 20개 샘플 중 16개+ 일치 |
| Rewind 복원 정확도 | tracked + untracked 완전 복원 | `git diff` 0 diff + `git clean -n` 0 files |
| Agent crash recovery | 자동 재시작 성공 | kill -9 후 30초 내 복구 |
| Context passing | 핵심 정보 누락 없음 | 수동 리뷰 10건 중 8건+ 충분 |

### v1.0 Gate (제품)

| Metric | Target | Measurement |
|--------|--------|-------------|
| 수동 복붙 | 0회 | 전체 세션 동안 |
| 세션 완주율 | 80%+ | 사용자가 중간 포기 없이 완료 |
| Rewind 사용 후 수렴 | rewind 후 3라운드 내 APPROVED | 5회 측정 |
| 실제 코드 품질 향상 | smux 사용 vs 미사용 A/B | 버그 수, 테스트 커버리지 비교 |
