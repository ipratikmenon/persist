/**
 * RenewalDashboard — firm-wide view of what is about to lapse.
 * Accessed via /dockets/renewals
 *
 * Two sections, in order of consequence:
 *   1. Open escalations — statutory deadlines the abandonment watcher has
 *      flagged. L4 means already missed.
 *   2. Upcoming renewals — IP assets whose renewal date falls inside the horizon.
 *
 * Firm-wide on purpose: a renewal missed because nobody happened to open that
 * matter is still a lost right.
 */
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { motion, AnimatePresence, useReducedMotion } from 'motion/react';
import type { Escalation, EscalationLevel, UpcomingRenewal } from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { IpAssetTypePill } from '@/components/dockets/IpAssetStatusBadge';
import { colors, fonts, fontSizes, radius, shadows, spacing } from '@/design-system/tokens';
import { transition } from '@/design-system/motion';

// ---------------------------------------------------------------------------
// Escalation level styling
// ---------------------------------------------------------------------------

const LEVEL_STYLE: Record<EscalationLevel, { bg: string; color: string; label: string }> = {
  1: { bg: 'rgba(212,135,42,0.11)', color: colors.statusWarning,  label: '14 days' },
  2: { bg: 'rgba(212,135,42,0.16)', color: colors.statusWarning,  label: '7 days'  },
  3: { bg: 'rgba(181,96,74,0.14)',  color: colors.accentSecondary, label: '3 days' },
  4: { bg: 'rgba(192,57,43,0.12)',  color: colors.statusUrgent,   label: 'MISSED' },
};

function LevelBadge({ level }: { level: EscalationLevel }) {
  const style = LEVEL_STYLE[level] ?? LEVEL_STYLE[1];
  return (
    <span style={{
      display: 'inline-flex',
      alignItems: 'center',
      padding: '2px 8px',
      borderRadius: radius.chip,
      background: style.bg,
      color: style.color,
      fontSize: fontSizes.label,
      fontFamily: fonts.ui,
      fontWeight: level === 4 ? 600 : 500,
      whiteSpace: 'nowrap',
      letterSpacing: '0.02em',
    }}>
      {style.label}
    </span>
  );
}

/** Days until a date; negative once past. */
function daysUntil(dateStr: string): number {
  const due = new Date(dateStr);
  const today = new Date();
  due.setHours(0, 0, 0, 0);
  today.setHours(0, 0, 0, 0);
  return Math.round((due.getTime() - today.getTime()) / 86_400_000);
}

function formatDate(dateStr: string): string {
  if (!dateStr) return '—';
  return new Date(dateStr).toLocaleDateString('en-IN', {
    day: 'numeric', month: 'short', year: 'numeric',
  });
}

/** Colour a renewal by how close it is. */
function renewalColour(days: number): string {
  if (days < 0)   return colors.statusUrgent;
  if (days <= 30) return colors.accentSecondary;
  if (days <= 90) return colors.statusWarning;
  return colors.textSecondary;
}

// ---------------------------------------------------------------------------
// Escalation row
// ---------------------------------------------------------------------------

interface EscalationRowProps {
  escalation: Escalation;
  onResolved: (id: string) => void;
  onMatterClick: (matterId: string) => void;
}

