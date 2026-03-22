# smux v0.5 — Enhanced Competitive Upgrade Plan

> 목표: VSCode급 터미널 UX + tmux급 멀티플렉싱 + 유일무이한 AI 교차검증 = 프로 개발자가 기본 터미널로 쓸 수 있는 수준

## 경쟁 리서치 결과 (2026-03-23)

### 분석한 제품 (12개)
| 제품 | 핵심 강점 | smux 위협도 |
|------|-----------|------------|
| **cmux** | Ghostty 기반, Socket API, 알림, 브라우저 | 🔴 직접 경쟁 |
| **Warp** | Blocks, Agent Mode, Oz 클라우드, Launch Configs | 🔴 시장 리더 |
| **iTerm2** | Shell Integration, Triggers, AI Chat | 🟡 macOS 디폴트 |
| **Ghostty** | libghostty, GPU 가속, 46.7k stars | 🟡 인프라 (직접 경쟁 아님) |
| **WezTerm** | Lua 스크립팅, 내장 멀티플렉서, Remote | 🟡 파워유저 |
| **Zellij** | WASM 플러그인, Floating Panes, 웹 클라이언트 | 🟡 모던 멀티플렉서 |
| **tmux** | 세션 영속성, 100+ 플러그인 생태계 | 🟡 레거시 표준 |
| **Wave Terminal** | 블록 레이아웃, BYOK AI, Durable SSH | 🟢 느린 Electron |
| **Kaku** | WezTerm fork, 자동 에러 분석, 에이전트 설정 관리 | 🟢 니치 |
| **VSCode Terminal** | Shell Integration(OSC 633), IntelliSense, Copilot | 🔴 UX 표준 |
| **Cursor Terminal** | Cmd+K 인라인, 에러 Quick Fix, YOLO 모드 | 🔴 AI 터미널 UX 표준 |
| **Windsurf Terminal** | 4-tier 자동 실행, Selection-to-AI, allow/deny | 🟡 좋은 안전 모델 |

### 핵심 발견
1. **Shell Integration(OSC 633)은 2026년 table-stakes** — VSCode, Cursor, Windsurf, Zed, iTerm2 전부 보유. 이것 없으면 커맨드 데코레이션, Sticky Scroll, 네비게이션, Quick Fix 6+ 기능 불가
2. **Blocks(커맨드+출력 그룹화)는 Warp가 증명한 혁신** — 터미널 UX의 패러다임 전환
3. **Socket API는 AI 에이전트 시대의 필수 primitive** — cmux가 증명
4. **세션 영속성은 tmux 사용자를 끌어올 핵심** — smux-daemon 아키텍처로 이미 가능
5. **cross-verification consensus는 여전히 유일** — 아무도 안 하고 있음

### smux만의 포지셔닝
```
                    기본 터미널          AI 터미널           AI 오케스트레이터
                    ─────────          ─────────           ──────────────
Performance    Alacritty, Ghostty
Multiplexing   tmux, Zellij
Full-featured  iTerm2, WezTerm
AI Single      ────────────────>  Warp, Cursor, Windsurf
AI Multi       ────────────────>  cmux, Claude Squad ────>  smux (유일)
                                                              ↑
                                                    Cross-Verification
                                                    Consensus Engine
```

---

## 변경 사항 요약 (기존 18 → 강화 28 태스크)

### 추가된 태스크 (+10)
| # | 태스크 | Phase | 근거 |
|---|--------|-------|------|
| 1-0 | Shell Integration (OSC 633) | 1 | **6+ 기능의 기반**, VSCode/Cursor/Windsurf 전부 보유 |
| 1-7 | xterm.js Addon 번들 | 1 | ligatures, unicode11, image, serialize — 기본 품질 |
| 1-8 | 커맨드 데코레이션 (exit code 거터) | 1 | Shell Integration 파생, Warp Blocks의 경량 버전 |
| 1-9 | Sticky Scroll | 1 | Shell Integration 파생, VSCode 2025 인기 기능 |
| 2-5 | AI 자동 실행 레벨 (4-tier) | 2 | Windsurf 모델 채택 — 안전성의 핵심 |
| 2-6 | Selection-to-AI (⌘L) | 2 | 에러 텍스트를 AI에 전달 — Cursor/Windsurf 필수 기능 |
| 2-7 | Failed Command Auto-Analysis | 2 | Shell Integration + AI 결합 킬러 유스케이스 |
| 4-5 | Socket API (프로그래매틱 패널 제어) | 4 | cmux의 핵심, 에이전트 자동화 필수 |
| 4-6 | 세션 영속성 (Detach/Reattach) | 4 | tmux의 존재 이유, daemon 아키텍처로 가능 |
| 4-7 | Launch Configurations (워크스페이스 프리셋) | 4 | Warp 기능, 프로 개발자 워크플로우 |

