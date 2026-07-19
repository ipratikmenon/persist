import type { MatterStatus, MatterPriority } from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius } from '@/design-system/tokens';

// ---------------------------------------------------------------------------
// Status badge — semantic colour pill
// ---------------------------------------------------------------------------

const STATUS_STYLE: Record<MatterStatus, { bg: string; color: string; label: string }> = {
  Active:                { bg: 'rgba(74,124,89,0.11)',   color: colors.statusClear,     label: 'Active'           },
  OnHold:                { bg: 'rgba(212,135,42,0.11)',  color: colors.statusWarning,   label: 'On Hold'          },
  PendingClientResponse: { bg: 'rgba(74,101,128,0.11)',  color: colors.accentPrimary,   label: 'Pending Client'   },
  Closed:                { bg: 'rgba(154,149,144,0.13)', color: colors.textTertiary,    label: 'Closed'           },
  Archived:              { bg: 'rgba(154,149,144,0.08)', color: colors.textTertiary,    label: 'Archived'         },
};

interface MatterStatusBadgeProps {
  status: MatterStatus;
}

export function MatterStatusBadge({ status }: MatterStatusBadgeProps) {
  const style = STATUS_STYLE[status] ?? STATUS_STYLE.Active;
  return (
    <span style={{
      display: 'inline-flex',
      alignItems: 'center',
      padding: '2px 8px',
      borderRadius: radius.chip,
      background: style.bg,
      color: style.color,
      fontSize: fontSizes.label,
      fontFamily: fonts.ui,
      fontWeight: 500,
      whiteSpace: 'nowrap',
      letterSpacing: '0.01em',
    }}>
      {style.label}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Priority indicator — left border colour on matter rows/cards
// ---------------------------------------------------------------------------

const PRIORITY_BORDER: Record<MatterPriority, string> = {
  Urgent: colors.accentSecondary,
  High:   colors.statusWarning,
  Normal: 'transparent',
};

interface PriorityBarProps {
  priority: MatterPriority;
}

export function PriorityBar({ priority }: PriorityBarProps) {
  return (
    <div style={{
      width: 3,
      alignSelf: 'stretch',
      flexShrink: 0,
      background: PRIORITY_BORDER[priority],
      borderRadius: '2px 0 0 2px',
    }} />
  );
}

// ---------------------------------------------------------------------------
// Matter type label — compact text pill
// ---------------------------------------------------------------------------

const TYPE_ABBREV: Record<string, string> = {
  Trademark:  'TM',
  Patent:     'Patent',
  Design:     'Design',
  Copyright:  'Copyright',
  Corporate:  'Corporate',
  Litigation: 'Litigation',
  Paralegal:  'Paralegal',
};

interface MatterTypePillProps {
  matterType: string;
}

export function MatterTypePill({ matterType }: MatterTypePillProps) {
  return (
    <span style={{
      display: 'inline-flex',
      alignItems: 'center',
      padding: '2px 7px',
      borderRadius: radius.chip,
      background: 'rgba(74,101,128,0.07)',
      color: colors.accentPrimary,
      fontSize: 11,
      fontFamily: fonts.ui,
      fontWeight: 500,
      whiteSpace: 'nowrap',
    }}>
      {TYPE_ABBREV[matterType] ?? matterType}
    </span>
  );
}