function EscalationRow({ escalation: e, onResolved, onMatterClick }: EscalationRowProps) {
  const shouldReduce = useReducedMotion();
  const [resolving, setResolving] = useState(false);
  const [action, setAction] = useState('');
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    if (!action.trim()) {
      setError('Describe what was done.');
      return;
    }
    setResolving(true);
    setError(null);
    try {
      await keel.escalations.resolve(e.id, action.trim());
      onResolved(e.id);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
      setResolving(false);
    }
  };

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, x: -8 }}
      transition={transition.fast}
      style={{
        padding: spacing[4],
        borderRadius: radius.card,
        background: e.escalationLevel === 4 ? 'rgba(192,57,43,0.04)' : colors.bgSecondary,
        border: `0.5px solid ${e.escalationLevel === 4 ? colors.statusUrgent : colors.border}`,
        boxShadow: shadows.card,
        marginBottom: spacing[3],
      }}
    >
      <div style={{
        display: 'flex', alignItems: 'flex-start',
        justifyContent: 'space-between', gap: spacing[4],
      }}>
        <div style={{ minWidth: 0 }}>
          <div style={{
            display: 'flex', alignItems: 'center', gap: spacing[2], marginBottom: 4,
          }}>
            <LevelBadge level={e.escalationLevel} />
            <span style={{
              fontFamily: fonts.ui, fontSize: fontSizes.cardTitle,
              fontWeight: 500, color: colors.textPrimary,
            }}>
              {e.docketingEvent}
            </span>
          </div>
          <button
            onClick={() => onMatterClick(e.matterId)}
            style={{
              background: 'none', border: 'none', padding: 0, cursor: 'pointer',
              color: colors.accentPrimary, fontFamily: fonts.ui,
              fontSize: fontSizes.label,
            }}
          >
            {e.matterTitle}
          </button>
          <span style={{
            marginLeft: spacing[3], fontFamily: fonts.ui,
            fontSize: fontSizes.label, color: colors.textSecondary,
          }}>
            due {formatDate(e.dueDate)}
          </span>
        </div>

        {!open && (
          <motion.button
            onClick={() => setOpen(true)}
            whileHover={!shouldReduce ? { opacity: 0.85 } : undefined}
            style={{
              padding: '4px 12px', borderRadius: radius.button,
              border: `0.5px solid ${colors.border}`, background: 'transparent',
              color: colors.textSecondary, fontFamily: fonts.ui,
              fontSize: fontSizes.label, cursor: 'pointer', flexShrink: 0,
            }}
          >
            Resolve
          </motion.button>
        )}
      </div>

      {open && (
        <div style={{ marginTop: spacing[3] }}>
          <input
            value={action}
            onChange={ev => setAction(ev.target.value)}
            placeholder="What was done? e.g. Response filed 12 Aug, receipt on file"
            style={{
              width: '100%', padding: '8px 10px', borderRadius: radius.input,
              border: `0.5px solid ${colors.border}`, background: colors.bgPrimary,
              color: colors.textPrimary, fontFamily: fonts.ui,
              fontSize: fontSizes.body, outline: 'none',
              boxSizing: 'border-box' as const,
            }}
          />
          {error && (
            <p style={{
              margin: `${spacing[2]} 0 0`, fontSize: fontSizes.label,
              color: colors.statusUrgent, fontFamily: fonts.ui,
            }}>
              {error}
            </p>
          )}
          <div style={{
            display: 'flex', gap: spacing[2], justifyContent: 'flex-end',
            marginTop: spacing[3],
          }}>
            <button
              onClick={() => { setOpen(false); setError(null); }}
              style={{
                padding: '5px 12px', borderRadius: radius.button,
                border: `0.5px solid ${colors.border}`, background: 'transparent',
                color: colors.textSecondary, fontFamily: fonts.ui,
                fontSize: fontSizes.label, cursor: 'pointer',
              }}
            >
              Cancel
            </button>
            <motion.button
              onClick={submit}
              disabled={resolving}
              whileHover={!shouldReduce && !resolving ? { opacity: 0.88 } : undefined}
              style={{
                padding: '5px 14px', borderRadius: radius.button, border: 'none',
                background: colors.accentPrimary, color: '#fff',
                fontFamily: fonts.ui, fontSize: fontSizes.label, fontWeight: 500,
                cursor: resolving ? 'not-allowed' : 'pointer',
                opacity: resolving ? 0.7 : 1,
              }}
            >
              {resolving ? 'Saving…' : 'Record'}
            </motion.button>
          </div>
        </div>
      )}
    </motion.div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const HORIZONS = [90, 180, 365, 730] as const;

