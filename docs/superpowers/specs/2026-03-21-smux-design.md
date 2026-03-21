# smux — Multi-AI Agent Ping-Pong Orchestrator

## Overview

smux는 macOS 네이티브 앱으로, 여러 AI 코딩 에이전트(Claude Code, Codex CLI, Gemini CLI 등)를 **자동 핑퐁 검증 루프**로 연결하는 도구다.

기존 도구(cmux, Claude Squad, AMUX 등)는 에이전트 **병렬 실행**에 초점을 맞추지만, smux는 **"계획자 ↔ 검증자" 간 자동화된 adversarial debate 루프**를 핵심으로 한다. 이 패턴은 학술적으로 품질 향상이 입증되어 있다 (Du et al. 2023, AgentCoder 2023).

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
| Backend | Rust | 시스템 도구 포지셔닝, 최고 성능, 메모리 안전성 |
| App Shell | Tauri v2 | Rust 백엔드 + macOS WebKit, 바이너리 ~5-8MB |
| Frontend | HTML/CSS/TypeScript (WebView) | Tauri WebView 안에서 렌더링, 터미널 스타일 UI |
| Terminal Emulation | xterm.js (in WebView) | 에이전트 출력을 WebView 안에서 터미널처럼 렌더링 |
| Browser Panel | macOS WebKit (WKWebView) | 내장 브라우저, Tauri multi-webview |
| Agent I/O | PTY (pseudo-terminal) | 대화형 CLI를 가짜 터미널로 감싸서 제어 |
| Platform | macOS only (Apple Silicon + Intel) | Tauri + WebKit 네이티브 최적화 |

> **설계 결정: Ratatui 대신 Tauri WebView**
>
> Ratatui는 터미널(stdout)에 그리는 라이브러리이므로 Tauri의 WebView 안에서 직접 사용할 수 없다.
> 따라서 UI 전체를 WebView(HTML/CSS/TS)로 구현하고, 터미널 느낌은 xterm.js + 모노스페이스 폰트로 재현한다.
> 이 방식이 내장 브라우저 패널과도 자연스럽게 통합된다.

### Agent I/O Protocol

> **설계 결정: stdin/stdout pipe 대신 PTY**
>
> `claude --pipe` 같은 파이프 모드는 실제로 존재하지 않는다. 각 CLI의 실제 인터페이스:
> - Claude Code: `claude -p "prompt"` (단발성), 대화형은 tty 필요
> - Codex CLI: `codex --quiet` (단발성), 대화형은 tty 필요
> - Gemini CLI: `gemini -p "prompt"` (단발성), 대화형은 tty 필요
>
> 연속 대화를 위해 **PTY(pseudo-terminal) 에뮬레이션**을 사용한다.
> smux가 가짜 터미널을 만들어 CLI가 "진짜 터미널에서 실행 중"이라고 착각하게 한다.

```rust
struct AgentProcess {
    pty: PtyPair,               // 가짜 터미널 쌍 (master/slave)
    process: Child,             // CLI 프로세스
    output_buffer: Vec<u8>,     // 출력 버퍼
    ansi_parser: AnsiParser,    // ANSI escape 코드 파싱
}

impl AgentProcess {
    /// 에이전트에게 메시지 전송 (PTY master에 쓰기)
    fn send(&mut self, message: &str) -> Result<()>;

    /// 에이전트 출력 읽기 (PTY master에서 읽기)
    /// ANSI 코드 파싱하여 순수 텍스트 + 스타일 분리
    fn read_until_idle(&mut self, timeout: Duration) -> Result<AgentOutput>;

    /// 에이전트가 입력 대기 중인지 감지
    fn is_waiting_for_input(&self) -> bool;
}
```

**Idle Detection (응답 완료 감지):**
1. 출력 스트림이 일정 시간(2초) 멈추면 → idle 후보
2. 프롬프트 패턴 감지 (CLI마다 다른 프롬프트 정규식)
3. 두 조건 모두 충족 시 → "응답 완료"로 판정

```toml
# ~/.smux/config.toml — 에이전트별 프롬프트 패턴
[agents.patterns]
claude = "^[❯›>\\$] "        # Claude Code 프롬프트
codex = "^[❯›>\\$] "         # Codex CLI 프롬프트
gemini = "^[❯›>\\$] "        # Gemini CLI 프롬프트
custom = "^\\$ "              # 커스텀 CLI 기본값
```

### System Diagram

