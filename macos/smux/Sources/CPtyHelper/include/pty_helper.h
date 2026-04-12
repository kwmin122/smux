#ifndef PTY_HELPER_H
#define PTY_HELPER_H

#include <sys/types.h>

/// Fork a child shell in a new PTY. Returns master fd, sets child_pid.
int smux_forkpty(pid_t *child_pid, unsigned short rows, unsigned short cols);

/// Resize PTY window.
int smux_pty_resize(int master_fd, unsigned short rows, unsigned short cols);

#endif
