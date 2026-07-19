/**
 * InvoiceList — all invoices with status filter tabs.
 *
 * Columns: Invoice ID, Client, Date, Total, Status badge, Balance, PDF icon.
 * Status tabs: All / Draft / Sent / Paid / Overdue.
 *
 * Clicking a row drills into InvoiceDetail.
 * "New Invoice" button opens InvoiceComposer drawer.
 */
import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { useReducedMotion } from 'motion/react';
import { keel } from '@/lib/tauri';
import type { InvoiceSummary } from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';
import { transition } from '@/design-system/motion';
import InvoiceDetail from './InvoiceDetail';
import InvoiceComposer from './InvoiceComposer';

function formatCurrency(n: number): string {
  return new Intl.NumberFormat('en-IN', { style: 'currency', currency: 'INR', minimumFractionDigits: 0 }).format(n);
}

const STATUS_TABS: { key: string; label: string }[] = [
  { key: 'all',      label: 'All' },
  { key: 'Draft',    label: 'Draft' },
  { key: 'Sent',     label: 'Sent' },
  { key: 'Paid',     label: 'Paid' },
  { key: 'overdue',  label: 'Overdue' },
];

const STATUS_COLORS: Record<string, { bg: string; fg: string }> = {
  Draft:         { bg: 'rgba(154,149,144,0.10)', fg: colors.textSecondary },
  Sent:          { bg: 'rgba(74,101,128,0.10)',  fg: colors.accentPrimary },
  Paid:          { bg: 'rgba(74,124,89,0.10)',   fg: colors.statusClear },
  PartiallyPaid: { bg: 'rgba(212,135,42,0.10)',  fg: colors.statusWarning },
  Cancelled:     { bg: 'rgba(192,57,43,0.08)',   fg: colors.statusUrgent },
};

