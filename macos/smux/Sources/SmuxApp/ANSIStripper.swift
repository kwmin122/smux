import Foundation

enum ANSIStripper {
    // Covers: CSI sequences (\e[...X), OSC sequences (\e]...\a or \e]...\e\\), standalone ESC+char
    private static let ansiRegex: Regex<Substring> = {
        try! Regex(#"\x1B(?:[@-Z\\-_]|\[[0-?]*[ -\/]*[@-~]|\][^\x07\x1B]*(?:\x07|\x1B\\))"#)
    }()

    static func strip(_ input: String) -> String {
        input.replacing(ansiRegex, with: "")
    }
}
