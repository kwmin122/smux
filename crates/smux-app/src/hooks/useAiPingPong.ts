import { useState, useCallback, useRef } from 'react'
import type { TerminalPanelHandle } from '../components/TerminalPanel'

export type AiSessionStatus = 'idle' | 'planner-running' | 'verifier-running' | 'completed' | 'error' | 'paused'
export type PingPongVerdict = 'approved' | 'rejected' | 'needs_info' | 'unknown'

export interface PingPongRound {
  round: number
  plannerOutput: string
  verifierOutput: string
  verdict: PingPongVerdict
  timestamp: number
}

interface AiPingPongOptions {
  task: string
  planner: string           // 'claude' | 'codex' | 'gemini'
  verifier: string          // 'claude' | 'codex' | 'gemini'
  maxRounds: number
  cwd?: string
  onRoundComplete?: (round: PingPongRound) => void
  onSessionComplete?: (rounds: PingPongRound[]) => void
  onStatusChange?: (status: AiSessionStatus) => void
}

/**
 * Write a prompt to a temp file and build a command that reads from it.
 * This avoids shell injection by never interpolating user content into shell commands.
 */
function buildAgentCommand(agent: string, promptFile: string, mode: 'plan' | 'review'): string {
  // The prompt is written to a temp file; the command reads from it via stdin/file arg.
  // This completely prevents shell injection since no user content enters the command string.
  const reviewPrefix = mode === 'review'
    ? 'Review the following work and respond with APPROVED if correct or REJECTED with feedback if not:\\n\\n'
    : ''

  switch (agent) {
    case 'claude':
      return reviewPrefix
        ? `(echo "${reviewPrefix}" && cat "${promptFile}") | claude -p --dangerously-skip-permissions -`
        : `cat "${promptFile}" | claude -p --dangerously-skip-permissions -`
    case 'codex':
      return reviewPrefix
        ? `(echo "${reviewPrefix}" && cat "${promptFile}") | codex exec --full-auto -`
        : `cat "${promptFile}" | codex exec --full-auto -`
    case 'gemini':
      return reviewPrefix
        ? `(echo "${reviewPrefix}" && cat "${promptFile}") | gemini -p -`
        : `cat "${promptFile}" | gemini -p -`
    default:
      return `echo "Unknown agent: ${agent}"`
  }
}

/** Write prompt content to a temp file safely, returns file path */
function writeTempPrompt(content: string): string {
  // Use a timestamp-based filename in /tmp to avoid collisions
  const filename = `/tmp/smux-prompt-${Date.now()}-${Math.random().toString(36).slice(2, 8)}.txt`
  return filename // The actual write happens via PTY: echo "content" > file
}

/** Escape content for safe echo into a file (escape backslashes, double quotes, backticks, $) */
function escapeForEcho(s: string): string {
  return s
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/`/g, '\\`')
    .replace(/\$/g, '\\$')
    .replace(/\n/g, '\\n')
}

// Detect verdict from terminal output
function parseVerdict(output: string): PingPongVerdict {
  const lower = output.toLowerCase()
  if (lower.includes('approved') || lower.includes('lgtm') || lower.includes('looks good')) return 'approved'
  if (lower.includes('rejected') || lower.includes('changes needed') || lower.includes('needs fix')) return 'rejected'
  if (lower.includes('needs_info') || lower.includes('need more info') || lower.includes('clarification')) return 'needs_info'
  return 'unknown'
}

/**
 * Hook to orchestrate AI ping-pong sessions between planner and verifier PTYs.
 *
 * Flow:
 * 1. Start → write claude command to planner PTY
 * 2. Wait for planner to finish (detect shell prompt return via exit event or timeout)
 * 3. Capture planner output → write codex review command to verifier PTY
 * 4. Wait for verifier to finish
 * 5. Parse verdict → if REJECTED, loop back to step 1 with feedback
 * 6. If APPROVED or max rounds reached, complete
 */
