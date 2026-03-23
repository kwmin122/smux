import { useCallback, useRef, useEffect } from 'react'

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

export type SplitDirection = 'horizontal' | 'vertical'

export interface SplitNode {
  id: string
  type: 'leaf' | 'split'
  /** For leaf nodes — the associated tab */
  tabId?: string
  /** For split nodes — how children are arranged */
  direction?: SplitDirection
  /** Exactly two children when type === 'split' */
  children?: [SplitNode, SplitNode]
  /** 0-100, percentage allocated to the first child */
  ratio?: number
}

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------

function uid(): string {
  return `split-${crypto.randomUUID()}`
}

/** Create a new leaf node for a given tab. */
export function createLeaf(tabId: string): SplitNode {
  return { id: uid(), type: 'leaf', tabId }
}

/**
 * Split an existing leaf into two panes.
 * The original leaf keeps its tabId in the first child;
 * the second child receives `newTabId`.
 * Returns a **new** tree (immutable).
 */
export function splitLeaf(
  root: SplitNode,
  leafId: string,
  direction: SplitDirection,
  newTabId: string,
): SplitNode {
  if (root.type === 'leaf') {
    if (root.id === leafId) {
      return {
        id: uid(),
        type: 'split',
        direction,
        ratio: 50,
        children: [
          { ...root },
          createLeaf(newTabId),
        ],
      }
    }
    return root
  }

  // split node — recurse into children
  const [a, b] = root.children!
  const newA = splitLeaf(a, leafId, direction, newTabId)
  const newB = splitLeaf(b, leafId, direction, newTabId)
  if (newA === a && newB === b) return root
  return { ...root, children: [newA, newB] }
}

/**
 * Remove a leaf from the tree.
 * Its sibling is promoted to take the parent split's place.
 * Returns `null` when the entire tree is removed.
 */
