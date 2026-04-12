import AppKit
import WebKit

/// Embedded browser panel using WKWebView — shows localhost or any URL alongside terminal.
/// Supports URL bar navigation, back/forward, reload, and focus toggle (⌘⇧B).
class BrowserPanelView: NSView {

    private let webView: WKWebView
    private let urlBar = NSTextField()
    private let backButton = NSButton()
    private let forwardButton = NSButton()
    private let reloadButton = NSButton()
    private let focusToggle = NSButton()
    private let toolbar = NSView()

    /// Current URL displayed in the browser.
    var currentURL: URL? { webView.url }

    /// Callback when page finishes loading.
    var onPageLoaded: ((URL?) -> Void)?

    /// Callback when console.log messages arrive (for automation).
    var onConsoleMessage: ((String) -> Void)?

    override init(frame: NSRect) {
        let config = WKWebViewConfiguration()
        config.preferences.setValue(true, forKey: "developerExtrasEnabled")

        // Inject console.log bridge for automation
        let script = WKUserScript(
            source: """
            (function() {
                var origLog = console.log;
                console.log = function() {
                    var msg = Array.prototype.slice.call(arguments).map(String).join(' ');
                    window.webkit.messageHandlers.consoleLog.postMessage(msg);
                    origLog.apply(console, arguments);
                };
            })();
            """,
            injectionTime: .atDocumentStart,
            forMainFrameOnly: true
        )
        config.userContentController.addUserScript(script)

        webView = WKWebView(frame: .zero, configuration: config)
        super.init(frame: frame)

        config.userContentController.add(WeakScriptHandler(self), name: "consoleLog")

        wantsLayer = true
        layer?.backgroundColor = NSColor(white: 0.08, alpha: 1).cgColor

        setupToolbar()
        setupWebView()
        layoutViews()
    }

    required init?(coder: NSCoder) { fatalError() }

    // MARK: - Setup

    private func setupToolbar() {
        toolbar.wantsLayer = true
        toolbar.layer?.backgroundColor = NSColor(white: 0.12, alpha: 1).cgColor
        toolbar.translatesAutoresizingMaskIntoConstraints = false
        addSubview(toolbar)

        // Back button
        backButton.image = NSImage(systemSymbolName: "chevron.left", accessibilityDescription: "Back")
        backButton.bezelStyle = .inline
        backButton.isBordered = false
        backButton.imagePosition = .imageOnly
        backButton.contentTintColor = .secondaryLabelColor
        backButton.target = self
        backButton.action = #selector(goBack)
        backButton.translatesAutoresizingMaskIntoConstraints = false
        toolbar.addSubview(backButton)

        // Forward button
        forwardButton.image = NSImage(systemSymbolName: "chevron.right", accessibilityDescription: "Forward")
        forwardButton.bezelStyle = .inline
        forwardButton.isBordered = false
        forwardButton.imagePosition = .imageOnly
        forwardButton.contentTintColor = .secondaryLabelColor
        forwardButton.target = self
        forwardButton.action = #selector(goForward)
        forwardButton.translatesAutoresizingMaskIntoConstraints = false
        toolbar.addSubview(forwardButton)

        // Reload button
        reloadButton.image = NSImage(systemSymbolName: "arrow.clockwise", accessibilityDescription: "Reload")
        reloadButton.bezelStyle = .inline
        reloadButton.isBordered = false
        reloadButton.imagePosition = .imageOnly
        reloadButton.contentTintColor = .secondaryLabelColor
        reloadButton.target = self
        reloadButton.action = #selector(reloadPage)
        reloadButton.translatesAutoresizingMaskIntoConstraints = false
        toolbar.addSubview(reloadButton)

        // URL bar
        urlBar.font = .monospacedSystemFont(ofSize: 11, weight: .regular)
        urlBar.textColor = .labelColor
        urlBar.backgroundColor = NSColor(white: 0.18, alpha: 1)
        urlBar.drawsBackground = true
        urlBar.isBordered = true
        urlBar.isBezeled = true
        urlBar.bezelStyle = .roundedBezel
        urlBar.placeholderString = "http://localhost:3000"
        urlBar.target = self
        urlBar.action = #selector(urlBarAction)
        urlBar.translatesAutoresizingMaskIntoConstraints = false
        toolbar.addSubview(urlBar)

        // Focus toggle (terminal ↔ browser)
        focusToggle.image = NSImage(systemSymbolName: "rectangle.split.2x1", accessibilityDescription: "Toggle Focus")
        focusToggle.bezelStyle = .inline
        focusToggle.isBordered = false
        focusToggle.imagePosition = .imageOnly
        focusToggle.contentTintColor = .secondaryLabelColor
        focusToggle.target = self
        focusToggle.action = #selector(toggleFocus)
        focusToggle.translatesAutoresizingMaskIntoConstraints = false
        toolbar.addSubview(focusToggle)
    }

    private func setupWebView() {
        webView.navigationDelegate = self
        webView.translatesAutoresizingMaskIntoConstraints = false
        webView.allowsBackForwardNavigationGestures = true
        addSubview(webView)
    }

