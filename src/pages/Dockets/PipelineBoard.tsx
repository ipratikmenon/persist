/**
 * PipelineBoard — Kanban view of matters grouped by prosecution stage.
 *
 * Columns = matter status buckets (Active / On Hold / Pending Client / Closed)
 * Each card shows the matter + its most urgent pending deadline.
 * Accessed via /dockets/pipeline
 */
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { motion } from 'motion/react';
import { useReducedMotion } from 'motion/react';
import type { MatterSummary, MatterStatus } from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { MatterTypePill } from '@/components/matters/MatterStatusBadge';
import { UrgencyBadge } from '@/components/dockets/UrgencyBadge';
import { colors, fonts, fontSizes, radius, shadows, spacing } from '@/design-system/tokens';
import { transition, stagger, cardHover } from '@/design-system/motion';

// ---------------------------------------------------------------------------
// Column definitions
// ---------------------------------------------------------------------------

interface Column {
  status: MatterStatus;
  label: string;
  headerColor: string;
}

const COLUMNS: Column[] = [
  { status: 'Active',                label: 'Active',         headerColor: colors.statusClear    },
  { status: 'OnHold',                label: 'On Hold',        headerColor: colors.statusWarning  },
  { status: 'PendingClientResponse', label: 'Pending Client', headerColor: colors.accentPrimary  },
  { status: 'Closed',                label: 'Closed',         headerColor: colors.textTertiary   },
];

// ---------------------------------------------------------------------------
// Matter card
// ---------------------------------------------------------------------------

interface MatterCardProps {
  matter: MatterSummary;
  onClick: () => void;
}

function MatterCard({ matter, onClick }: MatterCardProps) {
  const shouldReduce = useReducedMotion();

  const urgency = matter.nextDeadlineDate
    ? (() => {
        const today = new Date();
        const due   = new Date(matter.nextDeadlineDate);
        const days  = Math.floor((due.getTime() - today.setHours(0,0,0,0)) / 86_400_000);
        if (days < 0)  return 'Overdue'  as const;
        if (days <= 3) return 'Critical' as const;
        if (days <= 7) return 'Warning'  as const;
        return 'Normal' as const;
      })()
    : null;

  return (
    <motion.div
      layout
      {...(shouldReduce ? {} : cardHover)}
      onClick={onClick}
      style={{
        background: colors.bgPrimary,
        border: `0.5px solid ${colors.border}`,
        borderRadius: radius.card,
        boxShadow: shadows.card,
        padding: `${spacing[4]} ${spacing[4]} ${spacing[3]}`,
        cursor: 'pointer',
        display: 'flex',
        flexDirection: 'column',
        gap: spacing[2],
        // Priority left border
        borderLeft: matter.priority === 'Urgent'
          ? `3px solid ${colors.accentSecondary}`
          : matter.priority === 'High'
          ? `3px solid ${colors.statusWarning}`
          : `0.5px solid ${colors.border}`,
      }}
    >
      {/* Matter ID + type */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span style={{
          fontFamily: fonts.mono,
          fontSize: 10,
          color: colors.textTertiary,
          letterSpacing: '0.02em',
        }}>
          {matter.id}
        </span>
        <MatterTypePill matterType={matter.matterType} />
      </div>

      {/* Title */}
      <div style={{
        fontSize: fontSizes.body,
        fontWeight: 500,
        color: colors.textPrimary,
        fontFamily: fonts.ui,
        lineHeight: 1.35,
      }}>
        {matter.title}
      </div>

      {/* Client */}
      <div style={{
        fontSize: fontSizes.label,
        color: colors.textSecondary,
        fontFamily: fonts.ui,
      }}>
        {matter.clientName}
      </div>

      {/* Next deadline */}
      {matter.nextDeadlineDate && urgency && (
        <div style={{
          marginTop: spacing[1],
          display: 'flex',
          alignItems: 'center',
          gap: spacing[2],
          paddingTop: spacing[2],
          borderTop: `0.5px solid ${colors.border}`,
        }}>
          <UrgencyBadge urgency={urgency} dot />
          <span style={{
            fontSize: fontSizes.label,
            fontFamily: fonts.ui,
            color: colors.textTertiary,
            flex: 1,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            {matter.nextDeadlineEvent ?? 'Deadline'}
          </span>
          <span style={{
            fontSize: fontSizes.label,
            fontFamily: fonts.ui,
            color: urgency === 'Normal' ? colors.textTertiary : (
              urgency === 'Overdue' ? colors.statusUrgent :
              urgency === 'Critical' ? colors.accentSecondary :
              colors.statusWarning
            ),
            fontWeight: urgency !== 'Normal' ? 500 : 400,
            whiteSpace: 'nowrap',
            flexShrink: 0,
          }}>
            {new Date(matter.nextDeadlineDate).toLocaleDateString('en-IN', { day: 'numeric', month: 'short' })}
          </span>
        </div>
      )}
    </motion.div>
  );
}

// ---------------------------------------------------------------------------
// Pipeline column
// ---------------------------------------------------------------------------

interface PipelineColumnProps {
  column: Column;
  matters: MatterSummary[];
  onMatterClick: (id: string) => void;
}

function PipelineColumn({ column, matters, onMatterClick }: PipelineColumnProps) {
  const shouldReduce = useReducedMotion();

  const cardVariants = {
    hidden:  { opacity: 0, y: 8 },
    visible: { opacity: 1, y: 0, transition: transition.fast },
  };

  const listVariants = {
    hidden:  {},
    visible: { transition: shouldReduce ? {} : stagger.fast },
  };

  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      minWidth: 260,
      maxWidth: 300,
      flex: '1 1 260px',
      background: colors.bgSecondary,
      borderRadius: radius.card,
      border: `0.5px solid ${colors.border}`,
      overflow: 'hidden',
    }}>
      {/* Column header */}
      <div style={{
        padding: `${spacing[3]} ${spacing[4]}`,
        borderBottom: `2px solid ${column.headerColor}`,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        background: colors.bgSecondary,
        flexShrink: 0,
      }}>
        <span style={{
          fontSize: fontSizes.label,
          fontWeight: 500,
          color: colors.textPrimary,
          fontFamily: fonts.ui,
          textTransform: 'uppercase' as const,
          letterSpacing: '0.06em',
        }}>
          {column.label}
        </span>
        <span style={{
          fontSize: fontSizes.label,
          fontFamily: fonts.mono,
          color: colors.textTertiary,
          background: colors.bgPrimary,
          borderRadius: 10,
          padding: '1px 7px',
          border: `0.5px solid ${colors.border}`,
        }}>
          {matters.length}
        </span>
      </div>

      {/* Cards */}
      <motion.div
        variants={listVariants}
        initial="hidden"
        animate="visible"
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: spacing[3],
          display: 'flex',
          flexDirection: 'column',
          gap: spacing[3],
        }}
      >
        {matters.length === 0 ? (
          <div style={{
            textAlign: 'center',
            padding: spacing[8],
            color: colors.textTertiary,
            fontSize: fontSizes.label,
            fontFamily: fonts.ui,
          }}>
            No matters
          </div>
        ) : (
          matters.map(m => (
            <motion.div key={m.id} variants={cardVariants}>
              <MatterCard
                matter={m}
                onClick={() => onMatterClick(m.id)}
              />
            </motion.div>
          ))
        )}
      </motion.div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PipelineBoard page
