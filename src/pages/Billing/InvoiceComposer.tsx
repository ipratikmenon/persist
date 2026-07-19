/**
 * InvoiceComposer — slide-in drawer for creating a new invoice.
 *
 * Flow:
 *   1. Select client  →  matters for that client load
 *   2. Pick one or more matters  →  unbilled time entries load
 *   3. Check/uncheck entries to include  →  optionally add fixed-fee lines
 *   4. Set GST type (Intra / Inter), invoice date, due date, notes
 *   5. Live totals panel  →  Create Invoice button
 *
 * On success: calls onCreated(newInvoiceId) so the parent can open InvoiceDetail.
 */
import { useState, useEffect, useCallback } from 'react';
import { motion } from 'motion/react';
import { keel } from '@/lib/tauri';
import type {
  MatterSummary,
  TimeEntry,
  GstType,
  LineItemInput,
  CreateInvoiceInput,
} from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';
import { transition } from '@/design-system/motion';
import { useAuthStore } from '@/stores/auth';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function fmt(n: number) {
  return new Intl.NumberFormat('en-IN', {
    style: 'currency',
    currency: 'INR',
    minimumFractionDigits: 0,
  }).format(n);
}

interface FixedLine {
  id: string; // local UUID for list key
  description: string;
  amount: number;
}