### 강화된 기존 태스크
- Task 1-1: 설정에 `ai.auto_execution_level`, `ai.allow_commands`, `ai.deny_commands` 추가
- Task 1-2: `addon-ligatures` 연동으로 Fira Code 합자 지원
- Task 1-4: URL 외에 `file:line:col` 패턴 링크 추가 (Phase 2에서)
- Task 1-5: 탭에 색상, 아이콘, 우클릭 컨텍스트 메뉴 추가
- Task 1-6: 패널 줌 (maximize/restore), 리사이즈 핸들 추가
- Task 2-3: OSC 9/99/777 이스케이프 시퀀스 지원 (cmux 호환)

---

## Phase 1: 터미널 기본기 + 인프라 (P0) — 10 태스크
> 이것 없이는 터미널 앱이라고 부를 수 없음 + VSCode급 UX의 기반

### Task 1-0: Shell Integration (OSC 633) ⭐ NEW — 최우선
> 이 태스크가 Phase 1의 나머지 절반과 Phase 2의 AI 기능을 언락함

- `~/.smux/shell-integration.zsh` 자동 생성 + PTY 시작 시 source
- OSC 633 시퀀스 구현:
  - `A` = prompt start, `B` = prompt end (커맨드 입력 영역)
  - `C` = pre-execution (커맨드 실행 시작)
  - `D;exitcode` = execution finished (커맨드 완료 + exit code)
  - `P;Cwd=/path` = working directory 변경 추적
- xterm.js에서 OSC 633 파싱 → 구조화된 커맨드 히스토리 유지:
  ```typescript
  interface CommandRecord {
    command: string;
    cwd: string;
    exitCode: number;
    startLine: number;
    endLine: number;
    startTime: number;
    endTime: number;
  }
  ```
- Rust daemon에도 커맨드 이벤트 전달 (AI 컨텍스트로 활용)
- **검증**: `ls && false` 실행 → 두 커맨드의 exit code (0, 1) 정확 감지, CWD 추적 확인

### Task 1-1: 설정 파일 시스템 (강화)
- `~/.smux/config.toml`에 설정 저장/로드
- 항목 확장:
  ```toml
  [general]
  shell = "/bin/zsh"
  scrollback = 10000

  [appearance]
  font_family = "JetBrains Mono"
  font_size = 14
  theme = "deep-navy"
  cursor_style = "block"        # block | underline | bar
  cursor_blink = true
  cursor_style_inactive = "outline"
  minimum_contrast_ratio = 4.5  # WCAG AA (NEW)

  [ai]
  auto_execution_level = "auto"  # disabled | allowlist | auto | turbo (NEW)
  allow_commands = ["git", "cargo", "npm", "pnpm", "yarn"]  # (NEW)
  deny_commands = ["rm -rf /", "sudo rm", "shutdown"]        # (NEW)
  max_rounds = 5
  default_planner = "claude"
  default_verifier = "codex"

  [keybindings]
  preset = "default"  # default | tmux | vim
  ```
- Tauri 커맨드: `load_app_config`, `save_app_config`
- 설정 변경 → 파일에 저장 → 앱 재시작 없이 적용
- **검증**: 설정 변경 → 앱 재시작 → 설정 유지 확인

### Task 1-2: 폰트 설정 + 합자 (강화)
- 설정 모달에 font family 드롭다운 (JetBrains Mono, SF Mono, Menlo, Fira Code, Cascadia Code)
- font size 슬라이더 (10-24px)
- `@xterm/addon-ligatures` 설치 → Fira Code/Cascadia Code 합자 지원
- xterm.js Terminal options 동적 변경
- **검증**: Fira Code 선택 → `=>` `!==` `->` 합자 렌더링 확인

