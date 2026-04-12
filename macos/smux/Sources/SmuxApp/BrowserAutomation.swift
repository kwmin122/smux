import AppKit
import WebKit

/// Browser automation engine — DOM snapshots, click, form input, screenshots.
/// Operates on a BrowserPanelView's WKWebView via JavaScript injection.
/// Modeled after cmux's browser automation command set.
class BrowserAutomation {
    private weak var browserPanel: BrowserPanelView?

    init(browserPanel: BrowserPanelView) {
        self.browserPanel = browserPanel
    }

    // MARK: - DOM Snapshot

    /// Get a full DOM snapshot as HTML string.
    func domSnapshot(completion: @escaping (Result<String, AutomationError>) -> Void) {
        executeJS("document.documentElement.outerHTML") { result in
            switch result {
            case .success(let value):
                if let html = value as? String {
                    completion(.success(html))
                } else {
                    completion(.failure(.unexpectedResult))
                }
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    /// Get a simplified DOM tree with element structure.
    func domTree(selector: String = "body", completion: @escaping (Result<String, AutomationError>) -> Void) {
        let script = """
        (function() {
            function walk(el, depth) {
                if (!el || depth > 10) return '';
                var indent = '  '.repeat(depth);
                var tag = el.tagName ? el.tagName.toLowerCase() : '#text';
                var id = el.id ? '#' + el.id : '';
                var cls = el.className && typeof el.className === 'string'
                    ? '.' + el.className.trim().split(/\\s+/).join('.') : '';
                var text = '';
                if (el.childNodes.length === 1 && el.childNodes[0].nodeType === 3) {
                    text = ' "' + el.childNodes[0].textContent.trim().substring(0, 50) + '"';
                }
                var line = indent + '<' + tag + id + cls + '>' + text + '\\n';
                for (var i = 0; i < el.children.length && i < 20; i++) {
                    line += walk(el.children[i], depth + 1);
                }
                return line;
            }
            var root = document.querySelector('\(selector.escapedForJS)');
            return root ? walk(root, 0) : 'selector not found';
        })()
        """
        executeJS(script) { result in
            switch result {
            case .success(let value):
                completion(.success(value as? String ?? ""))
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    // MARK: - Element Interaction

    /// Click an element matching the CSS selector.
    func click(selector: String, completion: @escaping (Result<Void, AutomationError>) -> Void) {
        let script = """
        (function() {
            var el = document.querySelector('\(selector.escapedForJS)');
            if (!el) return 'NOT_FOUND';
            el.click();
            return 'OK';
        })()
        """
        executeJS(script) { result in
            switch result {
            case .success(let value):
                if let str = value as? String, str == "OK" {
                    completion(.success(()))
                } else {
                    completion(.failure(.elementNotFound(selector)))
                }
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    /// Double-click an element.
    func doubleClick(selector: String, completion: @escaping (Result<Void, AutomationError>) -> Void) {
        let script = """
        (function() {
            var el = document.querySelector('\(selector.escapedForJS)');
            if (!el) return 'NOT_FOUND';
            el.dispatchEvent(new MouseEvent('dblclick', {bubbles: true}));
            return 'OK';
        })()
        """
        executeJS(script) { result in
            switch result {
            case .success(let value):
                completion(value as? String == "OK" ? .success(()) : .failure(.elementNotFound(selector)))
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    /// Type text into an input field.
    func type(selector: String, text: String, completion: @escaping (Result<Void, AutomationError>) -> Void) {
        let script = """
        (function() {
            var el = document.querySelector('\(selector.escapedForJS)');
            if (!el) return 'NOT_FOUND';
            el.focus();
            el.value = '\(text.escapedForJS)';
            el.dispatchEvent(new Event('input', {bubbles: true}));
            el.dispatchEvent(new Event('change', {bubbles: true}));
            return 'OK';
        })()
        """
        executeJS(script) { result in
            switch result {
            case .success(let value):
                completion(value as? String == "OK" ? .success(()) : .failure(.elementNotFound(selector)))
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    /// Fill a form field and optionally submit.
    func fillForm(fields: [(selector: String, value: String)], submit: Bool = false,
                  completion: @escaping (Result<Void, AutomationError>) -> Void) {
        let assignments = fields.map { field in
            """
            var el = document.querySelector('\(field.selector.escapedForJS)');
            if (!el) return 'NOT_FOUND:\(field.selector.escapedForJS)';
            el.value = '\(field.value.escapedForJS)';
            el.dispatchEvent(new Event('input', {bubbles: true}));
            el.dispatchEvent(new Event('change', {bubbles: true}));
            """
        }.joined(separator: "\n")

        let submitCode = submit ? """
        var form = document.querySelector('form');
        if (form) form.submit();
        """ : ""

        let script = """
        (function() {
            \(assignments)
            \(submitCode)
            return 'OK';
        })()
        """
        executeJS(script) { result in
            switch result {
            case .success(let value):
                if let str = value as? String, str == "OK" {
                    completion(.success(()))
                } else if let str = value as? String, str.hasPrefix("NOT_FOUND:") {
                    completion(.failure(.elementNotFound(String(str.dropFirst(10)))))
                } else {
                    completion(.failure(.unexpectedResult))
                }
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    // MARK: - Query

    /// Get text content of an element.
    func getText(selector: String, completion: @escaping (Result<String, AutomationError>) -> Void) {
        let script = """
        (function() {
            var el = document.querySelector('\(selector.escapedForJS)');
            return el ? el.textContent : null;
        })()
        """
        executeJS(script) { result in
            switch result {
            case .success(let value):
                if let text = value as? String {
                    completion(.success(text))
                } else {
                    completion(.failure(.elementNotFound(selector)))
                }
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    /// Get attribute value of an element.
    func getAttribute(selector: String, attribute: String,
                      completion: @escaping (Result<String?, AutomationError>) -> Void) {
        let script = """
        (function() {
            var el = document.querySelector('\(selector.escapedForJS)');
            return el ? el.getAttribute('\(attribute.escapedForJS)') : '__NOT_FOUND__';
        })()
        """
        executeJS(script) { result in
            switch result {
            case .success(let value):
                if let str = value as? String, str == "__NOT_FOUND__" {
                    completion(.failure(.elementNotFound(selector)))
                } else {
                    completion(.success(value as? String))
                }
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    /// Check if an element exists.
    func elementExists(selector: String, completion: @escaping (Result<Bool, AutomationError>) -> Void) {
        let script = "document.querySelector('\(selector.escapedForJS)') !== null"
        executeJS(script) { result in
            switch result {
            case .success(let value):
                completion(.success(value as? Bool ?? false))
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    /// Wait for an element to appear (polls every 200ms, max timeout).
    func waitForElement(selector: String, timeout: TimeInterval = 5.0,
                        completion: @escaping (Result<Void, AutomationError>) -> Void) {
        let start = Date()
        func poll() {
            elementExists(selector: selector) { result in
                switch result {
                case .success(true):
                    completion(.success(()))
                case .success(false):
                    if Date().timeIntervalSince(start) < timeout {
                        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) { poll() }
                    } else {
                        completion(.failure(.timeout(selector)))
                    }
                case .failure(let error):
                    completion(.failure(error))
                }
            }
        }
        poll()
    }

    // MARK: - Screenshots

    /// Take a screenshot of the browser panel.
    func screenshot(completion: @escaping (Result<NSImage, AutomationError>) -> Void) {
        browserPanel?.takeScreenshot { image in
            if let image = image {
                completion(.success(image))
            } else {
                completion(.failure(.screenshotFailed))
            }
        }
    }

    /// Take a screenshot and save to file.
    func screenshotToFile(path: String, completion: @escaping (Result<String, AutomationError>) -> Void) {
        screenshot { result in
            switch result {
            case .success(let image):
                guard let tiffData = image.tiffRepresentation,
                      let bitmap = NSBitmapImageRep(data: tiffData),
                      let pngData = bitmap.representation(using: .png, properties: [:]) else {
                    completion(.failure(.screenshotFailed))
                    return
                }
                do {
                    try pngData.write(to: URL(fileURLWithPath: path))
                    completion(.success(path))
                } catch {
                    completion(.failure(.fileWriteFailed(error.localizedDescription)))
                }
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    // MARK: - Navigation

    /// Get the current URL.
    var currentURL: String? {
        browserPanel?.currentURL?.absoluteString
    }

    /// Get the current page title.
    var pageTitle: String? {
        browserPanel?.pageTitle
    }

    /// Navigate to a URL.
    func navigate(to url: String) {
        browserPanel?.navigate(to: url)
    }

    // MARK: - Scroll

    /// Scroll to element.
    func scrollTo(selector: String, completion: @escaping (Result<Void, AutomationError>) -> Void) {
        let script = """
        (function() {
            var el = document.querySelector('\(selector.escapedForJS)');
            if (!el) return 'NOT_FOUND';
            el.scrollIntoView({behavior: 'smooth', block: 'center'});
            return 'OK';
        })()
        """
        executeJS(script) { result in
            switch result {
            case .success(let value):
                completion(value as? String == "OK" ? .success(()) : .failure(.elementNotFound(selector)))
            case .failure(let error):
                completion(.failure(error))
            }
        }
    }

    // MARK: - JavaScript Execution

    /// Execute raw JavaScript.
    func executeRawJS(_ script: String, completion: @escaping (Result<Any?, AutomationError>) -> Void) {
        executeJS(script, completion: completion)
    }

    // MARK: - Internal

    private func executeJS(_ script: String, completion: @escaping (Result<Any?, AutomationError>) -> Void) {
        guard let panel = browserPanel else {
            completion(.failure(.noBrowser))
            return
        }
        panel.evaluateJavaScript(script) { value, error in
            if let error = error {
                completion(.failure(.jsError(error.localizedDescription)))
            } else {
                completion(.success(value))
            }
        }
    }
}

// MARK: - Error Types

enum AutomationError: Error, LocalizedError {
    case noBrowser
    case elementNotFound(String)
    case jsError(String)
    case unexpectedResult
    case timeout(String)
    case screenshotFailed
    case fileWriteFailed(String)

    var errorDescription: String? {
        switch self {
        case .noBrowser: return "No browser panel open"
        case .elementNotFound(let sel): return "Element not found: \(sel)"
        case .jsError(let msg): return "JavaScript error: \(msg)"
        case .unexpectedResult: return "Unexpected result"
        case .timeout(let sel): return "Timeout waiting for: \(sel)"
        case .screenshotFailed: return "Screenshot failed"
        case .fileWriteFailed(let msg): return "File write failed: \(msg)"
        }
    }
}

// MARK: - String JS Escaping

extension String {
    var escapedForJS: String {
        self.replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "'", with: "\\'")
            .replacingOccurrences(of: "\"", with: "\\\"")
            .replacingOccurrences(of: "\n", with: "\\n")
            .replacingOccurrences(of: "\r", with: "\\r")
            .replacingOccurrences(of: "\0", with: "\\0")
            .replacingOccurrences(of: "\u{2028}", with: "\\u2028")
            .replacingOccurrences(of: "\u{2029}", with: "\\u2029")
    }
}
