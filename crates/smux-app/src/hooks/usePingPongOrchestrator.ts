import { useState, useCallback, useRef } from 'react'
import type { TerminalPanelHandle } from '../components/TerminalPanel'

/**
 * Terminal-to-Terminal Ping-Pong Orchestrator
 *
 * Flow: User goal → Claude PTY runs → output captured → Codex PTY runs with that output
 *       → Codex output captured → if APPROVED auto-advance, if REJECTED re-run Claude
 *
 * Phases:
 *   1. Ideation: Claude brainstorms → Codex critiques → auto-iterate until APPROVED
 *   2. Planning: Claude plans → Codex reviews → auto-iterate until APPROVED
 *   3. Execution: per-task Claude codes → Codex verifies → auto-iterate → next task
 *
 * User can type in either terminal at any time (direct PTY access).
 * Notifications on phase transitions and errors.
 */

export type Phase = 'idle' | 'ideation' | 'planning' | 'execution' | 'complete'

export interface PingPongState {
  phase: Phase
  round: number
  maxRounds: number
  status: string  // human-readable status for the status bar
  isRunning: boolean
}

// Phase progression: ideation → planning → execution → complete (hardcoded in start())

/** Escape content for safe shell echo (double-quote context) */
function shellEscape(s: string): string {
  return s
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/`/g, '\\`')
    .replace(/\$/g, '\\$')
    .replace(/!/g, '\\!')
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
}

/** Strip ANSI escape sequences from text before feeding to AI */
function stripAnsi(s: string): string {
  return s.replace(/\x1b\[[0-9;]*[a-zA-Z]/g, '').replace(/\x1b\][^\x07]*\x07/g, '')
}

/** Build the command to run an agent with a prompt from a temp file */
function agentCmd(agent: string, promptFile: string): string {
  // NOTE: Do NOT hardcode --dangerously-skip-permissions or --full-auto.
  // The user must explicitly opt-in via execution level settings.
  // Default: agents run in interactive/safe mode requiring confirmation.
  switch (agent) {
    case 'claude':
      return `cat "${promptFile}" | claude -p -`
    case 'codex':
      return `cat "${promptFile}" | codex exec -`
    case 'gemini':
      return `cat "${promptFile}" | gemini -p -`
    default:
      return `echo "Unknown agent"`
  }
}

/** Detect verdict from output text */
function detectVerdict(output: string): 'approved' | 'rejected' | 'unknown' {
  const lower = output.toLowerCase()
  if (lower.includes('approved') || lower.includes('lgtm') || lower.includes('looks good')) return 'approved'
  if (lower.includes('rejected') || lower.includes('changes needed') || lower.includes('needs fix')) return 'rejected'
  return 'unknown'
}

/** Phase-specific prompt builders */
const PROMPTS = {
  ideation: {
    planner: (goal: string, feedback?: string) =>
      feedback
        ? `Previous feedback:\n${feedback}\n\nRefine the ideas for: ${goal}\n\nPresent improved, concrete feature ideas.`
        : `Brainstorm concrete feature ideas for: ${goal}\n\nPresent 3-5 specific, actionable ideas with brief implementation notes.`,
    verifier: (plannerOutput: string) =>
      `Review these ideas:\n\n${plannerOutput}\n\nAdd missing ideas, flag issues, suggest improvements. If the ideas are solid and ready for planning, say APPROVED. Otherwise say what needs to change.`,
  },
  planning: {
    planner: (goal: string, ideas: string, feedback?: string) =>
      feedback
        ? `Feedback on plan:\n${feedback}\n\nRevise the implementation plan.\n\nOriginal goal: ${goal}\nApproved ideas:\n${ideas}`
        : `Create a detailed implementation plan for: ${goal}\n\nApproved ideas:\n${ideas}\n\nInclude: phases, tasks per phase, dependencies, tech choices, testing strategy.`,
    verifier: (plan: string) =>
      `Review this implementation plan:\n\n${plan}\n\nCheck: missing edge cases, security, performance, testing gaps, scope. If ready to execute, say APPROVED. Otherwise list specific issues.`,
  },
  execution: {
    planner: (task: string, context?: string) =>
      context
        ? `Fix based on review feedback:\n${context}\n\nOriginal task: ${task}`
        : `Implement this task:\n${task}\n\nWrite complete, production-ready code.`,
    verifier: (implementation: string) =>
      `Review this implementation:\n\n${implementation}\n\nCheck: correctness, security, edge cases, code quality. Say APPROVED if good, or REJECTED with specific fixes.`,
  },
}

