export interface RoundEntry {
  round: number
  verdict: 'approved' | 'rejected' | 'pending' | 'needs_info'
  summary?: string
}

export interface HealthState {
  planner: number // 0-100
  verifier: number // 0-100
}

export interface EventLogEntry {
  timestamp: string
  kind: string
  message: string
}

export interface VerifierVerdictEntry {
  verifier: string
  verdict: 'approved' | 'rejected' | 'needs_info'
  confidence: number
  reason: string
}

export interface CrossVerifyState {
  round: number
  individual: VerifierVerdictEntry[]
  finalVerdict: string
  strategy: string
  agreementRatio: number
}

interface MissionControlProps {
  currentRound: number
  maxRounds: number
  rounds: RoundEntry[]
  health: HealthState
  safetyOk: boolean
  gitBranch: string
  gitFilesChanged: number
  eventLog: EventLogEntry[]
  crossVerify: CrossVerifyState | null
  onRewind?: (round: number) => void
}

function verdictIcon(v: RoundEntry['verdict']) {
  switch (v) {
    case 'approved': return { symbol: '✓', color: 'text-secondary' }
    case 'rejected': return { symbol: '✗', color: 'text-error' }
    case 'needs_info': return { symbol: '?', color: 'text-tertiary' }
    case 'pending': return { symbol: '○', color: 'text-outline' }
  }
}

function healthColor(pct: number) {
  if (pct >= 70) return 'bg-secondary'
  if (pct >= 40) return 'bg-tertiary'
  return 'bg-error'
}

