# smux Design System

> Extracted from 9 Stitch screens. Single source of truth for v0.3 Tauri UI implementation.

## Screens Reference

| Screen | Theme | File |
|--------|-------|------|
| Operator Console (Focus) | Amber | `screens/smux-operator-console-1.html` |
| Operator Console (Control) | Amber | `screens/smux-operator-console-2.html` |
| Rewind Mode | Amber | `screens/smux-rewind-mode-1.html` |
| Rewind Mode (variant) | Amber | `screens/smux-rewind-mode-2.html` |
| Console | Forest Green | `screens/smux-console-forest-green.html` |
| Console | Deep Navy | `screens/smux-console-deep-navy.html` |
| Rewind | Forest Green | `screens/smux-rewind-forest-green.html` |
| Compare | Deep Navy | `screens/smux-compare-deep-navy.html` |
| Rewind | Deep Navy | `screens/smux-rewind-deep-navy.html` |

---

## 1. Themes

Three themes. User selects in settings. Default: Deep Navy.

### Deep Navy (Default)

```
primary:                    #a4c9ff
primary-container:          #60a5fa
primary-fixed:              #d4e3ff
primary-fixed-dim:          #a4c9ff
on-primary:                 #00315d
on-primary-container:       #003a6b
on-primary-fixed:           #001c39
on-primary-fixed-variant:   #004883

secondary:                  #44e2cd
secondary-container:        #03c6b2
secondary-fixed:            #62fae3
secondary-fixed-dim:        #3cddc7
on-secondary:               #003731
on-secondary-container:     #004d44
on-secondary-fixed:         #00201c
on-secondary-fixed-variant: #005047

tertiary:                   #ffb3b0
tertiary-container:         #ff7978
tertiary-fixed:             #ffdad8
tertiary-fixed-dim:         #ffb3b0
on-tertiary:                #670211
on-tertiary-container:      #740d18
on-tertiary-fixed:          #410006
on-tertiary-fixed-variant:  #881d24

error:                      #ffb4ab
error-container:            #93000a
on-error:                   #690005
on-error-container:         #ffdad6

background:                 #10131a
surface:                    #10131a
surface-dim:                #10131a
surface-variant:            #32353c
surface-bright:             #363940
surface-tint:               #a4c9ff
surface-container-lowest:   #0b0e14
surface-container-low:      #191c22
surface-container:          #1d2026
surface-container-high:     #272a31
surface-container-highest:  #32353c

on-surface:                 #e1e2eb
on-surface-variant:         #c1c7d3
on-background:              #e1e2eb
outline:                    #8b919d
outline-variant:            #414751

inverse-surface:            #e1e2eb
inverse-on-surface:         #2e3037
inverse-primary:            #0060ac
```

### Amber

```
primary:                    #ffb77d
primary-container:          #d97707
primary-fixed:              #ffdcc3
primary-fixed-dim:          #ffb77d
on-primary:                 #4d2600
on-primary-container:       #432100
on-primary-fixed:           #2f1500
on-primary-fixed-variant:   #6e3900

secondary:                  #d9c1bb
secondary-container:        #54433f
secondary-fixed:            #f6ddd7
secondary-fixed-dim:        #d9c1bb
on-secondary:               #3c2d29
on-secondary-container:     #c7b0aa
on-secondary-fixed:         #251815
on-secondary-fixed-variant: #54433f

tertiary:                   #96d947
tertiary-container:         #63a109
tertiary-fixed:             #b1f661
tertiary-fixed-dim:         #96d947
on-tertiary:                #1e3700
on-tertiary-container:      #192f00
on-tertiary-fixed:          #0f2000
on-tertiary-fixed-variant:  #2e4f00

error:                      #ffb4ab
error-container:            #93000a
on-error:                   #690005
on-error-container:         #ffdad6

background:                 #181210
surface:                    #181210
surface-dim:                #181210
surface-variant:            #3b3331
surface-bright:             #3f3835
surface-tint:               #ffb77d
surface-container-lowest:   #130d0b
surface-container-low:      #201a18
surface-container:          #251e1c
surface-container-high:     #2f2826
surface-container-highest:  #3b3331

on-surface:                 #ede0dc
on-surface-variant:         #dbc2b0
on-background:              #ede0dc
outline:                    #a38c7c
outline-variant:            #554336

inverse-surface:            #ede0dc
inverse-on-surface:         #362f2c
inverse-primary:            #904d00
```

