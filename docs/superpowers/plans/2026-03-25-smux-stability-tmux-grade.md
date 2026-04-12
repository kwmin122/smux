# smux 안정화 — tmux/cmux급 품질 달성

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 코드 리뷰에서 발견된 7 Critical + 9 High 버그를 전수 수정하여, smux 네이티브 앱을 tmux/cmux급 안정성으로 끌어올린다. 터미널이 처음부터 보이고, split이 작동하고, 핑퐁이 멈추지 않고, ⌘W가 올바르게 동작하는 것이 최소 기준.

**Architecture:** 레이아웃을 Auto Layout 100%로 통일 (autoresizingMask 혼용 제거). IPC를 비동기 + 타임아웃으로 전환. 틱 타이머를 30Hz로 감소. 핑퐁 라우터를 receive_buffer thread-safe 콜백 기반으로 재작성. WKWebView retain cycle 수정.

**Tech Stack:** Swift/AppKit, libghostty C API, GCD (DispatchQueue), NSSplitView, WKWebView, Unix socket IPC.

---

## File Map

| File | 변경 유형 | 책임 |
|------|----------|------|
| `GhosttyTerminalView.swift` | **Modify** | surface 생성, 레이아웃, receive_buffer 콜백, flagsChanged, scrollWheel |
| `WorkspaceWindowController.swift` | **Modify** | setupLayout, doSplit, closePane, toggleBrowser, togglePingPong — 모두 Auto Layout 통일 |
| `main.swift` | **Modify** | 틱 타이머 30Hz, wakeupCb 타겟팅, closeTab 로직, IPC 비동기 |
| `PingPongRouter.swift` | **Rewrite** | receive_buffer 기반 thread-safe 폴링으로 전환 |
| `IpcClient.swift` | **Modify** | connect 타임아웃 2초, 비동기 래퍼 |
| `BrowserPanelView.swift` | **Modify** | WKWebView retain cycle 수정 |
| `SidebarView.swift` | **Minor** | 알림 패널 높이 계산 |

---

### Task 1: 레이아웃 통일 — Auto Layout 100% (C4, H2, H3, H4)

모든 뷰 조작을 Auto Layout으로 통일. `autoresizingMask` 사용 완전 제거.

**Files:**
- Modify: `macos/smux/Sources/SmuxApp/WorkspaceWindowController.swift`

- [ ] **Step 1: `doSplit` 재작성 — Auto Layout 기반**

현재 `doSplit`이 `autoresizingMask`를 사용하고 `translatesAutoresizingMaskIntoConstraints = false`인 뷰를 NSSplitView에 넣어서 충돌. 수정:

```swift
private func doSplit(vertical: Bool) {
    guard let window = window else { return }
    guard let termView = terminalViews.last else { return }
    let container = termView.superview ?? terminalContainer!

    let splitView = NSSplitView()
    splitView.isVertical = vertical
    splitView.dividerStyle = .thin
    splitView.translatesAutoresizingMaskIntoConstraints = false

    // Remove termView's constraints from container
    let oldConstraints = container.constraints.filter {
        $0.firstItem === termView || $0.secondItem === termView
    }
    NSLayoutConstraint.deactivate(oldConstraints)
    termView.removeFromSuperview()

    // Both children: translatesAutoresizing = true inside NSSplitView
    termView.translatesAutoresizingMaskIntoConstraints = true
    termView.autoresizingMask = [.width, .height]
    splitView.addSubview(termView)

    let newTerm = GhosttyTerminalView(frame: NSRect(x: 0, y: 0, width: 800, height: 600), app: ghosttyApp)
    newTerm.translatesAutoresizingMaskIntoConstraints = true
    newTerm.autoresizingMask = [.width, .height]
    splitView.addSubview(newTerm)
    terminalViews.append(newTerm)

    // Add splitView to container with constraints
    container.addSubview(splitView)
    NSLayoutConstraint.activate([
        splitView.topAnchor.constraint(equalTo: container.topAnchor),
        splitView.leadingAnchor.constraint(equalTo: container.leadingAnchor),
        splitView.trailingAnchor.constraint(equalTo: container.trailingAnchor),
        splitView.bottomAnchor.constraint(equalTo: container.bottomAnchor),
    ])

    // Set position after layout
    DispatchQueue.main.async {
        let pos = vertical ? splitView.bounds.width / 2 : splitView.bounds.height / 2
        if pos > 0 { splitView.setPosition(pos, ofDividerAt: 0) }
        window.makeFirstResponder(newTerm)
    }
}
```

