import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { BrowserPanel } from '../components/BrowserPanel'

describe('BrowserPanel', () => {
  it('renders URL input with default localhost URL', () => {
    render(<BrowserPanel onClose={() => {}} />)
    const input = screen.getByPlaceholderText('http://localhost:3000')
    expect(input).toBeTruthy()
  })

  it('renders navigate button', () => {
    render(<BrowserPanel onClose={() => {}} />)
    expect(screen.getByTitle('Navigate')).toBeTruthy()
  })

  it('renders close button', () => {
    render(<BrowserPanel onClose={() => {}} />)
    expect(screen.getByTitle(/Close browser/)).toBeTruthy()
  })

  it('does not render an iframe (uses native WebView)', () => {
    const { container } = render(<BrowserPanel onClose={() => {}} />)
    const iframe = container.querySelector('iframe')
    expect(iframe).toBeNull()
  })
})