### Forest Green

```
primary:                    #6bfb9a
primary-container:          #4ade80
primary-fixed:              #6dfe9c
primary-fixed-dim:          #4de082
on-primary:                 #003919
on-primary-container:       #005e2d
on-primary-fixed:           #00210c
on-primary-fixed-variant:   #005227

secondary:                  #4ae176
secondary-container:        #00b954
secondary-fixed:            #6bff8f
secondary-fixed-dim:        #4ae176
on-secondary:               #003915
on-secondary-container:     #004119
on-secondary-fixed:         #002109
on-secondary-fixed-variant: #005321

tertiary:                   #ffd7d3
tertiary-container:         #ffb0aa
tertiary-fixed:             #ffdad7
tertiary-fixed-dim:         #ffb3ad
on-tertiary:                #68000a
on-tertiary-container:      #a50318
on-tertiary-fixed:          #410004
on-tertiary-fixed-variant:  #930013

error:                      #ffb4ab
error-container:            #93000a
on-error:                   #690005
on-error-container:         #ffdad6

background:                 #121413
surface:                    #121413
surface-dim:                #121413
surface-variant:            #333534
surface-bright:             #383a38
surface-tint:               #4de082
surface-container-lowest:   #0d0f0e
surface-container-low:      #1a1c1b
surface-container:          #1e201f
surface-container-high:     #282a29
surface-container-highest:  #333534

on-surface:                 #e2e3e0
on-surface-variant:         #bccabb
on-background:              #e2e3e0
outline:                    #869486
outline-variant:            #3d4a3e

inverse-surface:            #e2e3e0
inverse-on-surface:         #2f3130
inverse-primary:            #006d36
```

---

## 2. Typography

### Per-Theme Font Mapping

| Role | Deep Navy | Amber | Forest Green |
|------|-----------|-------|--------------|
| headline | Inter | Space Grotesk | Space Grotesk |
| body | Inter | Inter | JetBrains Mono |
| label | Inter | Inter | Space Grotesk |
| mono | JetBrains Mono | JetBrains Mono | JetBrains Mono |

Weights: 300, 400, 500, 600, 700, 800, 900

### Scale

| Token | Size | Usage |
|-------|------|-------|
| micro | 6px | Micro-labels |
| xxs | 8-9px | Dense meta text, tags |
| xs | 10px | Status bar, labels, uppercase tags |
| sm | 11px | Panel headers, log lines |
| base | 12-13px | Terminal content, body |
| lg | 14px | Secondary headings |
| xl | 16-18px | Section headers |
| 2xl | 20-24px | Primary headings, logo |

Letter-spacing: `tracking-widest` for uppercase labels, `tracking-tight` for nav items, `tracking-[-0.02em]` for logo.

---

## 3. Layout

### Shell Structure

```
┌─────────────────────────────────────────────────────┐
│ TOP BAR (h-12)  Logo │ Tabs │ Search │ Icons        │
├──────┬──────────────────────────────────────────────┤
│      │                                              │
│ SIDE │            MAIN CONTENT                      │
│ BAR  │   ┌──────────────┬──────────────┐            │
│      │   │  PLANNER     │  VERIFIER    │            │
│      │   │  TERMINAL    │  TERMINAL    │            │
│      │   │              │              │            │
│      │   └──────────────┴──────────────┘            │
│      │                                              │
├──────┴──────────────────────────────────────────────┤
│ STATUS BAR (h-8)  CPU │ Latency │ Git │ Uptime      │
└─────────────────────────────────────────────────────┘
```

### Dimensions

