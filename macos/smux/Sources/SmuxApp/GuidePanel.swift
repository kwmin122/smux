import AppKit

/// Guide panel — shortcuts reference + screen layout guide.
/// Triggered by ⌘? or sidebar "?" button or Command Palette "Guide".
class GuidePanel: NSPanel {

    private static var shared: GuidePanel?

    static func toggle(relativeTo window: NSWindow?) {
        if let panel = shared, panel.isVisible {
            panel.close()
            shared = nil
        } else {
            let panel = GuidePanel()
            shared = panel
            if let window = window {
                let wx = window.frame.midX - 220
                let wy = window.frame.midY - 280
                panel.setFrameOrigin(NSPoint(x: wx, y: wy))
            }
            panel.makeKeyAndOrderFront(nil)
        }
    }

    init() {
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 440, height: 560),
            styleMask: [.titled, .closable, .fullSizeContentView],
            backing: .buffered, defer: false
        )
        title = "smux 가이드"
        titlebarAppearsTransparent = true
        isFloatingPanel = true
        isMovableByWindowBackground = true
        backgroundColor = NSColor(white: 0.10, alpha: 1)
        isReleasedWhenClosed = false

        guard let content = contentView else { return }
        content.wantsLayer = true

        let scroll = NSScrollView()
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(scroll)

        let textView = NSTextView()
        textView.isEditable = false
        textView.isSelectable = true
        textView.drawsBackground = false
        textView.textContainerInset = NSSize(width: 20, height: 16)
        textView.textContainer?.widthTracksTextView = true

        scroll.documentView = textView

        NSLayoutConstraint.activate([
            scroll.topAnchor.constraint(equalTo: content.topAnchor, constant: 28),
            scroll.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: content.bottomAnchor),
        ])

        textView.textStorage?.setAttributedString(buildContent())
    }

    // MARK: - Content

    private func buildContent() -> NSAttributedString {
        let result = NSMutableAttributedString()

        let titleFont = NSFont.systemFont(ofSize: 18, weight: .bold)
        let headingFont = NSFont.systemFont(ofSize: 13, weight: .bold)
        let bodyFont = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
        let dimColor = NSColor.secondaryLabelColor
        let labelColor = NSColor.labelColor
        let accentColor = NSColor.systemBlue

        func heading(_ icon: String, _ text: String) {
            result.append(NSAttributedString(string: "\n\(icon) \(text)\n",
                attributes: [.font: headingFont, .foregroundColor: labelColor]))
            result.append(NSAttributedString(string: line() + "\n",
                attributes: [.font: bodyFont, .foregroundColor: NSColor(white: 0.3, alpha: 1)]))
        }

        func shortcut(_ key: String, _ desc: String) {
            let keyStr = NSAttributedString(string: "  \(key.padding(toLength: 12, withPad: " ", startingAt: 0))",
                attributes: [.font: bodyFont, .foregroundColor: accentColor])
            let descStr = NSAttributedString(string: "\(desc)\n",
                attributes: [.font: bodyFont, .foregroundColor: dimColor])
            result.append(keyStr)
            result.append(descStr)
        }

        func info(_ text: String) {
            result.append(NSAttributedString(string: "  \(text)\n",
                attributes: [.font: bodyFont, .foregroundColor: dimColor]))
        }

        func line() -> String {
            String(repeating: "─", count: 44)
        }

        // Title
        result.append(NSAttributedString(string: "smux 가이드\n",
            attributes: [.font: titleFont, .foregroundColor: labelColor]))

        // Core Feature
        heading("🔄", "핑퐁 모드 (핵심 기능)")
        info("두 터미널이 대화하듯 자동으로 핑퐁합니다.")
        info("")
        info("사용법:")
        info("  1. ⌘D로 화면 분할")
        info("  2. 왼쪽에서 claude 실행")
        info("  3. 오른쪽에서 codex 실행")
        info("  4. ⌘⇧P 핑퐁 모드 ON")
        info("  5. 한쪽 에이전트에게 작업 입력")
        info("  6. 자고 일어나면 → 둘이 전부 완료")
        info("")
        info("┌──────────┐    ┌──────────┐")
        info("│ Terminal A│───▶│ Terminal B│")
        info("│ (claude)  │◀───│ (codex)  │")
        info("└──────────┘    └──────────┘")
        info("")
        info("동작 원리:")
        info("  A가 응답 완료 → smux가 출력 캡처")
        info("  → B에 자동 입력 → B가 리뷰/응답")
        info("  → A에 자동 입력 → 반복")
        info("")
        info("미션 컨트롤에서 Pause/Resume 가능")

        // Shortcuts
        heading("⌨", "단축키")
        shortcut("⌘T", "새 탭")
        shortcut("⌘W", "탭 닫기")
        shortcut("⌘D", "세로 분할")
        shortcut("⌘⇧D", "가로 분할")
        shortcut("⌘⇧P", "핑퐁 모드 토글")
        shortcut("⌘⇧B", "브라우저 패널 토글")
        shortcut("⌘I", "인스펙터 토글")
        shortcut("⌘F", "검색")
        shortcut("⌘P", "커맨드 팔레트")
        shortcut("⌘?", "이 가이드")

        // Screen layout
        heading("📋", "화면 구성")
        info("왼쪽     사이드바 — 워크스페이스, git branch, 포트")
        info("상단     스테이지 타임라인 (Ideate → Harden)")
        info("가운데   터미널 (libghostty 네이티브)")
        info("하단     미션 컨트롤 (Approve/Pause/Retry)")
        info("오른쪽   인스펙터 (⌘I로 토글)")

        // Notifications
        heading("🔔", "알림")
        info("사이드바 벨 아이콘 클릭 → 알림 패널")
        info("에이전트 실패/승인 요청 시 자동 알림")
        info("빨간 테두리 = 주의 필요한 워크스페이스")

        // Browser
        heading("🌐", "내장 브라우저")
        info("⌘⇧B로 터미널 옆에 브라우저 패널 열기")
        info("localhost 자동 로드, URL 바로 변경 가능")
        info("사람이 직접 보기 + 자동화 명령 모두 지원")
        info("DOM 스냅샷, 클릭, 폼 입력, 스크린샷")

        // AppleScript
        heading("🤖", "외부 자동화 (AppleScript)")
        info("osascript -e 'tell app \"SmuxApp\" to do script \"ls\"'")
        info("split / browser / screenshot / notify 지원")

        // Session
        heading("💾", "세션 관리")
        info("앱 종료 시 세션 자동 저장 (daemon 유지)")
        info("재실행 시 이전 세션 자동 복원")
        info("⌃⌘⇧D  현재 세션 detach")

        return result
    }

    // MARK: - Keyboard

    override func keyDown(with event: NSEvent) {
        if event.keyCode == 53 { // ESC
            close()
            GuidePanel.shared = nil
        } else {
            super.keyDown(with: event)
        }
    }
}
