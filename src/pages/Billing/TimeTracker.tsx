/**
 * TimeTracker — time entry form + recent entries table.
 *
 * Matter selector → date → hours (+/-) → description → activity code chips → save.
 * Running total shown live. Recent entries grouped by date below the form.
 */
import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { keel } from '@/lib/tauri';
import type {
  TimeEntry,
  MatterSummary,
  ActivityCode,
  CreateTimeEntryInput,
} from '@/lib/ipc-types';
import { ACTIVITY_CODES } from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';
import { transition } from '@/design-system/motion';

function formatCurrency(n: number): string {
  return new Intl.NumberFormat('en-IN', { style: 'currency', currency: 'INR', minimumFractionDigits: 0 }).format(n);
}

export default function TimeTracker() {
  const [matters, setMatters] = useState<MatterSummary[]>([]);
  const [entries, setEntries] = useState<TimeEntry[]>([]);
  const [matterId, setMatterId] = useState('');
  const [date, setDate]       = useState(new Date().toISOString().slice(0, 10));
  const [hours, setHours]     = useState(1.0);
  const [desc, setDesc]       = useState('');
  const [code, setCode]       = useState<ActivityCode>('L300');
  const [billable, setBillable] = useState(true);
  const [saving, setSaving]   = useState(false);
  const [error, setError]     = useState<string | null>(null);
  const [rate, setRate]       = useState(0);

  const loadMatters = useCallback(async () => {
    try {
      const list = await keel.matters.list({ status: ['Active'] });
      setMatters(list);
      if (list.length > 0 && !matterId) setMatterId(list[0].id);
    } catch { /* ignore */ }
  }, []);

  const loadEntries = useCallback(async () => {
    try {
      const all = await keel.billing.listTimeEntries();
      setEntries(all.slice(0, 20));
    } catch { /* ignore */ }
  }, []);

  const loadRate = useCallback(async () => {
    try {
      const s = await keel.billing.getFirmSettings();
      // Pick rate based on session role — but simplified: just use partner rate for display
      setRate(s.partnerRate);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => { loadMatters(); loadEntries(); loadRate(); }, []);

  const handleSave = async () => {
    if (!matterId || !desc.trim() || hours < 0.25) return;
    setSaving(true);
    setError(null);
    try {
      const input: CreateTimeEntryInput = {
        matterId,
        date,
        hours,
        description: desc.trim(),
        activityCode: code,
        isBillable: billable,
      };
      await keel.billing.createTimeEntry(input);
      setDesc('');
      setHours(1.0);
      await loadEntries();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const runningTotal = hours * rate;

  return (
    <div style={{ display: 'flex', gap: spacing[8], alignItems: 'flex-start' }}>
      {/* ── Entry form ──────────────────────────────────── */}
      <div style={{
        width: 420,
        flexShrink: 0,
        background: colors.bgSecondary,
        borderRadius: radius.card,
        border: `0.5px solid ${colors.border}`,
        padding: spacing[5],
        display: 'flex',
        flexDirection: 'column',
        gap: spacing[4],
      }}>
        <h3 style={{
          fontFamily: fonts.display,
          fontSize: 17,
          fontWeight: 600,
          color: colors.textPrimary,
          margin: 0,
        }}>
          Log Time
        </h3>

        {/* Matter */}
        <div>
          <Label>Matter</Label>
          <select
            value={matterId}
            onChange={e => setMatterId(e.target.value)}
            style={selectStyle}
          >
            <option value="">Select matter...</option>
            {matters.map(m => (
              <option key={m.id} value={m.id}>{m.id} — {m.title}</option>
            ))}
          </select>
        </div>

        {/* Date + Hours row */}
        <div style={{ display: 'flex', gap: spacing[3] }}>
          <div style={{ flex: 1 }}>
            <Label>Date</Label>
            <input
              type="date"
              value={date}
              onChange={e => setDate(e.target.value)}
              style={inputStyle}
            />
          </div>
          <div style={{ flex: 1 }}>
            <Label>Hours</Label>
            <div style={{ display: 'flex', alignItems: 'center', gap: spacing[2] }}>
              <button
                onClick={() => setHours(h => Math.max(0.25, h - 0.25))}
                style={stepBtnStyle}
              >
                −
              </button>
              <input
                type="number"
                min={0.25}
                step={0.25}
                value={hours}
                onChange={e => setHours(Math.max(0.25, parseFloat(e.target.value) || 0.25))}
                style={{ ...inputStyle, textAlign: 'center', flex: 1 }}
              />
              <button
                onClick={() => setHours(h => h + 0.25)}
                style={stepBtnStyle}
              >
                +
              </button>
            </div>
          </div>
        </div>

        {/* Description */}
        <div>
          <Label>Description</Label>
          <textarea
            value={desc}
            onChange={e => setDesc(e.target.value)}
            placeholder="What did you work on?"
            rows={3}
            style={{ ...inputStyle, resize: 'vertical', minHeight: 72 }}
          />
        </div>

        {/* Activity code chips */}
        <div>
          <Label>Activity</Label>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: spacing[1] }}>
            {ACTIVITY_CODES.map(ac => {
              const selected = code === ac.code;
              return (
                <button
                  key={ac.code}
                  onClick={() => setCode(ac.code)}
                  style={{
                    padding: `4px 10px`,
                    borderRadius: radius.button,
                    border: `0.5px solid ${selected ? colors.accentPrimary : colors.border}`,
                    background: selected ? 'rgba(74,101,128,0.10)' : 'transparent',
                    color: selected ? colors.accentPrimary : colors.textSecondary,
                    fontFamily: fonts.ui,
                    fontSize: fontSizes.label,
                    fontWeight: selected ? 500 : 400,
                    cursor: 'pointer',
                    transition: `all ${transition.fast.duration}s`,
                  }}
                >
                  {ac.code} {ac.label}
                </button>
              );
            })}
          </div>
        </div>

        {/* Billable toggle */}
        <div style={{ display: 'flex', alignItems: 'center', gap: spacing[2] }}>
          <button
            onClick={() => setBillable(v => !v)}
            style={{
              width: 36, height: 20, borderRadius: 10,
              background: billable ? colors.accentPrimary : colors.border,
              border: 'none', cursor: 'pointer', position: 'relative',
              transition: `background ${transition.fast.duration}s`,
            }}
          >
            <div style={{
              width: 16, height: 16, borderRadius: '50%', background: '#fff',
              position: 'absolute', top: 2,
              left: billable ? 18 : 2,
              transition: `left ${transition.fast.duration}s`,
              boxShadow: '0 1px 3px rgba(0,0,0,0.15)',
            }} />
          </button>
          <span style={{
            fontFamily: fonts.ui, fontSize: fontSizes.label,
            color: billable ? colors.textPrimary : colors.textTertiary,
          }}>
            {billable ? 'Billable' : 'Non-billable'}
          </span>
        </div>

        {/* Running total */}
        {billable && rate > 0 && (
          <div style={{
            padding: `${spacing[3]} ${spacing[4]}`,
            background: 'rgba(74,101,128,0.05)',
            borderRadius: radius.card,
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'baseline',
          }}>
            <span style={{ fontFamily: fonts.ui, fontSize: fontSizes.label, color: colors.textSecondary }}>
              {hours} hrs × {formatCurrency(rate)}/hr
            </span>
            <span style={{ fontFamily: fonts.display, fontSize: 18, fontWeight: 600, color: colors.accentPrimary }}>
              {formatCurrency(runningTotal)}
            </span>
          </div>
        )}

        {/* Error */}
        <AnimatePresence>
          {error && (
            <motion.div
              initial={{ opacity: 0, y: -4 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              style={{
                padding: `${spacing[2]} ${spacing[3]}`,
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

        {/* Save */}
        <button
          onClick={handleSave}
          disabled={saving || !matterId || !desc.trim() || hours < 0.25}
          style={{
            padding: `${spacing[3]} 0`,
            borderRadius: radius.button,
            border: 'none',
            background: (saving || !matterId || !desc.trim()) ? colors.border : colors.accentPrimary,
            color: (saving || !matterId || !desc.trim()) ? colors.textTertiary : '#fff',
            fontFamily: fonts.ui,
            fontSize: fontSizes.body,
            fontWeight: 500,
            cursor: (saving || !matterId || !desc.trim()) ? 'default' : 'pointer',
          }}
        >
          {saving ? 'Saving...' : 'Log Time Entry'}
        </button>
      </div>

      {/* ── Recent entries table ────────────────────────── */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <h3 style={{
          fontFamily: fonts.display,
          fontSize: 17,
          fontWeight: 500,
          color: colors.textPrimary,
          margin: `0 0 ${spacing[4]}`,
        }}>
          Recent Entries
        </h3>

        {entries.length === 0 ? (
          <p style={{ fontFamily: fonts.ui, fontSize: fontSizes.body, color: colors.textTertiary }}>
            No time entries yet. Log your first entry to get started.
          </p>
        ) : (
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>
              <tr>
                {['Date', 'Matter', 'Activity', 'Hours', 'Amount', 'Status'].map(h => (
                  <th key={h} style={thStyle}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {entries.map(e => (
                <tr key={e.id}>
                  <td style={tdStyle}>
                    <span style={{ fontFamily: fonts.mono, fontSize: fontSizes.label }}>{e.date}</span>
                  </td>
                  <td style={tdStyle}>
                    <span style={{ fontFamily: fonts.mono, fontSize: fontSizes.label, color: colors.textTertiary }}>
                      {e.matterId}
                    </span>
                  </td>
                  <td style={tdStyle}>
                    <span style={{
                      display: 'inline-block',
                      padding: '2px 8px',
                      borderRadius: radius.button,
                      background: colors.bgTertiary,
                      fontFamily: fonts.mono,
                      fontSize: 10,
                      color: colors.textSecondary,
                    }}>
                      {e.activityCode}
                    </span>
                  </td>
                  <td style={{ ...tdStyle, textAlign: 'right', fontFamily: fonts.mono }}>
                    {e.hours.toFixed(2)}
                  </td>
                  <td style={{ ...tdStyle, textAlign: 'right', fontFamily: fonts.mono }}>
                    {formatCurrency(e.amount)}
                  </td>
                  <td style={tdStyle}>
                    {e.isInvoiced ? (
                      <span style={{
                        fontSize: 10, fontFamily: fonts.mono, color: colors.statusClear,
                        padding: '2px 6px', borderRadius: 4,
                        background: 'rgba(74,124,89,0.08)',
                      }}>
                        Invoiced
                      </span>
                    ) : (
                      <span style={{
                        fontSize: 10, fontFamily: fonts.mono, color: colors.statusWarning,
                        padding: '2px 6px', borderRadius: 4,
                        background: 'rgba(212,135,42,0.08)',
                      }}>
                        Unbilled
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shared styles
// ---------------------------------------------------------------------------

function Label({ children }: { children: string }) {
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

const stepBtnStyle: React.CSSProperties = {
  width: 28, height: 28, borderRadius: 6,
  border: `0.5px solid ${colors.border}`,
  background: colors.bgPrimary,
  color: colors.textSecondary,
  fontFamily: fonts.mono,
  fontSize: 14,
  cursor: 'pointer',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
};

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
