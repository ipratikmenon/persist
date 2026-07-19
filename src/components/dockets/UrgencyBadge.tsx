import type { UrgencyTier } from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius } from '@/design-system/tokens';

const URGENCY_STYLE: Record<UrgencyTier, { bg: string; color: string; label: string }> = {
  Overdue:  { bg: 'rgba(192,57,43,0.11)',   color: colors.statusUrgent,     label: 'Overdue'   },
  Critical: { bg: 'rgba(181,96,74,0.11)',   color: colors.accentSecondary,  label: 'Critical'  },
  Warning:  { bg: 'rgba(212,135,42,0.11)',  color: colors.statusWarning,    label: 'Warning'   },
  Normal:   { bg: 'rgba(74,124,89,0.09)',   color: colors.statusClear,      label: 'On track'  },
};

interface UrgencyBadgeProps {
  urgency: UrgencyTier;
  /** Show as a compact dot instead of a labelled pill */
  dot?: boolean;
}

export function UrgencyBadge({ urgency, dot }: UrgencyBadgeProps) {
  const s = URGENCY_STYLE[urgency];

  if (dot) {
    return (
      <span
        title={s.label}
        style={{
          display: 'inline-block',
          width: 8,
          height: 8,
          borderRadius: '50%',
          background: s.color,
          flexShrink: 0,
        }}
      />
    );
  }

  return (
    <span style={{
      display: 'inline-flex',
      alignItems: 'center',
      padding: '2px 8px',
      borderRadius: radius.chip,
      background: s.bg,
      color: s.color,
      fontSize: fontSizes.label,
      fontFamily: fonts.ui,
      fontWeight: 500,
      whiteSpace: 'nowrap',
    }}>
      {s.label}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Due date display — formats with urgency-aware colour
// ---------------------------------------------------------------------------

interface DueDateProps {
  dueDate: string;
  urgency: UrgencyTier;
}

export function DueDate({ dueDate, urgency }: DueDateProps) {
  const date = new Date(dueDate);
  const today = new Date();
  const diffDays = Math.floor((date.getTime() - today.setHours(0,0,0,0)) / 86_400_000);

  let suffix = '';
  if (diffDays < 0)      suffix = ` (${Math.abs(diffDays)}d overdue)`;
  else if (diffDays === 0) suffix = ' (today)';
  else if (diffDays === 1) suffix = ' (tomorrow)';
  else if (diffDays <= 7)  suffix = ` (${diffDays}d)`;

  const color = urgency === 'Normal'
    ? colors.textSecondary
    : URGENCY_STYLE[urgency].color;

  const formatted = date.toLocaleDateString('en-IN', {
    day: 'numeric', month: 'short', year: 'numeric',
  });

  return (
    <span style={{
      fontSize: fontSizes.body,
      fontFamily: fonts.ui,
      color,
      fontWeight: urgency === 'Overdue' || urgency === 'Critical' ? 500 : 400,
    }}>
      {formatted}
      {suffix && (
        <span style={{ fontSize: fontSizes.label, marginLeft: 4 }}>
          {suffix}
        </span>
      )}
    </span>
  );
}
