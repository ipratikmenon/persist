import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { motion, AnimatePresence } from 'motion/react';
import { useReducedMotion } from 'motion/react';
import type { DeadlineSummary, UrgencyTier } from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { UrgencyBadge, DueDate } from '@/components/dockets/UrgencyBadge';
import { MatterTypePill } from '@/components/matters/MatterStatusBadge';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';
import { transition, stagger } from '@/design-system/motion';

// ---------------------------------------------------------------------------
// Urgency filter chips
// ---------------------------------------------------------------------------

const URGENCY_CHIPS: { value: UrgencyTier | 'All'; label: string }[] = [
  { value: 'All',      label: 'All pending' },
  { value: 'Overdue',  label: 'Overdue'     },
  { value: 'Critical', label: 'Critical'    },
  { value: 'Warning',  label: 'Warning'     },
  { value: 'Normal',   label: 'On track'    },
];

// ---------------------------------------------------------------------------
// Row component
// ---------------------------------------------------------------------------

const rowVariants = {
  hidden:  { opacity: 0, y: 6 },
  visible: { opacity: 1, y: 0, transition: transition.fast },
};

interface DocketRowProps {
  deadline: DeadlineSummary;
  onComplete: (id: string) => void;
  onMatterClick: (matterId: string) => void;
}

function DocketRow({ deadline: d, onComplete, onMatterClick }: DocketRowProps) {
  const shouldReduce = useReducedMotion();
  const [hovered, setHovered] = useState(false);
  const [completing, setCompleting] = useState(false);

  const handleComplete = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setCompleting(true);
    try {
      await keel.deadlines.markComplete(d.id, '');
      onComplete(d.id);
    } finally {
      setCompleting(false);
    }
  };

  return (
    <motion.tr
      variants={rowVariants}
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
      style={{
        background: hovered ? colors.bgSecondary : 'transparent',
        transition: shouldReduce ? 'none' : `background ${transition.fast.duration}s`,
        cursor: 'default',
      }}
    >
      {/* Urgency dot */}
      <td style={{ width: 32, paddingLeft: spacing[5], verticalAlign: 'middle' }}>
        <UrgencyBadge urgency={d.urgency as UrgencyTier} dot />
      </td>

      {/* Due date */}
      <td style={{ padding: `${spacing[3]} ${spacing[4]}`, verticalAlign: 'middle', whiteSpace: 'nowrap' }}>
        <DueDate dueDate={d.dueDate} urgency={d.urgency as UrgencyTier} />
      </td>

      {/* Event */}
      <td style={{ padding: `${spacing[3]} ${spacing[4]}`, verticalAlign: 'middle' }}>
        <div style={{
          fontSize: fontSizes.cardTitle,
          fontWeight: 500,
          color: colors.textPrimary,
          fontFamily: fonts.ui,
          marginBottom: 2,
        }}>
          {d.docketingEvent}
        </div>
        {d.notes && (
          <div style={{
            fontSize: fontSizes.label,
            color: colors.textTertiary,
            fontFamily: fonts.ui,
            maxWidth: 360,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            {d.notes}
          </div>
        )}
      </td>

      {/* Matter + client */}
      <td style={{ padding: `${spacing[3]} ${spacing[4]}`, verticalAlign: 'middle' }}>
        <button
          onClick={() => onMatterClick(d.matterId)}
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            padding: 0,
            textAlign: 'left',
          }}
        >
          <div style={{
            fontSize: fontSizes.body,
            color: colors.accentPrimary,
            fontFamily: fonts.ui,
            fontWeight: 500,
            marginBottom: 2,
            textDecoration: hovered ? 'underline' : 'none',
          }}>
            {d.matterTitle}
          </div>
          <div style={{
            fontSize: fontSizes.label,
            color: colors.textTertiary,
            fontFamily: fonts.ui,
          }}>
            {d.clientName}
          </div>
        </button>
      </td>

      {/* Type */}
      <td style={{ padding: `${spacing[3]} ${spacing[4]}`, verticalAlign: 'middle', whiteSpace: 'nowrap' }}>
        <MatterTypePill matterType={d.matterType} />
      </td>

      {/* Event type tag */}
      <td style={{ padding: `${spacing[3]} ${spacing[4]}`, verticalAlign: 'middle', whiteSpace: 'nowrap' }}>
        {d.eventType !== 'Custom' && (
          <span style={{
            fontSize: 10,
            fontFamily: fonts.mono,
            color: d.eventType === 'Statutory' ? colors.accentSecondary : colors.textTertiary,
            letterSpacing: '0.05em',
            textTransform: 'uppercase' as const,
          }}>
            {d.eventType}
          </span>
        )}
      </td>

      {/* Mark complete */}
      <td style={{
        padding: `${spacing[3]} ${spacing[5]} ${spacing[3]} ${spacing[4]}`,
        verticalAlign: 'middle',
        textAlign: 'right' as const,
      }}>
        <AnimatePresence>
          {hovered && (
            <motion.button
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.9 }}
              transition={transition.fast}
              onClick={handleComplete}
              disabled={completing}
              style={{
                padding: '4px 12px',
                borderRadius: radius.button,
                border: `0.5px solid ${colors.statusClear}`,
                background: 'transparent',
                color: colors.statusClear,
                fontSize: fontSizes.label,
                fontFamily: fonts.ui,
                cursor: completing ? 'not-allowed' : 'pointer',
                opacity: completing ? 0.5 : 1,
                whiteSpace: 'nowrap',
              }}
            >
              {completing ? '…' : '✓ Done'}
            </motion.button>
          )}
        </AnimatePresence>
      </td>
    </motion.tr>
  );
}

