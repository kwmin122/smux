import { useState, useRef, useCallback } from 'react'

interface BrowserPanelProps {
  onClose: () => void
}

export function BrowserPanel({ onClose }: BrowserPanelProps) {
  const [url, setUrl] = useState('http://localhost:3000')
  const [inputUrl, setInputUrl] = useState('http://localhost:3000')
  const iframeRef = useRef<HTMLIFrameElement>(null)

  const navigate = useCallback((target: string) => {
    let normalized = target.trim()
    if (!normalized.startsWith('http://') && !normalized.startsWith('https://')) {
      normalized = 'http://' + normalized
    }
    // Only allow localhost URLs for security — prevents loading arbitrary external content
    try {
      const parsed = new URL(normalized)
      if (parsed.hostname !== 'localhost' && parsed.hostname !== '127.0.0.1' && parsed.hostname !== '[::1]') {
        return // silently reject non-localhost URLs
      }
    } catch {
      return // reject malformed URLs
    }
    setUrl(normalized)
    setInputUrl(normalized)
  }, [])

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Enter') {
      navigate(inputUrl)
    }
  }

  function reload() {
    if (iframeRef.current) {
      iframeRef.current.src = url
    }
  }

  return (
    <div className="flex flex-col h-full bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)] overflow-hidden">
      {/* Browser toolbar */}
      <div className="h-8 bg-surface-container-high px-2 flex items-center gap-2 border-b border-outline-variant/20 shrink-0">
        <div className="flex items-center gap-1">
          <button
            onClick={() => iframeRef.current?.contentWindow?.history.back()}
            className="w-6 h-6 flex items-center justify-center text-outline hover:text-primary transition-colors"
            title="Back"
          >
            <span className="material-symbols-outlined text-[14px]">arrow_back</span>
          </button>
          <button
            onClick={() => iframeRef.current?.contentWindow?.history.forward()}
            className="w-6 h-6 flex items-center justify-center text-outline hover:text-primary transition-colors"
            title="Forward"
          >
            <span className="material-symbols-outlined text-[14px]">arrow_forward</span>
          </button>
          <button
            onClick={reload}
            className="w-6 h-6 flex items-center justify-center text-outline hover:text-primary transition-colors"
            title="Reload"
          >
            <span className="material-symbols-outlined text-[14px]">refresh</span>
          </button>
        </div>
        <input
          type="text"
          value={inputUrl}
          onChange={e => setInputUrl(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1 h-5 bg-surface-container-lowest border border-outline-variant/30 rounded-sm px-2 font-mono text-[11px] text-on-surface-variant outline-none focus:border-primary"
          spellCheck={false}
        />
        <button
          onClick={onClose}
          className="w-6 h-6 flex items-center justify-center text-outline hover:text-error transition-colors"
          title="Close browser (Cmd+B)"
        >
          <span className="material-symbols-outlined text-[14px]">close</span>
        </button>
      </div>

      {/* Browser content */}
      <div className="flex-1 overflow-hidden bg-white">
        <iframe
          ref={iframeRef}
          src={url}
          className="w-full h-full border-0"
          sandbox="allow-scripts allow-forms allow-popups"
          title="Browser preview"
        />
      </div>
    </div>
  )
}
