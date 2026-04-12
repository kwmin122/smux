# Verification Gate Skill — 테스트 리포트

**날짜:** 2026-03-21
**스킬 경로:** `~/.claude/skills/verification-gate/`
**테스트 대상 프로젝트:** mmux (Rust workspace)

---

## 1. 왜 이 스킬을 만들었나

### 해결하려는 근본 문제

**검증 기준이 대화 맥락에 의존한다.** Claude가 태스크를 완료하고 "다 됐다"고 할 때, 어떤 기준으로 검증했는지가 세션마다 다르다. 어떤 때는 cargo test만 돌리고, 어떤 때는 clippy까지 돌리고, semantic review는 거의 안 한다.

**구체적 문제들:**

1. **증거 없는 완료 주장** — "테스트 통과했습니다"라고 하면서 실제 출력을 보여주지 않음
2. **부분 검증** — cargo test만 돌리고 lint/format/type-check 건너뜀
3. **관대한 판정** — 발견한 문제를 "non-blocking note"로 분류하고 넘어감
4. **비구조적 리포트** — finding ID 없이 free-form으로 "다 괜찮아 보인다"
5. **semantic review 부재** — 코드가 컴파일되고 테스트 통과하면 스펙 일치 여부를 안 봄
6. **태스크 간 게이트 없음** — 한 태스크 끝나면 검증 없이 바로 다음으로 넘어감

### 설계 목표

- 단일 스킬(`verification-gate`)로 모든 검증을 일관된 규칙으로 수행
- independent verifier 역할 분리 (구현자 ≠ 검증자)
- fresh evidence 없이는 어떤 성공 주장도 불허
- High/Medium/Low 전부 보고하되, Low 하나라도 남으면 PROCEED 금지
- 서브에이전트로 태스크 간 자동 게이트 가능

---

## 2. 테스트 설계

### 3개 테스트 케이스

| # | 이름 | 프롬프트 | 검증 포인트 |
|---|------|---------|------------|
| 1 | task-completion-gate | "방금 FakeAdapter에 send/recv 구현했어. 다음 태스크 가도 되나?" | 태스크 완료 후 게이트가 제대로 동작하는가 |
| 2 | claim-without-evidence | "cargo test 다 통과했다. 커밋해도 되지?" | 증거 없는 주장을 그대로 믿지 않는가 |
| 3 | direct-verification | "최근 커밋 변경사항 검증해줘. base는 main~3이야." | 직접 검증 요청 시 프로토콜을 정확히 따르는가 |

### 평가 방법

- 각 테스트를 **with-skill** (스킬 적용)과 **without-skill** (baseline, 스킬 없이)로 병렬 실행
- 총 6개 서브에이전트 동시 디스패치
- 17개 assertion으로 정량 평가

---

## 3. 테스트에서 발견된 실제 코드 문제들 (mmux 프로젝트)

스킬을 테스트하면서 mmux 프로젝트의 **실제 코드 이슈**들이 발견되었다.

### VG-001 [Medium] VerifyResult가 design spec과 불일치

**위치:** `crates/smux-core/src/types.rs:74-86`

**내용:** 구현체와 설계 스펙(`docs/superpowers/specs/2026-03-21-smux-design.md:263-266`)이 다르다:
- `Approved`가 unit variant인데 스펙은 `{ reason: String, confidence: f64 }`
- `Rejected`에 `confidence: f64` 필드 누락
- `NeedsInfo(String)` 튜플 variant인데 스펙은 `{ question: String }` named field

**왜 중요한가:** Task 4 (Stop Detection)가 `VerifyResult`를 직접 소비한다. 지금 안 고치면 Task 4에서 잘못된 contract에 맞춰 구현하거나 나중에 rework해야 한다.

**발견한 주체:** Eval 1 with-skill만 발견. Baseline은 완전히 놓침.

### VG-002 [Low] Mutex::lock().unwrap() — poisoned lock 시 panic

**위치:** `crates/smux-core/src/adapter/fake.rs:81, 89, 125`

**내용:** 3곳에서 `.lock().unwrap()` 사용. 다른 스레드가 lock 잡고 panic하면 poisoned lock이 되어 이후 접근 시 panic. `FakeAdapter`가 test double이라 실무 위험은 낮지만, 나머지 코드베이스가 `Result`를 일관되게 사용하는 것과 불일치.

**발견한 주체:** Eval 2 with-skill. Baseline은 발견 못함.

### VG-003 [Low] tokio::spawn JoinHandle 무시 — 에러 소실

**위치:** `crates/smux-core/src/adapter/fake.rs:74-78`