핵심: NSSplitView 자체는 Auto Layout으로 부모에 붙이고, NSSplitView 자식들은 `translatesAutoresizingMaskIntoConstraints = true` + `autoresizingMask`로 NSSplitView가 관리하게 한다.

- [ ] **Step 2: `closePane` 재작성 — constraint 복원**

```swift
func closePane() {
    guard terminalViews.count > 1 else { return }
    guard let window = window else { return }
    guard let focused = window.firstResponder as? GhosttyTerminalView else { return }
    guard let split = focused.superview as? NSSplitView else { return }
    let parent = split.superview ?? terminalContainer!

    focused.removeFromSuperview()
    terminalViews.removeAll { $0 === focused }

    if split.subviews.count == 1, let remaining = split.subviews.first {
        // Remove split's constraints
        let splitConstraints = parent.constraints.filter {
            $0.firstItem === split || $0.secondItem === split
        }
        NSLayoutConstraint.deactivate(splitConstraints)

        remaining.removeFromSuperview()
        split.removeFromSuperview()

        // Re-add with constraints
        remaining.translatesAutoresizingMaskIntoConstraints = false
        parent.addSubview(remaining)
        NSLayoutConstraint.activate([
            remaining.topAnchor.constraint(equalTo: parent.topAnchor),
            remaining.leadingAnchor.constraint(equalTo: parent.leadingAnchor),
            remaining.trailingAnchor.constraint(equalTo: parent.trailingAnchor),
            remaining.bottomAnchor.constraint(equalTo: parent.bottomAnchor),
        ])
    }

    if let last = terminalViews.last {
        window.makeFirstResponder(last)
    }
}
```

- [ ] **Step 3: `toggleBrowser` 재작성 — constraint 기반**

브라우저 OFF 시 터미널 복원을 constraint로:

```swift
// Browser OFF path:
if let browser = browserPanel, let split = browser.superview as? NSSplitView {
    browser.removeFromSuperview()
    if let termView = terminalViews.first {
        termView.removeFromSuperview()
        split.removeFromSuperview()

        termView.translatesAutoresizingMaskIntoConstraints = false
        terminalContainer.addSubview(termView)
        NSLayoutConstraint.activate([
            termView.topAnchor.constraint(equalTo: terminalContainer.topAnchor),
            termView.leadingAnchor.constraint(equalTo: terminalContainer.leadingAnchor),
            termView.trailingAnchor.constraint(equalTo: terminalContainer.trailingAnchor),
            termView.bottomAnchor.constraint(equalTo: terminalContainer.bottomAnchor),
        ])
    }
    browserPanel = nil
    browserAutomation = nil
    return
}
```

브라우저 ON 시에도 NSSplitView를 constraint로 부모에 붙이기 (doSplit과 동일 패턴).

- [ ] **Step 4: 빌드 + 실행 확인**

```bash
cd /Users/min-kyungwook/Desktop/mmux && swift build --package-path macos/smux
```

Expected: Build complete, 앱 실행 시 터미널 즉시 보임, ⌘D 분할 정상.

- [ ] **Step 5: 커밋**

```bash
git add macos/smux/Sources/SmuxApp/WorkspaceWindowController.swift
git commit -m "fix: unify layout to Auto Layout — fix split, closePane, toggleBrowser"
```

---

### Task 2: 틱 타이머 + wakeupCb 최적화 (C2, H5)

120Hz → 30Hz. wakeupCb가 모든 윈도우 대신 터미널 뷰만 타겟.

**Files:**
- Modify: `macos/smux/Sources/SmuxApp/main.swift`

- [ ] **Step 1: 틱 타이머 30Hz로 감소**

