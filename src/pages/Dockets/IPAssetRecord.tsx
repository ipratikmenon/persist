/**
 * IPAssetRecord — per-matter docket timeline.
 * Accessed via /dockets/:matterId
 * Shows all deadlines for one matter as a timeline sorted by due date,
 * with inline "mark complete", add deadline, and statutory template generation.
 */
import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router';
import { motion, AnimatePresence } from 'motion/react';
import { useReducedMotion } from 'motion/react';
import type { Deadline, Matter, MatterType, StatutoryTemplate } from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { UrgencyBadge, DueDate } from '@/components/dockets/UrgencyBadge';
import { MatterStatusBadge, MatterTypePill } from '@/components/matters/MatterStatusBadge';
import { colors, fonts, fontSizes, radius, shadows, spacing } from '@/design-system/tokens';
import { transition, panelVariants } from '@/design-system/motion';

// ---------------------------------------------------------------------------
// Add deadline drawer (minimal — full form in a later pass)
// ---------------------------------------------------------------------------

interface AddDeadlineDrawerProps {
  open: boolean;
  matterId: string;
  matterType: MatterType;
  onClose: () => void;
  onAdded: () => void;
}

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

function AddDeadlineDrawer({ open, matterId, matterType, onClose, onAdded }: AddDeadlineDrawerProps) {
  const shouldReduce = useReducedMotion();
  const [tab, setTab] = useState<'custom' | 'templates'>('custom');
  const [templates, setTemplates] = useState<StatutoryTemplate[]>([]);
  const [event, setEvent] = useState('');
  const [dueDate, setDueDate] = useState('');
  const [eventType, setEventType] = useState<'Statutory' | 'Procedural' | 'Custom'>('Custom');
  const [notes, setNotes] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setEvent(''); setDueDate(''); setNotes(''); setError(null);
      keel.deadlines.getTemplates(matterType).then(setTemplates).catch(() => setTemplates([]));
    }
  }, [open, matterType]);

  const save = async () => {
    if (!event.trim() || !dueDate) {
      setError('Event name and due date are required.');
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await keel.deadlines.create({ matterId, docketingEvent: event.trim(), eventType, dueDate, notes: notes.trim() || undefined });
      onAdded();
      onClose();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const applyTemplate = (t: StatutoryTemplate) => {
    setEvent(t.event);
    setEventType(t.eventType as 'Statutory' | 'Procedural');
    setNotes(t.description);
    setTab('custom');
  };

  const variants = shouldReduce
    ? { hidden: { opacity: 0 }, visible: { opacity: 1 }, exit: { opacity: 0 } }
    : panelVariants.right;

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
            }}>
              <h2 style={{ fontFamily: fonts.display, fontSize: 18, fontWeight: 500, margin: 0, color: colors.textPrimary }}>
                Add Deadline
              </h2>
              <button onClick={onClose} style={{ background: 'none', border: 'none', cursor: 'pointer', color: colors.textTertiary, fontSize: 20 }}>×</button>
            </div>

            {/* Tab switcher */}
            <div style={{ display: 'flex', borderBottom: `0.5px solid ${colors.border}`, padding: `0 ${spacing[6]}` }}>
              {(['custom', 'templates'] as const).map(t => (
                <button key={t} onClick={() => setTab(t)} style={{
                  padding: `${spacing[3]} 0`, marginRight: spacing[5], border: 'none',
                  borderBottom: tab === t ? `2px solid ${colors.accentPrimary}` : '2px solid transparent',
                  background: 'transparent', cursor: 'pointer',
                  fontSize: fontSizes.body, fontFamily: fonts.ui, fontWeight: tab === t ? 500 : 400,
                  color: tab === t ? colors.accentPrimary : colors.textSecondary,
                }}>
                  {t === 'custom' ? 'Custom' : `Templates (${templates.length})`}
                </button>
              ))}
            </div>

            <div style={{ flex: 1, overflowY: 'auto', padding: spacing[6] }}>
              {tab === 'templates' ? (
                <div style={{ display: 'flex', flexDirection: 'column', gap: spacing[3] }}>
                  {templates.length === 0 && (
                    <p style={{ color: colors.textTertiary, fontSize: fontSizes.body }}>No templates for this matter type.</p>
                  )}
                  {templates.map((t, i) => (
                    <motion.button key={i} onClick={() => applyTemplate(t)}
                      whileHover={!shouldReduce ? { x: 2 } : undefined}
                      style={{
                        textAlign: 'left', padding: spacing[4],
                        background: colors.bgSecondary,
                        border: `0.5px solid ${colors.border}`,
                        borderRadius: radius.card, cursor: 'pointer',
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: spacing[2], marginBottom: 4 }}>
                        <span style={{ fontSize: fontSizes.body, fontWeight: 500, color: colors.textPrimary, fontFamily: fonts.ui }}>
                          {t.event}
                        </span>
                        <span style={{
                          fontSize: 10, fontFamily: fonts.mono, letterSpacing: '0.05em',
                          color: t.eventType === 'Statutory' ? colors.accentSecondary : colors.textTertiary,
                          textTransform: 'uppercase' as const,
                        }}>
                          {t.eventType}
                        </span>
                      </div>
                      <div style={{ fontSize: fontSizes.label, color: colors.textSecondary, fontFamily: fonts.ui }}>{t.description}</div>
                    </motion.button>
                  ))}
                </div>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: spacing[5] }}>
                  <div>
                    <label style={{ display: 'block', fontSize: fontSizes.label, fontWeight: 500, color: colors.textSecondary, marginBottom: 5, fontFamily: fonts.ui }}>Event name *</label>
                    <input type="text" value={event} onChange={e => setEvent(e.target.value)} placeholder="e.g. Examination Report Response" style={inputStyle} />
                  </div>
                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: spacing[4] }}>
                    <div>
                      <label style={{ display: 'block', fontSize: fontSizes.label, fontWeight: 500, color: colors.textSecondary, marginBottom: 5, fontFamily: fonts.ui }}>Due date *</label>
                      <input type="date" value={dueDate} onChange={e => setDueDate(e.target.value)} style={inputStyle} />
                    </div>
                    <div>
                      <label style={{ display: 'block', fontSize: fontSizes.label, fontWeight: 500, color: colors.textSecondary, marginBottom: 5, fontFamily: fonts.ui }}>Type</label>
                      <select value={eventType} onChange={e => setEventType(e.target.value as typeof eventType)} style={{ ...inputStyle, cursor: 'pointer' }}>
                        <option value="Statutory">Statutory</option>
                        <option value="Procedural">Procedural</option>
                        <option value="Custom">Custom</option>
                      </select>
                    </div>
                  </div>
                  <div>
                    <label style={{ display: 'block', fontSize: fontSizes.label, fontWeight: 500, color: colors.textSecondary, marginBottom: 5, fontFamily: fonts.ui }}>Notes</label>
                    <textarea value={notes} onChange={e => setNotes(e.target.value)} rows={3} style={{ ...inputStyle, resize: 'vertical' }} />
                  </div>
                  {error && <p style={{ fontSize: fontSizes.label, color: colors.statusUrgent, margin: 0 }}>{error}</p>}
                </div>
              )}
            </div>

            {tab === 'custom' && (
              <div style={{ padding: `${spacing[4]} ${spacing[6]}`, borderTop: `0.5px solid ${colors.border}`, display: 'flex', gap: spacing[3], justifyContent: 'flex-end' }}>
                <button onClick={onClose} style={{ padding: `${spacing[2]} ${spacing[5]}`, borderRadius: radius.button, border: `0.5px solid ${colors.border}`, background: 'transparent', color: colors.textSecondary, fontSize: fontSizes.body, fontFamily: fonts.ui, cursor: 'pointer' }}>Cancel</button>
                <motion.button onClick={save} disabled={saving}
                  whileHover={!shouldReduce ? { opacity: 0.88 } : undefined}
                  style={{ padding: `${spacing[2]} ${spacing[5]}`, borderRadius: radius.button, border: 'none', background: colors.accentPrimary, color: '#fff', fontSize: fontSizes.body, fontFamily: fonts.ui, fontWeight: 500, cursor: saving ? 'not-allowed' : 'pointer', opacity: saving ? 0.6 : 1 }}>
                  {saving ? 'Saving…' : 'Add deadline'}
                </motion.button>
              </div>
            )}
          </motion.aside>
        </>
      )}
    </AnimatePresence>
  );
}