function newFixedLine(): FixedLine {
  return { id: Math.random().toString(36).slice(2), description: '', amount: 0 };
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface Props {
  onClose: () => void;
  onCreated: (invoiceId: string) => void;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function InvoiceComposer({ onClose, onCreated }: Props) {
  // ── Loading state ─────────────────────────────────────────────────────────
  const [allMatters, setAllMatters] = useState<MatterSummary[]>([]);
  const [entries, setEntries]       = useState<TimeEntry[]>([]);

  // ── Selection state ───────────────────────────────────────────────────────
  const [clientId, setClientId]           = useState('');
  const [selectedMatters, setSelectedMatters] = useState<Set<string>>(new Set());
  const [checkedEntries, setCheckedEntries]   = useState<Set<string>>(new Set());
  const [fixedLines, setFixedLines]           = useState<FixedLine[]>([]);

  // ── Invoice meta ──────────────────────────────────────────────────────────
  const [gstType, setGstType]   = useState<GstType>('Intra');
  const [invoiceDate, setInvoiceDate] = useState(new Date().toISOString().slice(0, 10));
  const [dueDate, setDueDate]   = useState('');
  const [notes, setNotes]       = useState('');

  // ── UI state ──────────────────────────────────────────────────────────────
  const [saving, setSaving]   = useState(false);
  const [error, setError]     = useState<string | null>(null);
  const [gstRate, setGstRate] = useState(0.18);

  const { session } = useAuthStore();

  // ── Load all active matters once ──────────────────────────────────────────
  const loadMatters = useCallback(async () => {
    try {
      const list = await keel.matters.list({ status: ['Active'] });
      setAllMatters(list);
    } catch { /* ignore */ }
  }, []);

  const loadGstRate = useCallback(async () => {
    try {
      const s = await keel.billing.getFirmSettings();
      setGstRate(s.gstRate);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => { loadMatters(); loadGstRate(); }, [loadMatters, loadGstRate]);

  // ── Load unbilled entries when matter selection changes ───────────────────
  useEffect(() => {
    if (selectedMatters.size === 0) {
      setEntries([]);
      setCheckedEntries(new Set());
      return;
    }

    const fetchAll = async () => {
      try {
        const results = await Promise.all(
          Array.from(selectedMatters).map(mid =>
            keel.billing.listTimeEntries(mid, undefined, undefined, undefined)
          )
        );
        const unbilled = results.flat().filter(e => e.isBillable && !e.isInvoiced);
        setEntries(unbilled);
        // Auto-check all unbilled entries
        setCheckedEntries(new Set(unbilled.map(e => e.id)));
      } catch { /* ignore */ }
    };

    fetchAll();
  }, [selectedMatters]);

  // ── Derived client list ───────────────────────────────────────────────────
  const clientOptions = Array.from(
    new Map(allMatters.map(m => [m.clientName, m.clientName])).entries()
  ).map(([name]) => ({ label: name, value: name }));

  // For the purposes of invoice creation, we need clientId from a matter
  const clientIdForSelected = allMatters.find(
    m => m.clientName === clientId
  )?.id ?? '';

  // Matters belonging to selected client (use clientName as proxy since MatterSummary has clientName)
  const clientMatters = allMatters.filter(m => m.clientName === clientId);

  // ── Totals ────────────────────────────────────────────────────────────────
  const entriesSubtotal = entries
    .filter(e => checkedEntries.has(e.id))
    .reduce((s, e) => s + e.amount, 0);

  const fixedSubtotal = fixedLines.reduce((s, l) => s + l.amount, 0);
  const subtotal = entriesSubtotal + fixedSubtotal;

  const gstAmount = subtotal * gstRate;
  const cgst = gstType === 'Intra' ? gstAmount / 2 : 0;
  const sgst = gstType === 'Intra' ? gstAmount / 2 : 0;
  const igst = gstType === 'Inter' ? gstAmount : 0;
  const total = subtotal + gstAmount;

  // ── Handlers ──────────────────────────────────────────────────────────────
  function toggleMatter(mid: string) {
    setSelectedMatters(prev => {
      const next = new Set(prev);
      if (next.has(mid)) next.delete(mid); else next.add(mid);
      return next;
    });
  }

  function toggleEntry(eid: string) {
    setCheckedEntries(prev => {
      const next = new Set(prev);
      if (next.has(eid)) next.delete(eid); else next.add(eid);
      return next;
    });
  }

  function addFixedLine() {
    setFixedLines(prev => [...prev, newFixedLine()]);
  }

  function updateFixedLine(id: string, field: keyof FixedLine, value: string | number) {
    setFixedLines(prev => prev.map(l =>
      l.id === id ? { ...l, [field]: value } : l
    ));
  }

  function removeFixedLine(id: string) {
    setFixedLines(prev => prev.filter(l => l.id !== id));
  }

  const canCreate = clientId && selectedMatters.size > 0 &&
    (checkedEntries.size > 0 || fixedLines.some(l => l.amount > 0)) &&
    subtotal > 0;

  async function handleCreate() {
    if (!canCreate || !session) return;
    setSaving(true);
    setError(null);

    try {
      // Resolve the actual clientId from the matter records
      const resolvedClientId = allMatters.find(m =>
        m.clientName === clientId && selectedMatters.has(m.id)
      )?.id ?? clientIdForSelected;

      if (!resolvedClientId) {
        setError('Could not resolve client ID. Please select a client.');
        setSaving(false);
        return;
      }

      // Build line items — time entries first, then fixed lines
      const lineItems: LineItemInput[] = [];
      let sortOrder = 0;

      entries.filter(e => checkedEntries.has(e.id)).forEach(e => {
        lineItems.push({
          timeEntryId: e.id,
          description: e.description,
          activityCode: e.activityCode,
          hours: e.hours,
          rate: e.ratePerHour,
          amount: e.amount,
          sortOrder: sortOrder++,
        });
      });

      fixedLines.filter(l => l.description && l.amount > 0).forEach(l => {
        lineItems.push({
          description: l.description,
          rate: l.amount,
          amount: l.amount,
          sortOrder: sortOrder++,
        });
      });

      const input: CreateInvoiceInput = {
        clientId: resolvedClientId,
        matterIds: Array.from(selectedMatters),
        invoiceDate,
        dueDate: dueDate || undefined,
        gstType,
        lineItems,
        notes: notes.trim() || undefined,
      };

      const inv = await keel.billing.createInvoice(input);
      onCreated(inv.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  // ── Render ────────────────────────────────────────────────────────────────
  return (
    <>
      {/* Backdrop */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        onClick={onClose}
        style={{
          position: 'fixed', inset: 0,
          background: 'rgba(44,44,42,0.35)',
          zIndex: 40,
        }}
      />

      {/* Drawer */}
      <motion.div
        initial={{ x: '100%' }}
        animate={{ x: 0 }}
        exit={{ x: '100%' }}
        transition={{ type: 'spring', damping: 30, stiffness: 280 }}
        style={{
          position: 'fixed', top: 0, right: 0, bottom: 0,
          width: 560,
          background: colors.bgPrimary,
          borderLeft: `0.5px solid ${colors.border}`,
          zIndex: 41,
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
        }}
      >
        {/* Header */}
        <div style={{
          padding: `${spacing[5]} ${spacing[6]}`,
          borderBottom: `0.5px solid ${colors.border}`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexShrink: 0,
        }}>
          <div>
            <h2 style={{ fontFamily: fonts.display, fontSize: 20, fontWeight: 600, color: colors.textPrimary, margin: 0 }}>
              New Invoice
            </h2>
            <p style={{ fontFamily: fonts.ui, fontSize: fontSizes.label, color: colors.textTertiary, margin: `${spacing[1]} 0 0` }}>
              SAC 998212 · GST Compliant
            </p>
          </div>
          <button
            onClick={onClose}
            style={{
              width: 32, height: 32, borderRadius: '50%',
              border: `0.5px solid ${colors.border}`,
              background: 'transparent', cursor: 'pointer',
              fontFamily: fonts.mono, fontSize: 16,
              color: colors.textSecondary,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
            }}
          >
            ×
          </button>
        </div>

        {/* Scrollable body */}
        <div style={{ flex: 1, overflowY: 'auto', padding: `${spacing[5]} ${spacing[6]}`, display: 'flex', flexDirection: 'column', gap: spacing[5] }}>

          {/* ── Step 1: Client ── */}
          <Section title="Client">
            <select
              value={clientId}
              onChange={e => { setClientId(e.target.value); setSelectedMatters(new Set()); }}
              style={selectStyle}
            >
              <option value="">Select client…</option>
              {clientOptions.map(c => (
                <option key={c.value} value={c.value}>{c.label}</option>
              ))}
            </select>
          </Section>

          {/* ── Step 2: Matters ── */}
          {clientId && (
            <Section title="Matters">
              {clientMatters.length === 0 ? (
                <p style={emptyText}>No active matters for this client.</p>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: spacing[1] }}>
                  {clientMatters.map(m => {
                    const checked = selectedMatters.has(m.id);
                    return (
                      <label key={m.id} style={{ display: 'flex', alignItems: 'center', gap: spacing[3], cursor: 'pointer', padding: `${spacing[2]} ${spacing[3]}`, borderRadius: radius.card, background: checked ? 'rgba(74,101,128,0.05)' : 'transparent' }}>
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={() => toggleMatter(m.id)}
                          style={{ accentColor: colors.accentPrimary, width: 15, height: 15 }}
                        />
                        <div>
                          <span style={{ fontFamily: fonts.mono, fontSize: fontSizes.label, color: colors.accentPrimary }}>{m.id}</span>
                          <span style={{ fontFamily: fonts.ui, fontSize: fontSizes.body, color: colors.textPrimary, marginLeft: spacing[2] }}>{m.title}</span>
                        </div>
                      </label>
                    );
                  })}
                </div>
              )}
            </Section>
          )}

          {/* ── Step 3: Unbilled time entries ── */}
          {selectedMatters.size > 0 && (
            <Section title={`Unbilled Time Entries ${entries.length > 0 ? `(${entries.length})` : ''}`}>
              {entries.length === 0 ? (
                <p style={emptyText}>No unbilled entries for selected matters.</p>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
                  {entries.map(e => {
                    const checked = checkedEntries.has(e.id);
                    return (
                      <label key={e.id} style={{ display: 'flex', alignItems: 'flex-start', gap: spacing[3], cursor: 'pointer', padding: `${spacing[2]} ${spacing[3]}`, borderRadius: radius.card, background: checked ? 'rgba(74,101,128,0.04)' : 'transparent' }}>
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={() => toggleEntry(e.id)}
                          style={{ accentColor: colors.accentPrimary, width: 15, height: 15, marginTop: 2 }}
                        />
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
                            <span style={{ fontFamily: fonts.ui, fontSize: fontSizes.body, color: colors.textPrimary, flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                              {e.description}
                            </span>
                            <span style={{ fontFamily: fonts.mono, fontSize: fontSizes.label, color: colors.textPrimary, marginLeft: spacing[3], flexShrink: 0 }}>
                              {fmt(e.amount)}
                            </span>
                          </div>
                          <div style={{ fontFamily: fonts.mono, fontSize: 10, color: colors.textTertiary, marginTop: 2 }}>
                            {e.date} · {e.hours}h · {e.activityCode} · {e.matterId}
                          </div>
                        </div>
                      </label>
                    );
                  })}
                </div>
              )}

              {/* Fixed-fee lines */}
              {fixedLines.length > 0 && (
                <div style={{ marginTop: spacing[3], display: 'flex', flexDirection: 'column', gap: spacing[2] }}>
                  <FieldLabel>Fixed-fee lines</FieldLabel>
                  {fixedLines.map(l => (
                    <div key={l.id} style={{ display: 'flex', gap: spacing[2], alignItems: 'center' }}>
                      <input
                        type="text"
                        value={l.description}
                        onChange={e => updateFixedLine(l.id, 'description', e.target.value)}
                        placeholder="Description"
                        style={{ ...inputStyle, flex: 1 }}
                      />
                      <input
                        type="number"
                        value={l.amount || ''}
                        onChange={e => updateFixedLine(l.id, 'amount', parseFloat(e.target.value) || 0)}
                        placeholder="Amount"
                        style={{ ...inputStyle, width: 100, textAlign: 'right' }}
                      />
                      <button
                        onClick={() => removeFixedLine(l.id)}
                        style={{ background: 'transparent', border: 'none', cursor: 'pointer', color: colors.statusUrgent, fontFamily: fonts.mono, fontSize: 16, padding: '4px 6px' }}
                      >
                        ×
                      </button>
                    </div>
                  ))}
                </div>
              )}

              <button
                onClick={addFixedLine}
                style={{
                  marginTop: spacing[3],
                  background: 'transparent',
                  border: `0.5px dashed ${colors.border}`,
                  borderRadius: radius.button,
                  padding: `6px 14px`,
                  fontFamily: fonts.ui,
                  fontSize: fontSizes.label,
                  color: colors.textSecondary,
                  cursor: 'pointer',
                  width: '100%',
                }}
              >
                + Add fixed-fee line
              </button>
            </Section>
          )}

          {/* ── Step 4: Invoice meta ── */}
          {selectedMatters.size > 0 && (
            <Section title="Invoice Details">
              {/* GST type */}
              <div style={{ marginBottom: spacing[4] }}>
                <FieldLabel>GST Treatment</FieldLabel>
                <div style={{ display: 'flex', gap: spacing[2] }}>
                  {(['Intra', 'Inter'] as GstType[]).map(t => (
                    <button
                      key={t}
                      onClick={() => setGstType(t)}
                      style={{
                        flex: 1,
                        padding: `${spacing[2]} 0`,
                        borderRadius: radius.button,
                        border: `0.5px solid ${gstType === t ? colors.accentPrimary : colors.border}`,
                        background: gstType === t ? 'rgba(74,101,128,0.08)' : 'transparent',
                        color: gstType === t ? colors.accentPrimary : colors.textSecondary,
                        fontFamily: fonts.ui,
                        fontSize: fontSizes.label,
                        fontWeight: gstType === t ? 500 : 400,
                        cursor: 'pointer',
                      }}
                    >
                      {t === 'Intra' ? 'Intra-state (CGST + SGST)' : 'Inter-state (IGST)'}
                    </button>
                  ))}
                </div>
              </div>

              {/* Dates */}
              <div style={{ display: 'flex', gap: spacing[3] }}>
                <div style={{ flex: 1 }}>
                  <FieldLabel>Invoice Date</FieldLabel>
                  <input type="date" value={invoiceDate} onChange={e => setInvoiceDate(e.target.value)} style={inputStyle} />
                </div>
                <div style={{ flex: 1 }}>
                  <FieldLabel>Due Date (optional)</FieldLabel>
                  <input type="date" value={dueDate} onChange={e => setDueDate(e.target.value)} style={inputStyle} />
                </div>
              </div>

              {/* Notes */}
              <div style={{ marginTop: spacing[3] }}>
                <FieldLabel>Notes</FieldLabel>
                <textarea
                  value={notes}
                  onChange={e => setNotes(e.target.value)}
                  placeholder="Payment terms, bank details reminder, etc."
                  rows={3}
                  style={{ ...inputStyle, resize: 'vertical', minHeight: 64 }}
                />
              </div>
            </Section>
          )}

          {/* ── Totals panel ── */}
          {subtotal > 0 && (
            <div style={{
              background: colors.bgSecondary,
              borderRadius: radius.card,
              border: `0.5px solid ${colors.border}`,
              padding: `${spacing[4]} ${spacing[5]}`,
            }}>
              <TotalRow label="Subtotal" value={fmt(subtotal)} />
              {gstType === 'Intra' ? (
                <>
                  <TotalRow label={`CGST @ ${(gstRate * 50).toFixed(0)}%`} value={fmt(cgst)} />
                  <TotalRow label={`SGST @ ${(gstRate * 50).toFixed(0)}%`} value={fmt(sgst)} />
                </>
              ) : (
                <TotalRow label={`IGST @ ${(gstRate * 100).toFixed(0)}%`} value={fmt(igst)} />
              )}
              <div style={{ height: 1, background: colors.border, margin: `${spacing[2]} 0` }} />
              <TotalRow
                label="Total"
                value={fmt(total)}
                bold
              />
            </div>
          )}

          {/* Error */}
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
        </div>

        {/* Footer */}
        <div style={{
          padding: `${spacing[4]} ${spacing[6]}`,
          borderTop: `0.5px solid ${colors.border}`,
          display: 'flex',
          gap: spacing[3],
          flexShrink: 0,
          background: colors.bgPrimary,
        }}>
          <button
            onClick={onClose}
            style={{
              flex: 1, padding: `${spacing[3]} 0`,
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
          <motion.button
            onClick={handleCreate}
            disabled={!canCreate || saving}
            whileHover={canCreate && !saving ? { y: -1 } : undefined}
            transition={transition.fast}
            style={{
              flex: 2, padding: `${spacing[3]} 0`,
              borderRadius: radius.button,
              border: 'none',
              background: canCreate && !saving ? colors.accentPrimary : colors.border,
              color: canCreate && !saving ? '#fff' : colors.textTertiary,
              fontFamily: fonts.ui,
              fontSize: fontSizes.body,
              fontWeight: 500,
              cursor: canCreate && !saving ? 'pointer' : 'default',
            }}
          >
            {saving ? 'Creating…' : `Create Invoice${total > 0 ? ` · ${fmt(total)}` : ''}`}
          </motion.button>
        </div>
      </motion.div>
    </>
  );
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h4 style={{
        fontFamily: fonts.ui, fontSize: 11, fontWeight: 500,
        color: colors.textTertiary, textTransform: 'uppercase' as const,
        letterSpacing: '0.06em', margin: `0 0 ${spacing[3]}`,
      }}>
        {title}
      </h4>
      {children}
    </div>
  );
}

function FieldLabel({ children }: { children: string }) {
  return (
    <label style={{
      display: 'block',
      fontSize: 11, fontWeight: 500,
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

function TotalRow({ label, value, bold }: { label: string; value: string; bold?: boolean }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: spacing[1] }}>
      <span style={{ fontFamily: fonts.ui, fontSize: fontSizes.label, color: bold ? colors.textPrimary : colors.textSecondary, fontWeight: bold ? 600 : 400 }}>
        {label}
      </span>
      <span style={{ fontFamily: fonts.mono, fontSize: bold ? 16 : fontSizes.label, color: bold ? colors.textPrimary : colors.textSecondary, fontWeight: bold ? 600 : 400 }}>
        {value}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shared styles
// ---------------------------------------------------------------------------

const inputStyle: React.CSSProperties = {
  width: '100%',
  padding: `${spacing[2]} ${spacing[3]}`,
  borderRadius: radius.input,
  border: `0.5px solid ${colors.border}`,
  background: colors.bgPrimary,
  color: colors.textPrimary,
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  outline: 'none',
  boxSizing: 'border-box' as const,
};

const selectStyle: React.CSSProperties = {
  ...inputStyle,
  appearance: 'none' as const,
  backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='%239A9590'/%3E%3C/svg%3E")`,
  backgroundRepeat: 'no-repeat',
  backgroundPosition: 'right 12px center',
  paddingRight: 32,
};

const emptyText: React.CSSProperties = {
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  color: colors.textTertiary,
  margin: 0,
};