### Task 1-3: 터미널 내 검색 (⌘F)
- xterm.js `@xterm/addon-search` 사용
- ⌘F → 검색 바 표시 (터미널 상단, 반투명 오버레이)
- 기능: 하이라이트 + 이전/다음 + **매치 카운트** ("3 of 12") + **정규식 토글** + **대소문자 구분 토글**
- ESC로 닫기
- **검증**: 텍스트 출력 → ⌘F → 검색어 입력 → 하이라이트 + 카운트 확인

### Task 1-4: 클릭 가능한 URL + 파일 링크 (강화)
- xterm.js `@xterm/addon-web-links` 사용
- URL 자동 감지 → 밑줄 표시 → ⌘+클릭 시 기본 브라우저로 열기
- **파일 경로 링크 감지** (NEW):
  - Rust: `--> src/lib.rs:10:5` → ⌘+클릭으로 시스템 에디터 열기
  - TypeScript: `src/app.ts(10,5)`
  - Python: `File "foo.py", line 10`
  - 일반: `path/to/file.ext:line:col`
- Tauri 이벤트로 파일 열기 위임
- **검증**: `cargo build` 에러 → 파일 경로 클릭 → 에디터에서 해당 라인 열림

### Task 1-5: 탭 지원 (강화)
- 사이드바에 탭 목록 표시 ("Terminals")
- "+" 버튼으로 새 탭(PTY) 생성
- 탭 클릭으로 전환 (기존 PTY 유지)
- 탭 닫기 (PTY 종료)
- CWD 기반 자동 이름 표시 (Shell Integration 연동)
- **탭 색상 팔레트** (8색) — 시각적 구분 (NEW)
- **프로세스 기반 아이콘** — zsh/python/node/docker 자동 감지 (NEW)
- **우클릭 컨텍스트 메뉴** — 이름 변경, 색상, 복제, 닫기 (NEW)
- **드래그 앤 드롭 재정렬** (NEW)
- **검증**: 탭 3개 생성 → 각각 다른 색상/이름 → 전환 시 상태 유지

### Task 1-6: 자유 분할 (H+V) (강화)
- ⌘D: 수평 분할, ⌘Shift+D: 수직 분할
- 각 분할 패널에 독립 PTY
- 분할 패널 간 포커스 이동 (⌘[ / ⌘])
- 분할 패널 닫기 (⌘W)
- **리사이즈 핸들** — 패널 경계 드래그 (NEW)
- **패널 줌** — ⌘Shift+Enter로 현재 패널 풀스크린, 다시 누르면 복원 (NEW)
- **검증**: 3-way 분할 → 리사이즈 → 줌 → 복원 → 각 패널 독립 확인

### Task 1-7: xterm.js Addon 번들 ⭐ NEW
> 프로 터미널에 필요한 기본 addon 일괄 설치

- `@xterm/addon-webgl` — GPU 가속 렌더링 (기존 Task 4-4에서 승격)
- `@xterm/addon-ligatures` — 코딩 폰트 합자 (Task 1-2 연동)
- `@xterm/addon-unicode11` — **한국어/CJK 문자 폭 정확도** (한국 사용자 필수)
- `@xterm/addon-image` — Sixel/iTerm 인라인 이미지 (AI 차트 렌더링)
- `@xterm/addon-serialize` — 버퍼 직렬화 (세션 저장/복원 기반)
- `@xterm/addon-canvas` — WebGL 실패 시 canvas 2D fallback 렌더러
- 각 addon lazy-load (필요 시 활성화)
- WebGL 실패 시 addon-canvas fallback
- **검증**:
  - 한국어 `echo "안녕하세요"` → 정확한 폭 렌더링
  - `cat large-file.log` → WebGL로 프레임 드롭 없음
  - Fira Code `=>` `!==` → 합자 정상 표시

### Task 1-8: 커맨드 데코레이션 (Exit Code 거터) ⭐ NEW
> Shell Integration 파생 기능. Warp Blocks의 핵심 UX를 경량 구현

- Shell Integration(1-0)의 CommandRecord 기반
- 터미널 좌측에 얇은 거터 컬럼 (16px)
- 각 커맨드 옆에 상태 아이콘:
  - 🟢 녹색 점: exit 0 (성공)
  - 🔴 빨간 점: non-zero exit (실패)
  - ⏳ 스피너: 실행 중
  - ⚪ 빈 원: exit code 없음