**내용:** `send_turn`에서 `tokio::spawn`의 `JoinHandle`을 버림. spawned task가 panic해도 에러가 소멸. `let _ = tx.send(...)` 로 send 결과도 무시. 테스트에서 이벤트 누락으로만 나타나고 근본 원인이 보이지 않는다.

**발견한 주체:** Eval 2 with-skill. Baseline은 발견 못함.

### VG-004 [Low] VerifyResult/RejectCategory dead code

**위치:** `crates/smux-core/src/types.rs:74-103`

**내용:** 두 타입이 정의만 되고 코드베이스 어디에서도 사용되지 않음. 함수 인자로도, 반환값으로도, 테스트에서도 안 쓰임. forward declaration 의도이지만 현재 iteration에서는 dead code.

**판단 불일치 발생:** Eval 2는 "dead code"로 Low 분류, Eval 3는 "intentional API surface for upcoming tasks"로 finding 아님 판정. (아래 '판단 일관성' 참조)

---

## 4. Baseline vs With-Skill 비교에서 드러난 문제

### 4.1 Baseline이 놓친 것들

| Eval | Baseline 행동 | 문제 |
|------|-------------|------|
| **1 (task-gate)** | "All gates passed. Proceed to Task 4." | VerifyResult 스펙 불일치(Medium) 완전 놓침. 자동화 도구 통과 = 완료로 판정 |
| **2 (claim)** | cargo test만 돌리고 "confirmed, nothing to commit" | lint/format/semantic review 전부 건너뜀. 3개 실제 이슈 놓침 |
| **3 (direct)** | 꽤 괜찮은 리뷰를 했지만 "APPROVED" + "non-blocking notes" | tracing/tokio 중복을 발견했으나 "non-blocking"으로 넘김. Zero-tolerance 없음 |

### 4.2 With-Skill이 해낸 것들

| Eval | With-Skill 행동 | 가치 |
|------|----------------|------|
| **1** | CHANGES_REQUIRED — VG-001 Medium (스펙 불일치) | **자동화 도구가 절대 못 잡는 아키텍처 이슈** 발견 |
| **2** | CLEANUP_REQUIRED — 3개 Low 차단 | 유저 주장 불신 + 전체 프로토콜 실행 + 실제 문제 발견 |
| **3** | PROCEED — 5 phase 전부 실행, zero findings | 깨끗한 상태를 **증거로** 확인 (그냥 "괜찮아 보인다"가 아님) |

### 4.3 정량 비교

| Metric | With Skill | Baseline | Delta |
|--------|-----------|----------|-------|
| **Assertion Pass Rate** | 100% (17/17) | 34.4% (6/17) | **+65.6%** |
| **평균 토큰** | 38,208 | 24,623 | +55% |
| **평균 시간** | 166.8s | 90.4s | +85% |

**해석:** 스킬은 토큰 55%, 시간 85% 더 소모하지만 assertion pass rate를 34% → 100%로 올린다. 더 철저한 검증의 대가이며, 나중에 rework하는 비용보다 훨씬 싸다.

---

## 5. 스킬 자체에서 발견된 문제 / 개선점

### 5.1 Semantic Review 판단 일관성 문제

**증상:** 같은 코드베이스를 같은 스킬로 검증했는데 verifier마다 판단이 다르다.

- Eval 1: `VerifyResult` 스펙 불일치를 **Medium** 으로 발견 (가장 가치 있는 finding)
- Eval 2: `VerifyResult`/`RejectCategory`를 **dead code(Low)** 로 분류
- Eval 3: 같은 타입을 **"intentional API surface, not dead code"** 로 판정하여 finding 아님

**원인:** Semantic review는 본질적으로 판단(judgment)이다. 스킬은 프로토콜(무엇을 검토할지)을 제공하지만, 결론(이것이 문제인지)은 verifier의 해석에 따라 달라진다. 이건 인간 코드 리뷰에서도 동일한 한계.

**가능한 개선:**
- 스펙 문서 경로를 명시적으로 제공하면 스펙 대비 검증 일관성 향상 가능
- 여러 verifier를 돌려서 합의(consensus) 방식 도입 가능 (비용 증가)

### 5.2 Zero-Tolerance의 Trade-off

**장점:** "나중에 고치자"를 원천 차단. Eval 2에서 3개 Low가 실제로 코드 품질 이슈였음.

**잠재 문제:** Forward declaration 같은 의도적 설계 선택도 "dead code"로 차단할 수 있음 (Eval 2의 VG-002). 이 경우 유저가 명시적으로 waive해야 하는데, 자동 게이트 루프에서 이걸 처리하는 UX가 아직 부족.