// ---------------------------------------------------------------------------
// Empty state
// ---------------------------------------------------------------------------

function EmptyState({ filtered }: { filtered: boolean }) {
  return (
    <tr>
      <td colSpan={7}>
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          padding: `${spacing[12]} ${spacing[8]}`,
          gap: spacing[3],
          textAlign: 'center',
        }}>
          <div style={{
            fontFamily: fonts.display,
            fontSize: fontSizes.displaySm,
            color: colors.statusClear,
            fontWeight: 500,
          }}>
            {filtered ? 'No deadlines in this tier' : 'All clear'}
          </div>
          <p style={{
            fontSize: fontSizes.body,
            color: colors.textTertiary,
            margin: 0,
            maxWidth: 320,
          }}>
            {filtered
              ? 'Switch to "All pending" to see everything.'
              : 'No pending deadlines across all matters.'}
          </p>
        </div>
      </td>
    </tr>
  );
}

// ---------------------------------------------------------------------------
// DocketList page
// ---------------------------------------------------------------------------

export default function DocketList() {
  const navigate = useNavigate();
  const shouldReduce = useReducedMotion();

  const [deadlines, setDeadlines] = useState<DeadlineSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeUrgency, setActiveUrgency] = useState<UrgencyTier | 'All'>('All');

  const load = async () => {
    setLoading(true);
    try {
      const all = await keel.deadlines.listAll();
      setDeadlines(all);
    } catch {
      setDeadlines([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const handleComplete = (id: string) => {
    setDeadlines(prev => prev.filter(d => d.id !== id));
  };

  const filtered = activeUrgency === 'All'
    ? deadlines
    : deadlines.filter(d => d.urgency === activeUrgency);

  const counts: Record<string, number> = {};
  for (const d of deadlines) counts[d.urgency] = (counts[d.urgency] ?? 0) + 1;

  const listVariants = {
    hidden:  {},
    visible: { transition: shouldReduce ? {} : stagger.normal },
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* ── Header ────────────────────────────────────────── */}
      <header style={{
        padding: `${spacing[6]} ${spacing[8]} ${spacing[5]}`,
        borderBottom: `0.5px solid ${colors.border}`,
        flexShrink: 0,
      }}>
        <h1 style={{
          fontFamily: fonts.display,
          fontSize: fontSizes.displayLg,
          fontWeight: 600,
          color: colors.textPrimary,
          margin: '0 0 4px',
          lineHeight: 1.2,
        }}>
          Dockets
        </h1>
        <p style={{
          fontSize: fontSizes.label,
          color: colors.textTertiary,
          margin: 0,
          fontFamily: fonts.ui,
        }}>
          {loading
            ? 'Loading…'
            : `${deadlines.length} pending deadline${deadlines.length !== 1 ? 's' : ''}`
              + (counts['Overdue'] ? ` · ${counts['Overdue']} overdue` : '')}
        </p>
      </header>

      {/* ── Urgency filter bar ────────────────────────────── */}
      <div style={{
        padding: `${spacing[3]} ${spacing[8]}`,
        borderBottom: `0.5px solid ${colors.border}`,
        display: 'flex',
        gap: spacing[2],
        flexShrink: 0,
      }}>
        {URGENCY_CHIPS.map(chip => {
          const active = chip.value === activeUrgency;
          const count = chip.value === 'All'
            ? deadlines.length
            : (counts[chip.value] ?? 0);

          return (
            <button
              key={chip.value}
              onClick={() => setActiveUrgency(chip.value)}
              style={{
                padding: '4px 12px',
                borderRadius: 20,
                border: active
                  ? `1px solid ${colors.accentPrimary}`
                  : `0.5px solid ${colors.border}`,
                background: active ? 'rgba(74,101,128,0.10)' : 'transparent',
                color: active ? colors.accentPrimary : colors.textSecondary,
                fontSize: fontSizes.label,
                fontFamily: fonts.ui,
                fontWeight: active ? 500 : 400,
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: 5,
                transition: shouldReduce ? 'none' : `all ${transition.fast.duration}s`,
              }}
            >
              {chip.label}
              {count > 0 && (
                <span style={{
                  fontSize: 10,
                  fontFamily: fonts.mono,
                  background: active ? colors.accentPrimary : colors.border,
                  color: active ? '#fff' : colors.textTertiary,
                  borderRadius: 20,
                  padding: '1px 5px',
                }}>
                  {count}
                </span>
              )}
            </button>
          );
        })}
      </div>

      {/* ── Table ──────────────────────────────────────────── */}
      <div style={{ flex: 1, overflow: 'auto', padding: `0 ${spacing[8]}` }}>
        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
          <thead>
            <tr style={{ borderBottom: `0.5px solid ${colors.border}` }}>
              {['', 'Due', 'Event', 'Matter', 'Type', '', ''].map((h, i) => (
                <th key={i} style={{
                  padding: i === 0
                    ? `${spacing[3]} ${spacing[4]} ${spacing[3]} ${spacing[5]}`
                    : `${spacing[3]} ${spacing[4]}`,
                  textAlign: i === 6 ? 'right' : 'left' as const,
                  fontSize: fontSizes.label,
                  fontWeight: 500,
                  color: colors.textTertiary,
                  fontFamily: fonts.ui,
                  whiteSpace: 'nowrap',
                }}>
                  {h}
                </th>
              ))}
            </tr>
          </thead>

          <AnimatePresence mode="wait">
            <motion.tbody
              key={activeUrgency}
              variants={listVariants}
              initial="hidden"
              animate="visible"
            >
              {loading ? (
                <tr>
                  <td colSpan={7} style={{
                    padding: `${spacing[12]} 0`,
                    textAlign: 'center',
                    color: colors.textTertiary,
                    fontSize: fontSizes.body,
                    fontFamily: fonts.ui,
                  }}>
                    Loading dockets…
                  </td>
                </tr>
              ) : filtered.length === 0 ? (
                <EmptyState filtered={activeUrgency !== 'All'} />
              ) : (
                filtered.map(d => (
                  <DocketRow
                    key={d.id}
                    deadline={d}
                    onComplete={handleComplete}
                    onMatterClick={(id) => navigate(`/matters/${id}`)}
                  />
                ))
              )}
            </motion.tbody>
          </AnimatePresence>
        </table>
      </div>
    </div>
  );
}
