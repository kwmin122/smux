import { useState, useCallback, useEffect } from 'react'

const isTauri = !!window.__TAURI_INTERNALS__

function isLocalhostUrl(url: string): boolean {
  try {
    const parsed = new URL(url)
    return parsed.hostname === 'localhost' || parsed.hostname === '127.0.0.1' || parsed.hostname === '[::1]'
  } catch {
    return false
  }
}

interface BrowserPanelProps {
  onClose: () => void
}

export function BrowserPanel({ onClose }: BrowserPanelProps) {
  const [inputUrl, setInputUrl] = useState('http://localhost:3000')
  const [activeUrl, setActiveUrl] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const openInWebview = useCallback(async (target: string) => {
    let normalized = target.trim()
    if (!normalized.startsWith('http://') && !normalized.startsWith('https://')) {
      normalized = 'http://' + normalized
    }
    if (!isLocalhostUrl(normalized)) {
      setError('Only localhost URLs are allowed')
      return
    }
    setError(null)

    if (isTauri) {
      try {
        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('open_browser_window', { url: normalized })
        setActiveUrl(normalized)
      } catch (e) {
        setError(String(e))
      }
    } else {
      setActiveUrl(normalized)
    }
  }, [])

  const closeBrowserWindow = useCallback(async () => {
    if (isTauri && activeUrl) {
      try {
        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('close_browser_window')
      } catch { /* ignore */ }
    }
    setActiveUrl(null)
    onClose()
  }, [activeUrl, onClose])

  // Open default URL on mount
  useEffect(() => {
    openInWebview(inputUrl)
    return () => {
      // Close browser window when panel unmounts
      if (isTauri) {
        import('@tauri-apps/api/core').then(({ invoke }) => {
          invoke('close_browser_window').catch(() => {})
        })
      }
    }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Enter') {
      openInWebview(inputUrl)
    }
  }

  return (
    <div className="flex flex-col h-full bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)] overflow-hidden">
      {/* Browser toolbar */}
      <div className="h-8 bg-surface-container-high px-2 flex items-center gap-2 border-b border-outline-variant/20 shrink-0">
        <input
          type="text"
          value={inputUrl}
          onChange={e => setInputUrl(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1 h-5 bg-surface-container-lowest border border-outline-variant/30 rounded-sm px-2 font-mono text-[11px] text-on-surface-variant outline-none focus:border-primary"
          spellCheck={false}
          placeholder="http://localhost:3000"
        />
        <button
          onClick={() => openInWebview(inputUrl)}
          className="w-6 h-6 flex items-center justify-center text-outline hover:text-primary transition-colors"
          title="Navigate"
          aria-label="Navigate"
        >
          <span className="material-symbols-outlined text-[14px]">open_in_new</span>
        </button>
        <button
          onClick={closeBrowserWindow}
          className="w-6 h-6 flex items-center justify-center text-outline hover:text-error transition-colors"
          title="Close browser (Cmd+B)"
          aria-label="Close browser"
        >
          <span className="material-symbols-outlined text-[14px]">close</span>
        </button>
      </div>

      {/* Status */}
      <div className="flex-1 flex items-center justify-center p-4">
        {error ? (
          <div className="font-mono text-[11px] text-error text-center">{error}</div>
        ) : activeUrl ? (
          <div className="font-mono text-[11px] text-on-surface-variant text-center space-y-2">
            <div className="text-primary">{activeUrl}</div>
            <div className="text-outline text-[10px]">Opened in native WebView window</div>
          </div>
        ) : (
          <div className="font-mono text-[11px] text-outline">Enter a localhost URL and press Enter</div>
        )}
      </div>
    </div>
  )
}