**가능한 개선:** 스킬에 "유저가 waive한 finding ID 목록"을 받는 메커니즘 추가 검토.

### 5.3 PROCEED 후 자동 전진 실패 (실전 발견)

**증상:** Task 4 완료 후 verification-gate가 6개 finding을 보고 → 전부 수정 → 재검증 통과(PROCEED) → 그런데 에이전트가 "Codex 재리뷰 또는 Task 5로 바로 갈까요?"라고 물어봄. 자동으로 다음 태스크로 넘어가지 않음.

**원인:** Auto-Gate Protocol의 pseudocode에 `ADVANCE — mark task complete, move to next task`라고만 써 있었고, "유저에게 묻지 말라"는 명시적 금지가 없었음. Claude는 기본적으로 중요한 전환점에서 유저 확인을 구하려는 경향이 있어서, 스킬의 "자동 전진" 의도를 무시하고 확인 질문을 던짐.

**수정 내용:**
- **"PROCEED = Go 규칙"** 별도 섹션 신설
- pseudocode에 명시적 금지 주석 추가: `Do NOT ask "다음 태스크 가도 될까요?"`, `Do NOT offer options like "re-review or move on?"`
- 멈출 수 있는 조건 3가지만 허용: 플랜 완료, 3회 에스컬레이션, 유저 인터럽트
- 핵심 메시지: "verification gate이 곧 confirmation — 다시 물어보는 것은 자동화를 무력화하는 행위"

**교훈:** LLM 스킬에서 "X를 하라"보다 "Y를 하지 마라"가 더 강력한 제어 수단. 특히 Claude가 기본 경향으로 하는 행동(확인 질문)을 억제하려면 명시적 금지가 필수.

### 5.4 시간/토큰 비용

With-skill이 baseline 대비 시간 85% 더 소모. 태스크 10개짜리 plan이면 검증만으로 ~28분 추가.

**가능한 개선:**
- 자동화 체크는 빠르므로 비용의 대부분은 semantic review
- Diff 범위를 좁히면 (전체 codebase가 아닌 해당 태스크 변경분만) 시간 절약 가능
- 이미 스킬에 "diff-scoped review" 가이드가 있지만, 실행 시 verifier가 얼마나 잘 따르는지는 변동

---

## 6. 원래 설계 대비 업그레이드 내역

| 항목 | 유저 원래 설계 | 업그레이드 |
|------|-------------|-----------|
| 체크 발견 | 명시 안 됨 | Cargo/npm/pyproject/go/Makefile 자동 감지 테이블 |
| 실행 순서 | 명시 안 됨 | build → format → lint → type → test (build 실패 시 스킵) |
| Semantic review | finding 분류만 언급 | diff 기반 코드 리뷰 단계 추가 (논리 오류, 미구현, 보안) |
| Finding 추적 | 없음 | VG-NNN ID 체계로 re-verification 시 추적 |
| 재시도 제한 | 없음 | 같은 finding 3회 실패 → 유저 에스컬레이션 |
| 서브에이전트 | "서브에이전트로 돌리고 싶다" | 복붙 가능한 dispatch 템플릿 포함 |
| 위임 프롬프트 | 구상만 | `references/claude-code-verifier-prompt.md` 완성 |
| Auto-gate 루프 | "자동으로 돌려" | pseudocode + role separation + 3-attempt escalation |

---

## 7. 해결 이력 및 다음 단계

### 해결 완료

- [x] **판단 일관성 개선** — Core Rule #4 "Spec is the source of truth" 신설, Phase 3.0 "Find Spec Documents" 추가, `{SPEC_DOCUMENTS}` 플레이스홀더 도입
- [x] **Waive 메커니즘** — Waive Protocol 섹션 추가. 유저만 waive 가능, `[WAIVED]` 마킹, auto-gate에서 waive 목록 전달
- [x] **Dead code vs forward declaration** — 명시적 판단 기준 추가 (plan 참조 여부, pub library 여부, 애매하면 Low로 유저 판단)
- [x] **spec-mismatch 카테고리** — 스펙 불일치 전용 카테고리 신설, severity Medium 고정
- [x] **PROCEED 후 자동 전진 실패** — "PROCEED = Go 규칙" 섹션 신설, 명시적 금지 주석, 멈출 수 있는 조건 3개만 허용

### 남은 항목

- [ ] **Description 최적화** — trigger eval 20개로 description 자동 최적화 루프 실행 (skill-creator run_loop)
- [ ] **실전 재검증** — 보강된 스킬로 mmux Task 5 이후 구현 시 auto-gate 재테스트, PROCEED 후 자동 전진 실제 동작 확인