    private func layoutViews() {
        NSLayoutConstraint.activate([
            toolbar.topAnchor.constraint(equalTo: topAnchor),
            toolbar.leadingAnchor.constraint(equalTo: leadingAnchor),
            toolbar.trailingAnchor.constraint(equalTo: trailingAnchor),
            toolbar.heightAnchor.constraint(equalToConstant: 32),

            backButton.leadingAnchor.constraint(equalTo: toolbar.leadingAnchor, constant: 6),
            backButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            backButton.widthAnchor.constraint(equalToConstant: 22),

            forwardButton.leadingAnchor.constraint(equalTo: backButton.trailingAnchor, constant: 2),
            forwardButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            forwardButton.widthAnchor.constraint(equalToConstant: 22),

            reloadButton.leadingAnchor.constraint(equalTo: forwardButton.trailingAnchor, constant: 2),
            reloadButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            reloadButton.widthAnchor.constraint(equalToConstant: 22),

            urlBar.leadingAnchor.constraint(equalTo: reloadButton.trailingAnchor, constant: 6),
            urlBar.trailingAnchor.constraint(equalTo: focusToggle.leadingAnchor, constant: -6),
            urlBar.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            urlBar.heightAnchor.constraint(equalToConstant: 22),

            focusToggle.trailingAnchor.constraint(equalTo: toolbar.trailingAnchor, constant: -6),
            focusToggle.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            focusToggle.widthAnchor.constraint(equalToConstant: 22),

            webView.topAnchor.constraint(equalTo: toolbar.bottomAnchor),
            webView.leadingAnchor.constraint(equalTo: leadingAnchor),
            webView.trailingAnchor.constraint(equalTo: trailingAnchor),
            webView.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    // MARK: - Public API

    /// Navigate to a URL string.
    func navigate(to urlString: String) {
        var normalized = urlString.trimmingCharacters(in: .whitespaces)
        if !normalized.hasPrefix("http://") && !normalized.hasPrefix("https://") {
            normalized = "http://\(normalized)"
        }
        guard let url = URL(string: normalized) else { return }
        webView.load(URLRequest(url: url))
        urlBar.stringValue = normalized
    }

    /// Navigate to a URL.
    func navigate(to url: URL) {
        webView.load(URLRequest(url: url))
        urlBar.stringValue = url.absoluteString
    }

    /// Execute JavaScript in the browser context and return result.
    func evaluateJavaScript(_ script: String, completion: ((Any?, Error?) -> Void)? = nil) {
        webView.evaluateJavaScript(script, completionHandler: completion)
    }

    /// Take a screenshot of the current page.
    func takeScreenshot(completion: @escaping (NSImage?) -> Void) {
        let config = WKSnapshotConfiguration()
        webView.takeSnapshot(with: config) { image, error in
            if let error = error {
                NSLog("[smux-browser] screenshot error: %@", error.localizedDescription)
            }
            completion(image)
        }
    }

    /// Get the page title.
    var pageTitle: String? { webView.title }

    /// Check if webview is focused.
    var isWebViewFocused: Bool {
        window?.firstResponder === webView
    }

    // MARK: - Actions

    @objc private func goBack() { webView.goBack() }
    @objc private func goForward() { webView.goForward() }
    @objc private func reloadPage() { webView.reload() }

    @objc private func urlBarAction() {
        navigate(to: urlBar.stringValue)
    }

    @objc private func toggleFocus() {
        // Toggle between webview and terminal focus
        if isWebViewFocused {
            // Find terminal view in window and focus it
            if let term = window?.contentView?.findSubview(ofType: GhosttyTerminalView.self) {
                window?.makeFirstResponder(term)
            }
        } else {
            window?.makeFirstResponder(webView)
        }
    }
}

// MARK: - WKNavigationDelegate

extension BrowserPanelView: WKNavigationDelegate {
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        urlBar.stringValue = webView.url?.absoluteString ?? ""
        onPageLoaded?(webView.url)
        updateNavButtons()
    }

    func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation!) {
        urlBar.stringValue = webView.url?.absoluteString ?? ""
    }

    private func updateNavButtons() {
        backButton.isEnabled = webView.canGoBack
        forwardButton.isEnabled = webView.canGoForward
    }
}

// MARK: - WKScriptMessageHandler (console.log bridge)

extension BrowserPanelView: WKScriptMessageHandler {
    func userContentController(_ userContentController: WKUserContentController,
                               didReceive message: WKScriptMessage) {
        if message.name == "consoleLog", let body = message.body as? String {
            onConsoleMessage?(body)
        }
    }
}

// MARK: - Weak WKScriptMessageHandler (prevents retain cycle)

private class WeakScriptHandler: NSObject, WKScriptMessageHandler {
    weak var delegate: WKScriptMessageHandler?
    init(_ delegate: WKScriptMessageHandler) { self.delegate = delegate }
    func userContentController(_ c: WKUserContentController, didReceive message: WKScriptMessage) {
        delegate?.userContentController(c, didReceive: message)
    }
}

// MARK: - NSView helper

extension NSView {
    func findSubview<T: NSView>(ofType type: T.Type) -> T? {
        for sub in subviews {
            if let match = sub as? T { return match }
            if let match = sub.findSubview(ofType: type) { return match }
        }
        return nil
    }
}