- 실패한 커맨드는 배경 하이라이트 (연한 빨강)
- 거터 클릭 → 해당 커맨드 출력 범위로 스크롤
- **검증**: `ls && false && echo done` → 세 커맨드에 각각 녹색/빨강/녹색 점 표시

### Task 1-9: Sticky Scroll ⭐ NEW
> 긴 출력을 스크롤할 때 어떤 커맨드의 출력인지 항상 표시

- Shell Integration(1-0) 기반
- 스크롤 시 현재 보이는 출력의 원본 커맨드가 터미널 상단에 고정
- 표시 내용: `$ command` + exit code 뱃지 + 경과 시간
- 클릭 → 커맨드 시작 위치로 스크롤
- 설정에서 on/off 가능
- **검증**: `find / -name "*.txt"` (긴 출력) → 스크롤 → 상단에 커맨드 고정 표시

---

## Phase 2: AI 핑퐁 실동작 (P1) — 7 태스크
> 킬러 피처가 실제로 작동해야 함 + AI UX 프리미엄

### Task 2-1: AI 핑퐁 PTY 자동 실행
- AI 모드 시작 시 태스크 입력 프롬프트 (이미 구현)
- "Start" 클릭 → 왼쪽 PTY에 `claude -p --dangerously-skip-permissions "task..."` 자동 실행
- Claude 출력 완료 감지 → 오른쪽 PTY에 `codex exec --full-auto "Review: {claude output}"` 자동 실행
- Shell Integration의 CommandRecord로 완료 감지 (exit code 기반)
- 라운드 표시 (R1, R2...)
- **검증**: 태스크 입력 → Claude 실행 → Codex 검증 → 결과 표시

### Task 2-2: 핑퐁 자동 반복
- Codex가 REJECTED → Claude에게 피드백 전달 → 재실행
- Codex가 APPROVED → 세션 완료 표시
- 최대 라운드 제한 (config.toml의 `ai.max_rounds`)
- **검증**: 의도적으로 복잡한 태스크 → 2+ 라운드 핑퐁 확인

### Task 2-3: 에이전트 알림 (강화)
- 에이전트가 완료/에러/대기 시 macOS 알림
- **OSC 9/99/777 이스케이프 시퀀스 파싱** → cmux 호환 알림 (NEW)
- 탭에 상태 표시 (🟢 실행중, 🟡 대기, 🔴 에러, ✅ 완료)
- 사이드바 탭에 알림 뱃지 (cmux 스타일)
- **검증**: AI 세션 실행 → 완료 시 macOS 알림 + 탭 뱃지 표시

### Task 2-4: 핑퐁 중 수동 개입
- AI 패널에서 직접 타자 → 에이전트 stdin에 전달
- "Intervene" 버튼 → 에이전트 중단 + 사용자 입력 대기
- **검증**: AI 실행 중 타자 → 에이전트에 전달 확인

### Task 2-5: AI 자동 실행 레벨 (4-tier) ⭐ NEW
> Windsurf의 검증된 안전 모델 채택

- config.toml의 `ai.auto_execution_level` 연동:
  - **Disabled**: AI가 커맨드 제안만, 실행은 사용자
  - **Allowlist**: `ai.allow_commands`에 있는 커맨드만 자동 실행
  - **Auto**: AI가 안전성 판단 (위험한 커맨드는 확인 요청)
  - **Turbo**: 모든 커맨드 자동 실행 (YOLO 모드)
- `ai.deny_commands`는 모든 레벨에서 차단
- AI 패널 상단에 현재 레벨 표시 + 원클릭 전환
- **검증**: Allowlist 모드 → `git status` 자동 실행 / `rm -rf` 차단 확인

### Task 2-6: Selection-to-AI (⌘L) ⭐ NEW
> 에러 텍스트를 선택해서 AI에 바로 전달 — Cursor/Windsurf 필수 기능

- 터미널에서 텍스트 드래그 선택 → ⌘L → AI 에이전트에 컨텍스트로 전달
- 선택된 텍스트가 AI 프롬프트에 `context:` 블록으로 삽입
- AI 모드가 아닐 때: 커맨드 팔레트에 "Ask AI about selection" 옵션
- **검증**: 에러 스택 트레이스 선택 → ⌘L → AI가 해당 에러 분석