export default function InvoiceList() {
  const [invoices, setInvoices]           = useState<InvoiceSummary[]>([]);
  const [filter, setFilter]               = useState('all');
  const [selectedId, setSelectedId]       = useState<string | null>(null);
  const [composerOpen, setComposerOpen]   = useState(false);
  const shouldReduce = useReducedMotion();

  const loadInvoices = useCallback(async () => {
    try {
      const all = await keel.billing.listInvoices();
      setInvoices(all);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => { loadInvoices(); }, [loadInvoices]);

  const filtered = invoices.filter(inv => {
    if (filter === 'all')     return true;
    if (filter === 'overdue') return inv.isOverdue;
    return inv.status === filter;
  });

  // ── Drill into detail view ─────────────────────────────────────────────────
  if (selectedId) {
    return (
      <InvoiceDetail
        invoiceId={selectedId}
        onBack={() => setSelectedId(null)}
        onInvoiceUpdated={loadInvoices}
      />
    );
  }

  return (
    <div>
      {/* Toolbar: status tabs + New Invoice button */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: spacing[5] }}>
        <div style={{ display: 'flex', gap: spacing[2] }}>
          {STATUS_TABS.map(tab => {
            const isActive = filter === tab.key;
            const count = tab.key === 'all'
              ? invoices.length
              : tab.key === 'overdue'
              ? invoices.filter(i => i.isOverdue).length
              : invoices.filter(i => i.status === tab.key).length;
            return (
              <motion.button
                key={tab.key}
                onClick={() => setFilter(tab.key)}
                whileHover={!shouldReduce ? { y: -1 } : undefined}
                transition={transition.fast}
                style={{
                  padding: `6px 14px`,
                  borderRadius: radius.button,
                  border: `0.5px solid ${isActive ? colors.accentPrimary : colors.border}`,
                  background: isActive ? 'rgba(74,101,128,0.08)' : 'transparent',
                  color: isActive ? colors.accentPrimary : colors.textSecondary,
                  fontFamily: fonts.ui,
                  fontSize: fontSizes.label,
                  fontWeight: isActive ? 500 : 400,
                  cursor: 'pointer',
                  display: 'flex',
                  gap: 6,
                  alignItems: 'center',
                }}
              >
                {tab.label}
                <span style={{
                  fontFamily: fonts.mono,
                  fontSize: 10,
                  color: isActive ? colors.accentPrimary : colors.textTertiary,
                }}>
                  {count}
                </span>
              </motion.button>
            );
          })}
        </div>

        <motion.button
          onClick={() => setComposerOpen(true)}
          whileHover={!shouldReduce ? { y: -1 } : undefined}
          transition={transition.fast}
          style={{
            padding: `8px 18px`,
            borderRadius: radius.button,
            border: 'none',
            background: colors.accentPrimary,
            color: '#fff',
            fontFamily: fonts.ui,
            fontSize: fontSizes.body,
            fontWeight: 500,
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: 6,
          }}
        >
          <span style={{ fontSize: 16, lineHeight: 1 }}>+</span>
          New Invoice
        </motion.button>
      </div>

      {/* Invoice table */}
      {filtered.length === 0 ? (
        <div style={{
          textAlign: 'center',
          padding: spacing[8],
          color: colors.textTertiary,
          fontFamily: fonts.ui,
          fontSize: fontSizes.body,
        }}>
          {invoices.length === 0
            ? 'No invoices yet. Create one from the Time Entries tab after logging billable time.'
            : 'No invoices match this filter.'}
        </div>
      ) : (
        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
          <thead>
            <tr>
              {['Invoice', 'Client', 'Date', 'Total', 'Paid', 'Balance', 'Status'].map(h => (
                <th key={h} style={thStyle}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {filtered.map(inv => {
              const sc = STATUS_COLORS[inv.status] ?? STATUS_COLORS.Draft;
              return (
                <motion.tr
                  key={inv.id}
                  onClick={() => setSelectedId(inv.id)}
                  whileHover={!shouldReduce ? { backgroundColor: 'rgba(74,101,128,0.03)' } : undefined}
                  style={{ cursor: 'pointer' }}
                >
                  <td style={tdStyle}>
                    <span style={{ fontFamily: fonts.mono, fontSize: fontSizes.label, color: colors.accentPrimary }}>
                      {inv.id}
                    </span>
                  </td>
                  <td style={tdStyle}>{inv.clientName}</td>
                  <td style={tdStyle}>
                    <span style={{ fontFamily: fonts.mono, fontSize: fontSizes.label }}>
                      {inv.invoiceDate}
                    </span>
                  </td>
                  <td style={{ ...tdStyle, textAlign: 'right', fontFamily: fonts.mono }}>
                    {formatCurrency(inv.totalWithTax)}
                  </td>
                  <td style={{ ...tdStyle, textAlign: 'right', fontFamily: fonts.mono, color: colors.statusClear }}>
                    {formatCurrency(inv.amountPaid)}
                  </td>
                  <td style={{ ...tdStyle, textAlign: 'right', fontFamily: fonts.mono, fontWeight: 500 }}>
                    {inv.balanceDue > 0 ? formatCurrency(inv.balanceDue) : '—'}
                  </td>
                  <td style={tdStyle}>
                    <span style={{
                      display: 'inline-block',
                      padding: '3px 10px',
                      borderRadius: radius.button,
                      background: inv.isOverdue ? 'rgba(192,57,43,0.08)' : sc.bg,
                      color:      inv.isOverdue ? colors.statusUrgent : sc.fg,
                      fontFamily: fonts.ui,
                      fontSize: fontSizes.label,
                      fontWeight: 500,
                    }}>
                      {inv.isOverdue ? 'Overdue' : inv.status}
                    </span>
                  </td>
                </motion.tr>
              );
            })}
          </tbody>
        </table>
      )}

      {/* InvoiceComposer drawer */}
      <AnimatePresence>
        {composerOpen && (
          <InvoiceComposer
            onClose={() => setComposerOpen(false)}
            onCreated={(newId) => {
              loadInvoices();
              setComposerOpen(false);
              setSelectedId(newId);
            }}
          />
        )}
      </AnimatePresence>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shared styles
// ---------------------------------------------------------------------------

const thStyle: React.CSSProperties = {
  textAlign: 'left',
  padding: `${spacing[2]} ${spacing[3]}`,
  fontFamily: fonts.ui,
  fontSize: fontSizes.label,
  fontWeight: 500,
  color: colors.textTertiary,
  textTransform: 'uppercase' as const,
  letterSpacing: '0.06em',
  borderBottom: `0.5px solid ${colors.border}`,
};

const tdStyle: React.CSSProperties = {
  padding: `${spacing[3]} ${spacing[3]}`,
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  color: colors.textPrimary,
  borderBottom: `0.5px solid ${colors.border}`,
  verticalAlign: 'middle',
};
