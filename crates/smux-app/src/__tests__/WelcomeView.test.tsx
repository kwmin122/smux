import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { WelcomeView } from '../components/WelcomeView'

const defaultProps = {
  onNewSession: () => {},
  onOpenTerminal: () => {},
  daemonRunning: false,
}

describe('WelcomeView', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('renders welcome title in Korean by default', () => {
    render(<WelcomeView {...defaultProps} />)
    expect(screen.getByText('smux에 오신 걸 환영합니다')).toBeTruthy()
  })

  it('switches to English when EN button is clicked', () => {
    render(<WelcomeView {...defaultProps} />)
    fireEvent.click(screen.getByText('EN'))
    expect(screen.getByText('Welcome to smux')).toBeTruthy()
  })

  it('shows daemon status indicator when daemon is not running', () => {
    render(<WelcomeView {...defaultProps} />)
    expect(screen.getByText('Daemon 상태')).toBeTruthy()
  })

  it('shows AI session button when daemon is running', () => {
    render(<WelcomeView {...defaultProps} daemonRunning={true} />)
    expect(screen.getByText('AI 세션')).toBeTruthy()
  })

  it('calls onNewSession when AI session button is clicked', () => {
    let called = false
    render(<WelcomeView {...defaultProps} onNewSession={() => { called = true }} daemonRunning={true} />)
    fireEvent.click(screen.getByText('AI 세션'))
    expect(called).toBe(true)
  })

  it('shows terminal button and calls onOpenTerminal', () => {
    let called = false
    render(<WelcomeView {...defaultProps} onOpenTerminal={() => { called = true }} />)
    fireEvent.click(screen.getByText('터미널 열기'))
    expect(called).toBe(true)
  })

  it('renders feature cards', () => {
    render(<WelcomeView {...defaultProps} />)
    expect(screen.getByText('Cross-Verify')).toBeTruthy()
    expect(screen.getByText('Focus & Control')).toBeTruthy()
  })

  it('renders keyboard shortcuts section', () => {
    render(<WelcomeView {...defaultProps} />)
    expect(screen.getByText('Tab')).toBeTruthy()
  })
})