```swift
// 변경 전: 1.0/120.0
// 변경 후:
tickTimer = Timer.scheduledTimer(withTimeInterval: 1.0/30.0, repeats: true) { [weak self] _ in
    guard let a = self?.ghosttyApp else { return }
    ghostty_app_tick(a)
}
```

- [ ] **Step 2: wakeupCb 터미널 뷰만 타겟**

```swift
private let wakeupCb: @convention(c) (UnsafeMutableRawPointer?) -> Void = { _ in
    DispatchQueue.main.async {
        // Only redraw the main window's terminal, not all windows (panels, dialogs)
        if let mainWindow = NSApplication.shared.mainWindow {
            mainWindow.contentView?.setNeedsDisplay(mainWindow.contentView?.bounds ?? .zero)
        }
    }
}
```

- [ ] **Step 3: 빌드 확인**

- [ ] **Step 4: 커밋**

```bash
git add macos/smux/Sources/SmuxApp/main.swift
git commit -m "perf: reduce tick timer to 30Hz, target wakeupCb to main window only"
```

---

### Task 3: IPC 비동기 + 타임아웃 (C6)

메인 스레드에서 소켓 connect() 블로킹 제거.

**Files:**
- Modify: `macos/smux/Sources/SmuxApp/IpcClient.swift`
- Modify: `macos/smux/Sources/SmuxApp/main.swift`
- Modify: `macos/smux/Sources/SmuxApp/WorkspaceWindowController.swift`

- [ ] **Step 1: IpcClient에 connect 타임아웃 추가**

```swift
func connect() throws {
    let fd = socket(AF_UNIX, SOCK_STREAM, 0)
    guard fd >= 0 else { throw IpcError.socketCreationFailed }

    // Set 2-second send/receive timeout
    var tv = timeval(tv_sec: 2, tv_usec: 0)
    setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, socklen_t(MemoryLayout<timeval>.size))
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, socklen_t(MemoryLayout<timeval>.size))

    // Non-blocking connect with timeout
    var flags = fcntl(fd, F_GETFL, 0)
    fcntl(fd, F_SETFL, flags | O_NONBLOCK)

    // ... existing connect code ...

    // Restore blocking mode after connect
    flags = fcntl(fd, F_GETFL, 0)
    fcntl(fd, F_SETFL, flags & ~O_NONBLOCK)

    self.connection = FileHandle(fileDescriptor: fd, closeOnDealloc: true)
}
```

- [ ] **Step 2: isDaemonRunning을 비동기로**

```swift
func checkDaemonAsync(completion: @escaping (Bool) -> Void) {
    DispatchQueue.global(qos: .utility).async { [self] in
        let running = isDaemonRunning
        DispatchQueue.main.async { completion(running) }
    }
}
```

- [ ] **Step 3: main.swift에서 비동기 호출**

```swift
// 변경 전: if ipc.isDaemonRunning { ... }
// 변경 후:
let ipc = SmuxIpcClient()
ipc.checkDaemonAsync { running in
    if running {
        self.workspaceController?.window?.title = "smux — daemon ●"
    }
}
```

- [ ] **Step 4: refreshSessions도 비동기**

`WorkspaceWindowController.refreshSessions()`를 백그라운드에서 실행.

- [ ] **Step 5: 빌드 + 커밋**

---

### Task 4: 핑퐁 라우터 — receive_buffer thread-safe 재작성 (C3)

`readScreenText()` 호출 제거. receive_buffer 콜백을 thread-safe하게 재도입하되, 메인 스레드 폭격 방지.

**Files:**
- Modify: `macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift`
- Rewrite: `macos/smux/Sources/SmuxApp/PingPongRouter.swift`

- [ ] **Step 1: GhosttyTerminalView에 thread-safe 출력 버퍼**

```swift
// 프로퍼티 추가
private let outputLock = NSLock()
private var rawOutputBuffer = Data()
var onOutputReceived: (() -> Void)?  // 콜백 (메인 스레드 아님)

/// Thread-safe: 백그라운드에서 호출해도 안전
func drainOutputBuffer() -> Data {
    outputLock.lock()
    let data = rawOutputBuffer
    rawOutputBuffer = Data()
    outputLock.unlock()
    return data
}
```