```
                      +---------------------------+
                      |        smux App           |
                      |       (Tauri v2)          |
                      +--+----------+----------+--+
                         |          |          |
              +----------+--+  +----+----+  +--+----------+
              | WebView:    |  | WebView:|  | WebView:    |
              | Terminal UI |  | Browser |  | Terminal UI |
              | (xterm.js)  |  | Panel   |  | (xterm.js)  |
              | Planner     |  |         |  | Verifier    |
              +------+------+  +---------+  +------+------+
                     |                             |
              +------+------+              +-------+------+
              | Rust: PTY   |              | Rust: PTY    |
              | Agent Ctrl  |              | Agent Ctrl   |
              +------+------+              +-------+------+
                     |                             |
              +------+------+              +-------+------+
              | claude (tty)|              | codex (tty)  |
              | gemini (tty)|              | claude (tty) |
              | any CLI     |              | any CLI      |
              +-------------+              +--------------+
                     |                             |
                     +----------+--+---------------+
                                |
                     +----------+----------+
                     |  Orchestrator Core  |
                     |  (Rust)             |
                     |                     |
                     |  - Ping-Pong Engine |
                     |  - Phase Manager    |
                     |  - Stop Detector    |
                     |  - Context Passer   |
                     |  - Rewind System    |
                     |  - Health Monitor   |
                     +---------------------+
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

라운드별로 git tag + 대화 컨텍스트를 저장. 잘못된 방향이면 `[r]` 키로 되감기.

```rust
struct RoundSnapshot {
    round: u32,
    git_tag: String,               // git tag smux/<session>/<round>
    planner_context: String,       // planner 대화 (직렬화)
    verifier_context: String,      // verifier 대화 (직렬화)
    verdict: VerifyResult,
    files_changed: Vec<String>,
    timestamp: DateTime<Utc>,
}
```

**동작:**
1. 매 라운드 종료 시: `git tag smux/<session-id>/round-<N>` + 컨텍스트 JSON 저장
2. `[r]` 키 → 라운드 목록 → 선택 → `git checkout <tag>` + 컨텍스트 복원
3. 복원된 라운드부터 핑퐁 재개
4. 이전 tag들은 유지 (히스토리 보존)

> **설계 결정: git stash 대신 git tag**
>
> stash는 스택 기반이라 랜덤 접근이 불안정하고 GC될 수 있다.
> git tag는 랜덤 접근 가능하고, 명시적이며, 세션 종료 후에도 영구 보존된다.

### 2. Safety Guard (NTM 영감)

위험 명령어 자동 감지 및 차단:

```rust
const DANGEROUS_COMMANDS: &[&str] = &[
    "rm -rf",
    "git push --force",
    "git reset --hard",
    "DROP TABLE",
    "DELETE FROM",
    "chmod 777",
];
```

**동작:**
- 에이전트 출력에서 위험 명령어 감지 시 실행 중단
- 사용자에게 확인 요청 (approve/deny)
- Control Mode의 SAFETY 패널에 기록

### 3. Self-Healing (AMUX 영감)

```rust
enum AgentState {
    Working,
    WaitingForInput,
    Stuck { since: DateTime<Utc> },
    ContextLow { percentage: u8 },
    Dead,
}
```

**동작:**
- 컨텍스트 사용률 80%+ → 경고 표시
- 컨텍스트 사용률 20% 이하 → 자동 `/compact` 전송
- 30초 이상 응답 없음 → stuck 감지 → 재시작 + 마지막 메시지 재전송
- 프로세스 crash → 자동 재시작

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
| Agent process | Crash/exit | PTY read returns EOF | 자동 재시작 + 마지막 메시지 재전송 |
| Agent process | Hang (no output) | 30초 타임아웃 | 사용자에게 알림 → kill + 재시작 옵션 |
| Stop detection | JSON 파싱 실패 | 정규식 fallback | 키워드 매칭 → 실패 시 수동 판정 요청 |
| Stop detection | 모호한 응답 | 3차 감지 모두 실패 | 30초 내 수동 판정 → timeout 시 NeedsInfo |
| Context passing | 출력이 4000 토큰 초과 | 토큰 카운터 | 자동 truncation + 요약 |
| Rewind | git checkout 실패 (dirty) | git exit code | git stash → checkout → stash pop |
| Rewind | tag가 없음 | tag 존재 확인 | 에러 메시지 + 사용 가능한 라운드 목록 |
| Browser panel | localhost 연결 실패 | HTTP health check | 재시도 3회 → 실패 시 패널 비활성화 + 알림 |
| Git worktree | worktree 생성 실패 | git exit code | 기존 worktree cleanup → 재시도 → 실패 시 worktree 없이 진행 |
| PTY | ANSI 파싱 에러 | malformed escape seq | 원본 바이트 보존 + 깨진 부분 skip |

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

### Session Discovery

- `smux list`: `~/.smux/sessions/` 스캔 → status별 그룹핑
- `smux attach <id>`: session.json 로드 → PTY 재연결 → UI 복원
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
5. 3-way verify: Code Review + Unit Test + E2E = 우회 불가능

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
  --worktree auto \
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
worktree = "auto"       # auto | manual | off

[agents.planner]
default = "claude"
system_prompt = """
You are the planner/executor. Generate plans and implement code.
Think deeply before major decisions.
"""

[agents.verifier]
default = "codex"
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

[agents.patterns]
claude = '^[❯›>\\$] '
codex = '^[❯›>\\$] '
gemini = '^[❯›>\\$] '
custom = '^\\$ '

[agents.idle]
timeout_secs = 2            # 출력 멈춤 후 idle 판정까지
max_response_secs = 300     # 최대 응답 대기 (5분)

[safety]
block_dangerous = true
dangerous_commands = ["rm -rf", "git push --force", "DROP TABLE"]

[notifications]
desktop = true
sound = true

[health]
auto_compact_threshold = 20  # percent
stuck_timeout = 30           # seconds
auto_restart = true

[sessions]
cleanup_after_days = 7       # completed 세션 자동 삭제
```