// ---------------------------------------------------------------------------

export default function PipelineBoard() {
  const navigate = useNavigate();

  const [matters, setMatters] = useState<MatterSummary[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    keel.matters.list({})
      .then(setMatters)
      .catch(() => setMatters([]))
      .finally(() => setLoading(false));
  }, []);

  const byStatus = (status: MatterStatus) =>
    matters.filter(m => m.status === status);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Header */}
      <header style={{
        padding: `${spacing[6]} ${spacing[8]} ${spacing[5]}`,
        borderBottom: `0.5px solid ${colors.border}`,
        display: 'flex',
        alignItems: 'flex-end',
        justifyContent: 'space-between',
        flexShrink: 0,
      }}>
        <div>
          <h1 style={{
            fontFamily: fonts.display,
            fontSize: fontSizes.displayLg,
            fontWeight: 600,
            color: colors.textPrimary,
            margin: '0 0 4px',
            lineHeight: 1.2,
          }}>
            Pipeline
          </h1>
          <p style={{ fontSize: fontSizes.label, color: colors.textTertiary, margin: 0, fontFamily: fonts.ui }}>
            {loading ? 'Loading…' : `${matters.length} matter${matters.length !== 1 ? 's' : ''} across all stages`}
          </p>
        </div>

        <div style={{ display: 'flex', gap: spacing[3] }}>
          <button
            onClick={() => navigate('/dockets')}
            style={{
              padding: `${spacing[2]} ${spacing[4]}`,
              borderRadius: radius.button,
              border: `0.5px solid ${colors.border}`,
              background: 'transparent',
              color: colors.textSecondary,
              fontFamily: fonts.ui,
              fontSize: fontSizes.body,
              cursor: 'pointer',
            }}
          >
            ← Docket list
          </button>
        </div>
      </header>

      {/* Board */}
      <div style={{
        flex: 1,
        overflow: 'auto',
        padding: spacing[6],
        display: 'flex',
        gap: spacing[4],
        alignItems: 'flex-start',
      }}>
        {loading ? (
          <div style={{ color: colors.textTertiary, fontFamily: fonts.ui, fontSize: fontSizes.body, padding: spacing[8] }}>
            Loading pipeline…
          </div>
        ) : (
          COLUMNS.map(col => (
            <PipelineColumn
              key={col.status}
              column={col}
              matters={byStatus(col.status)}
              onMatterClick={(id) => navigate(`/matters/${id}`)}
            />
          ))
        )}
      </div>
    </div>
  );
}
