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

// Build the CLI command for each agent
function buildAgentCommand(agent: string, prompt: string, mode: 'plan' | 'review'): string {
  const escapedPrompt = prompt.replace(/'/g, "'\\''")

  switch (agent) {
    case 'claude':
      return mode === 'plan'
        ? `claude -p --dangerously-skip-permissions '${escapedPrompt}'`
        : `claude -p --dangerously-skip-permissions 'Review the following work and respond with APPROVED if correct or REJECTED with feedback if not:\n\n${escapedPrompt}'`
    case 'codex':
      return mode === 'plan'
        ? `codex exec --full-auto '${escapedPrompt}'`
        : `codex exec --full-auto 'Review and verify:\n\n${escapedPrompt}'`
    case 'gemini':
      return mode === 'plan'
        ? `gemini -p '${escapedPrompt}'`
        : `gemini -p 'Review:\n\n${escapedPrompt}'`
    default:
      return `echo "Unknown agent: ${agent}"`
  }
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

    const cmd = buildAgentCommand(opts.planner, prompt, 'plan')

    // Write command to planner PTY
    plannerRef.current.writeln(`\x1b[36m━━━ Round ${roundNum} / Planner (${opts.planner}) ━━━\x1b[0m`)
    plannerRef.current.writeln(`\x1b[90m$ ${cmd.substring(0, 80)}${cmd.length > 80 ? '...' : ''}\x1b[0m\n`)

    // We write the command to the PTY via the shell
    // The actual execution happens in the PTY process
    // For now we simulate the flow with a marker-based approach
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

    const cmd = buildAgentCommand(opts.verifier, plannerOutput.substring(0, 4000), 'review')

    verifierRef.current.writeln(`\x1b[35m━━━ Round ${roundNum} / Verifier (${opts.verifier}) ━━━\x1b[0m`)
    verifierRef.current.writeln(`\x1b[90m$ ${cmd.substring(0, 80)}${cmd.length > 80 ? '...' : ''}\x1b[0m\n`)

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