---

## Milestones

### v0.1 — MVP (2-3주)

핵심: **두 에이전트 간 자동 핑퐁이 동작한다.**

- [ ] Tauri v2 앱 스캐폴딩 (macOS)
- [ ] PTY로 AI CLI 프로세스 제어 (spawn, read, write)
- [ ] Idle detection (프롬프트 패턴 + 타임아웃)
- [ ] 기본 핑퐁 루프 (Planner → Verifier → Planner → ...)
- [ ] Stop detection (JSON verdict 파싱)
- [ ] Focus Mode UI (좌우 분할, xterm.js)
- [ ] 기본 context passing (전체 출력 전달)
- [ ] CLI: `smux start`, `smux list`

### v0.2 — Control & Safety (2주)

- [ ] Control Mode (가운데 관제 패널)
- [ ] Rewind (git tag + 컨텍스트 스냅샷)
- [ ] Safety Guard (위험 명령어 차단)
- [ ] Git worktree isolation
- [ ] Session persistence (attach/resume)

### v0.3 — Browser & 3-Way (2주)

- [ ] 내장 브라우저 패널 (Tauri multi-webview)
- [ ] 패널 자유 배치 (드래그, 프리셋, 커스텀 저장)
- [ ] Agent-browser integration (E2E 트리거)
- [ ] 3-way verification
- [ ] Diff viewer (WebView 렌더링)

### v0.4 — Polish & Integrations (2주)

- [ ] Self-healing (auto-compact, stuck detection, restart)
- [ ] Smart notifications (macOS 알림 센터)
- [ ] Superpowers skills integration (Claude Code)
- [ ] /think integration + graceful degradation
- [ ] Context management (토큰 제한, 요약)
- [ ] Phase system (0-3 자동 전환)

### v1.0 — Public Release

- [ ] brew install smux
- [ ] 문서 + README
- [ ] GitHub Actions CI/CD
- [ ] 오픈소스 라이선스 결정 (MIT or Apache 2.0)

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

smux가 유일하게 제공하는 것: **자동화된 adversarial 검증 루프 + 3중 검증 + rewind.**

다른 도구는 "여러 에이전트를 **병렬로** 돌리자"이고, smux는 "두 에이전트가 **서로 검증**하게 하자"이다.

| Feature | cmux | Claude Squad | AMUX | Superset | **smux** |
|---------|------|-------------|------|----------|----------|
| Auto ping-pong | ✗ | ✗ | ✗ | ✗ | **✓** |
| Stop detection | ✗ | ✗ | ✗ | ✗ | **✓** |
| Rewind | ✗ | ✗ | ✗ | ✗ | **✓** |
| 3-way verify | ✗ | ✗ | ✗ | ✗ | **✓** |
| Embedded browser | ✓ | ✗ | ✗ | ✗ | **✓** |
| Self-healing | ✗ | ✗ | ✓ | ✗ | **✓** |
| Git worktree | ✗ | ✓ | ✗ | ✓ | **✓** |
| Multi-provider | ✓ | ✓ | ✓ | ✓ | **✓** |
| Safety guard | ✗ (NTM has) | ✗ | ✗ | ✗ | **✓** |

---

## Success Criteria

- [ ] 두 AI CLI 간 자동 핑퐁 동작 (수동 복사-붙여넣기 0회)
- [ ] 검증자가 "mitigation" 감지 시 자동 거절 + 피드백 전달
- [ ] Rewind로 이전 라운드 복원 후 재시작 가능
- [ ] 내장 브라우저에서 E2E 테스트 실시간 관전
- [ ] 전체 세션 완료까지 사용자 개입 최소화
