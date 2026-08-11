// Shared primitives. Every visual value comes from the Deck design tokens —
// a client sees this portal and the firm's invoices side by side, and they
// should look like the same firm.

import type { CSSProperties, ReactNode } from 'react';
import { colors, fonts, fontSizes, radius, shadows, spacing } from '@/design-system/tokens';

export function Card({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return (
    <div
      style={{
        background: colors.bgSecondary,
        border: `0.5px solid ${colors.border}`,
        borderRadius: radius.card,
        boxShadow: shadows.card,
        padding: spacing[5],
        ...style,
      }}
    >
      {children}
    </div>
  );
}

export function PageTitle({ children, subtitle }: { children: ReactNode; subtitle?: string }) {
  return (
    <header style={{ marginBottom: spacing[6] }}>
      <h1
        style={{
          margin: 0,
          fontFamily: fonts.display,
          fontSize: fontSizes.displayLg,
          fontWeight: 600,
          color: colors.textPrimary,
        }}
      >
        {children}
      </h1>
      {subtitle && (
        <p
          style={{
            margin: `${spacing[2]} 0 0`,
            fontFamily: fonts.ui,
            fontSize: fontSizes.body,
            color: colors.textSecondary,
          }}
        >
          {subtitle}
        </p>
      )}
    </header>
  );
}

export function SectionLabel({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        fontFamily: fonts.ui,
        fontSize: fontSizes.label,
        fontWeight: 600,
        letterSpacing: '0.06em',
        textTransform: 'uppercase',
        color: colors.textSecondary,
        marginBottom: spacing[3],
      }}
    >
      {children}
    </div>
  );
}

const statusTone: Record<string, string> = {
  Active: colors.statusClear,
  Registered: colors.statusClear,
  Completed: colors.statusClear,
  Paid: colors.statusClear,
  OnHold: colors.statusWarning,
  Pending: colors.statusWarning,
  PartiallyPaid: colors.statusWarning,
  Sent: colors.accentPrimary,
  Objected: colors.statusUrgent,
  Overdue: colors.statusUrgent,
  Cancelled: colors.textTertiary,
  Closed: colors.textTertiary,
};

export function StatusBadge({ status }: { status: string }) {
  const tone = statusTone[status] ?? colors.textSecondary;
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '2px 8px',
        borderRadius: radius.chip,
        border: `0.5px solid ${tone}`,
        color: tone,
        background: 'transparent',
        fontFamily: fonts.ui,
        fontSize: fontSizes.label,
        fontWeight: 500,
        whiteSpace: 'nowrap',
      }}
    >
      {humanise(status)}
    </span>
  );
}

/** "PartiallyPaid" → "Partially Paid". Acronyms stay whole: "TMApplication"
 *  becomes "TM Application", not "T M Application". */
export function humanise(value: string): string {
  return value
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')
    .replace(/([a-z\d])([A-Z])/g, '$1 $2')
    .trim();
}

export function Button({
  children,
  onClick,
  variant = 'primary',
  disabled,
  type = 'button',
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: 'primary' | 'secondary' | 'quiet';
  disabled?: boolean;
  type?: 'button' | 'submit';
}) {
  const base: CSSProperties = {
    padding: `${spacing[2]} ${spacing[4]}`,
    borderRadius: radius.button,
    fontFamily: fonts.ui,
    fontSize: fontSizes.body,
    fontWeight: 500,
    cursor: disabled ? 'not-allowed' : 'pointer',
    opacity: disabled ? 0.55 : 1,
    transition: 'opacity 120ms ease',
  };

  const variants: Record<string, CSSProperties> = {
    primary: { ...base, background: colors.accentPrimary, color: '#fff', border: 'none' },
    secondary: {
      ...base,
      background: 'transparent',
      color: colors.accentPrimary,
      border: `0.5px solid ${colors.accentPrimary}`,
    },
    quiet: {
      ...base,
      background: 'transparent',
      color: colors.textSecondary,
      border: `0.5px solid ${colors.border}`,
    },
  };

  return (
    <button type={type} onClick={onClick} disabled={disabled} style={variants[variant]}>
      {children}
    </button>
  );
}

export const inputStyle: CSSProperties = {
  width: '100%',
  boxSizing: 'border-box',
  padding: `${spacing[2]} ${spacing[3]}`,
  borderRadius: radius.input,
  border: `0.5px solid ${colors.border}`,
  background: colors.bgPrimary,
  color: colors.textPrimary,
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  outline: 'none',
};

export function Notice({ message, tone = 'error' }: { message: string; tone?: 'error' | 'ok' }) {
  const colour = tone === 'error' ? colors.statusUrgent : colors.statusClear;
  return (
    <div
      role={tone === 'error' ? 'alert' : 'status'}
      style={{
        padding: spacing[3],
        borderRadius: radius.input,
        background: tone === 'error' ? 'rgba(192,57,43,0.08)' : 'rgba(74,124,89,0.09)',
        color: colour,
        fontFamily: fonts.ui,
        fontSize: fontSizes.body,
      }}
    >
      {message}
    </div>
  );
}

export function Empty({ children }: { children: ReactNode }) {
  return (
    <p
      style={{
        fontFamily: fonts.ui,
        fontSize: fontSizes.body,
        color: colors.textTertiary,
        padding: `${spacing[6]} 0`,
      }}
    >
      {children}
    </p>
  );
}

export function Loading() {
  return <Empty>Loading…</Empty>;
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/** Money arrives as a decimal string and stays one. Parsing it to a float to
 *  format it is how a bill drifts by a paisa, so this groups the digits
 *  textually — Indian style, three then twos, matching the invoice PDF. */
export function inr(amount: string): string {
  const negative = amount.startsWith('-');
  const [whole = '0', fraction = '00'] = amount.replace('-', '').split('.');

  let grouped = whole;
  if (whole.length > 3) {
    const lead = whole.slice(0, -3);
    const lastThree = whole.slice(-3);
    const pairs: string[] = [];
    for (let i = lead.length; i > 0; i -= 2) {
      pairs.unshift(lead.slice(Math.max(i - 2, 0), i));
    }
    grouped = `${pairs.join(',')},${lastThree}`;
  }

  return `${negative ? '-' : ''}₹${grouped}.${fraction.padEnd(2, '0').slice(0, 2)}`;
}

/** ISO date → "10 Aug 2026". Dates from the API are calendar dates with no
 *  timezone, so they are split rather than passed through `new Date`, which
 *  would shift them a day for anyone west of UTC. */
export function formatDate(iso: string | null): string {
  if (!iso) return '—';
  const [year, month, day] = iso.slice(0, 10).split('-').map(Number);
  if (!year || !month || !day) return iso;
  const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
                  'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  return `${day} ${months[month - 1]} ${year}`;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** Days until a date, for deadline emphasis. Negative means overdue. */
export function daysUntil(iso: string): number {
  const [y, m, d] = iso.slice(0, 10).split('-').map(Number);
  const target = Date.UTC(y, m - 1, d);
  const now = new Date();
  const today = Date.UTC(now.getFullYear(), now.getMonth(), now.getDate());
  return Math.round((target - today) / 86_400_000);
}

export function deadlineTone(iso: string): string {
  const days = daysUntil(iso);
  if (days < 0) return colors.statusUrgent;
  if (days <= 7) return colors.statusWarning;
  return colors.textPrimary;
}
