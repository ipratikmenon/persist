// Design system tokens — the single source of truth for all visual values.
// Do NOT hardcode any colour, font, or spacing in a component.
// Import from here instead.

export const colors = {
  bgPrimary:        '#F9F7F4',  // Warm white — main background
  bgSecondary:      '#F0ECE5',  // Soft sand — cards, sidebars
  bgTertiary:       '#E8EDE6',  // Light sage — section differentiation
  textPrimary:      '#2C2C2A',  // Deep charcoal — never pure black
  textSecondary:    '#6B6862',  // Warm grey — labels, metadata
  textTertiary:     '#9A9590',  // Muted stone — placeholders
  accentPrimary:    '#4A6580',  // Warm slate blue — buttons, active states
  accentSecondary:  '#B5604A',  // Terracotta — hover, selected, unread accents
  statusUrgent:     '#C0392B',  // Muted red — overdue only
  statusWarning:    '#D4872A',  // Warm amber — due within 3–7 days
  statusClear:      '#4A7C59',  // Sage green — completed, safe
  border:           '#E2DDD8',  // Hairline border
} as const;

// ---------------------------------------------------------------------------
// Fonts — SF Pro system stack (no network requests)
// -apple-system   → SF Pro Text/Display on macOS (size-adaptive)
// BlinkMacSystemFont → Chrome on macOS (equivalent to -apple-system)
// system-ui       → cross-platform fallback
// ---------------------------------------------------------------------------

export const fonts = {
  /** SF Pro Display — large headings, firm name, page titles */
  display: "-apple-system, 'SF Pro Display', BlinkMacSystemFont, system-ui, sans-serif",
  /** SF Pro Text — all UI chrome, labels, body copy, inputs */
  ui:      "-apple-system, 'SF Pro Text',    BlinkMacSystemFont, system-ui, sans-serif",
  /** Legal drafting default — Georgia. User can override via draftingFonts. */
  legal:   "Georgia, 'Times New Roman', serif",
  /** SF Mono — matter IDs, vault paths, code */
  mono:    "'SF Mono', ui-monospace, Menlo, Monaco, monospace",
} as const;

// ---------------------------------------------------------------------------
// Drafting fonts — available choices in document drafting / template areas.
// The attorney selects one; it overrides `fonts.legal` locally.
// All entries are system fonts (no download required on macOS).
// ---------------------------------------------------------------------------

export interface DraftingFont {
  /** Internal value stored in DB / user preference */
  value: string;
  /** Display name shown in the font picker */
  label: string;
  /** Full CSS font-family stack */
  stack: string;
  /** Short sample text shown in the picker at that font */
  sample: string;
}

export const draftingFonts: DraftingFont[] = [
  {
    value: 'georgia',
    label: 'Georgia',
    stack: "Georgia, 'Times New Roman', serif",
    sample: 'The quick brown fox — traditional legal serif',
  },
  {
    value: 'times',
    label: 'Times New Roman',
    stack: "'Times New Roman', Times, serif",
    sample: 'The quick brown fox — classic court font',
  },
  {
    value: 'palatino',
    label: 'Palatino',
    stack: "Palatino, 'Palatino Linotype', 'Book Antiqua', serif",
    sample: 'The quick brown fox — elegant humanist serif',
  },
  {
    value: 'garamond',
    label: 'Garamond',
    stack: "Garamond, 'Adobe Garamond Pro', 'EB Garamond', serif",
    sample: 'The quick brown fox — classical Renaissance serif',
  },
  {
    value: 'baskerville',
    label: 'Baskerville',
    stack: "Baskerville, 'Baskerville Old Face', 'Libre Baskerville', serif",
    sample: 'The quick brown fox — refined transitional serif',
  },
  {
    value: 'arial',
    label: 'Arial',
    stack: "Arial, 'Helvetica Neue', Helvetica, sans-serif",
    sample: 'The quick brown fox — clean modern sans-serif',
  },
  {
    value: 'calibri',
    label: 'Calibri',
    stack: "Calibri, Carlito, 'Gill Sans', sans-serif",
    sample: 'The quick brown fox — humanist sans, common in briefs',
  },
  {
    value: 'courier',
    label: 'Courier New',
    stack: "'Courier New', Courier, monospace",
    sample: 'The quick brown fox — typewriter, court pleadings',
  },
] as const;

/** Returns the CSS stack for a drafting font by its value key. Falls back to Georgia. */
export function draftingFontStack(value: string): string {
  return draftingFonts.find(f => f.value === value)?.stack ?? fonts.legal;
}

export const fontSizes = {
  displayLg:  '28px',  // SF Pro Display 600 — page headings
  displaySm:  '20px',  // SF Pro Display 500 — section headings
  cardTitle:  '15px',  // SF Pro Text 500
  body:       '14px',  // SF Pro Text 400
  label:      '12px',  // SF Pro Text 400
  legal:      '13px',  // drafting area — font is user-selectable
  mono:       '12px',  // SF Mono 400
} as const;

export const radius = {
  card:   '10px',
  button: '8px',
  input:  '6px',
  chip:   '4px',
} as const;

export const shadows = {
  card: '0 1px 4px rgba(0, 0, 0, 0.06)',
  dropdown: '0 4px 16px rgba(0, 0, 0, 0.10)',
  modal: '0 8px 32px rgba(0, 0, 0, 0.14)',
} as const;

export const spacing = {
  1:  '4px',
  2:  '8px',
  3:  '12px',
  4:  '16px',
  5:  '20px',
  6:  '24px',
  8:  '32px',
  10: '40px',
  12: '48px',
} as const;

// Priority indicator colours (left border on matter cards)
export const priorityColors = {
  Urgent: colors.accentSecondary,  // Terracotta
  High:   colors.statusWarning,    // Amber
  Normal: 'transparent',
} as const;

// Status badge colours
export const statusColors = {
  Active:                 colors.statusClear,
  OnHold:                 colors.statusWarning,
  PendingClientResponse:  colors.accentPrimary,
  Closed:                 colors.textTertiary,
  Archived:               colors.textTertiary,
} as const;

// Urgency colours for deadline rows
export const urgencyColors = {
  Overdue:  colors.statusUrgent,
  Critical: colors.accentSecondary,
  Warning:  colors.statusWarning,
  Normal:   colors.textPrimary,
} as const;
