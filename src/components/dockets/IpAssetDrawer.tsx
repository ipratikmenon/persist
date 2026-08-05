/**
 * IpAssetDrawer — create or edit an IP asset on a matter (B02).
 * Right-side drawer, same idiom as AddDeadlineDrawer / NewMatterDrawer.
 */
import { useEffect, useState } from 'react';
import { motion, AnimatePresence, useReducedMotion } from 'motion/react';
import type {
  ApplicantEntityType,
  CreateIpAssetInput,
  IpAsset,
  IpAssetStatus,
  IpAssetType,
  UpdateIpAssetInput,
} from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { colors, fonts, fontSizes, radius, shadows, spacing } from '@/design-system/tokens';
import { transition, panelVariants } from '@/design-system/motion';

const ASSET_TYPES: IpAssetType[] = ['Trademark', 'Patent', 'Design', 'Copyright', 'PlantVariety'];

const STATUSES: IpAssetStatus[] = [
  'Pending', 'Examination', 'Accepted', 'Advertised', 'Opposed',
  'Registered', 'Granted', 'Lapsed', 'Abandoned', 'Cancelled',
];

const ENTITY_TYPES: ApplicantEntityType[] = [
  'Individual', 'Startup', 'SmallEntity', 'Company', 'Government',
];

/** Startup/small-entity status changes official fee slabs at the Indian registries. */
const ENTITY_LABELS: Record<ApplicantEntityType, string> = {
  Individual:  'Individual',
  Startup:     'Startup',
  SmallEntity: 'Small Entity',
  Company:     'Company',
  Government:  'Government',
};

const inputStyle = {
  width: '100%',
  padding: '8px 10px',
  borderRadius: radius.input,
  border: `0.5px solid ${colors.border}`,
  background: colors.bgPrimary,
  color: colors.textPrimary,
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  outline: 'none',
  boxSizing: 'border-box' as const,
};

const labelStyle = {
  display: 'block',
  fontSize: fontSizes.label,
  color: colors.textSecondary,
  fontFamily: fonts.ui,
  marginBottom: 5,
};

/** Parse "9, 42" / "9 42" into [9, 42]; ignores anything non-numeric. */
function parseClasses(raw: string): number[] {
  return raw
    .split(/[,\s]+/)
    .map(s => s.trim())
    .filter(Boolean)
    .map(Number)
    .filter(n => Number.isInteger(n) && n > 0);
}

interface IpAssetDrawerProps {
  open: boolean;
  matterId: string;
  /** Supply to edit an existing asset; omit to create a new one. */
  asset?: IpAsset | null;
  /** Pre-selects the asset type for a new asset — usually the matter's type. */
  defaultAssetType?: IpAssetType;
  onClose: () => void;
  onSaved: () => void;
}

