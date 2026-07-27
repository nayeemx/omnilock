# OmniLock Design System

## Brand Identity

- **Product name**: OmniLock
- **Tagline**: Enterprise Desktop Security
- **By**: InnologyBD

---

## Icon System

### Primary Logo (Dashboard / App Header)

- **Component**: Lucide `Shield` icon (`lucide-react`)
- **Stroke width**: 2.5
- **Size**: 24x24px (w-6 h-6) inside 44x44px (w-11 h-11) container
- **Container**: Rounded-xl (`border-radius: 0.75rem`)
- **Background**: `var(--gradient-brand)` — `linear-gradient(135deg, oklch(0.65 0.18 210), oklch(0.55 0.22 295))` (cyan-to-violet)
- **Icon color**: `text-primary-foreground` (white in both themes)
- **Glow effect**: `.glow-cyan` overlay with 60% opacity

### Taskbar / Window Icon (Windows Shell)

- **File**: `src-tauri/icons/icon.ico` (must match the Shield logo design)
- **Also used**: `src-tauri/icons/icon.png` (512x512 source)
- **Sizes required in ICO**: 16, 32, 48, 64, 128, 256px
- **Background**: Transparent or matches `var(--gradient-brand)` cyan-to-violet gradient
- **Symbol**: Must be the same Shield shape as the dashboard logo
- **IMPORTANT**: The taskbar icon MUST visually match the dashboard Shield logo. Do not use a different lock/shield design for the .ico file.

### Status Icons

| State | Icon | Color |
|-------|------|-------|
| Protected / Locked | `Shield` / `Lock` | `var(--primary)` (cyan) |
| Unprotected / Unlocked | `Unlock` | `var(--muted-foreground)` |
| Active / Running | `Activity` | `var(--success)` |
| Warning | `AlertTriangle` | `var(--warning)` |
| Danger / Deny | `ShieldAlert` | `var(--destructive)` |
| 2FA Enforced | `Fingerprint` | `var(--violet)` |

---

## Color System

### Design Tokens (CSS Custom Properties)

All colors use OKLCH color space for perceptual uniformity.

#### Primary Palette

| Token | Light | Dark | Usage |
|-------|-------|------|-------|
| `--primary` | `oklch(0.65 0.18 210)` | `oklch(0.78 0.16 210)` | Cyan accent — buttons, active states, brand |
| `--accent` | `oklch(0.55 0.22 295)` | `oklch(0.65 0.22 295)` | Violet accent — 2FA, secondary brand |
| `--success` | `oklch(0.65 0.17 155)` | `oklch(0.75 0.17 155)` | Green — healthy, active, verified |
| `--warning` | `oklch(0.72 0.17 75)` | `oklch(0.82 0.17 75)` | Amber — alerts, caution |
| `--destructive` | `oklch(0.60 0.22 25)` | `oklch(0.65 0.22 25)` | Red — remove, danger, deny |

#### Gradient

```css
--gradient-brand: linear-gradient(135deg, var(--primary), var(--accent));
```

#### Surfaces

| Token | Light | Dark |
|-------|-------|------|
| `--background` | `oklch(0.97 0.01 260)` | `oklch(0.14 0.02 265)` |
| `--card` | `oklch(0.95 0.01 260 / 0.8)` | `oklch(0.19 0.025 265 / 0.55)` |
| `--surface` | `oklch(0 0 0 / 0.03)` | `oklch(1 0 0 / 0.03)` |
| `--surface-hover` | `oklch(0 0 0 / 0.06)` | `oklch(1 0 0 / 0.06)` |
| `--surface-active` | `oklch(0 0 0 / 0.08)` | `oklch(1 0 0 / 0.08)` |
| `--surface-border` | `oklch(0 0 0 / 0.06)` | `oklch(1 0 0 / 0.06)` |
| `--border` | `oklch(0 0 0 / 0.08)` | `oklch(1 0 0 / 0.08)` |

---

## Theme

- **Strategy**: Tailwind `class` strategy (`darkMode: "class"`)
- **Detection**: `prefers-color-scheme` media query via `applyTheme()` in `src/main.tsx`
- **Classes**: `.dark` or `.light` on `<html>` element
- **Rule**: Never use hardcoded colors like `bg-white/[0.04]` or `border-white/10`. Always use semantic tokens (`bg-surface`, `border-surface-border`).

---

## Typography

- **Font**: System font stack (no custom font loaded)
- **Headings**: `font-semibold`, `tracking-tight`
- **Labels**: `text-[10px] uppercase tracking-widest`
- **Body**: `text-sm` (14px)
- **Mono**: `font-mono` for codes, keys, paths

---

## Layout

- **Window**: 1280x800 default, 1024x680 minimum
- **Sidebar**: 288px (w-72), fixed left
- **TopBar**: 64px (h-16), fixed top
- **Content**: Flex-1, scrollable
- **Border radius**: `--radius-lg` (0.75rem), `--radius-xl` (1rem), `--radius-2xl` (1.25rem)

---

## Components

### Glass Effect

```css
.glass {
  background: var(--card);
  backdrop-filter: blur(16px);
  border: 1px solid var(--border);
}

.glass-subtle {
  background: var(--surface);
  backdrop-filter: blur(12px);
  border: 1px solid var(--surface-border);
}
```

### Card Pattern

All cards/panels use `glass rounded-2xl` with `border-[color:var(--border)]` bottom borders for sections.

### Button Patterns

- **Primary action**: Gradient background (`var(--gradient-brand)`), white text, `glow-cyan` shadow
- **Secondary**: `bg-surface border border-surface-border hover:bg-surface-active`
- **Destructive**: `text-[color:var(--destructive)]` on hover
- **Icon button**: `p-1.5 rounded-lg hover:bg-surface-hover`

### Toggle

- **On**: Gradient brand background with `glow-cyan` shadow
- **Off**: `bg-surface-active`

### Status Pill

- **Locked**: Cyan background with `text-[color:var(--primary)]`
- **Unlocked**: `bg-surface border border-surface-border`

---

## File Map

| Purpose | File |
|---------|------|
| CSS variables (both themes) | `src/index.css` |
| Theme detection | `src/main.tsx` |
| Tailwind config | `tailwind.config.js` |
| Dashboard logo | `src/components/layout/Sidebar.tsx:29-31` |
| Loading screen logo | `src/App.tsx:107-109` |
| Login screen logo | `src/components/auth/LoginScreen.tsx:357-360` |
| Setup wizard logo | `src/components/auth/SetupWizard.tsx:54-57` |
| Widget logo | `src/components/widget/UnlockWidget.tsx:64-65` |
| Windows icon (ICO) | `src-tauri/icons/icon.ico` |
| Windows icon (PNG source) | `src-tauri/icons/icon.png` |

---

## Rules

1. **Never use hardcoded dark-mode colors** — always use CSS custom properties
2. **All surface backgrounds** must use `bg-surface`, `bg-surface-hover`, or `bg-surface-active`
3. **All borders** must use `border-surface-border` or `border-[color:var(--border)]`
4. **Taskbar icon must match dashboard logo** — same Shield shape, same gradient colors
5. **Test in both light and dark mode** before committing UI changes
6. **Icons from lucide-react only** — no other icon libraries