export function usePingPongOrchestrator() {
  const [state, setState] = useState<PingPongState>({
    phase: 'idle',
    round: 0,
    maxRounds: 5,
    status: 'Ready',
    isRunning: false,
  })

  const plannerRef = useRef<TerminalPanelHandle | null>(null)
  const verifierRef = useRef<TerminalPanelHandle | null>(null)
  const abortRef = useRef(false)
  const goalRef = useRef('')
  const approvedIdeasRef = useRef('')
  const approvedPlanRef = useRef('')
  const plannerAgent = useRef('claude')
  const verifierAgent = useRef('codex')
  // Output capture buffer — filled by the PTY output listener
  const captureBufferRef = useRef('')
  const capturingRef = useRef(false)
  const capturingTerminalRef = useRef<'planner' | 'verifier' | null>(null)
  const tempFilesRef = useRef<string[]>([])
  const MAX_CAPTURE = 1024 * 512 // 512KB cap

  /** Start capturing output from a specific terminal */
  const startCapture = useCallback((terminal?: 'planner' | 'verifier') => {
    captureBufferRef.current = ''
    capturingRef.current = true
    capturingTerminalRef.current = terminal || null
  }, [])

  /** Stop capturing and return collected output (stripped of ANSI) */
  const stopCapture = useCallback((): string => {
    capturingRef.current = false
    capturingTerminalRef.current = null
    return stripAnsi(captureBufferRef.current)
  }, [])

  /** Feed data into capture buffer — only from the active terminal */
  const feedPlannerOutput = useCallback((data: string) => {
    if (capturingRef.current && (capturingTerminalRef.current === 'planner' || capturingTerminalRef.current === null)) {
      if (captureBufferRef.current.length < MAX_CAPTURE) {
        captureBufferRef.current += data
      }
    }
  }, [])

  const feedVerifierOutput = useCallback((data: string) => {
    if (capturingRef.current && (capturingTerminalRef.current === 'verifier' || capturingTerminalRef.current === null)) {
      if (captureBufferRef.current.length < MAX_CAPTURE) {
        captureBufferRef.current += data
      }
    }
  }, [])

  /** Clean up all temp files */
  const cleanupTempFiles = useCallback(() => {
    if (plannerRef.current && tempFilesRef.current.length > 0) {
      const files = tempFilesRef.current.join(' ')
      plannerRef.current.write(`rm -f ${files} 2>/dev/null\n`)
      tempFilesRef.current = []
    }
  }, [])

  /** Write a prompt to temp file via PTY, then run agent command */
  const runAgent = useCallback(async (
    terminal: TerminalPanelHandle,
    agent: string,
    prompt: string,
    label: string,
    which: 'planner' | 'verifier' = 'planner',
  ): Promise<string> => {
    // Use secure temp dir with crypto random name
    const rnd = crypto.getRandomValues(new Uint8Array(16))
    const hex = Array.from(rnd).map(b => b.toString(16).padStart(2, '0')).join('')
    const tmpFile = `$HOME/.smux/tmp/smux-pp-${hex}.txt`
    const escaped = shellEscape(prompt)

    // Ensure temp dir exists, write prompt, then run agent — all chained with &&
    tempFilesRef.current.push(tmpFile)
    terminal.writeln(`\x1b[36m━━━ ${label} ━━━\x1b[0m`)

    // Start capturing output from this specific terminal only
    startCapture(which)

    // Chain: mkdir → write prompt → run agent (no race condition)
    const cmd = agentCmd(agent, tmpFile)
    terminal.write(`mkdir -p "$HOME/.smux/tmp" && chmod 700 "$HOME/.smux/tmp" && printf '%s' "${escaped}" > "${tmpFile}" && chmod 600 "${tmpFile}" && ${cmd}\n`)

    // Wait for completion (output stabilizes)
    await waitForStable(45000)

    const output = stopCapture()

    // Cleanup temp file
    terminal.write(`rm -f "${tmpFile}"\n`)

    return output
  }, [startCapture, stopCapture])

  /** Wait until captured output stops changing */
  const waitForStable = (timeoutMs: number): Promise<void> => {
    return new Promise(resolve => {
      let lastLen = 0
      let stableCount = 0
      let resolved = false
      const done = () => { if (!resolved) { resolved = true; clearInterval(iv); clearTimeout(to); resolve() } }
      const iv = setInterval(() => {
        if (abortRef.current) { done(); return }
        const len = captureBufferRef.current.length
        if (len === lastLen && len > 0) { stableCount++; if (stableCount >= 6) done() }
        else stableCount = 0
        lastLen = len
      }, 500)
      const to = setTimeout(done, timeoutMs)
    })
  }

  const updateStatus = useCallback((phase: Phase, round: number, status: string, isRunning: boolean) => {
    setState({ phase, round, maxRounds: 5, status, isRunning })
  }, [])

  const notifyUser = useCallback((title: string, body: string) => {
    if ('Notification' in window && Notification.permission === 'granted') {
      new Notification(title, { body })
    }
  }, [])

  /** Run one phase until APPROVED or max rounds */
  const runPhase = useCallback(async (phase: Phase) => {
    const planner = plannerRef.current
    const verifier = verifierRef.current
    if (!planner || !verifier || abortRef.current) return

    let feedback: string | undefined

    for (let round = 1; round <= 5; round++) {
      if (abortRef.current) return

      updateStatus(phase, round, `${phase} R${round} — Planner working...`, true)

      // Build planner prompt
      let plannerPrompt: string
      if (phase === 'ideation') {
        plannerPrompt = PROMPTS.ideation.planner(goalRef.current, feedback)
      } else if (phase === 'planning') {
        plannerPrompt = PROMPTS.planning.planner(goalRef.current, approvedIdeasRef.current, feedback)
      } else {
        plannerPrompt = PROMPTS.execution.planner(goalRef.current, feedback)
      }

      // Run planner
      const plannerOutput = await runAgent(planner, plannerAgent.current, plannerPrompt, `${phase} R${round} / Planner`, 'planner')
      if (abortRef.current) return

      updateStatus(phase, round, `${phase} R${round} — Verifier reviewing...`, true)

      // Build verifier prompt
      let verifierPrompt: string
      if (phase === 'ideation') {
        verifierPrompt = PROMPTS.ideation.verifier(plannerOutput)
      } else if (phase === 'planning') {
        verifierPrompt = PROMPTS.planning.verifier(plannerOutput)
      } else {
        verifierPrompt = PROMPTS.execution.verifier(plannerOutput)
      }

      // Run verifier
      const verifierOutput = await runAgent(verifier, verifierAgent.current, verifierPrompt, `${phase} R${round} / Verifier`, 'verifier')
      if (abortRef.current) return

      const verdict = detectVerdict(verifierOutput)

      if (verdict === 'approved') {
        // Save approved output for next phase
        if (phase === 'ideation') approvedIdeasRef.current = plannerOutput
        if (phase === 'planning') approvedPlanRef.current = plannerOutput

        updateStatus(phase, round, `${phase} APPROVED — advancing...`, false)
        notifyUser(`Phase: ${phase}`, `APPROVED after ${round} rounds`)
        return // Phase complete
      }

      // Rejected or unknown — feed verifier output back as feedback
      feedback = verifierOutput
      updateStatus(phase, round, `${phase} R${round} — REJECTED, iterating...`, true)
    }

    // Max rounds reached
    notifyUser(`Phase: ${phase}`, 'Max rounds reached')
    updateStatus(phase, 5, `${phase} — max rounds reached`, false)
  }, [runAgent, updateStatus, notifyUser])

  /** Start the full ping-pong session */
  const start = useCallback(async (
    goal: string,
    planner: TerminalPanelHandle,
    verifier: TerminalPanelHandle,
    pAgent = 'claude',
    vAgent = 'codex',
  ) => {
    plannerRef.current = planner
    verifierRef.current = verifier
    plannerAgent.current = pAgent
    verifierAgent.current = vAgent
    goalRef.current = goal
    abortRef.current = false

    // Phase 1: Ideation
    await runPhase('ideation')
    if (abortRef.current) return

    // Phase 2: Planning
    await runPhase('planning')
    if (abortRef.current) return

    // Phase 3: Execution
    await runPhase('execution')
    if (abortRef.current) return

    updateStatus('complete', 0, 'Session complete', false)
    notifyUser('AI Ping-Pong', 'All phases complete!')
  }, [runPhase, updateStatus, notifyUser])

  const abort = useCallback(() => {
    abortRef.current = true
    capturingRef.current = false
    capturingTerminalRef.current = null
    cleanupTempFiles()
    updateStatus('idle', 0, 'Aborted', false)
  }, [updateStatus, cleanupTempFiles])

  return {
    ...state,
    start,
    abort,
    feedPlannerOutput,
    feedVerifierOutput,
  }
}
