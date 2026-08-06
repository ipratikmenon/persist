/**
 * CascadePreview — generate a statutory deadline chain from a template.
 *
 * Preview-then-commit by design (spec §2.9): the attorney sees the whole chain,
 * and the date the template was last checked against the Act, before anything is
 * written. Silently creating nineteen annuity deadlines from a mistyped anchor
 * date would be a genuine mess to unpick.
 */
import { useEffect, useState } from 'react';
import { motion, AnimatePresence, useReducedMotion } from 'motion/react';
import type { CascadePreview as Preview, IpAsset } from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { colors, fonts, fontSizes, radius, shadows, spacing } from '@/design-system/tokens';
import { transition, panelVariants } from '@/design-system/motion';

/**
 * Anchor event names are stored PascalCase with acronyms ('TMApplication').
 * Split the acronym boundary first, then the ordinary word boundaries, or
 * 'TMApplication' comes out unchanged.
 */
function humanise(anchor: string): string {
  return anchor
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')  // TMApplication -> TM Application
    .replace(/([a-z])([A-Z])/g, '$1 $2')        // PatentFER     -> Patent FER
    .replace(/\bTM\b/, 'Trademark')
    .replace(/\bFER\b/, 'First Examination Report');
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

const labelStyle = {
  display: 'block',
  fontSize: fontSizes.label,
  fontWeight: 500,
  color: colors.textSecondary,
  fontFamily: fonts.ui,
  marginBottom: 5,
};

interface CascadePreviewDrawerProps {
  open: boolean;
  matterId: string;
  asset: IpAsset | null;
  onClose: () => void;
  onGenerated: () => void;
}

export function CascadePreviewDrawer({
  open, matterId, asset, onClose, onGenerated,
}: CascadePreviewDrawerProps) {
  const shouldReduce = useReducedMotion();

  const [anchors, setAnchors]   = useState<string[]>([]);
  const [anchor, setAnchor]     = useState('');
  const [date, setDate]         = useState('');
  const [preview, setPreview]   = useState<Preview | null>(null);
  const [loading, setLoading]   = useState(false);
  const [generating, setGen]    = useState(false);
  const [error, setError]       = useState<string | null>(null);

  // Reset and load available anchors whenever the drawer opens.
  useEffect(() => {
    if (!open || !asset) return;
    setPreview(null);
    setError(null);
    setAnchor('');
    // Sensible default: the asset's own filing date, which is what most chains
    // are anchored to.
    setDate(asset.filingDate ?? '');

    keel.cascade.listAnchors(asset.assetType)
      .then(setAnchors)
      .catch(() => setAnchors([]));
  }, [open, asset]);

  const runPreview = async () => {
    if (!asset || !anchor || !date) {
      setError('Choose an anchor event and the date it occurred.');
      return;
    }
    setLoading(true);
    setError(null);
    setPreview(null);
    try {
      const p = await keel.cascade.preview({
        matterId, ipAssetId: asset.id, eventType: anchor, anchorDate: date,
      });
      setPreview(p);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  const commit = async () => {
    if (!asset || !preview) return;
    setGen(true);
    setError(null);
    try {
      await keel.cascade.generate({
        matterId, ipAssetId: asset.id, eventType: anchor, anchorDate: date,
      });
      onGenerated();
      onClose();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
      setGen(false);
    }
  };

  const variants = shouldReduce
    ? { hidden: { opacity: 0 }, visible: { opacity: 1 }, exit: { opacity: 0 } }
    : panelVariants.right;

  return (
    <AnimatePresence>
      {open && asset && (
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
              position: 'fixed', top: 0, right: 0, bottom: 0, width: 480,
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
              display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start',
              flexShrink: 0,
            }}>
              <div>
                <h2 style={{
                  fontFamily: fonts.display, fontSize: 18, fontWeight: 500,
                  margin: '0 0 3px', color: colors.textPrimary,
                }}>
                  Generate deadline chain
                </h2>
                <div style={{
                  fontFamily: fonts.ui, fontSize: fontSizes.label, color: colors.textSecondary,
                }}>
                  {asset.title}
                </div>
              </div>
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
                  <label style={labelStyle}>Anchor event</label>
                  {anchors.length === 0 ? (
                    <p style={{
                      margin: 0, fontFamily: fonts.ui, fontSize: fontSizes.label,
                      color: colors.textTertiary,
                    }}>
                      No cascade template exists for {asset.assetType} in {asset.jurisdiction}.
                    </p>
                  ) : (
                    <div style={{ display: 'flex', flexWrap: 'wrap', gap: spacing[2] }}>
                      {anchors.map(a => (
                        <button
                          key={a}
                          onClick={() => { setAnchor(a); setPreview(null); }}
                          style={{
                            padding: '5px 11px',
                            borderRadius: radius.chip,
                            border: `0.5px solid ${anchor === a ? colors.accentPrimary : colors.border}`,
                            background: anchor === a ? 'rgba(74,101,128,0.09)' : 'transparent',
                            color: anchor === a ? colors.accentPrimary : colors.textSecondary,
                            fontFamily: fonts.ui, fontSize: fontSizes.label,
                            fontWeight: anchor === a ? 500 : 400, cursor: 'pointer',
                          }}
                        >
                          {humanise(a)}
                        </button>
                      ))}
                    </div>
                  )}
                </div>

                <div>
                  <label style={labelStyle}>Date the event occurred</label>
                  <input
                    type="date"
                    value={date}
                    onChange={e => { setDate(e.target.value); setPreview(null); }}
                    style={inputStyle}
                  />
                  <p style={{
                    fontSize: 11, color: colors.textTertiary, fontFamily: fonts.ui,
                    margin: `${spacing[1]} 0 0`,
                  }}>
                    Every date in the chain is computed from this. Registry events use
                    the date on the notice, not the date it arrived.
                  </p>
                </div>

                <motion.button
                  onClick={runPreview}
                  disabled={loading || !anchor || !date}
                  whileHover={!shouldReduce && anchor && date ? { opacity: 0.88 } : undefined}
                  style={{
                    padding: `${spacing[2]} ${spacing[5]}`,
                    borderRadius: radius.button,
                    border: `0.5px solid ${colors.accentPrimary}`,
                    background: 'transparent', color: colors.accentPrimary,
                    fontFamily: fonts.ui, fontSize: fontSizes.body, fontWeight: 500,
                    cursor: loading || !anchor || !date ? 'not-allowed' : 'pointer',
                    opacity: loading || !anchor || !date ? 0.5 : 1,
                    alignSelf: 'flex-start',
                  }}
                >
                  {loading ? 'Working…' : 'Preview chain'}
                </motion.button>

                {error && (
                  <div style={{
                    padding: spacing[3], borderRadius: radius.input,
                    background: 'rgba(192,57,43,0.08)', color: colors.statusUrgent,
                    fontFamily: fonts.ui, fontSize: fontSizes.label, lineHeight: 1.5,
                  }}>
                    {error}
                  </div>
                )}

                {preview && (
                  <div>
                    <div style={{
                      display: 'flex', alignItems: 'baseline',
                      justifyContent: 'space-between', marginBottom: spacing[3],
                    }}>
                      <span style={{
                        fontSize: fontSizes.label, fontWeight: 500, color: colors.textTertiary,
                        textTransform: 'uppercase' as const, letterSpacing: '0.06em',
                      }}>
                        {preview.deadlines.length} deadline{preview.deadlines.length === 1 ? '' : 's'}
                      </span>
                      {/* Staleness must be visible, not buried in the DB. */}
                      <span style={{
                        fontFamily: fonts.ui, fontSize: 11, color: colors.textTertiary,
                      }}>
                        rules verified {preview.lastVerified}
                      </span>
                    </div>

                    {preview.templateNotes && (
                      <p style={{
                        margin: `0 0 ${spacing[4]}`, padding: spacing[3],
                        borderRadius: radius.input, background: colors.bgTertiary,
                        fontFamily: fonts.ui, fontSize: fontSizes.label,
                        color: colors.textSecondary, lineHeight: 1.5,
                      }}>
                        {preview.templateNotes}
                      </p>
                    )}

                    <div style={{ display: 'flex', flexDirection: 'column', gap: spacing[2] }}>
                      {preview.deadlines.map((d, i) => (
                        <div
                          key={`${d.dueDate}-${i}`}
                          style={{
                            display: 'flex', alignItems: 'center', gap: spacing[3],
                            padding: `${spacing[2]} ${spacing[3]}`,
                            borderRadius: radius.input,
                            background: d.isInternalBuffer ? 'transparent' : colors.bgSecondary,
                            border: `0.5px solid ${d.isInternalBuffer ? colors.border : 'transparent'}`,
                          }}
                        >
                          <span style={{
                            fontFamily: fonts.mono, fontSize: fontSizes.mono,
                            color: colors.textSecondary, flexShrink: 0, width: 88,
                          }}>
                            {d.dueDate}
                          </span>
                          <span style={{
                            flex: 1, fontFamily: fonts.ui, fontSize: fontSizes.label,
                            color: d.isInternalBuffer ? colors.textTertiary : colors.textPrimary,
                          }}>
                            {d.docketingEvent}
                          </span>
                          {d.isClientVisible && (
                            <span
                              title="Visible to the client in the portal"
                              style={{
                                fontFamily: fonts.ui, fontSize: 10,
                                color: colors.accentPrimary, flexShrink: 0,
                              }}
                            >
                              client
                            </span>
                          )}
                        </div>
                      ))}
                    </div>
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
                  padding: `${spacing[2]} ${spacing[4]}`, borderRadius: radius.button,
                  border: `0.5px solid ${colors.border}`, background: 'transparent',
                  color: colors.textSecondary, fontFamily: fonts.ui,
                  fontSize: fontSizes.body, cursor: 'pointer',
                }}
              >
                Cancel
              </button>
              <motion.button
                onClick={commit}
                disabled={!preview || generating}
                whileHover={!shouldReduce && preview && !generating ? { opacity: 0.88 } : undefined}
                whileTap={!shouldReduce && preview && !generating ? { scale: 0.97 } : undefined}
                style={{
                  padding: `${spacing[2]} ${spacing[5]}`, borderRadius: radius.button,
                  border: 'none', background: colors.accentPrimary, color: '#fff',
                  fontFamily: fonts.ui, fontSize: fontSizes.body, fontWeight: 500,
                  cursor: !preview || generating ? 'not-allowed' : 'pointer',
                  opacity: !preview || generating ? 0.5 : 1,
                }}
              >
                {generating
                  ? 'Creating…'
                  : preview
                    ? `Create ${preview.deadlines.length} deadline${preview.deadlines.length === 1 ? '' : 's'}`
                    : 'Create deadlines'}
              </motion.button>
            </div>
          </motion.aside>
        </>
      )}
    </AnimatePresence>
  );
}
