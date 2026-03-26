import Foundation
import CPtyHelper

/// Manages a PTY pair for HOST_MANAGED ghostty mode.
/// Reads raw bytes from PTY master fd, delivers them via onOutput callback.
/// Writes bytes to PTY master fd via write() for keyboard input relay.
class PTYManager {
    private var masterFd: Int32 = -1
    private var childPid: pid_t = 0
    private var readSource: DispatchSourceRead?
    private var running = false

    /// Called with raw PTY output bytes (on main thread).
    var onOutput: ((UnsafePointer<UInt8>, Int) -> Void)?

    /// Called when child process exits.
    var onExit: ((Int32) -> Void)?

    /// Start PTY: fork child shell, begin reading master fd.
    func start(rows: UInt16, cols: UInt16) -> Bool {
        var pid: pid_t = 0
        let fd = smux_forkpty(&pid, rows, cols)
        guard fd >= 0 else {
            NSLog("[pty] forkpty failed")
            return false
        }

        masterFd = fd
        childPid = pid
        running = true

        NSLog("[pty] started — master_fd=%d child_pid=%d", fd, pid)

        // Async read from master fd
        let source = DispatchSource.makeReadSource(fileDescriptor: fd, queue: .global(qos: .userInteractive))
        source.setEventHandler { [weak self] in
            self?.readFromMaster()
        }
        source.setCancelHandler { [weak self] in
            guard let self = self else { return }
            if self.masterFd >= 0 {
                close(self.masterFd)
                self.masterFd = -1
            }
        }
        source.resume()
        readSource = source

        // Monitor child exit
        DispatchQueue.global(qos: .utility).async { [weak self] in
            var status: Int32 = 0
            waitpid(pid, &status, 0)
            DispatchQueue.main.async {
                self?.onExit?(status)
            }
        }

        return true
    }

    /// Write bytes to PTY master fd (→ child stdin).
    func writeToPTY(_ data: UnsafePointer<UInt8>, length: Int) {
        guard masterFd >= 0 else { return }
        _ = write(masterFd, data, length)
    }

    /// Write string to PTY master fd.
    func writeString(_ text: String) {
        guard masterFd >= 0 else { return }
        text.withCString { ptr in
            let len = strlen(ptr)
            _ = write(masterFd, ptr, len)
        }
    }

    /// Resize PTY.
    func resize(rows: UInt16, cols: UInt16) {
        guard masterFd >= 0 else { return }
        smux_pty_resize(masterFd, rows, cols)
    }

    /// Stop PTY: cancel read source, kill child, close fd.
    func stop() {
        guard running else { return }
        running = false

        readSource?.cancel()
        readSource = nil

        if childPid > 0 {
            kill(childPid, SIGHUP)
            childPid = 0
        }

        // fd closed by source cancel handler, or here if source wasn't set
        if masterFd >= 0 {
            close(masterFd)
            masterFd = -1
        }

        NSLog("[pty] stopped")
    }

    deinit {
        stop()
    }

    // MARK: - Private

    private func readFromMaster() {
        var buf = [UInt8](repeating: 0, count: 8192)
        let n = read(masterFd, &buf, buf.count)
        guard n > 0 else {
            if n == 0 || (n < 0 && errno != EAGAIN && errno != EINTR) {
                // EOF or error — child exited
                DispatchQueue.main.async { [weak self] in
                    self?.stop()
                }
            }
            return
        }

        // Deliver raw bytes on main thread (ghostty_surface_write_buffer needs main thread for Metal)
        let bytes = Array(buf.prefix(n))
        DispatchQueue.main.async { [weak self] in
            bytes.withUnsafeBufferPointer { ptr in
                guard let base = ptr.baseAddress else { return }
                self?.onOutput?(base, n)
            }
        }
    }
}