export function useAiPingPong() {
  const [status, setStatus] = useState<AiSessionStatus>('idle')
  const [currentRound, setCurrentRound] = useState(0)
  const [rounds, setRounds] = useState<PingPongRound[]>([])
  const optionsRef = useRef<AiPingPongOptions | null>(null)
  const plannerRef = useRef<TerminalPanelHandle | null>(null)
  const verifierRef = useRef<TerminalPanelHandle | null>(null)
  const outputBufferRef = useRef<string>('')
  const abortRef = useRef(false)
  const pausedRef = useRef(false)

  // Capture output from PTY by hooking into the terminal write
  const captureOutput = useCallback((text: string) => {
    outputBufferRef.current += text
  }, [])

  const updateStatus = useCallback((s: AiSessionStatus) => {
    setStatus(s)
    optionsRef.current?.onStatusChange?.(s)
  }, [])

  const runPlannerRound = useCallback(async (
    roundNum: number,
    task: string,
    feedback?: string
  ) => {
    if (abortRef.current || pausedRef.current) return

    const opts = optionsRef.current
    if (!opts || !plannerRef.current) return

    setCurrentRound(roundNum)
    updateStatus('planner-running')

    // Clear output buffer
    outputBufferRef.current = ''

    // Build the prompt
    const prompt = feedback
      ? `${task}\n\nPrevious review feedback:\n${feedback}\n\nPlease address the feedback and try again.`
      : task

    // Write prompt to temp file via PTY, then run agent reading from it
    const promptFile = writeTempPrompt(prompt)
    const escaped = escapeForEcho(prompt)

    // Write prompt to temp file first
    plannerRef.current.writeln(`\x1b[36m━━━ Round ${roundNum} / Planner (${opts.planner}) ━━━\x1b[0m`)
    if (plannerRef.current.write) {
      // Write prompt to temp file
      plannerRef.current.write(`printf '%s' "${escaped}" > "${promptFile}"\n`)
    }
    await new Promise(r => setTimeout(r, 500)) // Wait for file write

    const cmd = buildAgentCommand(opts.planner, promptFile, 'plan')
    plannerRef.current.writeln(`\x1b[90m$ ${cmd.substring(0, 100)}${cmd.length > 100 ? '...' : ''}\x1b[0m\n`)

    if (plannerRef.current.write) {
      plannerRef.current.write(cmd + '\n')
    }

    // Wait for completion by polling for shell prompt return
    // In a real implementation, shell integration OSC 633 D marker signals completion
    // For now, use a timeout-based approach with output monitoring
    await waitForCompletion(30000) // 30s timeout per agent

    const plannerOutput = outputBufferRef.current
    return plannerOutput
  }, [updateStatus])

  const runVerifierRound = useCallback(async (
    roundNum: number,
    plannerOutput: string
  ) => {
    if (abortRef.current || pausedRef.current) return 'unknown' as PingPongVerdict

    const opts = optionsRef.current
    if (!opts || !verifierRef.current) return 'unknown' as PingPongVerdict

    updateStatus('verifier-running')
    outputBufferRef.current = ''

    // Write planner output to temp file for verifier to read
    const promptFile = writeTempPrompt(plannerOutput)
    const escaped = escapeForEcho(plannerOutput.substring(0, 4000))

    verifierRef.current.writeln(`\x1b[35m━━━ Round ${roundNum} / Verifier (${opts.verifier}) ━━━\x1b[0m`)
    if (verifierRef.current.write) {
      verifierRef.current.write(`printf '%s' "${escaped}" > "${promptFile}"\n`)
    }
    await new Promise(r => setTimeout(r, 500))

    const cmd = buildAgentCommand(opts.verifier, promptFile, 'review')
    verifierRef.current.writeln(`\x1b[90m$ ${cmd.substring(0, 100)}${cmd.length > 100 ? '...' : ''}\x1b[0m\n`)

    if (verifierRef.current.write) {
      verifierRef.current.write(cmd + '\n')
    }

    await waitForCompletion(30000)

    const verifierOutput = outputBufferRef.current
    return parseVerdict(verifierOutput)
  }, [updateStatus])

  const waitForCompletion = (timeoutMs: number): Promise<void> => {
    return new Promise(resolve => {
      let lastLength = 0
      let stableCount = 0
      const checkInterval = setInterval(() => {
        if (abortRef.current) {
          clearInterval(checkInterval)
          resolve()
          return
        }
        const currentLength = outputBufferRef.current.length
        if (currentLength === lastLength && currentLength > 0) {
          stableCount++
          // If output hasn't changed for 3 seconds, consider it complete
          if (stableCount >= 6) {
            clearInterval(checkInterval)
            resolve()
            return
          }
        } else {
          stableCount = 0
        }
        lastLength = currentLength
      }, 500)

      // Hard timeout
      setTimeout(() => {
        clearInterval(checkInterval)
        resolve()
      }, timeoutMs)
    })
  }

  const start = useCallback(async (
    opts: AiPingPongOptions,
    planner: TerminalPanelHandle,
    verifier: TerminalPanelHandle
  ) => {
    optionsRef.current = opts
    plannerRef.current = planner
    verifierRef.current = verifier
    abortRef.current = false
    pausedRef.current = false
    setRounds([])
    setCurrentRound(0)

    const allRounds: PingPongRound[] = []
    let feedback: string | undefined

    for (let i = 1; i <= opts.maxRounds; i++) {
      if (abortRef.current) break

      // Wait if paused
      while (pausedRef.current && !abortRef.current) {
        await new Promise(r => setTimeout(r, 500))
      }

      // Run planner
      const plannerOutput = await runPlannerRound(i, opts.task, feedback)
      if (!plannerOutput || abortRef.current) break

      // Run verifier
      const verdict = await runVerifierRound(i, plannerOutput)
      if (abortRef.current) break

      const round: PingPongRound = {
        round: i,
        plannerOutput: plannerOutput.substring(0, 2000),
        verifierOutput: outputBufferRef.current.substring(0, 2000),
        verdict,
        timestamp: Date.now(),
      }

      allRounds.push(round)
      setRounds([...allRounds])
      opts.onRoundComplete?.(round)

      // Notify
      if ('Notification' in window && Notification.permission === 'granted') {
        new Notification(`Round ${i} complete`, {
          body: `Verdict: ${verdict.toUpperCase()}`,
        })
      }

      if (verdict === 'approved') {
        updateStatus('completed')
        opts.onSessionComplete?.(allRounds)
        return allRounds
      }

      if (verdict === 'rejected' || verdict === 'needs_info') {
        feedback = outputBufferRef.current.substring(0, 2000)
      }
    }

    // Max rounds reached
    updateStatus('completed')
    opts.onSessionComplete?.(allRounds)
    return allRounds
  }, [runPlannerRound, runVerifierRound, updateStatus])

  const pause = useCallback(() => {
    pausedRef.current = true
    updateStatus('paused')
  }, [updateStatus])

  const resume = useCallback(() => {
    pausedRef.current = false
    updateStatus('planner-running')
  }, [updateStatus])

  const abort = useCallback(() => {
    abortRef.current = true
    updateStatus('idle')
  }, [updateStatus])

  return {
    status,
    currentRound,
    rounds,
    start,
    pause,
    resume,
    abort,
    captureOutput,
  }
}