export function MissionControl({
  currentRound,
  maxRounds,
  rounds,
  health,
  safetyOk,
  gitBranch,
  gitFilesChanged,
  eventLog,
  crossVerify,
  onRewind,
}: MissionControlProps) {
  return (
    <div className="h-full flex flex-col bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)] overflow-hidden">
      <div className="h-7 bg-surface-container-high px-3 flex items-center border-b border-outline-variant/20 shrink-0">
        <span className="font-mono text-[10px] font-bold uppercase tracking-widest text-tertiary">
          Mission Control
        </span>
      </div>

      <div className="flex-1 overflow-y-auto p-3 space-y-4">
        {/* Round History */}
        <section>
          <h3 className="font-mono text-[9px] uppercase tracking-widest text-outline mb-2">
            Round History
          </h3>
          <div className="flex flex-wrap gap-1">
            {Array.from({ length: maxRounds }, (_, i) => {
              const r = rounds.find(x => x.round === i + 1)
              const vi = verdictIcon(r?.verdict ?? (i + 1 <= currentRound ? 'pending' : 'pending'))
              const isCurrent = i + 1 === currentRound
              return (
                <button
                  key={i}
                  onClick={() => r && onRewind?.(i + 1)}
                  className={`w-7 h-7 flex items-center justify-center font-mono text-[10px] rounded-sm border transition-colors ${
                    isCurrent
                      ? 'border-primary bg-primary/10 text-primary'
                      : 'border-outline-variant/30 hover:border-primary/50'
                  } ${vi.color}`}
                  title={r?.summary ?? `Round ${i + 1}`}
                >
                  {r ? vi.symbol : i + 1}
                </button>
              )
            })}
          </div>
          <div className="mt-1 font-mono text-[9px] text-outline">
            R{currentRound}/{maxRounds}
          </div>
        </section>

        {/* Health Indicators */}
        <section>
          <h3 className="font-mono text-[9px] uppercase tracking-widest text-outline mb-2">
            Health
          </h3>
          <div className="space-y-2">
            <div>
              <div className="flex justify-between mb-0.5">
                <span className="font-mono text-[10px] text-on-surface-variant flex items-center gap-1">
                  <span className={`w-1.5 h-1.5 rounded-full ${healthColor(health.planner)}`} />
                  Planner
                </span>
                <span className="font-mono text-[10px] text-on-surface-variant">
                  {health.planner}%
                </span>
              </div>
              <div className="h-1 bg-surface-container-high rounded-full overflow-hidden">
                <div
                  className={`h-full rounded-full transition-all ${healthColor(health.planner)}`}
                  style={{ width: `${health.planner}%` }}
                />
              </div>
            </div>
            <div>
              <div className="flex justify-between mb-0.5">
                <span className="font-mono text-[10px] text-on-surface-variant flex items-center gap-1">
                  <span className={`w-1.5 h-1.5 rounded-full ${healthColor(health.verifier)}`} />
                  Verifier
                </span>
                <span className="font-mono text-[10px] text-on-surface-variant">
                  {health.verifier}%
                </span>
              </div>
              <div className="h-1 bg-surface-container-high rounded-full overflow-hidden">
                <div
                  className={`h-full rounded-full transition-all ${healthColor(health.verifier)}`}
                  style={{ width: `${health.verifier}%` }}
                />
              </div>
            </div>
          </div>
        </section>

        {/* Safety Status */}
        <section>
          <h3 className="font-mono text-[9px] uppercase tracking-widest text-outline mb-2">
            Safety
          </h3>
          <div className={`flex items-center gap-2 font-mono text-[11px] ${
            safetyOk ? 'text-secondary' : 'text-error'
          }`}>
            <span className={`w-2 h-2 rounded-full ${safetyOk ? 'bg-secondary' : 'bg-error animate-pulse'}`} />
            {safetyOk ? 'All checks passed' : 'Alert — review required'}
          </div>
        </section>

        {/* Git Info */}
        <section>
          <h3 className="font-mono text-[9px] uppercase tracking-widest text-outline mb-2">
            Git
          </h3>
          <div className="space-y-1">
            <div className="font-mono text-[11px] text-on-surface-variant flex items-center gap-1">
              <span className="material-symbols-outlined text-[14px]">commit</span>
              {gitBranch}
            </div>
            <div className="font-mono text-[11px] text-on-surface-variant flex items-center gap-1">
              <span className="material-symbols-outlined text-[14px]">description</span>
              {gitFilesChanged} files changed
            </div>
          </div>
        </section>

        {/* Cross-Verify */}
        {crossVerify && (
          <section>
            <h3 className="font-mono text-[9px] uppercase tracking-widest text-outline mb-2">
              Cross-Verify (R{crossVerify.round})
            </h3>
            <div className="space-y-2">
              {crossVerify.individual.map((v) => (
                <div key={v.verifier} className="space-y-0.5">
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-[11px] text-on-surface-variant capitalize">
                      {v.verifier}
                    </span>
                    <span className={`font-mono text-[10px] px-1.5 py-0.5 rounded-sm ${
                      v.verdict === 'approved'
                        ? 'bg-secondary/20 text-secondary'
                        : v.verdict === 'rejected'
                        ? 'bg-error/20 text-error'
                        : 'bg-tertiary/20 text-tertiary'
                    }`}>
                      {v.verdict === 'approved' ? '✓' : v.verdict === 'rejected' ? '✗' : '?'} {v.verdict.toUpperCase()}
                    </span>
                  </div>
                  <div className="h-1 bg-surface-container-high rounded-full overflow-hidden">
                    <div
                      className={`h-full rounded-full ${
                        v.verdict === 'approved' ? 'bg-secondary' : v.verdict === 'rejected' ? 'bg-error' : 'bg-tertiary'
                      }`}
                      style={{ width: `${v.confidence * 100}%` }}
                    />
                  </div>
                  <div className="font-mono text-[9px] text-outline truncate" title={v.reason}>
                    {v.reason}
                  </div>
                </div>
              ))}
              {/* Final verdict banner */}
              <div className={`mt-2 p-2 rounded-sm border font-mono text-[11px] ${
                crossVerify.finalVerdict.toLowerCase().includes('approved')
                  ? 'bg-secondary/10 border-secondary/30 text-secondary'
                  : 'bg-error/10 border-error/30 text-error'
              }`}>
                <div className="font-bold">
                  Final: {crossVerify.finalVerdict.toUpperCase()}
                </div>
                <div className="text-[9px] mt-0.5 opacity-70">
                  {crossVerify.strategy} — {Math.round(crossVerify.agreementRatio * 100)}% agreement
                </div>
              </div>
            </div>
          </section>
        )}

        {/* Event Log */}
        <section>
          <h3 className="font-mono text-[9px] uppercase tracking-widest text-outline mb-2">
            Event Log
          </h3>
          <div className="space-y-0.5 max-h-40 overflow-y-auto">
            {eventLog.length === 0 ? (
              <div className="font-mono text-[10px] text-outline">No events yet</div>
            ) : (
              eventLog.map((entry, i) => (
                <div key={i} className="font-mono text-[10px] text-on-surface-variant flex gap-2">
                  <span className="text-outline shrink-0">{entry.timestamp}</span>
                  <span className={
                    entry.kind === 'error' ? 'text-error' :
                    entry.kind === 'round' ? 'text-primary' :
                    entry.kind === 'safety' ? 'text-tertiary' :
                    ''
                  }>
                    {entry.message}
                  </span>
                </div>
              ))
            )}
          </div>
        </section>
      </div>
    </div>
  )
}
