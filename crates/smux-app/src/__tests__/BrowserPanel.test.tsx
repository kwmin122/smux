import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { BrowserPanel } from '../components/BrowserPanel'

describe('BrowserPanel', () => {
  it('renders URL bar with default localhost URL', () => {
    render(<BrowserPanel onClose={() => {}} />)
    const input = screen.getByDisplayValue('http://localhost:3000')
    expect(input).toBeTruthy()
  })

  it('renders navigation buttons', () => {
    render(<BrowserPanel onClose={() => {}} />)
    expect(screen.getByTitle('Back')).toBeTruthy()
    expect(screen.getByTitle('Forward')).toBeTruthy()
    expect(screen.getByTitle('Reload')).toBeTruthy()
  })

  it('renders close button', () => {
    render(<BrowserPanel onClose={() => {}} />)
    expect(screen.getByTitle(/Close browser/)).toBeTruthy()
  })

  it('renders iframe without allow-same-origin', () => {
    const { container } = render(<BrowserPanel onClose={() => {}} />)
    const iframe = container.querySelector('iframe')
    expect(iframe).toBeTruthy()
    expect(iframe?.getAttribute('sandbox')).not.toContain('allow-same-origin')
    expect(iframe?.getAttribute('sandbox')).toContain('allow-scripts')
  })
})
