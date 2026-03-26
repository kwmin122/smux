import Foundation

/// Swift client for communicating with smux-daemon over Unix socket.
/// Wire format: 4-byte big-endian length + JSON payload (same as Rust side).
class SmuxIpcClient {
    private var connection: FileHandle?
    private let socketPath: String

    init() {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        self.socketPath = "\(home)/.smux/smux.sock"
    }

    /// Connect to the daemon socket with 2-second timeout.
    func connect() throws {
        let fd = socket(AF_UNIX, SOCK_STREAM, 0)
        guard fd >= 0 else {
            throw IpcError.socketCreationFailed
        }

        // Set 2-second send/receive timeout to prevent blocking
        var tv = timeval(tv_sec: 2, tv_usec: 0)
        setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, socklen_t(MemoryLayout<timeval>.size))
        setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, socklen_t(MemoryLayout<timeval>.size))

        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        let pathBytes = socketPath.utf8CString
        withUnsafeMutablePointer(to: &addr.sun_path) { ptr in
            let raw = UnsafeMutableRawPointer(ptr)
            pathBytes.withUnsafeBufferPointer { buf in
                raw.copyMemory(from: buf.baseAddress!, byteCount: min(buf.count, 104))
            }
        }

        let connectResult = withUnsafePointer(to: &addr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                Darwin.connect(fd, sockPtr, socklen_t(MemoryLayout<sockaddr_un>.size))
            }
        }

        guard connectResult == 0 else {
            Darwin.close(fd)
            throw IpcError.connectionFailed(errno: Int(Darwin.errno))
        }

        self.connection = FileHandle(fileDescriptor: fd, closeOnDealloc: true)
    }

    /// Async wrapper — never blocks main thread.
    func checkDaemonAsync(completion: @escaping (Bool) -> Void) {
        DispatchQueue.global(qos: .utility).async { [self] in
            let running = isDaemonRunning
            DispatchQueue.main.async { completion(running) }
        }
    }

    /// Send a JSON message with length prefix.
    func send(_ message: [String: Any]) throws {
        guard let conn = connection else { throw IpcError.notConnected }

        let jsonData = try JSONSerialization.data(withJSONObject: message)
        var length = UInt32(jsonData.count).bigEndian
        let lengthData = Data(bytes: &length, count: 4)

        conn.write(lengthData)
        conn.write(jsonData)
    }

    /// Receive a length-prefixed JSON message.
    func receive() throws -> [String: Any] {
        guard let conn = connection else { throw IpcError.notConnected }

        // Read 4-byte length
        let lengthData = conn.readData(ofLength: 4)
        guard lengthData.count == 4 else { throw IpcError.connectionClosed }

        let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self).bigEndian }
        guard length <= 1_048_576 else { throw IpcError.messageTooLarge }

        // Read payload
        let payload = conn.readData(ofLength: Int(length))
        guard payload.count == Int(length) else { throw IpcError.connectionClosed }

        guard let json = try JSONSerialization.jsonObject(with: payload) as? [String: Any] else {
            throw IpcError.invalidJson
        }
        return json
    }

    /// Send ListSessions and return the result.
    func listSessions() throws -> [[String: Any]] {
        try send(["ListSessions": [:]])
        let response = try receive()

        if let sessionList = response["SessionList"] as? [String: Any],
           let sessions = sessionList["sessions"] as? [[String: Any]] {
            return sessions
        }
        if let error = response["Error"] as? [String: Any],
           let message = error["message"] as? String {
            throw IpcError.daemonError(message)
        }
        return []
    }

    /// Check if daemon is running by attempting connection.
    var isDaemonRunning: Bool {
        do {
            try connect()
            disconnect()
            return true
        } catch {
            return false
        }
    }

    func disconnect() {
        connection = nil
    }

    deinit {
        disconnect()
    }
}

enum IpcError: Error, LocalizedError {
    case socketCreationFailed
    case connectionFailed(errno: Int)
    case notConnected
    case connectionClosed
    case messageTooLarge
    case invalidJson
    case daemonError(String)

    var errorDescription: String? {
        switch self {
        case .socketCreationFailed: return "Failed to create socket"
        case .connectionFailed(let e): return "Connection failed (errno: \(e))"
        case .notConnected: return "Not connected to daemon"
        case .connectionClosed: return "Connection closed"
        case .messageTooLarge: return "Message too large"
        case .invalidJson: return "Invalid JSON response"
        case .daemonError(let msg): return "Daemon error: \(msg)"
        }
    }
}