| Element | Size |
|---------|------|
| Top bar | h-12 (48px), fixed top-0 |
| Sidebar | w-64 (full) / w-16 (collapsed, icon-only) |
| Status bar | h-8 (32px), fixed bottom-0 |
| Panel header | h-8 to h-10 |
| Minimum window | 1280 x 800 |

Sidebar state: Focus/Rewind modes use w-64 (full nav). Operator Console may use w-16 (icon-only).

### Responsive Breakpoints

| Breakpoint | Width | Usage |
|------------|-------|-------|
| sm | 640px | Show sidebar labels, rewind controls |
| md | 768px | Switch to flex-row layouts, show secondary panels |
| lg | 1024px | Full multi-column grid (e.g. lg:col-span-7) |

### Z-Index Layers

| Layer | z-index | Usage |
|-------|---------|-------|
| Base content | z-10 | Inline overlays, tooltips |
| Panels | z-20 | Floating panels |
| Sidebar header | z-30 | Sticky sidebar headers |
| Sidebar | z-40 | Left navigation sidebar |
| Top bar / Status bar | z-50 | Fixed chrome |

### Modes

**Focus Mode** — 2-panel split (Planner | Verifier)
**Control Mode** — 3-panel (Planner | Mission Control | Verifier), Tab toggle
**Rewind Mode** — Terminal + Timeline scrubber + Event history sidebar
**Compare Mode** — Snapshot diff + Live state + Timeline sidebar

---

## 4. Components

### Top Bar
```
bg-background | h-12 | flex justify-between items-center px-4 | fixed top-0 w-full z-50
Logo: font-headline font-bold text-xl tracking-[-0.02em] text-on-surface
Tabs: text-xs uppercase tracking-widest font-mono
Active tab: text-primary border-b-2 border-primary
```

### Sidebar Navigation
```
bg-surface-container-low | flex flex-col h-full | z-40
Full: w-64 | Collapsed: w-16

Item: flex items-center gap-3 px-4 py-3 text-sm
Active: bg-primary/10 text-primary border-l-2 border-primary
Hover: hover:bg-surface-container-high transition-all
Icon: material-symbols-outlined text-[18px]
```

### Terminal Panel
```
bg-surface-container-lowest | border border-outline-variant/20
Header: h-8 bg-surface-container-high px-3 flex items-center
  Title: font-mono text-[10px] font-bold uppercase tracking-widest
Content: p-4 font-mono text-[13px] leading-relaxed overflow-y-auto
Cursor: inline-block w-2 h-[1.2em] bg-primary shadow-[0_0_4px_var(--primary)]
  animation: blink 1s step-end infinite
```

