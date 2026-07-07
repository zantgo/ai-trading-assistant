# Design System: Grayscale Monochrome Dark Mode

## Philosophy

The AI Trading Assistant dashboard follows a strict **grayscale monochrome dark mode** design, inspired by Apple's native dark mode aesthetics. The goal is a clean, distraction-free professional trading desk where visual noise is minimized and semantic trading data takes precedence.

### Constraints

1. **Dark Mode Only** — No light mode or dynamic theme switching exists. The interface is exclusively dark.
2. **Semantic Color Preservation** — Green (bullish/win/long) and red (bearish/loss/short) are the only chromatic colors permitted. They are reserved exclusively for directional trading signals.
3. **No Decorative Chroma** — All borders, active states, buttons, sliders, branding, and chart crosshairs use only grayscale values.

---

## Color Palette

### Background Spectrum

| Token | Hex | Usage |
|-------|-----|-------|
| `--bg-obsidian` | `#060608` | Deepest page background |
| `--bg-dark` | `#0b0c10` | Primary surface background |
| `--bg-panel` | `#11131a` | Card and panel surfaces |
| `--bg-card` | `#161a24` | Elevated card backgrounds |
| `--bg-card-hover` | `#1e2330` | Card hover state |

### Borders & Structure

| Token | Hex | Usage |
|-------|-----|-------|
| `--border-muted` | `#1c212e` | Default container borders |
| `--border-active` | `#2d3448` | Elevated border (cards, inputs) |
| `--border-glow` | `rgba(255,255,255,0.06)` | Subtle highlight glow |

### Text Hierarchy

| Token | Hex | Usage |
|-------|-----|-------|
| `--text-primary` | `#f8fafc` | Headings, primary content |
| `--text-secondary` | `#94a3b8` | Body text, labels |
| `--text-muted` | `#64748b` | Secondary labels, metadata |
| `--text-dim` | `#475569` | Disabled/hint text |

### Monochrome Accent System

| Token | Value | Usage |
|-------|-------|-------|
| `--color-accent-mono` | `#ffffff` | Maximum emphasis highlights |
| `--color-accent-mono-dim` | `#94a3b8` | Secondary accent (borders, focus rings) |
| `--color-accent-strong` | `#cbd5e1` | Chart lines, prominent UI elements |
| `--color-accent-glow` | `rgba(255,255,255,0.06)` | Subtle background glow |

Backwards-compatible aliases:
- `--accent` = `--color-accent-mono`
- `--accent-bg` = `rgba(255,255,255,0.08)`
- `--accent-border` = `rgba(255,255,255,0.2)`

### Semantic Trading Signals (Preserved)

| Token | Hex | Usage |
|-------|-----|-------|
| `--signal-bullish` | `#10b981` | Bullish/long/profit/green |
| `--signal-bearish` | `#ef4444` | Bearish/short/loss/red |

These are the **only** chromatic colors in the system. They appear in:
- Price direction indicators (long/short labels)
- P&L displays (positive/negative)
- Divergence confirmation states
- Candlestick up/down colors (via lightweight-charts config)
- Risk calculator results
- Exit/close buttons (red only — destructive action)

---

## UI Component Guidelines

### Buttons

**Primary Action** (e.g., "Enter Dashboard", "Request Analysis", "Apply"):
- Background: `#f8fafc` (white)
- Text: `#0b0c10` (dark)
- Border: `1px solid #334155` (thin charcoal)
- Hover: `opacity: 0.85`
- Disabled: `opacity: 0.5`

**Secondary Action** (e.g., settings save, chat send):
- Background: `#334155` (slate)
- Text: `#f8fafc` (white)
- Border: `1px solid #475569`
- Hover: `opacity: 0.85`

**Destructive Action** (e.g., close position):
- Preserves red styling: `color: #ef4444`, `border-color: rgba(239,68,68,0.35)`

### Active States

**Tabs & Selectors:**
- Border: `#94a3b8` (silver-gray)
- Text: `#f8fafc` (white)
- Background: `rgba(255,255,255,0.08)`
- NO colored underlines, NO colored glows

**Input Focus:**
- Border: `#94a3b8` (replaces former `#facc15` yellow focus ring)

**Radio Buttons / Checkboxes:**
- Accent color: `#94a3b8`

### Sliders & Range Inputs

- Track: `#1c212e` (dark)
- Thumb: `#94a3b8` (silver-gray)
- Fill: `#94a3b8` or `#cbd5e1`

### Cards & Panels

- Default border: `--border-active` (`#2d3448`)
- Active/highlighted card border: `#94a3b8` or `rgba(255,255,255,0.12)`

### Chart Components (Lightweight Charts)

**Crosshairs:**
- Vertical & horizontal lines: `#334155` (subtle charcoal, replaces `#ca8a04` yellow)
- Style: Dashed (3)

**Non-semantic Series Lines:**
- Compound balance curve: `#e2e8f0`
- MACD zero line: `#334155`
- Aroon midlines (±50): `#334155`
- Z-Score zero line: `#334155`
- Entry price line: `#94a3b8`
- Stop-loss indicator: `#94a3b8`
- Margin baseline: `#94a3b8`

**Semantic Series Lines (Preserved):**
- Support levels: `#10b981` (green)
- Resistance levels: `#ef4444` (red)
- Confirmed divergence: `#22c55e` (green)

### Progress & Loading

- Spinner border: `rgba(255,255,255,0.12)`
- Spinner accent: `#94a3b8`
- Pulse animation: white glow (replaces yellow glow)
- Running status: `#94a3b8` text (replaces `#facc15` yellow)

### Status Badges

- Completed/Success: `#10b981` (green — preserved)
- Failed/Error: `#ef4444` (red — preserved)
- Neutral/Warning/Sideways: `#94a3b8` or `#f8fafc` on `rgba(255,255,255,0.08)` background

---

## Typography

| Token | Font Stack |
|-------|-----------|
| `--font-mono` | `'JetBrains Mono', 'Fira Code', 'Courier New', monospace` |
| `--font-sans` | `system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif` |

- Body text: `var(--sans)` at 18px/145%
- Code & data: `var(--mono)`
- Headings: `var(--heading)` (sans-serif, weight 500)

---

## Migration History

### From: Obsidian Neon (Pre-2026-07)

The previous design used a high-contrast neon-yellow accent system:
- `--color-neon-yellow: #facc15`
- `--color-neon-yellow-dim: #ca8a04`
- Active states, buttons, borders, chart crosshairs, and loading indicators were all yellow/amber

### To: Apple Dark Mode (2026-07)

All yellow/amber/gold decorative colors replaced with grayscale values. Additionally, all non-semantic blue (`#3498db`) and cyan (`#64ffda`) accents were converted to monochrome. Green and red trading signal colors are the only chromatic colors remaining.

---

## Maintenance Rules

1. **No new chromatic colors** may be introduced without explicit design review.
2. Green (`#10b981`, `#22c55e`) and red (`#ef4444`) are reserved for directional trading semantics only.
3. All UI chrome (borders, active indicators, buttons, sliders) must use colors from the Monochrome Accent System or the Text Hierarchy.
4. Chart crosshairs and zero-lines use `#334155` (slate).
5. Chart decorative series use `#e2e8f0` or `#94a3b8`.
6. New CSS variables should follow the naming convention `--color-accent-mono-*` or extend the existing hierarchy.