// ---------------------------------------------------------------------------
// Timeline row
// ---------------------------------------------------------------------------

interface TimelineRowProps {
  deadline: Deadline;
  onComplete: (id: string) => void;
}

function TimelineRow({ deadline: d, onComplete }: TimelineRowProps) {
  const shouldReduce = useReducedMotion();
  const [completing, setCompleting] = useState(false);

  const handleComplete = async () => {
    setCompleting(true);
    try {
      await keel.deadlines.markComplete(d.id, '');
      onComplete(d.id);
    } finally {
      setCompleting(false);
    }
  };

  const isDone = d.status !== 'Pending';

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: isDone ? 0.5 : 1, y: 0 }}
      transition={transition.fast}
      style={{
        display: 'flex',
        gap: spacing[4],
        padding: `${spacing[4]} 0`,
        borderBottom: `0.5px solid ${colors.border}`,
        alignItems: 'flex-start',
      }}
    >
      {/* Timeline dot + line */}
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', paddingTop: 4, width: 20, flexShrink: 0 }}>
        <div style={{
          width: 10, height: 10, borderRadius: '50%', flexShrink: 0,
          background: isDone ? colors.statusClear : (
            d.urgency === 'Overdue' ? colors.statusUrgent :
            d.urgency === 'Critical' ? colors.accentSecondary :
            d.urgency === 'Warning' ? colors.statusWarning :
            colors.border
          ),
          border: `2px solid ${isDone ? colors.statusClear : colors.border}`,
        }} />
      </div>

      {/* Content */}
      <div style={{ flex: 1 }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: spacing[4] }}>
          <div>
            <span style={{
              fontSize: fontSizes.cardTitle, fontWeight: 500,
              color: isDone ? colors.textTertiary : colors.textPrimary,
              fontFamily: fonts.ui,
              textDecoration: isDone ? 'line-through' : 'none',
            }}>
              {d.docketingEvent}
            </span>
            {d.eventType !== 'Custom' && (
              <span style={{
                marginLeft: spacing[2], fontSize: 10, fontFamily: fonts.mono,
                color: d.eventType === 'Statutory' ? colors.accentSecondary : colors.textTertiary,
                letterSpacing: '0.05em', textTransform: 'uppercase' as const,
              }}>
                {d.eventType}
              </span>
            )}
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: spacing[3], flexShrink: 0 }}>
            {!isDone && <UrgencyBadge urgency={d.urgency as any} />}
            {isDone
              ? <span style={{ fontSize: fontSizes.label, color: colors.statusClear, fontFamily: fonts.ui }}>✓ {d.status}</span>
              : (
                <motion.button onClick={handleComplete} disabled={completing}
                  whileHover={!shouldReduce ? { opacity: 0.8 } : undefined}
                  style={{ padding: '3px 10px', borderRadius: radius.button, border: `0.5px solid ${colors.statusClear}`, background: 'transparent', color: colors.statusClear, fontSize: fontSizes.label, fontFamily: fonts.ui, cursor: completing ? 'not-allowed' : 'pointer' }}>
                  {completing ? '…' : '✓ Done'}
                </motion.button>
              )
            }
          </div>
        </div>

        <div style={{ marginTop: 3 }}>
          <DueDate dueDate={d.dueDate} urgency={isDone ? 'Normal' : d.urgency as any} />
        </div>

        {d.notes && (
          <p style={{ fontSize: fontSizes.label, color: colors.textTertiary, fontFamily: fonts.ui, margin: `${spacing[1]} 0 0`, lineHeight: 1.5 }}>
            {d.notes}
          </p>
        )}
      </div>
    </motion.div>
  );
}