### Status Bar
Per-theme background:
- Deep Navy: `bg-surface-container-high` (#272a31)
- Amber: `bg-surface-container` (#251e1c)
- Forest Green: `bg-surface-container-lowest` (#0d0f0e)

```
fixed bottom-0 w-full h-8 z-50
flex items-center justify-end gap-6 px-4
font-mono text-[10px] uppercase text-outline
Metrics: CPU ● | LATENCY | GIT | UPTIME
Status dot: w-2 h-2 rounded-full bg-[status-color]
```

### Buttons
```
Primary: bg-primary text-on-primary px-4 py-2 font-bold text-xs uppercase
  hover: shadow-[0_0_15px_rgba(var(--primary-rgb),0.3)]
Secondary: bg-surface-container-high text-on-surface border border-outline-variant
  hover: border-primary/30
```

### Status Indicators
```
Dot: w-2 h-2 rounded-full
  Success: bg-secondary
  Warning: bg-tertiary
  Error: bg-error
  Active: animate-pulse
Badge: px-1 text-[8px] font-bold uppercase border
  bg-[color]/10 text-[color] border-[color]/30
```

### Diff Lines (Compare Mode)
```
Added: bg-secondary/10 border-l-2 border-secondary
Removed: bg-error/10 border-l-2 border-error
Line number: text-right text-outline w-8 font-mono text-[11px]
```

### Timeline Scrubber (Rewind Mode)
```
Container: h-24 bg-surface-container-lowest border-t border-outline-variant
Track: relative h-8 — bg-outline-variant/30 line, bg-primary progress
Markers: w-1.5 h-1.5 bg-primary rotate-45 (events)
Current: w-3 h-3 bg-primary shadow-[0_0_10px_var(--primary)] border-2 border-background
Controls: play/pause/skip material-symbols-outlined text-[18px]
Time display: font-mono text-sm text-on-surface
```

### Event History (Rewind Sidebar)
```
w-80 bg-surface-container-low border-l border-outline-variant
Item: p-4 border-b border-outline-variant/30
Selected: border-l-4 border-primary bg-surface-container-highest
Tag: px-1 bg-[color]/10 text-[color] text-[8px] font-bold border
```

---

## 5. Effects

### Keyframes
```css
@keyframes blink {
  from, to { opacity: 1; }
  50% { opacity: 0; }
}
```

### Terminal Glow
```css
box-shadow: inset 0 0 20px rgba(var(--primary-rgb), 0.05);
```

### Glass Panel
```css
background: rgba(29, 32, 38, 0.7);
backdrop-filter: blur(12px);
```

### Scanline Overlay (optional, Amber/Green themes)
```css
background: linear-gradient(rgba(18,16,16,0) 50%, rgba(0,0,0,0.1) 50%);
background-size: 100% 2px;
pointer-events: none;
```

### Scrollbar
```css
::-webkit-scrollbar { width: 4px; }
::-webkit-scrollbar-track { background: var(--surface-dim); }
::-webkit-scrollbar-thumb { background: var(--surface-container-highest); }
::-webkit-scrollbar-thumb:hover { background: var(--primary); }
```

### Hover Glow
```css
text-shadow: 0 0 8px rgba(var(--primary-rgb), 0.4);
box-shadow: 0 0 15px rgba(var(--primary-rgb), 0.3);
```

---

## 6. Border Radius

Per-theme border-radius:

| Token | Amber / Forest Green | Deep Navy |
|-------|---------------------|-----------|
| DEFAULT | 0px | 0.125rem (2px) |
| lg | 0px | 0.25rem (4px) |
| xl | 0px | 0.5rem (8px) |
| full | 9999px | 0.75rem (12px) |

Amber/Green: sharp industrial look, zero rounding.
Deep Navy: subtle rounding on cards and containers.

---

## 7. Icons

**Material Symbols Outlined** — Google Fonts CDN
- Default: FILL 0, WGHT 400, OPSZ 24
- Size in UI: 18px (font-size: 18px)
- Filled variant: `font-variation-settings: 'FILL' 1`

Key icons:
```
terminal, settings, history, hub, monitor_heart, security
fast_rewind, play_arrow, pause, skip_next, skip_previous
check_circle, error, verified_user, difference
keyboard_tab, pan_tool, account_tree, search
notifications, dns, radar, lock, database, memory, lan
settings_backup_restore, architecture, event_note
```

---

## 8. Keyboard Shortcuts

| Key | Action | Source |
|-----|--------|--------|
| Tab | Toggle Focus / Control mode | Task 19, 20 |
| i | Intervene (pause agent) | Task 19 |
| r | Enter Rewind mode | Task 19 |
| d | Show Diff / Compare | Task 19 |
| q | Quit session | Task 19 (bottom bar) |
| Cmd+B | Toggle Browser panel | Task 21 |
| Cmd+1/2/3 | Layout presets | Task 22 |
| Cmd+S | Save custom layout | Task 22 |
| Cmd+F | Fullscreen panel | Task 22 |

---

## 9. Implementation Notes

- All themes use CSS custom properties (`--primary`, `--surface`, etc.)
- Theme switching: toggle `data-theme` attribute on `<html>`
- Tailwind config references CSS vars: `primary: 'var(--primary)'`
- Terminal panels use xterm.js with theme colors mapped
- Font loading: Google Fonts CDN (Inter, JetBrains Mono, Space Grotesk, Material Symbols)
- Border-radius is theme-dependent (see section 6)
- Status bar background is theme-dependent (see section 4)
- Top bar uses `background` token, not `surface-container`
- Status bar metrics come from daemon IPC (CPU, latency, git branch, uptime)
