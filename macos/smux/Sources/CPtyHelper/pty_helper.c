// PTY helper — C bridge for forkpty() which Swift cannot call directly.
// This is the same approach used by iTerm2, Terminal.app, and every serious
// macOS terminal emulator. forkpty() is the gold standard for PTY creation.

#include <util.h>
#include <unistd.h>
#include <stdlib.h>
#include <signal.h>
#include <sys/ioctl.h>
#include <termios.h>
#include <string.h>
#include <errno.h>

/// Create a PTY pair, fork a child shell, and return the master fd.
/// Returns master fd on success, -1 on failure. Sets *child_pid.
int smux_forkpty(pid_t *child_pid, unsigned short rows, unsigned short cols) {
    int master_fd;
    struct winsize ws;
    ws.ws_row = rows > 0 ? rows : 24;
    ws.ws_col = cols > 0 ? cols : 80;
    ws.ws_xpixel = 0;
    ws.ws_ypixel = 0;

    pid_t pid = forkpty(&master_fd, NULL, NULL, &ws);
    if (pid < 0) {
        return -1; // fork failed
    }

    if (pid == 0) {
        // Child process — become the login shell
        signal(SIGPIPE, SIG_DFL);
        signal(SIGCHLD, SIG_DFL);

        // Set TERM for proper terminal emulation
        setenv("TERM", "xterm-256color", 1);
        setenv("COLORTERM", "truecolor", 1);

        const char *shell = getenv("SHELL");
        if (!shell) shell = "/bin/zsh";

        // Execute as login shell (prepend - to argv[0])
        char login_shell[256];
        const char *base = strrchr(shell, '/');
        base = base ? base + 1 : shell;
        snprintf(login_shell, sizeof(login_shell), "-%s", base);

        execlp(shell, login_shell, NULL);
        _exit(127); // exec failed
    }

    // Parent — return master fd
    *child_pid = pid;
    return master_fd;
}

/// Resize the PTY window.
int smux_pty_resize(int master_fd, unsigned short rows, unsigned short cols) {
    struct winsize ws;
    ws.ws_row = rows;
    ws.ws_col = cols;
    ws.ws_xpixel = 0;
    ws.ws_ypixel = 0;
    return ioctl(master_fd, TIOCSWINSZ, &ws);
}
