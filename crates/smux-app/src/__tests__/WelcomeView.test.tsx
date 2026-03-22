import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { WelcomeView } from '../components/WelcomeView'

const defaultProps = {
  onOpenFolder: (_path: string) => {},
  onNewSession: () => {},
  daemonRunning: false,
}

describe('WelcomeView', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('renders smux title', () => {
    render(<WelcomeView {...defaultProps} />)
    expect(screen.getByText('smux')).toBeTruthy()
  })

  it('switches to English when EN button is clicked', () => {
    render(<WelcomeView {...defaultProps} />)
    fireEvent.click(screen.getByText('EN'))
    expect(screen.getByText('Open Folder')).toBeTruthy()
  })

  it('shows folder open button in Korean by default', () => {
    render(<WelcomeView {...defaultProps} />)
    expect(screen.getByText('폴더 열기')).toBeTruthy()
  })

  it('shows AI session button disabled when daemon not running', () => {
    render(<WelcomeView {...defaultProps} />)
    const aiBtn = screen.getByText('AI 핑퐁 세션').closest('button')
    expect(aiBtn?.disabled).toBe(true)
  })

  it('shows AI session button enabled when daemon running', () => {
    render(<WelcomeView {...defaultProps} daemonRunning={true} />)
    const aiBtn = screen.getByText('AI 핑퐁 세션').closest('button')
    expect(aiBtn?.disabled).toBe(false)
  })

  it('shows getting started steps', () => {
    render(<WelcomeView {...defaultProps} />)
    expect(screen.getByText('시작하기')).toBeTruthy()
  })

  it('shows keyboard shortcuts', () => {
    render(<WelcomeView {...defaultProps} />)
    expect(screen.getByText('Tab')).toBeTruthy()
  })

  it('shows empty recent projects message', () => {
    render(<WelcomeView {...defaultProps} />)
    expect(screen.getByText('최근에 열었던 프로젝트가 없습니다')).toBeTruthy()
  })
})
