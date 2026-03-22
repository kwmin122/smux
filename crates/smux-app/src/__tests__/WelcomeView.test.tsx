import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { WelcomeView } from '../components/WelcomeView'

describe('WelcomeView', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('renders welcome title in Korean by default', () => {
    render(<WelcomeView onNewSession={() => {}} daemonRunning={false} />)
    expect(screen.getByText('smux에 오신 걸 환영합니다')).toBeTruthy()
  })

  it('switches to English when EN button is clicked', () => {
    render(<WelcomeView onNewSession={() => {}} daemonRunning={false} />)
    fireEvent.click(screen.getByText('EN'))
    expect(screen.getByText('Welcome to smux')).toBeTruthy()
  })

  it('shows daemon status indicator when daemon is not running', () => {
    render(<WelcomeView onNewSession={() => {}} daemonRunning={false} />)
    expect(screen.getByText('Daemon 상태')).toBeTruthy()
  })

  it('shows start session button when daemon is running', () => {
    render(<WelcomeView onNewSession={() => {}} daemonRunning={true} />)
    expect(screen.getByText('새 세션 시작')).toBeTruthy()
  })

  it('calls onNewSession when start button is clicked', () => {
    let called = false
    render(<WelcomeView onNewSession={() => { called = true }} daemonRunning={true} />)
    fireEvent.click(screen.getByText('새 세션 시작'))
    expect(called).toBe(true)
  })

  it('renders feature cards', () => {
    render(<WelcomeView onNewSession={() => {}} daemonRunning={false} />)
    expect(screen.getByText('Cross-Verify')).toBeTruthy()
    expect(screen.getByText('Focus & Control')).toBeTruthy()
  })

  it('renders keyboard shortcuts section', () => {
    render(<WelcomeView onNewSession={() => {}} daemonRunning={false} />)
    expect(screen.getByText('Tab')).toBeTruthy()
  })
})
