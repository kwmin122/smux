import { useState, useEffect, useRef, useCallback } from 'react'
import type { Terminal } from '@xterm/xterm'
import { SearchAddon } from '@xterm/addon-search'

interface SearchOverlayProps {
  terminal: Terminal
  onClose: () => void
}

export function SearchOverlay({ terminal, onClose }: SearchOverlayProps) {
  const inputRef = useRef<HTMLInputElement>(null)
  const searchAddonRef = useRef<SearchAddon | null>(null)

  const [query, setQuery] = useState('')
  const [regex, setRegex] = useState(false)
  const [caseSensitive, setCaseSensitive] = useState(false)
  const [resultIndex, setResultIndex] = useState(-1)
  const [resultCount, setResultCount] = useState(0)

  // Load addon on mount, dispose on unmount
  useEffect(() => {
    const addon = new SearchAddon()
    terminal.loadAddon(addon)
    searchAddonRef.current = addon

    const disposable = addon.onDidChangeResults(({ resultIndex: idx, resultCount: count }) => {
      setResultIndex(idx)
      setResultCount(count)
    })

    return () => {
      disposable.dispose()
      addon.clearDecorations()
      addon.dispose()
      searchAddonRef.current = null
    }
  }, [terminal])

  // Auto-focus input on mount
  useEffect(() => {
    inputRef.current?.focus()
  }, [])

  // Re-run search whenever query, regex, or caseSensitive changes
  useEffect(() => {
    const addon = searchAddonRef.current
    if (!addon) return

    if (query.length === 0) {
      addon.clearDecorations()
      setResultIndex(-1)
      setResultCount(0)
      return
    }

    addon.findNext(query, { regex, caseSensitive, wholeWord: false })
  }, [query, regex, caseSensitive])

  const findNext = useCallback(() => {
    if (!searchAddonRef.current || query.length === 0) return
    searchAddonRef.current.findNext(query, { regex, caseSensitive, wholeWord: false })
  }, [query, regex, caseSensitive])

  const findPrevious = useCallback(() => {
    if (!searchAddonRef.current || query.length === 0) return
    searchAddonRef.current.findPrevious(query, { regex, caseSensitive, wholeWord: false })
  }, [query, regex, caseSensitive])

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault()
      onClose()
      return
    }
    if (e.key === 'Enter') {
      e.preventDefault()
      if (e.shiftKey) {
        findPrevious()
      } else {
        findNext()
      }
    }
  }

  const matchDisplay =
    query.length === 0
      ? ''
      : resultCount === 0
        ? 'No results'
        : `${resultIndex + 1} of ${resultCount}`

  return (
    <div
      className="absolute top-0 right-0 z-50 flex items-center gap-1.5 px-2 py-1.5 m-2 rounded-sm border border-outline-variant/30 shadow-lg"
      style={{ backgroundColor: 'var(--surface-container)' }}
    >
      {/* Search input */}
      <input
        ref={inputRef}
        type="text"
        value={query}
        onChange={e => setQuery(e.target.value)}
        onKeyDown={handleKeyDown}
        className="h-6 w-48 bg-surface-container-lowest border border-outline-variant/30 rounded-sm px-2 font-mono text-[11px] text-on-surface outline-none focus:border-primary"
        spellCheck={false}
        placeholder="Search..."
      />

      {/* Match count */}
      <span className="font-mono text-[10px] text-outline min-w-[60px] text-center select-none">
        {matchDisplay}
      </span>

      {/* Previous */}
      <button
        onClick={findPrevious}
        disabled={resultCount === 0}
        className="w-6 h-6 flex items-center justify-center text-on-surface-variant hover:text-primary disabled:text-outline/40 transition-colors"
        title="Previous match (Shift+Enter)"
        aria-label="Previous match"
      >
        <span className="material-symbols-outlined text-[16px]">keyboard_arrow_up</span>
      </button>

      {/* Next */}
      <button
        onClick={findNext}
        disabled={resultCount === 0}
        className="w-6 h-6 flex items-center justify-center text-on-surface-variant hover:text-primary disabled:text-outline/40 transition-colors"
        title="Next match (Enter)"
        aria-label="Next match"
      >
        <span className="material-symbols-outlined text-[16px]">keyboard_arrow_down</span>
      </button>

      {/* Regex toggle */}
      <button
        onClick={() => setRegex(r => !r)}
        className={`w-6 h-6 flex items-center justify-center font-mono text-[11px] font-bold rounded-sm border transition-colors ${
          regex
            ? 'border-primary bg-primary/15 text-primary'
            : 'border-transparent text-outline hover:text-on-surface-variant'
        }`}
        title="Use regular expression"
        aria-label="Toggle regular expression"
        aria-pressed={regex}
      >
        .*
      </button>

      {/* Case-sensitive toggle */}
      <button
        onClick={() => setCaseSensitive(c => !c)}
        className={`w-6 h-6 flex items-center justify-center font-mono text-[11px] font-bold rounded-sm border transition-colors ${
          caseSensitive
            ? 'border-primary bg-primary/15 text-primary'
            : 'border-transparent text-outline hover:text-on-surface-variant'
        }`}
        title="Match case"
        aria-label="Toggle case sensitivity"
        aria-pressed={caseSensitive}
      >
        Aa
      </button>

      {/* Close */}
      <button
        onClick={onClose}
        className="w-6 h-6 flex items-center justify-center text-outline hover:text-error transition-colors"
        title="Close (Esc)"
        aria-label="Close search"
      >
        <span className="material-symbols-outlined text-[14px]">close</span>
      </button>
    </div>
  )
}
