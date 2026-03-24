# ADR-001: Terminal Rendering Engine

## Status: Proposed

## Context

smux v0.5는 Tauri + xterm.js로 터미널을 렌더링한다. xterm.js는 WebKit(WKWebView)에서 한글 IME 조합이 근본적으로 작동하지 않는다 (xterm.js issues #3836, #5374, #5778). Chrome에서는 작동하지만 Tauri는 macOS에서 WKWebView를 사용한다.

한국인 개발자가 첫 고객이므로 한글 입력은 최우선 요구사항이다.

## Decision

**옵션 E: 순수 Swift + libghostty (Kytos 방식)** 채택.

Tauri + xterm.js 아키텍처를 버리고, Swift/SwiftUI + libghostty 네이티브 앱으로 전환한다.

## Rationale

1. **한글 완벽 지원**: libghostty는 NSTextInputClient를 통해 macOS 네이티브 IME를 사용. 한글, 일본어, 중국어 조합이 100% 작동 (Kytos에서 검증됨).

2. **최고 성능**: Metal GPU 가속 120fps. xterm.js WebGL보다 빠름.

3. **최소 크기**: ~15MB. Electron(200MB+)이나 Tauri(30MB)보다 가벼움.

4. **검증된 패턴**: Kytos(2026-03-14)가 동일한 아키텍처로 성공. Swift ~1,500줄로 완성.

5. **cmux와 동일 기반**: 가장 완성도 높은 경쟁 제품 cmux도 libghostty 사용.

## Consequences

### 긍정적
- 한글 IME 네이티브 작동
- 최고 성능 + 최소 크기
- macOS 네이티브 앱 경험
- cmux 수준의 터미널 품질

### 부정적
- React 프론트엔드 코드 재작성 필요 (SwiftUI로)
- Zig 빌드 툴체인 필요 (libghostty 컴파일)
- macOS 전용 (Linux는 별도 구현 필요)
- 개발 기간 4-6주

### 유지되는 것
- smux-core (Rust): 오케스트레이션 로직, 컨센서스, IPC — 그대로 사용
- smux-daemon (Rust): 세션 관리, PTY — 그대로 사용
- smux-cli (Rust): CLI 인터페이스 — 그대로 사용
- 아키텍처: 3-tier (core/daemon/app) 유지

### 버려지는 것
- smux-app (Tauri + React): 완전 교체
- xterm.js 및 모든 웹 기반 터미널 코드
- package.json, node_modules, Vite, React 컴포넌트

## Alternatives Considered

1. **Tauri 유지**: 한글 불가 → 탈락
2. **Electron 전환**: 한글 작동하지만 200MB+, 느림 → 탈락
3. **tmux TUI (ratatui)**: 한글 작동, 가벼움, 하지만 GUI 기능 없음 → 탈락
4. **Tauri + libghostty 하이브리드**: 가능하지만 WebView와 네이티브 뷰 혼합이 복잡, Tauri에서 NSView 임베딩이 불안정 → 탈락

## Migration Plan

### Phase 1: libghostty 빌드 환경 설정 (2일)
- Zig 설치, libghostty 클론, xcframework 빌드
- 기본 Swift 프로젝트 + libghostty 링킹 확인

### Phase 2: 최소 터미널 (3일)
- GhosttyView (NSView) + PTY 연결
- 한글 입력 확인 (NSTextInputClient)
- 기본 셸 실행 + 입출력

### Phase 3: 탭 + 분할 (3일)
- SwiftUI TabView 또는 커스텀 탭 바
- NSSplitView로 분할 패널
- 각 패널에 독립 GhosttyView + PTY

### Phase 4: AI 핑퐁 + 기존 기능 포팅 (1주)
- smux-daemon 연결 (Unix socket IPC)
- 2패널 AI 모드
- 파일 탐색기 (SwiftUI List + FileManager)
- 설정 (SwiftUI Form)
- 검색 (libghostty 내장 기능)

### Phase 5: 폴리시 + 릴리스 (3일)
- 테마 시스템
- 키바인딩
- dmg 패키징