### Task 2-7: Failed Command Auto-Analysis ⭐ NEW
> Shell Integration + AI 결합의 킬러 유스케이스 (Kaku에서 영감)

- Shell Integration의 exit code 감지 (non-zero)
- 실패한 커맨드의 거터 데코레이션에 "Fix with AI" 버튼 표시
- 클릭 → 커맨드 + 출력을 AI 에이전트에 전달 → 수정 제안
- 자동 모드: `ai.auto_analysis = true` → 실패 시 자동으로 분석 시작
- 제안된 수정 커맨드는 터미널에 삽입 (실행은 사용자 확인)
- **검증**: `cargo build` (컴파일 에러) → "Fix with AI" → 수정 제안 표시

---

## Phase 3: UI 프로 수준 폴리시 (P2) — 4 태스크
> "AI가 만든 티"를 없애고 디자이너가 만든 수준으로

### Task 3-1: 디자인 리서치 + 스타일 가이드
- Warp, Ghostty, cmux, Cursor 스크린샷 수집
- 공통 디자인 패턴 추출 (간격, 색상, 타이포그래피, 아이콘)
- smux 스타일 가이드 문서 작성
- **검증**: 스타일 가이드 문서 완성

### Task 3-2: 레이아웃 리뉴얼
- 사이드바: cmux 스타일 (좁고 깔끔, 아이콘 중심)
- 헤더: 최소화 (Ghostty 스타일 — 탭 바와 통합)
- 터미널: 풀 화면 활용 극대화 (여백 최소화)
- **검증**: 스크린샷 비교 — before/after

### Task 3-3: Welcome 화면 리디자인
- VS Code/Cursor 스타일 참고하되 smux 정체성 반영
- 로고, 깔끔한 카드 UI, 세련된 색상
- **검증**: 비전공자에게 보여줘도 "이쁘다" 반응

### Task 3-4: 설정 화면 풀 구현
- 설정을 별도 뷰로 (모달이 아닌 전체 화면)
- 카테고리별 분류: General, Appearance, Terminal, AI, Keybindings
- 각 설정 항목에 설명 텍스트
- AI 자동 실행 레벨 (2-5) UI 포함
- **검증**: 모든 설정 변경 → 즉시 반영 → 저장 후 재시작 → 유지

---

## Phase 4: 차별화 기능 (P3) — 6 태스크
> 경쟁 우위 확보 + tmux 대체 가치

### Task 4-1: 커스텀 N-패널 모드
- 2패널뿐 아니라 3, 4패널 자유 구성
- 각 패널에 역할 지정 (Planner, Reviewer, Tester 등)
- 프리셋: "Code Review" (3패널), "Full Pipeline" (4패널)
- **검증**: 3패널 모드 → 각각 다른 AI 에이전트 실행

### Task 4-2: Git 통합 강화
- 사이드바에 git 브랜치, 변경 파일 수 표시
- AI 세션 결과를 자동 커밋 옵션
- diff 뷰어 (⌘D)
- **Secret Redaction** — API 키/토큰 자동 마스킹 (NEW, Warp 영감)
- **검증**: AI 세션 완료 → 변경사항 diff 표시 → 커밋 → 시크릿 마스킹 확인

### Task 4-3: 키바인딩 커스터마이저
- 설정에서 단축키 변경
- 프리셋: default / tmux 호환 (Ctrl+B) / vim
- **검증**: 커스텀 키바인딩 설정 → 동작 확인

### Task 4-4: Socket API (프로그래매틱 패널 제어) ⭐ NEW
> cmux의 핵심 — AI 에이전트가 터미널을 프로그래밍적으로 제어

- Unix domain socket API (`~/.smux/api.sock`)
- JSON-RPC 프로토콜:
  ```json
  {"method": "pane.create", "params": {"cwd": "/project", "command": "cargo test"}}
  {"method": "pane.write", "params": {"id": "abc123", "data": "ls\n"}}
  {"method": "pane.read", "params": {"id": "abc123", "lines": 50}}
  {"method": "pane.close", "params": {"id": "abc123"}}
  {"method": "session.list"}
  {"method": "layout.set", "params": {"preset": "code-review"}}
  ```
