# smux v0.5 — Competitive Upgrade Plan

> 목표: 터미널 기본기를 table-stakes 수준으로 올리고, AI 핑퐁을 실제 동작시키고, UI를 프로 수준으로 폴리시

## 현재 상태 (2026-03-22)
- PTY 터미널 작동 (portable-pty, zsh)
- Welcome 화면 (폴더 열기 + 최근 프로젝트)
- 설정 모달 (테마만)
- AI 핑퐁 2패널 (빈 쉘만 열림, AI 자동 실행 안됨)
- CI green (Run #14)
- 19 frontend + 169 Rust tests

## 경쟁 분석 요약
- 킬러 피처(Cross-Verification Consensus)는 유일하지만, 터미널 기본기 미달
- cmux가 가장 완성도 높은 AI 터미널 (Ghostty 기반, 알림, 브라우저)
- 기본기 없이는 킬러 피처에 도달할 기회 없음

---

## Phase 1: 터미널 기본기 (P0)
> 이것 없이는 터미널 앱이라고 부를 수 없음

### Task 1-1: 설정 파일 시스템
- `~/.smux/config.toml`에 설정 저장/로드 (daemon config과 통합)
- 항목: font_family, font_size, theme, shell, scrollback, cursor_style, cursor_blink
- Tauri 커맨드: `load_app_config`, `save_app_config`
- 설정 모달에서 변경 → 파일에 저장 → 앱 재시작 없이 적용
- **검증**: 설정 변경 → 앱 재시작 → 설정 유지 확인

### Task 1-2: 폰트 설정
- 설정 모달에 font family 드롭다운 (JetBrains Mono, SF Mono, Menlo, Fira Code)
- font size 슬라이더 (10-20px)
- xterm.js Terminal options 동적 변경
- **검증**: 폰트 변경 → 터미널에 즉시 반영 확인

### Task 1-3: 터미널 내 검색 (⌘F)
- xterm.js `@xterm/addon-search` 사용
- ⌘F → 검색 바 표시 (터미널 상단)
- 하이라이트 + 이전/다음 네비게이션
- ESC로 닫기
- **검증**: 텍스트 출력 → ⌘F → 검색어 입력 → 하이라이트 확인

### Task 1-4: 클릭 가능한 URL
- xterm.js `@xterm/addon-web-links` 사용
- URL 자동 감지 → 밑줄 표시 → 클릭 시 기본 브라우저로 열기
- **검증**: `echo https://github.com` → URL 클릭 → 브라우저 열림

### Task 1-5: 탭 지원
- 사이드바에 탭 목록 표시 (현재 "Sessions" → "Terminals"로 변경)
- "+" 버튼으로 새 탭(PTY) 생성
- 탭 클릭으로 전환 (기존 PTY 유지)
- 탭 닫기 (PTY 종료)
- 각 탭에 이름 표시 (cwd 기반)
- **검증**: 탭 3개 생성 → 각각 다른 명령 실행 → 전환 시 상태 유지

### Task 1-6: 자유 분할 (H+V)
- ⌘D: 수평 분할, ⌘Shift+D: 수직 분할
- 각 분할 패널에 독립 PTY
- 분할 패널 간 포커스 이동 (⌘[ / ⌘])
- 분할 패널 닫기 (⌘W)
- **검증**: 3-way 분할 → 각 패널에서 독립 명령 실행 → 포커스 전환

---

## Phase 2: AI 핑퐁 실동작 (P1)
> 킬러 피처가 실제로 작동해야 함

### Task 2-1: AI 핑퐁 PTY 자동 실행
- AI 모드 시작 시 태스크 입력 프롬프트 (이미 구현)
- "Start" 클릭 → 왼쪽 PTY에 `claude -p --dangerously-skip-permissions "task..."` 자동 실행
- Claude 출력 완료 감지 → 오른쪽 PTY에 `codex exec --full-auto "Review: {claude output}"` 자동 실행
- 라운드 표시 (R1, R2...)
- **검증**: 태스크 입력 → Claude 실행 → Codex 검증 → 결과 표시

### Task 2-2: 핑퐁 자동 반복
- Codex가 REJECTED → Claude에게 피드백 전달 → 재실행
- Codex가 APPROVED → 세션 완료 표시
- 최대 라운드 제한 (설정 가능)
- **검증**: 의도적으로 복잡한 태스크 → 2+ 라운드 핑퐁 확인

### Task 2-3: 에이전트 알림
- 에이전트가 완료/에러/대기 시 macOS 알림
- 탭에 상태 표시 (🟢 실행중, 🟡 대기, 🔴 에러, ✅ 완료)
- **검증**: AI 세션 실행 → 완료 시 macOS 알림 표시

### Task 2-4: 핑퐁 중 수동 개입
- AI 패널에서 직접 타자 → 에이전트 stdin에 전달
- "Intervene" 버튼 → 에이전트 중단 + 사용자 입력 대기
- **검증**: AI 실행 중 타자 → 에이전트에 전달 확인

---

## Phase 3: UI 프로 수준 폴리시 (P2)
> "AI가 만든 티"를 없애고 디자이너가 만든 수준으로

### Task 3-1: 디자인 리서치 + 스타일 가이드
- Warp, Ghostty, cmux 스크린샷 수집
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
- **검증**: 모든 설정 변경 → 즉시 반영 → 저장 후 재시작 → 유지

---

## Phase 4: 차별화 기능 (P3)
> 경쟁 우위 확보

### Task 4-1: 커스텀 N-패널 모드
- 2패널뿐 아니라 3, 4패널 자유 구성
- 각 패널에 역할 지정 (Planner, Reviewer, Tester 등)
- 프리셋: "Code Review" (3패널), "Full Pipeline" (4패널)
- **검증**: 3패널 모드 → 각각 다른 AI 에이전트 실행

### Task 4-2: Git 통합 강화
- 사이드바에 git 브랜치, 변경 파일 수 표시
- AI 세션 결과를 자동 커밋 옵션
- diff 뷰어 (⌘D)
- **검증**: AI 세션 완료 → 변경사항 diff 표시 → 커밋

### Task 4-3: 키바인딩 커스터마이저
- 설정에서 단축키 변경
- tmux 호환 모드 (Ctrl+B prefix)
- **검증**: 커스텀 키바인딩 설정 → 동작 확인

### Task 4-4: xterm.js WebGL 가속
- `@xterm/addon-webgl` 추가
- 렌더링 성능 향상 (큰 출력에서 체감)
- **검증**: 대량 출력 (`cat large-file.log`) → 프레임 드롭 없음

---

## 실행 원칙
1. 각 Task는 독립 커밋 + verification-gate
2. Phase 순서대로 (1→2→3→4), 같은 Phase 내 Task는 병렬 가능
3. 각 Task 완료 후 앱 빌드 + 직접 실행 테스트
4. CI green 유지
5. 질문 없이 자율 실행, 막히면 우회

## 의존성
```
Phase 1 (기본기) → Phase 2 (AI) → Phase 3 (UI) → Phase 4 (차별화)
                                    ↑
                              Phase 3은 Phase 1과 병렬 가능
```

## 예상 커밋 수
- Phase 1: ~6 커밋
- Phase 2: ~4 커밋
- Phase 3: ~4 커밋
- Phase 4: ~4 커밋
- 총 ~18 커밋
