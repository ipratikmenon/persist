/**
 * InvoiceDetail — full invoice view with line items, GST breakdown,
 * payment history, and action bar.
 *
 * Actions: Generate PDF · Mark as Sent · Record Payment · Cancel Invoice
 */
import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { keel } from '@/lib/tauri';
import type {
  Invoice,
  InvoiceLineItem,
  Payment,
  RecordPaymentInput,
  PaymentMethod,
} from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';
import { transition } from '@/design-system/motion';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function fmt(n: number) {
  return new Intl.NumberFormat('en-IN', {
    style: 'currency',
    currency: 'INR',
    minimumFractionDigits: 2,
  }).format(n);
}

const STATUS_STYLE: Record<string, { bg: string; fg: string }> = {
  Draft:         { bg: 'rgba(154,149,144,0.10)', fg: colors.textSecondary },
  Sent:          { bg: 'rgba(74,101,128,0.10)',  fg: colors.accentPrimary },
  Paid:          { bg: 'rgba(74,124,89,0.10)',   fg: colors.statusClear },
  PartiallyPaid: { bg: 'rgba(212,135,42,0.10)',  fg: colors.statusWarning },
  Cancelled:     { bg: 'rgba(192,57,43,0.08)',   fg: colors.statusUrgent },
};

const PAYMENT_METHODS: PaymentMethod[] = [
  'BankTransfer', 'NEFT', 'RTGS', 'UPI', 'Cheque', 'Cash',
];

// ---------------------------------------------------------------------------
// RecordPaymentModal
// ---------------------------------------------------------------------------

interface RecordPaymentModalProps {
  invoiceId: string;
  balanceDue: number;
  onSave: (p: Payment) => void;
  onClose: () => void;
}