export function IpAssetDrawer({
  open, matterId, asset, defaultAssetType, onClose, onSaved,
}: IpAssetDrawerProps) {
  const shouldReduce = useReducedMotion();
  const isEdit = Boolean(asset);

  const [assetType, setAssetType]     = useState<IpAssetType>(defaultAssetType ?? 'Trademark');
  const [title, setTitle]             = useState('');
  const [appNumber, setAppNumber]     = useState('');
  const [regNumber, setRegNumber]     = useState('');
  const [filingDate, setFilingDate]   = useState('');
  const [priorityDate, setPriorityDate] = useState('');
  const [expiryDate, setExpiryDate]   = useState('');
  const [entityType, setEntityType]   = useState<ApplicantEntityType>('Company');
  const [jurisdiction, setJurisdiction] = useState('India');
  const [classesRaw, setClassesRaw]   = useState('');
  const [status, setStatus]           = useState<IpAssetStatus>('Pending');
  const [notes, setNotes]             = useState('');

  const [saving, setSaving] = useState(false);
  const [error, setError]   = useState<string | null>(null);

  // Reset the form whenever the drawer opens, seeding from `asset` when editing.
  useEffect(() => {
    if (!open) return;
    setError(null);
    setAssetType(asset?.assetType ?? defaultAssetType ?? 'Trademark');
    setTitle(asset?.title ?? '');
    setAppNumber(asset?.applicationNumber ?? '');
    setRegNumber(asset?.registrationNumber ?? '');
    setFilingDate(asset?.filingDate ?? '');
    setPriorityDate(asset?.priorityDate ?? '');
    setExpiryDate(asset?.expiryDate ?? '');
    setEntityType(asset?.applicantEntityType ?? 'Company');
    setJurisdiction(asset?.jurisdiction ?? 'India');
    setClassesRaw(asset?.classes.join(', ') ?? '');
    setStatus(asset?.status ?? 'Pending');
    setNotes(asset?.notes ?? '');
  }, [open, asset, defaultAssetType]);

  const save = async () => {
    if (!title.trim()) {
      setError('Title is required.');
      return;
    }
    setSaving(true);
    setError(null);

    // Empty strings mean "not set" — send undefined so Keel's COALESCE
    // leaves the stored value alone rather than blanking it.
    const optional = (v: string) => (v.trim() ? v.trim() : undefined);

    try {
      if (isEdit && asset) {
        const patch: UpdateIpAssetInput = {
          assetType,
          title: title.trim(),
          applicationNumber:  optional(appNumber),
          registrationNumber: optional(regNumber),
          filingDate:         optional(filingDate),
          priorityDate:       optional(priorityDate),
          expiryDate:         optional(expiryDate),
          applicantEntityType: entityType,
          jurisdiction:       optional(jurisdiction),
          classes:            parseClasses(classesRaw),
          status,
          notes:              optional(notes),
        };
        await keel.ipAssets.update(asset.id, patch);
      } else {
        const input: CreateIpAssetInput = {
          matterId,
          assetType,
          title: title.trim(),
          applicationNumber:  optional(appNumber),
          registrationNumber: optional(regNumber),
          filingDate:         optional(filingDate),
          priorityDate:       optional(priorityDate),
          expiryDate:         optional(expiryDate),
          applicantEntityType: entityType,
          jurisdiction:       optional(jurisdiction),
          classes:            parseClasses(classesRaw),
          status,
          notes:              optional(notes),
        };
        await keel.ipAssets.create(input);
      }
      onSaved();
      onClose();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const variants = shouldReduce
    ? { hidden: { opacity: 0 }, visible: { opacity: 1 }, exit: { opacity: 0 } }
    : panelVariants.right;

  // Nice classes apply to trademarks, Locarno to designs; other types have none.
  const showClasses = assetType === 'Trademark' || assetType === 'Design';
  const classLabel  = assetType === 'Design' ? 'Locarno classes' : 'Nice classes';

  return (
    <AnimatePresence>
      {open && (
        <>
          <motion.div
            key="bd"
            initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
            transition={transition.fast}
            onClick={onClose}
            style={{ position: 'fixed', inset: 0, background: 'rgba(44,44,42,0.25)', zIndex: 40 }}
          />
          <motion.aside
            key="drawer"
            variants={variants} initial="hidden" animate="visible" exit="exit"
            style={{
              position: 'fixed', top: 0, right: 0, bottom: 0, width: 420,
              background: colors.bgPrimary,
              borderLeft: `0.5px solid ${colors.border}`,
              boxShadow: shadows.modal, zIndex: 50,
              display: 'flex', flexDirection: 'column', overflow: 'hidden',
            }}
          >
            {/* Header */}
            <div style={{
              padding: `${spacing[5]} ${spacing[6]}`,
              borderBottom: `0.5px solid ${colors.border}`,
              display: 'flex', justifyContent: 'space-between', alignItems: 'center',
              flexShrink: 0,
            }}>
              <h2 style={{
                fontFamily: fonts.display, fontSize: 18, fontWeight: 500,
                margin: 0, color: colors.textPrimary,
              }}>
                {isEdit ? 'Edit IP Asset' : 'Add IP Asset'}
              </h2>
              <button
                onClick={onClose}
                style={{
                  background: 'none', border: 'none', cursor: 'pointer',
                  color: colors.textTertiary, fontSize: 18, lineHeight: 1, padding: 0,
                }}
              >
                ✕
              </button>
            </div>

            {/* Body */}
            <div style={{ flex: 1, overflow: 'auto', padding: spacing[6] }}>
              <div style={{ display: 'flex', flexDirection: 'column', gap: spacing[4] }}>

                <div>
                  <label style={labelStyle}>Asset type</label>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: spacing[2] }}>
                    {ASSET_TYPES.map(t => (
                      <button
                        key={t}
                        onClick={() => setAssetType(t)}
                        style={{
                          padding: '5px 11px',
                          borderRadius: radius.chip,
                          border: `0.5px solid ${assetType === t ? colors.accentPrimary : colors.border}`,
                          background: assetType === t ? 'rgba(74,101,128,0.09)' : 'transparent',
                          color: assetType === t ? colors.accentPrimary : colors.textSecondary,
                          fontFamily: fonts.ui, fontSize: fontSizes.label,
                          fontWeight: assetType === t ? 500 : 400,
                          cursor: 'pointer',
                        }}
                      >
                        {t === 'PlantVariety' ? 'Plant Variety' : t}
                      </button>
                    ))}
                  </div>
                </div>

                <div>
                  <label style={labelStyle}>
                    {assetType === 'Patent' ? 'Invention title' : 'Title / mark'}
                  </label>
                  <input
                    value={title}
                    onChange={e => setTitle(e.target.value)}
                    placeholder={assetType === 'Patent' ? 'A method for…' : 'PETALVEDA'}
                    style={inputStyle}
                  />
                </div>

                <div style={{ display: 'flex', gap: spacing[3] }}>
                  <div style={{ flex: 1 }}>
                    <label style={labelStyle}>Application no.</label>
                    <input
                      value={appNumber}
                      onChange={e => setAppNumber(e.target.value)}
                      placeholder="4455121"
                      style={{ ...inputStyle, fontFamily: fonts.mono, fontSize: fontSizes.mono }}
                    />
                  </div>
                  <div style={{ flex: 1 }}>
                    <label style={labelStyle}>Registration no.</label>
                    <input
                      value={regNumber}
                      onChange={e => setRegNumber(e.target.value)}
                      placeholder="—"
                      style={{ ...inputStyle, fontFamily: fonts.mono, fontSize: fontSizes.mono }}
                    />
                  </div>
                </div>

                <div style={{ display: 'flex', gap: spacing[3] }}>
                  <div style={{ flex: 1 }}>
                    <label style={labelStyle}>Filing date</label>
                    <input type="date" value={filingDate}
                      onChange={e => setFilingDate(e.target.value)} style={inputStyle} />
                  </div>
                  <div style={{ flex: 1 }}>
                    <label style={labelStyle}>Priority date</label>
                    <input type="date" value={priorityDate}
                      onChange={e => setPriorityDate(e.target.value)} style={inputStyle} />
                  </div>
                </div>

                <div>
                  <label style={labelStyle}>Renewal / expiry date</label>
                  <input type="date" value={expiryDate}
                    onChange={e => setExpiryDate(e.target.value)} style={inputStyle} />
                </div>

                {showClasses && (
                  <div>
                    <label style={labelStyle}>{classLabel}</label>
                    <input
                      value={classesRaw}
                      onChange={e => setClassesRaw(e.target.value)}
                      placeholder="3, 5, 44"
                      style={{ ...inputStyle, fontFamily: fonts.mono, fontSize: fontSizes.mono }}
                    />
                    <p style={{
                      fontSize: 11, color: colors.textTertiary, fontFamily: fonts.ui,
                      margin: `${spacing[1]} 0 0`,
                    }}>
                      Comma separated.
                    </p>
                  </div>
                )}

                <div>
                  <label style={labelStyle}>Status</label>
                  <select
                    value={status}
                    onChange={e => setStatus(e.target.value as IpAssetStatus)}
                    style={inputStyle}
                  >
                    {STATUSES.map(s => <option key={s} value={s}>{s}</option>)}
                  </select>
                </div>

                <div style={{ display: 'flex', gap: spacing[3] }}>
                  <div style={{ flex: 1 }}>
                    <label style={labelStyle}>Applicant type</label>
                    <select
                      value={entityType}
                      onChange={e => setEntityType(e.target.value as ApplicantEntityType)}
                      style={inputStyle}
                    >
                      {ENTITY_TYPES.map(t => (
                        <option key={t} value={t}>{ENTITY_LABELS[t]}</option>
                      ))}
                    </select>
                  </div>
                  <div style={{ flex: 1 }}>
                    <label style={labelStyle}>Jurisdiction</label>
                    <input value={jurisdiction}
                      onChange={e => setJurisdiction(e.target.value)} style={inputStyle} />
                  </div>
                </div>

                <div>
                  <label style={labelStyle}>Notes</label>
                  <textarea
                    value={notes}
                    onChange={e => setNotes(e.target.value)}
                    rows={3}
                    style={{ ...inputStyle, resize: 'vertical' as const }}
                  />
                </div>

                {error && (
                  <div style={{
                    padding: spacing[3],
                    borderRadius: radius.input,
                    background: 'rgba(192,57,43,0.08)',
                    color: colors.statusUrgent,
                    fontFamily: fonts.ui, fontSize: fontSizes.label,
                  }}>
                    {error}
                  </div>
                )}
              </div>
            </div>

            {/* Footer */}
            <div style={{
              padding: `${spacing[4]} ${spacing[6]}`,
              borderTop: `0.5px solid ${colors.border}`,
              display: 'flex', justifyContent: 'flex-end', gap: spacing[3],
              flexShrink: 0,
            }}>
              <button
                onClick={onClose}
                style={{
                  padding: `${spacing[2]} ${spacing[4]}`,
                  borderRadius: radius.button,
                  border: `0.5px solid ${colors.border}`,
                  background: 'transparent', color: colors.textSecondary,
                  fontFamily: fonts.ui, fontSize: fontSizes.body, cursor: 'pointer',
                }}
              >
                Cancel
              </button>
              <motion.button
                onClick={save}
                disabled={saving}
                whileHover={!shouldReduce && !saving ? { opacity: 0.88 } : undefined}
                whileTap={!shouldReduce && !saving ? { scale: 0.97 } : undefined}
                style={{
                  padding: `${spacing[2]} ${spacing[5]}`,
                  borderRadius: radius.button, border: 'none',
                  background: colors.accentPrimary, color: '#fff',
                  fontFamily: fonts.ui, fontSize: fontSizes.body, fontWeight: 500,
                  cursor: saving ? 'not-allowed' : 'pointer',
                  opacity: saving ? 0.7 : 1,
                }}
              >
                {saving ? 'Saving…' : isEdit ? 'Save changes' : 'Add asset'}
              </motion.button>
            </div>
          </motion.aside>
        </>
      )}
    </AnimatePresence>
  );
}
