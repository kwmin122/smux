# smux Landing Page — Stitch Prompt

아래 프롬프트를 Stitch에 붙여넣어 GitHub Pages용 랜딩페이지를 생성하세요.

---

## Stitch Prompt

```
smux 제품 랜딩페이지를 만들어줘. macOS 데스크톱 앱의 다운로드 페이지야.

## 제품 정보

smux는 AI 에이전트를 멀티플렉싱하는 터미널 세션 매니저야.
- Planner(기획) + Verifier(검증) 에이전트가 ping-pong 방식으로 코드를 검증
- Claude, Codex, Gemini 등 여러 AI를 동시에 돌려서 cross-verify (교차 검증)
- 합의 전략: Majority, Weighted, Unanimous, Leader Delegate
- macOS 네이티브 앱 (Tauri v2 + React)
- Focus Mode (2-패널 터미널) / Control Mode (Mission Control 대시보드)
- 실시간 라운드 히스토리, 건강도 지표, 안전 점검, Git 상태
- 키보드 우선 워크플로우: Tab, Cmd+1/2/3, Cmd+B, Cmd+F

## 디자인 시스템

컬러 팔레트 (Deep Navy 테마 — 기본):
- Background: #10131a
- Surface: #0b0e14 (가장 어두운), #191c22, #1d2026, #272a31
- Primary: #a4c9ff (밝은 파랑)
- Secondary: #44e2cd (시안)
- Tertiary: #ffb3b0 (코랄)
- Error: #ffb4ab
- Text: #e1e2eb (밝은), #c1c7d3 (서브), #8b919d (음소거)

폰트:
- 헤드라인: Inter
- 모노스페이스: JetBrains Mono
- 대체 헤드라인: Space Grotesk

## 페이지 구조

1. **Hero Section**
   - 큰 타이틀: "smux" (왼쪽 정렬, 굵고 깨끗하게)
   - 서브타이틀: "AI-Multiplexed Terminal Sessions"
   - 한 줄 설명: "여러 AI 에이전트가 동시에 검증하는 코딩 워크플로우"
   - CTA 버튼: "Download for macOS" (primary color, 큰 버튼)
   - 바로 아래 작은 텍스트: "v0.4 · macOS 12+ · Apple Silicon & Intel"
   - 오른쪽에 앱 스크린샷 또는 터미널 애니메이션 영역 (placeholder)

2. **Features Section** (3열 그리드)
   - "Cross-Verify" — 아이콘 + "Claude, Codex, Gemini를 동시에 돌려 합의 도출"
   - "Focus & Control" — 아이콘 + "2-패널 Focus 모드와 Mission Control 대시보드 전환"
   - "Keyboard-First" — 아이콘 + "Tab, Cmd+1/2/3으로 모든 조작. 마우스 없이도 완전한 컨트롤"

3. **How It Works** (3-step)
   - Step 1: "Install" — 다운로드하고 앱을 열면 바로 시작
   - Step 2: "Configure" — Planner와 Verifier 선택, 합의 전략 설정
   - Step 3: "Run" — 에이전트들이 자동으로 코드를 작성하고 검증

4. **Terminal Demo** (어두운 배경 코드 블록)
   ```
   $ smux start --task "fix auth bug" --verifiers claude,codex --consensus majority
   smux: session created — a1b2c3d4-...
   smux: planner=claude verifier=codex verifiers=claude,codex consensus=majority

   [planner] Analyzing authentication flow...
   [verifier] Reviewing proposed changes...

   === Cross-Verify (round 1) ===
     claude: APPROVED (confidence: 92%) — clean fix
     codex: APPROVED (confidence: 87%) — tests pass
     Final: APPROVED (Majority, 100% agreement)

   smux: session complete — APPROVED at round 1
   ```

5. **Download Section**
   - macOS .dmg 다운로드 버튼 (큰 CTA)
   - "Requirements: macOS 12+, Claude/Codex/Gemini CLI 중 하나 이상"
   - GitHub 링크 (소스 코드 참조용, 오픈소스는 아님)

6. **Footer**
   - © 2026 Kyungwook Min
   - "Personal use only. Commercial licensing available."
   - GitHub · Documentation 링크

## 스타일 가이드

- 다크 테마 only (Deep Navy 배경)
- 미니멀리스트, cmux.com 느낌이지만 더 개발자 친화적
- 코드 블록은 JetBrains Mono
- 부드러운 그라데이션과 글로우 이펙트 최소한으로
- 반응형 (모바일에서도 잘 보이게)
- 앱 아이콘(S 로고)을 로고로 사용
- 애니메이션은 가볍게 — fade-in on scroll 정도
- 한국어/영어 전환은 불필요 (영어 only)

## 기술 요구사항

- 정적 HTML/CSS/JS (GitHub Pages 호스팅)
- 프레임워크 없이 바닐라 또는 가벼운 것만
- 한 페이지 (SPA 스크롤)
- 파비콘은 S 로고 사용
```

---

## 사용법

1. Stitch MCP에 위 프롬프트를 붙여넣기
2. 생성된 HTML/CSS/JS를 `docs/` 또는 별도 repo에 저장
3. GitHub Pages 설정에서 해당 폴더를 소스로 지정
4. 커스텀 도메인 연결 (선택)
