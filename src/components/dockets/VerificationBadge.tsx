import type { Deadline } from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius } from '@/design-system/tokens';

/**
 * Dual-verification indicator (spec §2.12).
 *
 * Only meaningful on Statutory deadlines — Procedural and Custom dates are the
 * firm's own working steps and are not subject to the control, so showing an
 * "unverified" flag on them would be noise that teaches attorneys to ignore it.
 */
interface VerificationBadgeProps {
  deadline: Deadline;
}

export function VerificationBadge({ deadline }: VerificationBadgeProps) {
  if (deadline.eventType !== 'Statutory') return null;

  const verified = deadline.isVerified;

  return (
    <span
      title={
        verified
          ? 'Checked by a second attorney'
          : 'Awaiting a second attorney — the person who entered this date cannot verify it'
      }
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 4,
        padding: '2px 8px',
        borderRadius: radius.chip,
        background: verified ? 'rgba(74,124,89,0.11)' : 'rgba(212,135,42,0.11)',
        color: verified ? colors.statusClear : colors.statusWarning,
        fontSize: fontSizes.label,
        fontFamily: fonts.ui,
        fontWeight: 500,
        whiteSpace: 'nowrap',
      }}
    >
      {verified ? '✓✓ Verified' : '⚠ Unverified'}
    </span>
  );
}

/**
 * The docket reference number, in monospace so it lines up in a list and reads
 * as an identifier rather than prose.
 */
export function ReferenceChip({ reference }: { reference: string | null }) {
  if (!reference) return null;
  return (
    <span style={{
      fontFamily: fonts.mono,
      fontSize: fontSizes.mono,
      color: colors.textTertiary,
      whiteSpace: 'nowrap',
    }}>
      {reference}
    </span>
  );
}