- CLI 래퍼: `smux api pane.create --cwd /project --command "cargo test"`
- 인증: 소켓 퍼미션 (0o600) + 선택적 토큰
- **검증**: 외부 스크립트에서 `smux api pane.create` → smux UI에 새 패널 생성

### Task 4-5: 세션 영속성 (Detach/Reattach) ⭐ NEW
> tmux의 존재 이유. smux-daemon 아키텍처로 자연스럽게 가능

- `smux detach` → UI 종료해도 daemon + PTY 유지
- `smux attach [session-id]` → 기존 세션에 재접속
- `addon-serialize`로 터미널 버퍼 직렬화 → 재접속 시 복원
- 앱 시작 시 "Resume Session" 옵션 (Welcome 화면에 표시)
- daemon 자동 시작 (launchd 등록)
- **검증**: 세션에서 명령 실행 → 앱 종료 → 앱 재시작 → Resume → 이전 출력 + 실행 중 프로세스 유지

### Task 4-6: Launch Configurations (워크스페이스 프리셋) ⭐ NEW
> Warp 기능. 프로 개발자의 반복 워크플로우 자동화

- `.smux/launch.toml` 또는 프로젝트별 `.smux/launch.toml`:
  ```toml
  [[configurations]]
  name = "Dev Environment"
  layout = "3-split-horizontal"

  [[configurations.panes]]
  command = "npm run dev"
  name = "Dev Server"
  color = "green"

  [[configurations.panes]]
  command = "cargo watch -x test"
  name = "Tests"
  color = "yellow"

  [[configurations.panes]]
  command = ""  # empty shell
  name = "Terminal"
  ```
- Welcome 화면에 "Launch Configurations" 섹션
- 커맨드 팔레트: "Launch: Dev Environment"
- **검증**: 설정 작성 → "Dev Environment" 클릭 → 3패널 + 각각 커맨드 자동 실행

---

## 실행 원칙
1. 각 Task는 독립 커밋 + verification-gate
2. Phase 순서대로 (1→2→3→4), 같은 Phase 내 Task는 병렬 가능
3. 각 Task 완료 후 앱 빌드 + 직접 실행 테스트
4. CI green 유지
5. 질문 없이 자율 실행, 막히면 우회
6. **Shell Integration(1-0)은 Phase 1의 첫 번째 태스크** — 나머지가 의존

## 의존성
```
Task 1-0 (Shell Integration) ──┬──> Task 1-8 (데코레이션)
                                ├──> Task 1-9 (Sticky Scroll)
                                ├──> Task 2-1 (완료 감지 강화)
                                └──> Task 2-7 (Failed Command Analysis)

Phase 1 (기본기) ──> Phase 2 (AI) ──> Phase 4 (차별화)
                      ↑
                Phase 3 (UI)은 Phase 1과 병렬 가능

Task 1-7 (addon-serialize) ──> Task 4-5 (세션 영속성)
Task 4-4 (Socket API) ──> Task 4-6 (Launch Config의 외부 트리거)
```

## 예상 커밋 수
- Phase 1: ~10 커밋
- Phase 2: ~7 커밋
- Phase 3: ~4 커밋
- Phase 4: ~6 커밋
- 총 ~27 커밋

## 미래 로드맵 (v0.6+, 참고용)
> 현재 스코프에는 포함하지 않지만 경쟁력 유지를 위해 기록

| 기능 | 영감 | 우선도 |
|------|------|--------|
| WASM 플러그인 시스템 | Zellij | v0.6 |
| 웹 클라이언트 (브라우저 접속) | Zellij | v0.6 |
| Floating Panes | Zellij | v0.6 |
| 터미널 IntelliSense/Autocomplete | VSCode, Amazon Q | v0.6 |
| Vi 모드 (터미널 내 vim 네비게이션) | Zed | v0.6 |
| Durable SSH 세션 | Wave Terminal | v0.7 |
| 팀 드라이브 (공유 워크플로우) | Warp Drive | v0.7 |
| 인앱 브라우저 스크립팅 API | cmux | v0.7 |
| 로컬 AI (Ollama 연동) | iTerm2, Wave | v0.7 |
- Triggers/Annotations (iTerm2)
- Multi-input to all terminals (Wave)
- Cross-pane AI context reading (TmuxAI)
