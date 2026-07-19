// AI visual indicators — always use these components, never create ad-hoc ones.
// Three signals: sparkle chip, draft banner, deep analysis indicator.

import { colors, fonts, fontSizes } from './tokens';

/** Inline sparkle chip — marks AI-generated content inline */
export function SparkleChip() {
  return (
    <span
      aria-label="AI generated"
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        fontSize: '10px',
        color: colors.accentPrimary,
        opacity: 0.8,
        marginLeft: 4,
        userSelect: 'none',
      }}
    >
      ✦
    </span>
  );
}

/** Full-width banner for AI-drafted documents */
export function DraftBanner() {
  return (
    <div
      role="status"
      aria-label="AI generated draft"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '6px 12px',
        background: `${colors.accentPrimary}14`,
        borderBottom: `0.5px solid ${colors.accentPrimary}30`,
        fontFamily: fonts.ui,
        fontSize: fontSizes.label,
        fontWeight: 500,
        color: colors.accentPrimary,
        letterSpacing: '0.05em',
        textTransform: 'uppercase' as const,
      }}
    >
      <SparkleChip />
      Draft — AI Generated
    </div>
  );
}

/** Small chip for Opus-level deep analysis responses */
export function DeepAnalysisChip() {
  return (
    <span
      aria-label="Deep analysis"
      title="Generated with deep analysis"
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 3,
        padding: '2px 6px',
        borderRadius: 4,
        background: `${colors.accentSecondary}15`,
        fontFamily: fonts.ui,
        fontSize: '11px',
        fontWeight: 500,
        color: colors.accentSecondary,
        userSelect: 'none',
      }}
    >
      ⚙ Deep
    </span>
  );
}