surface config에 receive_buffer 설정:
```swift
cfg.receive_userdata = Unmanaged.passUnretained(self).toOpaque()
cfg.receive_buffer = { (userdata, bytes, len) in
    guard let userdata = userdata, let bytes = bytes, len > 0 else { return }
    let view = Unmanaged<GhosttyTerminalView>.fromOpaque(userdata).takeUnretainedValue()
    let data = Data(bytes: bytes, count: len)
    view.outputLock.lock()
    view.rawOutputBuffer.append(data)
    view.outputLock.unlock()
    // NO DispatchQueue.main.async here — just accumulate
}
```

- [ ] **Step 2: PingPongRouter — 백그라운드 폴링**

```swift
// pollTimer를 백그라운드 큐에서 실행
private let pollQueue = DispatchQueue(label: "pingpong.poll", qos: .userInitiated)

func start() {
    // ...
    pollTimer = Timer(timeInterval: 0.5, repeats: true) { [weak self] _ in
        self?.pollQueue.async { self?.poll() }
    }
    RunLoop.main.add(pollTimer!, forMode: .common)
}

private func poll() {
    // 백그라운드에서 실행 — 메인 스레드 블록 없음
    let outputA = paneA?.drainOutputBuffer() ?? Data()
    let outputB = paneB?.drainOutputBuffer() ?? Data()
    // ... 턴 감지 로직 ...

    // UI 업데이트만 메인 스레드
    DispatchQueue.main.async { self.updateState(...) }
}
```

- [ ] **Step 3: readScreenText() 제거**

`readScreenText()`와 `hasScreenChanged()` 메서드 삭제. `ghostty_surface_read_text` 호출 완전 제거.

- [ ] **Step 4: 빌드 + 핑퐁 테스트**

- [ ] **Step 5: 커밋**

```bash
git add macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift macos/smux/Sources/SmuxApp/PingPongRouter.swift
git commit -m "fix: rewrite ping-pong with thread-safe receive_buffer — no main thread blocking"
```

---

### Task 5: ⌘W 올바르게 동작 (H1)

단일 탭이면 앱 종료, 여러 탭이면 탭만 닫기.

**Files:**
- Modify: `macos/smux/Sources/SmuxApp/main.swift`

- [ ] **Step 1: closeTab 로직 수정**

```swift
@objc func closeTab() {
    guard let window = workspaceController?.window else {
        NSApp.terminate(nil)
        return
    }
    if let tabbedWindows = window.tabbedWindows, tabbedWindows.count > 1 {
        window.close()
    } else {
        NSApp.terminate(nil)
    }
}
```

- [ ] **Step 2: closeCb 복원** — surface 닫힐 때 아무것도 안 함 (유지)

- [ ] **Step 3: 빌드 + 확인 + 커밋**

---

### Task 6: WKWebView retain cycle 수정 (C7)

**Files:**
- Modify: `macos/smux/Sources/SmuxApp/BrowserPanelView.swift`

- [ ] **Step 1: weak wrapper 클래스 추가**

```swift
private class WeakScriptHandler: NSObject, WKScriptMessageHandler {
    weak var delegate: WKScriptMessageHandler?
    init(_ delegate: WKScriptMessageHandler) { self.delegate = delegate }
    func userContentController(_ c: WKUserContentController, didReceive message: WKScriptMessage) {
        delegate?.userContentController(c, didReceive: message)
    }
}
```

- [ ] **Step 2: add() 호출을 weak wrapper로 변경**

```swift
// 변경 전: config.userContentController.add(self, name: "consoleLog")
// 변경 후:
config.userContentController.add(WeakScriptHandler(self), name: "consoleLog")
```

- [ ] **Step 3: deinit에서 handler 제거**

```swift
deinit {
    webView.configuration.userContentController.removeScriptMessageHandler(forName: "consoleLog")
}
```

- [ ] **Step 4: 커밋**

---

### Task 7: 핑퐁 Pause 콜백 복원 (H8)

**Files:**
- Modify: `macos/smux/Sources/SmuxApp/WorkspaceWindowController.swift`

- [ ] **Step 1: 원래 onPause 저장**

