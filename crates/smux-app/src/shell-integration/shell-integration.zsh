# smux Shell Integration for Zsh
# Emits OSC 633 escape sequences for command tracking.
# Sourced automatically by smux PTY sessions.

# Guard: only load once (use a different var than the env marker)
[[ -n "$__SMUX_INTEGRATION_LOADED" ]] && return
export __SMUX_INTEGRATION_LOADED=1

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

# Sanitize text for OSC sequences (strip BEL, ESC, and other control chars)
__smux_sanitize() {
  local text="$1"
  # Remove ESC (\e / \x1b), BEL (\a / \x07), and ST (\e\\) to prevent OSC injection
  text="${text//[$'\e']/}"
  text="${text//[$'\a']/}"
  printf '%s' "$text"
}

# C = Pre-Execution (command is about to run)
__smux_preexec() {
  __smux_osc633 "C"
  # E = Command Line (send the sanitized command text)
  local safe_cmd
  safe_cmd=$(__smux_sanitize "$1")
  __smux_osc633 "E;${safe_cmd}"
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