export function removeLeaf(root: SplitNode, leafId: string): SplitNode | null {
  if (root.type === 'leaf') {
    return root.id === leafId ? null : root
  }

  const [a, b] = root.children!

  // If one direct child is the target leaf, return the other child
  if (a.type === 'leaf' && a.id === leafId) return b
  if (b.type === 'leaf' && b.id === leafId) return a

  // Recurse
  const newA = removeLeaf(a, leafId)
  const newB = removeLeaf(b, leafId)

  // If a subtree collapsed, promote the surviving sibling
  if (newA === null) return newB
  if (newB === null) return newA

  if (newA === a && newB === b) return root
  return { ...root, children: [newA, newB] }
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface SplitContainerProps {
  root: SplitNode
  activeLeafId: string | null
  onActivateLeaf: (id: string) => void
  onResizeRatio: (splitId: string, ratio: number) => void
  renderLeaf: (node: SplitNode) => React.ReactNode
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SplitContainer({
  root,
  activeLeafId,
  onActivateLeaf,
  onResizeRatio,
  renderLeaf,
}: SplitContainerProps) {
  return (
    <div className="w-full h-full">
      <SplitNodeView
        node={root}
        activeLeafId={activeLeafId}
        onActivateLeaf={onActivateLeaf}
        onResizeRatio={onResizeRatio}
        renderLeaf={renderLeaf}
      />
    </div>
  )
}

// ---------------------------------------------------------------------------
// Recursive node renderer
// ---------------------------------------------------------------------------

interface SplitNodeViewProps {
  node: SplitNode
  activeLeafId: string | null
  onActivateLeaf: (id: string) => void
  onResizeRatio: (splitId: string, ratio: number) => void
  renderLeaf: (node: SplitNode) => React.ReactNode
}

function SplitNodeView({
  node,
  activeLeafId,
  onActivateLeaf,
  onResizeRatio,
  renderLeaf,
}: SplitNodeViewProps) {
  if (node.type === 'leaf') {
    const isActive = node.id === activeLeafId
    return (
      <div
        className={`w-full h-full min-w-0 min-h-0 overflow-hidden rounded-sm transition-[border-color] duration-150 ${
          isActive
            ? 'border border-primary/30'
            : 'border border-transparent'
        }`}
        onClick={(e) => {
          e.stopPropagation()
          onActivateLeaf(node.id)
        }}
      >
        {renderLeaf(node)}
      </div>
    )
  }

  // Split node
  const { direction, children, ratio = 50, id: splitId } = node
  const [first, second] = children!
  const isHorizontal = direction === 'horizontal'

  return (
    <div
      className={`flex w-full h-full min-w-0 min-h-0 ${
        isHorizontal ? 'flex-col' : 'flex-row'
      }`}
    >
      {/* First child */}
      <div
        className="min-w-0 min-h-0 overflow-hidden"
        style={{ flexBasis: `${ratio}%`, flexGrow: 0, flexShrink: 0 }}
      >
        <SplitNodeView
          node={first}
          activeLeafId={activeLeafId}
          onActivateLeaf={onActivateLeaf}
          onResizeRatio={onResizeRatio}
          renderLeaf={renderLeaf}
        />
      </div>

      {/* Draggable divider */}
      <Divider
        splitId={splitId}
        direction={direction!}
        onResizeRatio={onResizeRatio}
      />

      {/* Second child */}
      <div
        className="min-w-0 min-h-0 overflow-hidden"
        style={{ flexBasis: `${100 - ratio}%`, flexGrow: 0, flexShrink: 0 }}
      >
        <SplitNodeView
          node={second}
          activeLeafId={activeLeafId}
          onActivateLeaf={onActivateLeaf}
          onResizeRatio={onResizeRatio}
          renderLeaf={renderLeaf}
        />
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Divider
// ---------------------------------------------------------------------------

interface DividerProps {
  splitId: string
  direction: SplitDirection
  onResizeRatio: (splitId: string, ratio: number) => void
}

function Divider({ splitId, direction, onResizeRatio }: DividerProps) {
  const dividerRef = useRef<HTMLDivElement>(null)
  const dragging = useRef(false)

  const isHorizontal = direction === 'horizontal'

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      e.stopPropagation()
      dragging.current = true

      const parentEl = dividerRef.current?.parentElement
      if (!parentEl) return

      // Prevent text selection while dragging
      document.body.style.userSelect = 'none'
      document.body.style.cursor = isHorizontal ? 'row-resize' : 'col-resize'

      const handleMouseMove = (moveEvent: MouseEvent) => {
        if (!dragging.current || !parentEl) return

        const rect = parentEl.getBoundingClientRect()
        let newRatio: number

        if (isHorizontal) {
          newRatio = ((moveEvent.clientY - rect.top) / rect.height) * 100
        } else {
          newRatio = ((moveEvent.clientX - rect.left) / rect.width) * 100
        }

        // Clamp between 10% and 90% to keep both panes usable
        newRatio = Math.max(10, Math.min(90, newRatio))
        onResizeRatio(splitId, Math.round(newRatio * 100) / 100)
      }

      const handleMouseUp = () => {
        dragging.current = false
        document.body.style.userSelect = ''
        document.body.style.cursor = ''
        document.removeEventListener('mousemove', handleMouseMove)
        document.removeEventListener('mouseup', handleMouseUp)
      }

      document.addEventListener('mousemove', handleMouseMove)
      document.addEventListener('mouseup', handleMouseUp)
    },
    [splitId, isHorizontal, onResizeRatio],
  )

  // Clean up on unmount in case a drag is in progress
  useEffect(() => {
    return () => {
      document.body.style.userSelect = ''
      document.body.style.cursor = ''
    }
  }, [])

  return (
    <div
      ref={dividerRef}
      onMouseDown={handleMouseDown}
      className={`relative flex-shrink-0 group ${
        isHorizontal
          ? 'h-1 w-full cursor-row-resize'
          : 'w-1 h-full cursor-col-resize'
      }`}
    >
      {/* Visible 1px line centred inside the 4px hit area */}
      <div
        className={`absolute bg-outline-variant/40 group-hover:bg-primary/50 transition-colors ${
          isHorizontal
            ? 'left-0 right-0 top-1/2 h-px -translate-y-1/2'
            : 'top-0 bottom-0 left-1/2 w-px -translate-x-1/2'
        }`}
      />
    </div>
  )
}