export default function RenewalDashboard() {
  const navigate = useNavigate();
  const shouldReduce = useReducedMotion();

  const [escalations, setEscalations] = useState<Escalation[]>([]);
  const [renewals, setRenewals] = useState<UpcomingRenewal[]>([]);
  const [horizon, setHorizon] = useState<number>(365);
  const [loading, setLoading] = useState(true);

  const load = async (days: number) => {
    setLoading(true);
    try {
      const [esc, ren] = await Promise.all([
        keel.escalations.list(false),
        keel.ipAssets.upcomingRenewals(days),
      ]);
      setEscalations(esc);
      setRenewals(ren);
    } catch {
      setEscalations([]);
      setRenewals([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(horizon); }, [horizon]);

  const handleResolved = (id: string) =>
    setEscalations(prev => prev.filter(e => e.id !== id));

  const missedCount = escalations.filter(e => e.escalationLevel === 4).length;

  if (loading) {
    return (
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        height: '100%', color: colors.textTertiary, fontFamily: fonts.ui,
      }}>
        Loading…
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      {/* Header */}
      <header style={{
        padding: `${spacing[5]} ${spacing[8]} ${spacing[4]}`,
        borderBottom: `0.5px solid ${colors.border}`,
        flexShrink: 0,
      }}>
        <h1 style={{
          fontFamily: fonts.display, fontSize: fontSizes.displayLg, fontWeight: 600,
          color: colors.textPrimary, margin: '0 0 8px', lineHeight: 1.25,
        }}>
          Renewals &amp; Escalations
        </h1>
        <div style={{
          display: 'flex', alignItems: 'center', gap: spacing[3],
          fontSize: fontSizes.label, fontFamily: fonts.ui, color: colors.textSecondary,
        }}>
          <span>{escalations.length} open escalation{escalations.length === 1 ? '' : 's'}</span>
          {missedCount > 0 && (
            <span style={{ color: colors.statusUrgent, fontWeight: 500 }}>
              · {missedCount} MISSED
            </span>
          )}
          <span>· {renewals.length} renewal{renewals.length === 1 ? '' : 's'} in view</span>
        </div>
      </header>

      <div style={{ flex: 1, overflow: 'auto', padding: `${spacing[6]} ${spacing[8]}`, maxWidth: 860 }}>

        {/* Escalations */}
        <section style={{ marginBottom: spacing[10] }}>
          <div style={{
            fontSize: fontSizes.label, fontWeight: 500, color: colors.textTertiary,
            textTransform: 'uppercase' as const, letterSpacing: '0.06em',
            marginBottom: spacing[3],
          }}>
            Open escalations
          </div>

          {escalations.length === 0 ? (
            <div style={{
              padding: spacing[6], borderRadius: radius.card,
              background: colors.bgSecondary, border: `0.5px solid ${colors.border}`,
              color: colors.textSecondary, fontFamily: fonts.ui,
              fontSize: fontSizes.body, textAlign: 'center' as const,
            }}>
              No statutory deadline is currently escalated.
            </div>
          ) : (
            <AnimatePresence mode="popLayout">
              {escalations.map(e => (
                <EscalationRow
                  key={e.id}
                  escalation={e}
                  onResolved={handleResolved}
                  onMatterClick={id => navigate(`/dockets/${id}`)}
                />
              ))}
            </AnimatePresence>
          )}
        </section>

        {/* Renewals */}
        <section>
          <div style={{
            display: 'flex', alignItems: 'center', justifyContent: 'space-between',
            marginBottom: spacing[3],
          }}>
            <div style={{
              fontSize: fontSizes.label, fontWeight: 500, color: colors.textTertiary,
              textTransform: 'uppercase' as const, letterSpacing: '0.06em',
            }}>
              Upcoming renewals
            </div>
            <div style={{ display: 'flex', gap: spacing[2] }}>
              {HORIZONS.map(h => (
                <button
                  key={h}
                  onClick={() => setHorizon(h)}
                  style={{
                    padding: '3px 10px', borderRadius: radius.chip,
                    border: `0.5px solid ${horizon === h ? colors.accentPrimary : colors.border}`,
                    background: horizon === h ? 'rgba(74,101,128,0.09)' : 'transparent',
                    color: horizon === h ? colors.accentPrimary : colors.textSecondary,
                    fontFamily: fonts.ui, fontSize: fontSizes.label,
                    fontWeight: horizon === h ? 500 : 400, cursor: 'pointer',
                  }}
                >
                  {h < 365 ? `${h}d` : `${h / 365}y`}
                </button>
              ))}
            </div>
          </div>

          {renewals.length === 0 ? (
            <div style={{
              padding: spacing[6], borderRadius: radius.card,
              background: colors.bgSecondary, border: `0.5px solid ${colors.border}`,
              color: colors.textSecondary, fontFamily: fonts.ui,
              fontSize: fontSizes.body, textAlign: 'center' as const,
            }}>
              No renewals due in this window.
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: spacing[2] }}>
              {renewals.map(r => {
                const days = daysUntil(r.expiryDate);
                return (
                  <motion.div
                    key={r.id}
                    layout
                    whileHover={!shouldReduce ? { y: -2 } : undefined}
                    transition={transition.fast}
                    onClick={() => navigate(`/dockets/${r.matterId}`)}
                    style={{
                      display: 'flex', alignItems: 'center', gap: spacing[4],
                      padding: spacing[4], borderRadius: radius.card,
                      background: colors.bgSecondary,
                      border: `0.5px solid ${colors.border}`,
                      boxShadow: shadows.card, cursor: 'pointer',
                    }}
                  >
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{
                        display: 'flex', alignItems: 'center', gap: spacing[2], marginBottom: 3,
                      }}>
                        <span style={{
                          fontFamily: fonts.ui, fontSize: fontSizes.cardTitle,
                          fontWeight: 500, color: colors.textPrimary,
                        }}>
                          {r.title}
                        </span>
                        <IpAssetTypePill assetType={r.assetType} />
                      </div>
                      <div style={{
                        fontSize: fontSizes.label, fontFamily: fonts.ui,
                        color: colors.textSecondary,
                      }}>
                        {r.clientName} · {r.matterTitle}
                        {r.registrationNumber && (
                          <span style={{ fontFamily: fonts.mono, marginLeft: spacing[2] }}>
                            {r.registrationNumber}
                          </span>
                        )}
                      </div>
                    </div>

                    <div style={{ textAlign: 'right' as const, flexShrink: 0 }}>
                      <div style={{
                        fontFamily: fonts.ui, fontSize: fontSizes.body,
                        fontWeight: 500, color: renewalColour(days),
                      }}>
                        {formatDate(r.expiryDate)}
                      </div>
                      <div style={{
                        fontSize: fontSizes.label, fontFamily: fonts.ui,
                        color: renewalColour(days),
                      }}>
                        {days < 0 ? `${Math.abs(days)} days overdue` : `in ${days} days`}
                      </div>
                    </div>
                  </motion.div>
                );
              })}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
