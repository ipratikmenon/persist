import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { useReducedMotion } from 'motion/react';
import type { Client, MatterType, MatterPriority, CreateMatterInput } from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { colors, fonts, fontSizes, radius, shadows, spacing } from '@/design-system/tokens';
import { panelVariants, transition } from '@/design-system/motion';

const MATTER_TYPES: MatterType[] = [
  'Trademark', 'Patent', 'Design', 'Copyright',
  'Corporate', 'Litigation', 'Paralegal',
];

const PRIORITIES: MatterPriority[] = ['Normal', 'High', 'Urgent'];

interface NewMatterDrawerProps {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}

const today = () => new Date().toISOString().slice(0, 10);

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
  fontWeight: 500,
  color: colors.textSecondary,
  marginBottom: 5,
  fontFamily: fonts.ui,
};

export function NewMatterDrawer({ open, onClose, onCreated }: NewMatterDrawerProps) {
  const shouldReduce = useReducedMotion();

  const [clients, setClients] = useState<Client[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [form, setForm] = useState<{
    clientId: string;
    title: string;
    matterType: MatterType;
    priority: MatterPriority;
    forum: string;
    jurisdiction: string;
    openedDate: string;
    targetCloseDate: string;
    internalNotes: string;
  }>({
    clientId: '',
    title: '',
    matterType: 'Trademark',
    priority: 'Normal',
    forum: '',
    jurisdiction: 'India',
    openedDate: today(),
    targetCloseDate: '',
    internalNotes: '',
  });

  useEffect(() => {
    if (open) {
      keel.clients.list()
        .then(setClients)
        .catch(() => setClients([]));
      setError(null);
    }
  }, [open]);

  const set = <K extends keyof typeof form>(key: K, value: typeof form[K]) =>
    setForm(f => ({ ...f, [key]: value }));

  const handleSubmit = async () => {
    if (!form.clientId || !form.title.trim()) {
      setError('Client and title are required.');
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const input: CreateMatterInput = {
        clientId:        form.clientId,
        title:           form.title.trim(),
        matterType:      form.matterType,
        priority:        form.priority,
        forum:           form.forum.trim() || undefined,
        jurisdiction:    form.jurisdiction.trim() || undefined,
        openedDate:      form.openedDate,
        targetCloseDate: form.targetCloseDate || undefined,
        internalNotes:   form.internalNotes.trim() || undefined,
      };
      await keel.matters.create(input);
      onCreated();
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

  return (
    <AnimatePresence>
      {open && (
        <>
          {/* Backdrop */}
          <motion.div
            key="backdrop"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={transition.fast}
            onClick={onClose}
            style={{
              position: 'fixed',
              inset: 0,
              background: 'rgba(44,44,42,0.25)',
              zIndex: 40,
            }}
          />

          {/* Drawer panel */}
          <motion.aside
            key="drawer"
            variants={variants}
            initial="hidden"
            animate="visible"
            exit="exit"
            style={{
              position: 'fixed',
              top: 0,
              right: 0,
              bottom: 0,
              width: 460,
              background: colors.bgPrimary,
              borderLeft: `0.5px solid ${colors.border}`,
              boxShadow: shadows.modal,
              zIndex: 50,
              display: 'flex',
              flexDirection: 'column',
              overflow: 'hidden',
            }}
          >
            {/* Header */}
            <div style={{
              padding: `${spacing[6]} ${spacing[6]} ${spacing[5]}`,
              borderBottom: `0.5px solid ${colors.border}`,
              display: 'flex',
              alignItems: 'flex-start',
              justifyContent: 'space-between',
            }}>
              <div>
                <h2 style={{
                  fontFamily: fonts.display,
                  fontSize: fontSizes.displaySm,
                  fontWeight: 500,
                  color: colors.textPrimary,
                  margin: 0,
                }}>
                  New Matter
                </h2>
                <p style={{
                  fontSize: fontSizes.label,
                  color: colors.textTertiary,
                  margin: '4px 0 0',
                  fontFamily: fonts.ui,
                }}>
                  ID assigned automatically on save
                </p>
              </div>
              <button
                onClick={onClose}
                style={{
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  color: colors.textTertiary,
                  fontSize: 20,
                  lineHeight: 1,
                  padding: 4,
                }}
                aria-label="Close"
              >
                ×
              </button>
            </div>

            {/* Form body */}
            <div style={{
              flex: 1,
              overflowY: 'auto',
              padding: spacing[6],
              display: 'flex',
              flexDirection: 'column',
              gap: spacing[5],
            }}>
              {/* Client */}
              <div>
                <label style={labelStyle}>Client *</label>
                <select
                  value={form.clientId}
                  onChange={e => set('clientId', e.target.value)}
                  style={{ ...inputStyle, cursor: 'pointer' }}
                >
                  <option value="">Select client…</option>
                  {clients.map(c => (
                    <option key={c.id} value={c.id}>{c.name}</option>
                  ))}
                </select>
              </div>

              {/* Title */}
              <div>
                <label style={labelStyle}>Matter title *</label>
                <input
                  type="text"
                  value={form.title}
                  onChange={e => set('title', e.target.value)}
                  placeholder="e.g. Petalveda Floral Blend TM Application"
                  style={inputStyle}
                />
              </div>

              {/* Type + Priority row */}
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: spacing[4] }}>
                <div>
                  <label style={labelStyle}>Matter type</label>
                  <select
                    value={form.matterType}
                    onChange={e => set('matterType', e.target.value as MatterType)}
                    style={{ ...inputStyle, cursor: 'pointer' }}
                  >
                    {MATTER_TYPES.map(t => (
                      <option key={t} value={t}>{t}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <label style={labelStyle}>Priority</label>
                  <select
                    value={form.priority}
                    onChange={e => set('priority', e.target.value as MatterPriority)}
                    style={{ ...inputStyle, cursor: 'pointer' }}
                  >
                    {PRIORITIES.map(p => (
                      <option key={p} value={p}>{p}</option>
                    ))}
                  </select>
                </div>
              </div>

              {/* Forum */}
              <div>
                <label style={labelStyle}>Forum / Registry</label>
                <input
                  type="text"
                  value={form.forum}
                  onChange={e => set('forum', e.target.value)}
                  placeholder="e.g. Trade Marks Registry, Mumbai"
                  style={inputStyle}
                />
              </div>

              {/* Dates row */}
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: spacing[4] }}>
                <div>
                  <label style={labelStyle}>Opened date *</label>
                  <input
                    type="date"
                    value={form.openedDate}
                    onChange={e => set('openedDate', e.target.value)}
                    style={inputStyle}
                  />
                </div>
                <div>
                  <label style={labelStyle}>Target close</label>
                  <input
                    type="date"
                    value={form.targetCloseDate}
                    onChange={e => set('targetCloseDate', e.target.value)}
                    style={inputStyle}
                  />
                </div>
              </div>

              {/* Internal notes */}
              <div>
                <label style={labelStyle}>
                  Internal notes
                  <span style={{
                    marginLeft: 6,
                    fontSize: 10,
                    fontFamily: fonts.mono,
                    color: colors.statusUrgent,
                    fontWeight: 400,
                    letterSpacing: '0.04em',
                  }}>
                    NEVER shared with client
                  </span>
                </label>
                <textarea
                  value={form.internalNotes}
                  onChange={e => set('internalNotes', e.target.value)}
                  rows={3}
                  placeholder="Attorney-only notes…"
                  style={{ ...inputStyle, resize: 'vertical', minHeight: 72 }}
                />
              </div>

              {error && (
                <p style={{
                  fontSize: fontSizes.label,
                  color: colors.statusUrgent,
                  margin: 0,
                  padding: `${spacing[2]} ${spacing[3]}`,
                  background: 'rgba(192,57,43,0.07)',
                  borderRadius: radius.input,
                }}>
                  {error}
                </p>
              )}
            </div>

            {/* Footer */}
            <div style={{
              padding: `${spacing[4]} ${spacing[6]}`,
              borderTop: `0.5px solid ${colors.border}`,
              display: 'flex',
              gap: spacing[3],
              justifyContent: 'flex-end',
            }}>
              <button
                onClick={onClose}
                style={{
                  padding: `${spacing[2]} ${spacing[5]}`,
                  borderRadius: radius.button,
                  border: `0.5px solid ${colors.border}`,
                  background: 'transparent',
                  color: colors.textSecondary,
                  fontSize: fontSizes.body,
                  fontFamily: fonts.ui,
                  cursor: 'pointer',
                }}
              >
                Cancel
              </button>
              <motion.button
                onClick={handleSubmit}
                disabled={saving}
                whileHover={!shouldReduce ? { opacity: 0.88 } : undefined}
                whileTap={!shouldReduce ? { scale: 0.98 } : undefined}
                style={{
                  padding: `${spacing[2]} ${spacing[6]}`,
                  borderRadius: radius.button,
                  border: 'none',
                  background: colors.accentPrimary,
                  color: '#fff',
                  fontSize: fontSizes.body,
                  fontFamily: fonts.ui,
                  fontWeight: 500,
                  cursor: saving ? 'not-allowed' : 'pointer',
                  opacity: saving ? 0.6 : 1,
                }}
              >
                {saving ? 'Saving…' : 'Create matter'}
              </motion.button>
            </div>
          </motion.aside>
        </>
      )}
    </AnimatePresence>
  );
}
