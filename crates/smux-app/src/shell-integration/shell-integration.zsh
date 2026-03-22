# smux Shell Integration for Zsh
# Emits OSC 633 escape sequences for command tracking.
# Sourced automatically by smux PTY sessions.

# Guard: only load once and only inside smux
[[ -n "$SMUX_SHELL_INTEGRATION" ]] && return
export SMUX_SHELL_INTEGRATION=1

# OSC 633 helpers
__smux_osc633() {
  printf '\e]633;%s\a' "$1"
}

# A = Prompt Start
__smux_prompt_start() {
  __smux_osc633 "A"
}

# B = Prompt End (command input begins)
__smux_prompt_end() {
  __smux_osc633 "B"
}

# C = Pre-Execution (command is about to run)
__smux_preexec() {
  __smux_osc633 "C"
  # E = Command Line (send the actual command text)
  __smux_osc633 "E;${1}"
}

# D = Execution Finished with exit code
__smux_precmd() {
  local exit_code=$?
  __smux_osc633 "D;${exit_code}"
  # P = Property: Current Working Directory
  __smux_osc633 "P;Cwd=${PWD}"
  # Then emit prompt start for next prompt
  __smux_prompt_start
}

# Hook into Zsh's precmd / preexec arrays
autoload -Uz add-zsh-hook
add-zsh-hook precmd __smux_precmd
add-zsh-hook preexec __smux_preexec

# Emit initial prompt markers
__smux_prompt_start

# Set PS1 to include prompt end marker after the prompt
# We wrap the existing PROMPT so the B marker fires after prompt renders
if [[ -n "$SMUX_INJECT_PROMPT" ]]; then
  PROMPT="${PROMPT}%{$(__smux_osc633 'B')%}"
fi
