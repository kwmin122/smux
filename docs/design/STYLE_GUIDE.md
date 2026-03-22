# smux Style Guide

> Design principles derived from competitive analysis of Warp, Ghostty, cmux, and Cursor.

## Core Principles

1. **Terminal-first**: Maximum screen real estate for terminal content
2. **Minimal chrome**: Headers and sidebars should feel invisible until needed
3. **Consistent spacing**: 4px grid system (4, 8, 12, 16, 24, 32)
4. **Monospace everything**: Terminal apps should feel technical and precise
5. **Dark by default**: Optimized for dark environments and long coding sessions

## Typography

| Element | Font | Size | Weight | Tracking |
|---------|------|------|--------|----------|
| App title | Headline font | 14px | Bold | -0.02em |
| Section headers | Mono | 10px | Bold | 0.1em (uppercase) |
| Body text | Mono | 11-12px | Regular | 0 |
| Terminal | Mono (configurable) | 13-14px | Regular | 0 |
| Labels | Mono | 10px | Regular | 0.1em (uppercase) |
| Status badges | Mono | 9px | Bold | 0.05em |
| Tiny metadata | Mono | 8-9px | Regular | 0 |

## Spacing

- Sidebar width: 48px (icon-only) or 200px (expanded)
- Header height: 36px (was 40px — reduce for more terminal space)
- Panel padding: 4px
- Tab item height: 32px
- Status bar height: 24px
- Border radius: 2px (minimal) or 0px (brutalist themes)

## Colors

All colors are CSS variables. Three themes defined:
- Deep Navy (default): Cool blue-gray
- Amber: Warm orange-brown
- Forest Green: Nature-inspired

Key relationships:
- Background hierarchy: lowest → low → default → high → highest
- Text: on-surface (primary), on-surface-variant (secondary), outline (tertiary)
- Accent: primary (actions), secondary (success/AI), tertiary (warnings), error

## Icons

Use Material Symbols Outlined, weight 300, size 14-18px:
- `terminal` — terminal tab
- `add` — new tab/action
- `close` — close/dismiss
- `settings` — settings
- `search` — search
- `error` — errors
- `auto_fix_high` — AI fix
- `fullscreen` / `fullscreen_exit` — panel zoom

## Component Patterns

### Buttons
- Primary: `bg-primary text-on-primary` — main actions
- Ghost: `text-outline hover:text-primary` — secondary actions
- Danger: `text-error hover:bg-error/10` — destructive actions
- Size: height 24-28px, font-mono text-[10px]

### Badges
- Status: `font-mono text-[9px] px-1.5 py-0.5 rounded`
- Colors: success=secondary, error=error, warning=tertiary, info=primary

### Overlays
- Background: `bg-surface-container` or `bg-surface-container-high`
- Border: `border border-outline-variant/20`
- Shadow: `shadow-lg`
- Backdrop: `bg-black/50` for modals

## Layout Reference (Target)

```
┌─────────────────────────────────────────────┐
│ ▫ smux      [tab1] [tab2] [+]    ⚙        │ ← 36px header w/ inline tabs
├────┬────────────────────────────────────────┤
│ 📂 │                                        │
│ 🖥️ │  Terminal Content (maximum space)       │
│ 🖥️ │                                        │
│ 🤖 │                                        │
│    │  [sticky scroll: $ cargo build]        │
│    │  ● success                              │
│    │  ✕ error output here...                 │
│    │  ⏳ running...                          │
├────┼────────────────────────────────────────┤
│    │ zsh  ~/project  main  ⌘F               │ ← 24px status bar
└────┴────────────────────────────────────────┘
  ↑
  48px icon sidebar
```
