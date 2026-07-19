/**
 * FirmSettingsPanel — Partner-only form for billing configuration.
 *
 * Firm details (name, GSTIN, PAN, address), bank info, hourly rates by role.
 * All changes saved immediately on blur or submit.
 */
import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { keel } from '@/lib/tauri';
import type { FirmSettings, UpdateFirmSettingsInput } from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';

export default function FirmSettingsPanel() {
  const [settings, setSettings] = useState<FirmSettings | null>(null);
  const [saving, setSaving]     = useState(false);
  const [saved, setSaved]       = useState(false);
  const [error, setError]       = useState<string | null>(null);

  // Form fields
  const [firmName, setFirmName]       = useState('');
  const [firmGstin, setFirmGstin]     = useState('');
  const [firmAddress, setFirmAddress] = useState('');
  const [firmPan, setFirmPan]         = useState('');
  const [bankName, setBankName]       = useState('');
  const [bankAccount, setBankAccount] = useState('');
  const [bankIfsc, setBankIfsc]       = useState('');
  const [partnerRate, setPartnerRate]     = useState('');
  const [associateRate, setAssociateRate] = useState('');
  const [paralegalRate, setParalegalRate] = useState('');

  const load = useCallback(async () => {
    try {
      const s = await keel.billing.getFirmSettings();
      setSettings(s);
      setFirmName(s.firmName);
      setFirmGstin(s.firmGstin ?? '');
      setFirmAddress(s.firmAddress ?? '');
      setFirmPan(s.firmPan ?? '');
      setBankName(s.bankName ?? '');
      setBankAccount(s.bankAccount ?? '');
      setBankIfsc(s.bankIfsc ?? '');
      setPartnerRate(String(s.partnerRate));
      setAssociateRate(String(s.associateRate));
      setParalegalRate(String(s.paralegalRate));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => { load(); }, []);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setSaved(false);
    try {
      const input: UpdateFirmSettingsInput = {
        firmName:      firmName || undefined,
        firmGstin:     firmGstin || undefined,
        firmAddress:   firmAddress || undefined,
        firmPan:       firmPan || undefined,
        bankName:      bankName || undefined,
        bankAccount:   bankAccount || undefined,
        bankIfsc:      bankIfsc || undefined,
        partnerRate:   partnerRate  ? parseFloat(partnerRate)  : undefined,
        associateRate: associateRate ? parseFloat(associateRate) : undefined,
        paralegalRate: paralegalRate ? parseFloat(paralegalRate) : undefined,
      };
      const updated = await keel.billing.updateFirmSettings(input);
      setSettings(updated);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  if (!settings) {
    return <p style={{ fontFamily: fonts.ui, color: colors.textTertiary }}>Loading settings...</p>;
  }

  return (
    <div style={{ maxWidth: 600, display: 'flex', flexDirection: 'column', gap: spacing[6] }}>

      {/* Firm Details */}
      <Section title="Firm Details">
        <Field label="Firm Name" value={firmName} onChange={setFirmName} />
        <div style={{ display: 'flex', gap: spacing[3] }}>
          <Field label="GSTIN" value={firmGstin} onChange={setFirmGstin} placeholder="07AABCP1234Z1" />
          <Field label="PAN" value={firmPan} onChange={setFirmPan} placeholder="AABCP1234Z" />
        </div>
        <Field label="Address" value={firmAddress} onChange={setFirmAddress} multiline placeholder="Multi-line address for invoice header" />
      </Section>

      {/* Bank Details */}
      <Section title="Bank Details (shown on invoices)">
        <Field label="Bank Name" value={bankName} onChange={setBankName} />
        <div style={{ display: 'flex', gap: spacing[3] }}>
          <Field label="Account Number" value={bankAccount} onChange={setBankAccount} />
          <Field label="IFSC Code" value={bankIfsc} onChange={setBankIfsc} />
        </div>
      </Section>

      {/* Hourly Rates */}
      <Section title="Hourly Rates (INR)">
        <div style={{ display: 'flex', gap: spacing[3] }}>
          <Field label="Partner" value={partnerRate} onChange={setPartnerRate} type="number" />
          <Field label="Associate" value={associateRate} onChange={setAssociateRate} type="number" />
          <Field label="Paralegal" value={paralegalRate} onChange={setParalegalRate} type="number" />
        </div>
        <p style={{ fontFamily: fonts.ui, fontSize: fontSizes.label, color: colors.textTertiary, margin: 0 }}>
          GST rate: {(settings.gstRate * 100).toFixed(0)}% (SAC 998212 — Legal advisory). Rate changes only apply to new time entries.
        </p>
      </Section>

      {/* Error */}
      <AnimatePresence>
        {error && (
          <motion.div
            initial={{ opacity: 0, y: -4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            style={{
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

      {/* Save */}
      <div style={{ display: 'flex', gap: spacing[3], alignItems: 'center' }}>
        <button
          onClick={handleSave}
          disabled={saving}
          style={{
            padding: `${spacing[3]} ${spacing[6]}`,
            borderRadius: radius.button,
            border: 'none',
            background: saving ? colors.border : colors.accentPrimary,
            color: saving ? colors.textTertiary : '#fff',
            fontFamily: fonts.ui,
            fontSize: fontSizes.body,
            fontWeight: 500,
            cursor: saving ? 'default' : 'pointer',
          }}
        >
          {saving ? 'Saving...' : 'Save Settings'}
        </button>
        <AnimatePresence>
          {saved && (
            <motion.span
              initial={{ opacity: 0, x: -8 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0 }}
              style={{
                fontFamily: fonts.ui,
                fontSize: fontSizes.label,
                color: colors.statusClear,
                fontWeight: 500,
              }}
            >
              Saved
            </motion.span>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={{
      background: colors.bgSecondary,
      borderRadius: radius.card,
      border: `0.5px solid ${colors.border}`,
      padding: spacing[5],
      display: 'flex',
      flexDirection: 'column',
      gap: spacing[3],
    }}>
      <h3 style={{
        fontFamily: fonts.display,
        fontSize: 15,
        fontWeight: 600,
        color: colors.textPrimary,
        margin: 0,
      }}>
        {title}
      </h3>
      {children}
    </div>
  );
}

function Field({
  label, value, onChange, multiline, placeholder, type,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  multiline?: boolean;
  placeholder?: string;
  type?: string;
}) {
  const style: React.CSSProperties = {
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
    ...(multiline ? { minHeight: 64, resize: 'vertical' as const } : {}),
  };

  return (
    <div style={{ flex: 1 }}>
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
        {label}
      </label>
      {multiline ? (
        <textarea value={value} onChange={e => onChange(e.target.value)} placeholder={placeholder} style={style} />
      ) : (
        <input type={type ?? 'text'} value={value} onChange={e => onChange(e.target.value)} placeholder={placeholder} style={style} />
      )}
    </div>
  );
}
