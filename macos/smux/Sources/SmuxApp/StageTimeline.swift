import AppKit

/// Horizontal stage timeline bar: Ideate → Plan → Execute → Harden
class StageTimeline: NSView {
    private let stages = ["Ideate", "Plan", "Execute", "Harden"]
    private var currentIndex = 0
    private var labels: [NSTextField] = []
    private var dots: [NSView] = []

    override init(frame: NSRect) {
        super.init(frame: NSRect(x: 0, y: 0, width: frame.width, height: 28))
        setupUI()
    }
    required init?(coder: NSCoder) { fatalError() }

    private func setupUI() {
        wantsLayer = true
        layer?.backgroundColor = NSColor(white: 0.12, alpha: 1).cgColor

        let stack = NSStackView()
        stack.orientation = .horizontal
        stack.spacing = 4
        stack.translatesAutoresizingMaskIntoConstraints = false

        for (i, name) in stages.enumerated() {
            let dot = NSView()
            dot.wantsLayer = true
            dot.layer?.cornerRadius = 4
            dot.layer?.backgroundColor = (i == 0 ? NSColor.systemGreen : NSColor.systemGray).cgColor
            dot.translatesAutoresizingMaskIntoConstraints = false
            NSLayoutConstraint.activate([
                dot.widthAnchor.constraint(equalToConstant: 8),
                dot.heightAnchor.constraint(equalToConstant: 8),
            ])
            dots.append(dot)

            let label = NSTextField(labelWithString: name)
            label.font = .monospacedSystemFont(ofSize: 9, weight: i == 0 ? .bold : .regular)
            label.textColor = i == 0 ? .systemGreen : .tertiaryLabelColor
            labels.append(label)

            stack.addArrangedSubview(dot)
            stack.addArrangedSubview(label)

            if i < stages.count - 1 {
                let arrow = NSTextField(labelWithString: "→")
                arrow.font = .monospacedSystemFont(ofSize: 9, weight: .regular)
                arrow.textColor = .tertiaryLabelColor
                stack.addArrangedSubview(arrow)
            }
        }

        addSubview(stack)
        NSLayoutConstraint.activate([
            stack.centerXAnchor.constraint(equalTo: centerXAnchor),
            stack.centerYAnchor.constraint(equalTo: centerYAnchor),
        ])
    }

    func setCurrentStage(_ index: Int) {
        currentIndex = index
        for (i, dot) in dots.enumerated() {
            if i < index {
                dot.layer?.backgroundColor = NSColor.systemGreen.cgColor
                labels[i].textColor = .systemGreen
                labels[i].font = .monospacedSystemFont(ofSize: 9, weight: .regular)
            } else if i == index {
                dot.layer?.backgroundColor = NSColor.systemBlue.cgColor
                labels[i].textColor = .white
                labels[i].font = .monospacedSystemFont(ofSize: 9, weight: .bold)
            } else {
                dot.layer?.backgroundColor = NSColor.systemGray.cgColor
                labels[i].textColor = .tertiaryLabelColor
                labels[i].font = .monospacedSystemFont(ofSize: 9, weight: .regular)
            }
        }
    }
}