function RecordPaymentModal({ invoiceId, balanceDue, onSave, onClose }: RecordPaymentModalProps) {
  const [amount, setAmount]     = useState(balanceDue);
  const [date, setDate]         = useState(new Date().toISOString().slice(0, 10));
  const [method, setMethod]     = useState<PaymentMethod>('BankTransfer');
  const [ref, setRef]           = useState('');
  const [notes, setNotes]       = useState('');
  const [saving, setSaving]     = useState(false);
  const [error, setError]       = useState<string | null>(null);

  const handleSave = async () => {
    if (amount <= 0) { setError('Amount must be greater than zero'); return; }
    setSaving(true);
    setError(null);
    try {
      const input: RecordPaymentInput = {
        invoiceId,
        amount,
        paymentDate: date,
        method,
        reference: ref.trim() || undefined,
        notes: notes.trim() || undefined,
      };
      const p = await keel.billing.recordPayment(input);
      onSave(p);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={{
      position: 'fixed', inset: 0, zIndex: 200,
      background: 'rgba(44,44,42,0.45)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
    }} onClick={onClose}>
      <motion.div
        initial={{ opacity: 0, scale: 0.97, y: 8 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.97 }}
        transition={{ duration: 0.18, ease: [0.16, 1, 0.3, 1] }}
        onClick={e => e.stopPropagation()}
        style={{
          width: 440,
          background: colors.bgPrimary,
          borderRadius: radius.card,
          border: `0.5px solid ${colors.border}`,
          boxShadow: '0 8px 32px rgba(0,0,0,0.12)',
          padding: spacing[6],
          display: 'flex',
          flexDirection: 'column',
          gap: spacing[4],
        }}
      >
        <h3 style={{ margin: 0, fontFamily: fonts.display, fontSize: 18, fontWeight: 600, color: colors.textPrimary }}>
          Record Payment
        </h3>

        <div style={{ display: 'flex', gap: spacing[3] }}>
          <div style={{ flex: 1 }}>
            <FieldLabel>Amount (₹)</FieldLabel>
            <input
              type="number"
              min={0.01}
              step={0.01}
              value={amount}
              onChange={e => setAmount(parseFloat(e.target.value) || 0)}
              style={inputSt}
            />
          </div>
          <div style={{ flex: 1 }}>
            <FieldLabel>Date</FieldLabel>
            <input type="date" value={date} onChange={e => setDate(e.target.value)} style={inputSt} />
          </div>
        </div>

        <div>
          <FieldLabel>Method</FieldLabel>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: spacing[1] }}>
            {PAYMENT_METHODS.map(m => (
              <button
                key={m}
                onClick={() => setMethod(m)}
                style={{
                  padding: `4px 12px`,
                  borderRadius: radius.button,
                  border: `0.5px solid ${method === m ? colors.accentPrimary : colors.border}`,
                  background: method === m ? 'rgba(74,101,128,0.10)' : 'transparent',
                  color: method === m ? colors.accentPrimary : colors.textSecondary,
                  fontFamily: fonts.ui,
                  fontSize: fontSizes.label,
                  fontWeight: method === m ? 500 : 400,
                  cursor: 'pointer',
                }}
              >
                {m}
              </button>
            ))}
          </div>
        </div>

        <div>
          <FieldLabel>Reference / UTR / Cheque No.</FieldLabel>
          <input
            type="text"
            value={ref}
            onChange={e => setRef(e.target.value)}
            placeholder="Optional"
            style={inputSt}
          />
        </div>

        <div>
          <FieldLabel>Notes</FieldLabel>
          <input
            type="text"
            value={notes}
            onChange={e => setNotes(e.target.value)}
            placeholder="Optional"
            style={inputSt}
          />
        </div>

        {error && (
          <div style={{
            padding: `${spacing[2]} ${spacing[3]}`,
            background: 'rgba(192,57,43,0.06)',
            border: `0.5px solid rgba(192,57,43,0.25)`,
            borderRadius: radius.card,
            fontSize: fontSizes.label,
            color: colors.statusUrgent,
            fontFamily: fonts.ui,
          }}>
            {error}
          </div>
        )}

        <div style={{ display: 'flex', gap: spacing[3], justifyContent: 'flex-end', marginTop: spacing[2] }}>
          <button
            onClick={onClose}
            style={{
              padding: `${spacing[2]} ${spacing[5]}`,
              borderRadius: radius.button,
              border: `0.5px solid ${colors.border}`,
              background: 'transparent',
              color: colors.textSecondary,
              fontFamily: fonts.ui,
              fontSize: fontSizes.body,
              cursor: 'pointer',
            }}
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={saving || amount <= 0}
            style={{
              padding: `${spacing[2]} ${spacing[5]}`,
              borderRadius: radius.button,
              border: 'none',
              background: (saving || amount <= 0) ? colors.border : colors.accentPrimary,
              color: (saving || amount <= 0) ? colors.textTertiary : '#fff',
              fontFamily: fonts.ui,
              fontSize: fontSizes.body,
              fontWeight: 500,
              cursor: (saving || amount <= 0) ? 'default' : 'pointer',
            }}
          >
            {saving ? 'Recording...' : 'Record Payment'}
          </button>
        </div>
      </motion.div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// InvoiceDetail
// ---------------------------------------------------------------------------

interface InvoiceDetailProps {
  invoiceId: string;
  onBack: () => void;
  onInvoiceUpdated?: () => void;
}

export default function InvoiceDetail({ invoiceId, onBack, onInvoiceUpdated }: InvoiceDetailProps) {
  const [invoice, setInvoice]       = useState<Invoice | null>(null);
  const [lineItems, setLineItems]   = useState<InvoiceLineItem[]>([]);
  const [payments, setPayments]     = useState<Payment[]>([]);
  const [loading, setLoading]       = useState(true);
  const [showPayModal, setShowPayModal] = useState(false);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [error, setError]           = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [inv, items, pays] = await Promise.all([
        keel.billing.getInvoice(invoiceId),
        keel.billing.listLineItems(invoiceId),
        keel.billing.listPayments(invoiceId),
      ]);
      setInvoice(inv);
      setLineItems(items);
      setPayments(pays);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [invoiceId]);

  useEffect(() => { load(); }, [load]);

  const handleStatusChange = async (newStatus: string) => {
    if (!invoice) return;
    setActionLoading(newStatus);
    setError(null);
    try {
      const updated = await keel.billing.updateInvoiceStatus(invoice.id, newStatus);
      setInvoice(updated);
      onInvoiceUpdated?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setActionLoading(null);
    }
  };

  const handleGeneratePdf = async () => {
    if (!invoice) return;
    setActionLoading('pdf');
    setError(null);
    try {
      await keel.billing.generateInvoicePdf(invoice.id);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setActionLoading(null);
    }
  };

  const handlePaymentSaved = async (p: Payment) => {
    setShowPayModal(false);
    setPayments(prev => [p, ...prev]);
    const updated = await keel.billing.getInvoice(invoiceId);
    setInvoice(updated);
    onInvoiceUpdated?.();
  };

  if (loading) {
    return (
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        height: 200, color: colors.textTertiary, fontFamily: fonts.ui,
      }}>
        Loading invoice...
      </div>
    );
  }

  if (!invoice) {
    return (
      <div style={{ color: colors.statusUrgent, fontFamily: fonts.ui, padding: spacing[4] }}>
        {error ?? 'Invoice not found.'}
      </div>
    );
  }

  const sc = STATUS_STYLE[invoice.status] ?? STATUS_STYLE.Draft;
  const canMarkSent     = invoice.status === 'Draft';
  const canCancel       = invoice.status === 'Draft' || invoice.status === 'Sent';
  const canRecordPayment = invoice.status === 'Sent' || invoice.status === 'PartiallyPaid';
  const isIntra         = invoice.gstType === 'Intra';

  return (
    <>
      <div>
        {/* ── Back + Actions bar ──────────────────────── */}
        <div style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: spacing[6],
          flexWrap: 'wrap',
          gap: spacing[3],
        }}>
          <button
            onClick={onBack}
            style={{
              background: 'none', border: 'none', cursor: 'pointer',
              fontFamily: fonts.ui, fontSize: fontSizes.body,
              color: colors.textSecondary,
              display: 'flex', alignItems: 'center', gap: 6,
              padding: 0,
            }}
          >
            ← Back to Invoices
          </button>

          <div style={{ display: 'flex', gap: spacing[2] }}>
            {/* Generate PDF */}
            <ActionBtn
              label={actionLoading === 'pdf' ? 'Generating...' : invoice.pdfDocId ? 'Regenerate PDF' : 'Generate PDF'}
              onClick={handleGeneratePdf}
              disabled={!!actionLoading}
              variant="secondary"
            />
            {/* Mark as Sent */}
            {canMarkSent && (
              <ActionBtn
                label={actionLoading === 'Sent' ? 'Sending...' : 'Mark as Sent'}
                onClick={() => handleStatusChange('Sent')}
                disabled={!!actionLoading}
                variant="primary"
              />
            )}
            {/* Record Payment */}
            {canRecordPayment && (
              <ActionBtn
                label="Record Payment"
                onClick={() => setShowPayModal(true)}
                disabled={!!actionLoading}
                variant="primary"
              />
            )}
            {/* Cancel */}
            {canCancel && (
              <ActionBtn
                label={actionLoading === 'Cancelled' ? 'Cancelling...' : 'Cancel Invoice'}
                onClick={() => handleStatusChange('Cancelled')}
                disabled={!!actionLoading}
                variant="danger"
              />
            )}
          </div>
        </div>

        {/* Error */}
        <AnimatePresence>
          {error && (
            <motion.div
              initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
              style={{
                marginBottom: spacing[4],
                padding: `${spacing[3]} ${spacing[4]}`,
                background: 'rgba(192,57,43,0.06)',
                border: `0.5px solid rgba(192,57,43,0.25)`,
                borderRadius: radius.card,
                fontSize: fontSizes.label,
                color: colors.statusUrgent,
                fontFamily: fonts.ui,
              }}
            >
              {error}
            </motion.div>
          )}
        </AnimatePresence>

        {/* ── Invoice header card ──────────────────────── */}
        <div style={{
          background: colors.bgSecondary,
          borderRadius: radius.card,
          border: `0.5px solid ${colors.border}`,
          padding: spacing[6],
          marginBottom: spacing[5],
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: spacing[5] }}>
            <div>
              <div style={{
                fontFamily: fonts.mono,
                fontSize: 11,
                color: colors.textTertiary,
                letterSpacing: '0.08em',
                textTransform: 'uppercase',
                marginBottom: spacing[1],
              }}>
                Tax Invoice
              </div>
              <div style={{
                fontFamily: fonts.display,
                fontSize: 26,
                fontWeight: 600,
                color: colors.textPrimary,
                letterSpacing: '-0.02em',
              }}>
                {invoice.id}
              </div>
            </div>
            <span style={{
              display: 'inline-block',
              padding: `6px 16px`,
              borderRadius: radius.button,
              background: sc.bg,
              color: sc.fg,
              fontFamily: fonts.ui,
              fontSize: fontSizes.label,
              fontWeight: 500,
            }}>
              {invoice.status}
            </span>
          </div>

          {/* Meta grid */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: spacing[4] }}>
            {[
              { label: 'Invoice Date',  value: invoice.invoiceDate },
              { label: 'Due Date',      value: invoice.dueDate ?? '—' },
              { label: 'GST Type',      value: isIntra ? 'Intra-State (CGST + SGST)' : 'Inter-State (IGST)' },
              { label: 'Client ID',     value: invoice.clientId },
            ].map(({ label, value }) => (
              <div key={label}>
                <div style={{ fontFamily: fonts.ui, fontSize: fontSizes.label, color: colors.textTertiary, textTransform: 'uppercase', letterSpacing: '0.06em', marginBottom: 2 }}>
                  {label}
                </div>
                <div style={{ fontFamily: fonts.mono, fontSize: fontSizes.body, color: colors.textPrimary }}>
                  {value}
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* ── Line items ───────────────────────────────── */}
        <Section title="Line Items">
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>
              <tr>
                {['Activity', 'Description', 'Hours', 'Rate (₹/hr)', 'Amount'].map(h => (
                  <th key={h} style={{ ...thSt, textAlign: h === 'Description' ? 'left' : 'right' }}>
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {lineItems.map(li => (
                <tr key={li.id}>
                  <td style={tdSt}>
                    {li.activityCode && (
                      <span style={{
                        fontFamily: fonts.mono, fontSize: 10,
                        background: colors.bgTertiary,
                        padding: '2px 8px', borderRadius: 4,
                        color: colors.textSecondary,
                      }}>
                        {li.activityCode}
                      </span>
                    )}
                  </td>
                  <td style={{ ...tdSt, textAlign: 'left' }}>{li.description}</td>
                  <td style={{ ...tdSt, textAlign: 'right', fontFamily: fonts.mono }}>
                    {li.hours != null ? li.hours.toFixed(2) : '—'}
                  </td>
                  <td style={{ ...tdSt, textAlign: 'right', fontFamily: fonts.mono }}>
                    {li.rate > 0 ? fmt(li.rate) : '—'}
                  </td>
                  <td style={{ ...tdSt, textAlign: 'right', fontFamily: fonts.mono, fontWeight: 500 }}>
                    {fmt(li.amount)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Section>

        {/* ── GST Totals ───────────────────────────────── */}
        <div style={{ display: 'flex', justifyContent: 'flex-end', marginBottom: spacing[5] }}>
          <div style={{
            width: 320,
            background: colors.bgSecondary,
            borderRadius: radius.card,
            border: `0.5px solid ${colors.border}`,
            padding: spacing[4],
            display: 'flex',
            flexDirection: 'column',
            gap: spacing[2],
          }}>
            <TotalRow label="Subtotal" value={fmt(invoice.subtotal)} />
            {isIntra ? (
              <>
                <TotalRow label="CGST (9%)" value={fmt(invoice.cgstAmount)} />
                <TotalRow label="SGST (9%)" value={fmt(invoice.sgstAmount)} />
              </>
            ) : (
              <TotalRow label="IGST (18%)" value={fmt(invoice.igstAmount)} />
            )}
            <div style={{ borderTop: `0.5px solid ${colors.border}`, paddingTop: spacing[2] }}>
              <TotalRow label="Total" value={fmt(invoice.totalWithTax)} bold />
            </div>
            <TotalRow label="Amount Paid" value={fmt(invoice.amountPaid)} color={colors.statusClear} />
            {invoice.balanceDue > 0 && (
              <TotalRow label="Balance Due" value={fmt(invoice.balanceDue)} color={colors.statusUrgent} bold />
            )}
          </div>
        </div>

        {/* ── Payment history ──────────────────────────── */}
        {payments.length > 0 && (
          <Section title="Payment History">
            <table style={{ width: '100%', borderCollapse: 'collapse' }}>
              <thead>
                <tr>
                  {['Date', 'Method', 'Reference', 'Amount'].map(h => (
                    <th key={h} style={thSt}>{h}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {payments.map(p => (
                  <tr key={p.id}>
                    <td style={{ ...tdSt, fontFamily: fonts.mono }}>{p.paymentDate}</td>
                    <td style={tdSt}>{p.method}</td>
                    <td style={{ ...tdSt, fontFamily: fonts.mono, color: colors.textSecondary }}>
                      {p.reference ?? '—'}
                    </td>
                    <td style={{ ...tdSt, textAlign: 'right', fontFamily: fonts.mono, fontWeight: 500, color: colors.statusClear }}>
                      {fmt(p.amount)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Section>
        )}

        {/* ── SAC Code footnote ─────────────────────────── */}
        <div style={{
          marginTop: spacing[4],
          padding: `${spacing[3]} ${spacing[4]}`,
          background: colors.bgSecondary,
          borderRadius: radius.card,
          border: `0.5px solid ${colors.border}`,
          fontFamily: fonts.mono,
          fontSize: 10,
          color: colors.textTertiary,
          letterSpacing: '0.04em',
        }}>
          SAC Code: 998212 — Legal advisory, litigation and related legal services
        </div>
      </div>

      {/* ── Record Payment Modal ─────────────────────── */}
      <AnimatePresence>
        {showPayModal && (
          <RecordPaymentModal
            invoiceId={invoice.id}
            balanceDue={invoice.balanceDue}
            onSave={handlePaymentSaved}
            onClose={() => setShowPayModal(false)}
          />
        )}
      </AnimatePresence>
    </>
  );
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: spacing[5] }}>
      <div style={{
        fontFamily: fonts.ui,
        fontSize: fontSizes.label,
        fontWeight: 500,
        color: colors.textTertiary,
        textTransform: 'uppercase' as const,
        letterSpacing: '0.06em',
        marginBottom: spacing[3],
      }}>
        {title}
      </div>
      <div style={{
        background: colors.bgSecondary,
        borderRadius: radius.card,
        border: `0.5px solid ${colors.border}`,
        overflow: 'hidden',
      }}>
        {children}
      </div>
    </div>
  );
}

function TotalRow({ label, value, bold, color }: { label: string; value: string; bold?: boolean; color?: string }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
      <span style={{
        fontFamily: fonts.ui,
        fontSize: fontSizes.label,
        color: color ?? colors.textSecondary,
        fontWeight: bold ? 500 : 400,
      }}>
        {label}
      </span>
      <span style={{
        fontFamily: fonts.mono,
        fontSize: bold ? fontSizes.body : fontSizes.label,
        color: color ?? colors.textPrimary,
        fontWeight: bold ? 600 : 400,
      }}>
        {value}
      </span>
    </div>
  );
}

function ActionBtn({ label, onClick, disabled, variant }: {
  label: string;
  onClick: () => void;
  disabled: boolean;
  variant: 'primary' | 'secondary' | 'danger';
}) {
  const bg = disabled
    ? colors.border
    : variant === 'primary' ? colors.accentPrimary
    : variant === 'danger'  ? 'rgba(192,57,43,0.10)'
    : 'transparent';
  const fg = disabled
    ? colors.textTertiary
    : variant === 'primary' ? '#fff'
    : variant === 'danger'  ? colors.statusUrgent
    : colors.textSecondary;
  const border = variant === 'secondary' || variant === 'danger'
    ? `0.5px solid ${colors.border}`
    : 'none';

  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        padding: `${spacing[2]} ${spacing[4]}`,
        borderRadius: radius.button,
        border,
        background: bg,
        color: fg,
        fontFamily: fonts.ui,
        fontSize: fontSizes.body,
        fontWeight: 500,
        cursor: disabled ? 'default' : 'pointer',
        transition: `all ${transition.fast.duration}s`,
      }}
    >
      {label}
    </button>
  );
}

function FieldLabel({ children }: { children: string }) {
  return (
    <label style={{
      display: 'block',
      fontSize: 11,
      fontWeight: 500,
      color: colors.textTertiary,
      fontFamily: fonts.ui,
      textTransform: 'uppercase' as const,
      letterSpacing: '0.06em',
      marginBottom: spacing[1],
    }}>
      {children}
    </label>
  );
}

const thSt: React.CSSProperties = {
  textAlign: 'right',
  padding: `${spacing[2]} ${spacing[3]}`,
  fontFamily: fonts.ui,
  fontSize: fontSizes.label,
  fontWeight: 500,
  color: colors.textTertiary,
  textTransform: 'uppercase' as const,
  letterSpacing: '0.06em',
  borderBottom: `0.5px solid ${colors.border}`,
};

const tdSt: React.CSSProperties = {
  padding: `${spacing[3]} ${spacing[3]}`,
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  color: colors.textPrimary,
  borderBottom: `0.5px solid ${colors.border}`,
  textAlign: 'right',
  verticalAlign: 'middle',
};

const inputSt: React.CSSProperties = {
  width: '100%',
  padding: `${spacing[2]} ${spacing[3]}`,
  borderRadius: radius.input,
  border: `0.5px solid ${colors.border}`,
  background: colors.bgSecondary,
  color: colors.textPrimary,
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  outline: 'none',
  boxSizing: 'border-box' as const,
};
