import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MissionControl } from '../components/MissionControl'

describe('MissionControl', () => {
  const defaultProps = {
    currentRound: 3,
    maxRounds: 10,
    rounds: [
      { round: 1, verdict: 'approved' as const, summary: 'ok' },
      { round: 2, verdict: 'rejected' as const, summary: 'fail' },
    ],
    health: { planner: 80, verifier: 60 },
    safetyOk: true,
    gitBranch: 'main',
    gitFilesChanged: 5,
    eventLog: [
      { timestamp: '12:00:00', kind: 'round', message: 'Round 1 complete' },
    ],
    crossVerify: null,
  }

  it('renders round history', () => {
    render(<MissionControl {...defaultProps} />)
    expect(screen.getByText('Round History')).toBeTruthy()
    expect(screen.getByText('R3/10')).toBeTruthy()
  })

  it('renders health indicators', () => {
    render(<MissionControl {...defaultProps} />)
    expect(screen.getByText('80%')).toBeTruthy()
    expect(screen.getByText('60%')).toBeTruthy()
  })

  it('renders safety status', () => {
    render(<MissionControl {...defaultProps} />)
    expect(screen.getByText('All checks passed')).toBeTruthy()
  })

  it('renders safety alert when not ok', () => {
    render(<MissionControl {...defaultProps} safetyOk={false} />)
    expect(screen.getByText(/review required/i)).toBeTruthy()
  })

  it('renders git info', () => {
    render(<MissionControl {...defaultProps} />)
    expect(screen.getByText('main')).toBeTruthy()
    expect(screen.getByText('5 files changed')).toBeTruthy()
  })

  it('renders event log', () => {
    render(<MissionControl {...defaultProps} />)
    expect(screen.getByText('Round 1 complete')).toBeTruthy()
  })

  it('renders cross-verify panel when data present', () => {
    render(
      <MissionControl
        {...defaultProps}
        crossVerify={{
          round: 1,
          individual: [
            { verifier: 'claude', verdict: 'approved', confidence: 0.92, reason: 'ok' },
            { verifier: 'codex', verdict: 'rejected', confidence: 0.7, reason: 'missing tests' },
          ],
          finalVerdict: 'APPROVED',
          strategy: 'Majority',
          agreementRatio: 0.5,
        }}
      />
    )
    expect(screen.getByText(/Cross-Verify/)).toBeTruthy()
    expect(screen.getAllByText(/APPROVED/).length).toBeGreaterThanOrEqual(1)
    expect(screen.getByText(/50% agreement/)).toBeTruthy()
  })
})
