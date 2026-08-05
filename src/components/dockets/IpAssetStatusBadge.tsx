import type { IpAssetStatus, IpAssetType } from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius } from '@/design-system/tokens';

// ---------------------------------------------------------------------------
// IP asset status badge — semantic colour pill
//
// Colour follows the prosecution arc: neutral while the application is in the
// registry's hands, sage once the right is secured, amber for contested,
// muted red for rights that are gone.
// ---------------------------------------------------------------------------

const STATUS_STYLE: Record<IpAssetStatus, { bg: string; color: string; label: string }> = {
  Pending:     { bg: 'rgba(154,149,144,0.13)', color: colors.textSecondary,   label: 'Pending'     },
  Examination: { bg: 'rgba(74,101,128,0.11)',  color: colors.accentPrimary,   label: 'Examination' },
  Accepted:    { bg: 'rgba(74,101,128,0.11)',  color: colors.accentPrimary,   label: 'Accepted'    },
  Advertised:  { bg: 'rgba(74,101,128,0.11)',  color: colors.accentPrimary,   label: 'Advertised'  },
  Opposed:     { bg: 'rgba(212,135,42,0.11)',  color: colors.statusWarning,   label: 'Opposed'     },
  Registered:  { bg: 'rgba(74,124,89,0.11)',   color: colors.statusClear,     label: 'Registered'  },
  Granted:     { bg: 'rgba(74,124,89,0.11)',   color: colors.statusClear,     label: 'Granted'     },
  Lapsed:      { bg: 'rgba(192,57,43,0.10)',   color: colors.statusUrgent,    label: 'Lapsed'      },
  Abandoned:   { bg: 'rgba(154,149,144,0.13)', color: colors.textTertiary,    label: 'Abandoned'   },
  Cancelled:   { bg: 'rgba(192,57,43,0.10)',   color: colors.statusUrgent,    label: 'Cancelled'   },
};

interface IpAssetStatusBadgeProps {
  status: IpAssetStatus;
}

export function IpAssetStatusBadge({ status }: IpAssetStatusBadgeProps) {
  const style = STATUS_STYLE[status] ?? STATUS_STYLE.Pending;
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
// Asset type pill
// ---------------------------------------------------------------------------

const TYPE_ABBREV: Record<IpAssetType, string> = {
  Trademark:    'TM',
  Patent:       'Patent',
  Design:       'Design',
  Copyright:    'Copyright',
  PlantVariety: 'Plant Variety',
};

interface IpAssetTypePillProps {
  assetType: IpAssetType;
}

export function IpAssetTypePill({ assetType }: IpAssetTypePillProps) {
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
      {TYPE_ABBREV[assetType] ?? assetType}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Class chips — Nice (trademark) / Locarno (design) numbers
// ---------------------------------------------------------------------------

interface ClassChipsProps {
  classes: number[];
}

export function ClassChips({ classes }: ClassChipsProps) {
  if (classes.length === 0) return null;

  return (
    <span style={{ display: 'inline-flex', gap: 4, flexWrap: 'wrap' }}>
      {classes.map(c => (
        <span
          key={c}
          title={`Class ${c}`}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            padding: '1px 6px',
            borderRadius: radius.chip,
            background: colors.bgTertiary,
            color: colors.textSecondary,
            fontSize: 11,
            fontFamily: fonts.mono,
            whiteSpace: 'nowrap',
          }}
        >
          {c}
        </span>
      ))}
    </span>
  );
}