```swift
private var originalPauseHandler: (() -> Void)?
```

- [ ] **Step 2: togglePingPong에서 저장/복원**

```swift
// 시작 시:
originalPauseHandler = missionControl.onPause
missionControl.onPause = { [weak router] in ... }

// 종료 시:
missionControl.onPause = originalPauseHandler
```

- [ ] **Step 3: 커밋**

---

### Task 8: 메뉴 정리 + 기타 High 수정 (H7, H9, L2)

**Files:**
- Modify: `macos/smux/Sources/SmuxApp/main.swift` — 메뉴 항목 올바른 위치로
- Modify: `macos/smux/Sources/SmuxApp/GhosttyTerminalView.swift` — scrollWheel mods, flagsChanged

- [ ] **Step 1: Find, Close Pane을 viewMenu로 이동**

- [ ] **Step 2: scrollWheel에 modifier flags 전달**

```swift
override func scrollWheel(with event: NSEvent) {
    guard let surface = surface else { return }
    ghostty_surface_mouse_scroll(surface, Double(event.scrollingDeltaX),
                                  Double(event.scrollingDeltaY),
                                  Self.ghosttyMods(event.modifierFlags))
}
```

- [ ] **Step 3: flagsChanged에서 press/release 구분**

```swift
private var lastModifierFlags: NSEvent.ModifierFlags = []

override func flagsChanged(with event: NSEvent) {
    guard let surface = surface else { return }
    let action: ghostty_input_action_e = event.modifierFlags.rawValue > lastModifierFlags.rawValue
        ? GHOSTTY_ACTION_PRESS : GHOSTTY_ACTION_RELEASE
    lastModifierFlags = event.modifierFlags
    var key = ghostty_input_key_s()
    key.action = action
    key.mods = Self.ghosttyMods(event.modifierFlags)
    key.keycode = UInt32(event.keyCode)
    key.text = nil
    key.composing = false
    ghostty_surface_key(surface, key)
}
```

- [ ] **Step 4: 빌드 + 전체 테스트 + 커밋**

---

### Task 9: 최종 검증 — verification-gate

**Files:**
- All modified files

- [ ] **Step 1: 전체 빌드**

```bash
cd /Users/min-kyungwook/Desktop/mmux && swift build --package-path macos/smux
```

- [ ] **Step 2: Rust 테스트**

```bash
export PATH="$HOME/.cargo/bin:$PATH" && cargo test --workspace && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 3: 앱 실행 수동 테스트 체크리스트**

| 항목 | 예상 결과 |
|------|----------|
| 앱 실행 | 터미널 + 사이드바 + 하단바 즉시 보임 |
| 터미널 타이핑 | 셸 프롬프트 보이고 입력 가능 |
| ⌘D | 세로 분할, 두 터미널 모두 작동 |
| ⌘⇧D | 가로 분할 작동 |
| ⌘⇧W | 분할된 패인 하나 닫기 |
| ⌘W | 앱 종료 (단일 탭), 탭 닫기 (복수 탭) |
| ⌘⇧B | 브라우저 열림/닫힘 반복 |
| ⌘⇧P | 핑퐁 모드 ON (2패인 필요) → 멈추지 않음 |
| ⌘I | 인스펙터 토글 |
| ⌘P | 커맨드 팔레트 |
| ⌘/ | 가이드 패널 |
| ⌘Q | 앱 종료 |
| 한글 입력 | 조합 + 완성 정상 |
| 윈도우 리사이즈 | 모든 컴포넌트 비례 리사이즈 |

- [ ] **Step 4: /verification-gate 실행**

- [ ] **Step 5: 최종 커밋**

```bash
git add -A
git commit -m "feat: smux v0.7 — tmux-grade stability, ping-pong mode, full UI"
```

---

## Execution Notes

- Task 1 (레이아웃)이 가장 핵심 — 나머지 모든 UI 버그의 근본 원인
- Task 4 (핑퐁)는 Task 1 완료 후 진행 — 레이아웃이 안정적이어야 split 기반 핑퐁 테스트 가능
- 각 Task마다 `swift build` + 실행 확인 후 커밋
- @verification-gate를 Task 9에서 실행