// ---------------------------------------------------------------------------
// IPAssetRecord page
// ---------------------------------------------------------------------------

export default function IPAssetRecord() {
  const { matterId } = useParams<{ matterId: string }>();
  const navigate = useNavigate();
  const shouldReduce = useReducedMotion();

  const [matter, setMatter] = useState<Matter | null>(null);
  const [deadlines, setDeadlines] = useState<Deadline[]>([]);
  const [loading, setLoading] = useState(true);
  const [drawerOpen, setDrawerOpen] = useState(false);

  const load = async () => {
    if (!matterId) return;
    setLoading(true);
    try {
      const [m, dl] = await Promise.all([
        keel.matters.get(matterId),
        keel.deadlines.list(matterId),
      ]);
      setMatter(m);
      setDeadlines(dl);
    } catch {
      navigate('/dockets');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, [matterId]);

  const handleComplete = (id: string) => {
    setDeadlines(prev => prev.map(d => d.id === id ? { ...d, status: 'Complete', urgency: 'Normal' } : d));
  };

  if (loading || !matter) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: colors.textTertiary, fontFamily: fonts.ui }}>
        Loading…
      </div>
    );
  }

  const pending = deadlines.filter(d => d.status === 'Pending');
  const done    = deadlines.filter(d => d.status !== 'Pending');

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      {/* Header */}
      <header style={{
        padding: `${spacing[5]} ${spacing[8]} ${spacing[4]}`,
        borderBottom: `0.5px solid ${colors.border}`,
        flexShrink: 0,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: spacing[4], marginBottom: spacing[3] }}>
          <button onClick={() => navigate('/dockets')} style={{ background: 'none', border: 'none', cursor: 'pointer', color: colors.textTertiary, fontFamily: fonts.ui, fontSize: fontSizes.body, padding: 0 }}>
            ← Dockets
          </button>
          <button onClick={() => navigate(`/matters/${matter.id}`)} style={{ background: 'none', border: 'none', cursor: 'pointer', color: colors.accentPrimary, fontFamily: fonts.mono, fontSize: fontSizes.label, padding: 0 }}>
            {matter.id}
          </button>
        </div>

        <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between' }}>
          <div>
            <h1 style={{ fontFamily: fonts.display, fontSize: fontSizes.displayLg, fontWeight: 600, color: colors.textPrimary, margin: '0 0 8px', lineHeight: 1.25 }}>
              {matter.title}
            </h1>
            <div style={{ display: 'flex', gap: spacing[3], alignItems: 'center' }}>
              <MatterTypePill matterType={matter.matterType} />
              <MatterStatusBadge status={matter.status as any} />
              <span style={{ fontSize: fontSizes.label, color: colors.textTertiary, fontFamily: fonts.ui }}>
                {pending.length} pending · {done.length} completed
              </span>
            </div>
          </div>

          <motion.button
            onClick={() => setDrawerOpen(true)}
            whileHover={!shouldReduce ? { opacity: 0.88 } : undefined}
            whileTap={!shouldReduce ? { scale: 0.97 } : undefined}
            style={{ padding: `${spacing[2]} ${spacing[5]}`, borderRadius: radius.button, border: 'none', background: colors.accentPrimary, color: '#fff', fontFamily: fonts.ui, fontSize: fontSizes.body, fontWeight: 500, cursor: 'pointer', flexShrink: 0 }}
          >
            + Add deadline
          </motion.button>
        </div>
      </header>

      {/* Timeline */}
      <div style={{ flex: 1, overflow: 'auto', padding: `${spacing[6]} ${spacing[8]}`, maxWidth: 720 }}>
        {deadlines.length === 0 ? (
          <div style={{ textAlign: 'center', paddingTop: spacing[10], color: colors.textTertiary, fontFamily: fonts.ui }}>
            <div style={{ fontFamily: fonts.display, fontSize: fontSizes.displaySm, marginBottom: spacing[3] }}>No deadlines yet</div>
            <p style={{ fontSize: fontSizes.body, margin: 0 }}>Add a deadline or generate from statutory templates.</p>
          </div>
        ) : (
          <>
            {pending.map(d => (
              <TimelineRow key={d.id} deadline={d} onComplete={handleComplete} />
            ))}
            {done.length > 0 && (
              <>
                <div style={{ fontSize: fontSizes.label, fontWeight: 500, color: colors.textTertiary, textTransform: 'uppercase' as const, letterSpacing: '0.06em', margin: `${spacing[6]} 0 ${spacing[3]}` }}>
                  Completed
                </div>
                {done.map(d => (
                  <TimelineRow key={d.id} deadline={d} onComplete={handleComplete} />
                ))}
              </>
            )}
          </>
        )}
      </div>

      <AddDeadlineDrawer
        open={drawerOpen}
        matterId={matter.id}
        matterType={matter.matterType as MatterType}
        onClose={() => setDrawerOpen(false)}
        onAdded={load}
      />
    </div>
  );
}
