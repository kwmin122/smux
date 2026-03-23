import { useEffect, useState } from 'react'

interface HasSelection {
  getSelection(): string
}

/**
 * Hook for Selection-to-AI (⌘L): select text in terminal, send to AI as context.
 *
 * When ⌘L is pressed and there's a selection in the terminal,
 * the selected text is read directly from the terminal ref and passed to the callback.
 */
export function useSelectionToAi(
  terminalRef: React.RefObject<HasSelection | null>,
  onSendToAi?: (text: string) => void,
) {
  const [showToast, setShowToast] = useState(false)

  // ⌘L keyboard handler — reads selection directly from terminal to avoid stale closure
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'l') {
        const selection = terminalRef.current?.getSelection()
        if (selection && onSendToAi) {
          e.preventDefault()
          onSendToAi(selection)
          setShowToast(true)
          setTimeout(() => setShowToast(false), 2000)
        }
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [terminalRef, onSendToAi])

  return {
    showToast,
  }
}
