import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { motion, AnimatePresence } from 'motion/react';
import { useReducedMotion } from 'motion/react';
import type { MatterStatus, MatterSummary } from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { useMattersStore } from '@/stores/matters';
import { MatterStatusBadge, PriorityBar, MatterTypePill } from '@/components/matters/MatterStatusBadge';
import { NewMatterDrawer } from './NewMatterDrawer';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';
import { transition, stagger } from '@/design-system/motion';

// ---------------------------------------------------------------------------
// Status filter chips
// ---------------------------------------------------------------------------

const STATUS_CHIPS: { value: MatterStatus | 'All'; label: string }[] = [
  { value: 'All',                    label: 'All'     },
  { value: 'Active',                 label: 'Active'  },
  { value: 'OnHold',                 label: 'On Hold' },
  { value: 'PendingClientResponse',  label: 'Pending' },
  { value: 'Closed',                 label: 'Closed'  },
];

// ---------------------------------------------------------------------------
// Relative date formatting
// ---------------------------------------------------------------------------

function relativeDate(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const days = Math.floor(diff / 86_400_000);
  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days < 7)  return `${days}d ago`;
  if (days < 30) return `${Math.floor(days / 7)}w ago`;
  return new Date(iso).toLocaleDateString('en-IN', { day: 'numeric', month: 'short' });
}

// ---------------------------------------------------------------------------
// Row component
// ---------------------------------------------------------------------------

const rowVariants = {
  hidden:  { opacity: 0, y: 6 },
  visible: { opacity: 1, y: 0, transition: transition.fast },
};

interface MatterRowProps {
  matter: MatterSummary;
  onClick: () => void;
}

function MatterRow({ matter, onClick }: MatterRowProps) {
  const shouldReduce = useReducedMotion();
  const [hovered, setHovered] = useState(false);

  return (
    <motion.tr
      variants={rowVariants}
      onClick={onClick}
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
      style={{
        cursor: 'pointer',
        background: hovered ? colors.bgSecondary : 'transparent',
        transition: shouldReduce ? 'none' : `background ${transition.fast.duration}s`,
      }}
    >
      {/* Priority bar cell */}
      <td style={{ width: 4, padding: 0 }}>
        <PriorityBar priority={matter.priority} />
      </td>

      {/* Matter ID */}
      <td style={{
        padding: `${spacing[3]} ${spacing[4]} ${spacing[3]} ${spacing[3]}`,
        width: 180,
        verticalAlign: 'middle',
      }}>
        <span style={{
          fontFamily: fonts.mono,
          fontSize: fontSizes.label,
          color: colors.textTertiary,
          letterSpacing: '0.02em',
        }}>
          {matter.id}
        </span>
      </td>

      {/* Title + client */}
      <td style={{ padding: `${spacing[3]} ${spacing[4]}`, verticalAlign: 'middle' }}>
        <div style={{
          fontSize: fontSizes.cardTitle,
          fontWeight: 500,
          color: colors.textPrimary,
          fontFamily: fonts.ui,
          marginBottom: 2,
          lineHeight: 1.3,
        }}>
          {matter.title}
        </div>
        <div style={{
          fontSize: fontSizes.label,
          color: colors.textSecondary,
          fontFamily: fonts.ui,
        }}>
          {matter.clientName}
        </div>
      </td>

      {/* Type */}
      <td style={{
        padding: `${spacing[3]} ${spacing[4]}`,
        verticalAlign: 'middle',
        whiteSpace: 'nowrap',
      }}>
        <MatterTypePill matterType={matter.matterType} />
      </td>

      {/* Status */}
      <td style={{
        padding: `${spacing[3]} ${spacing[4]}`,
        verticalAlign: 'middle',
        whiteSpace: 'nowrap',
      }}>
        <MatterStatusBadge status={matter.status} />
      </td>

      {/* Deadline */}
      <td style={{
        padding: `${spacing[3]} ${spacing[4]}`,
        verticalAlign: 'middle',
        whiteSpace: 'nowrap',
      }}>
        {matter.nextDeadlineDate ? (
          <span style={{
            fontSize: fontSizes.label,
            color: colors.statusWarning,
            fontFamily: fonts.ui,
          }}>
            {matter.nextDeadlineEvent ?? 'Deadline'} · {relativeDate(matter.nextDeadlineDate)}
          </span>
        ) : (
          <span style={{ fontSize: fontSizes.label, color: colors.textTertiary }}>—</span>
        )}
      </td>

      {/* Updated */}
      <td style={{
        padding: `${spacing[3]} ${spacing[5]} ${spacing[3]} ${spacing[4]}`,
        verticalAlign: 'middle',
        textAlign: 'right' as const,
        whiteSpace: 'nowrap',
      }}>
        <span style={{
          fontSize: fontSizes.label,
          color: colors.textTertiary,
          fontFamily: fonts.ui,
        }}>
          {relativeDate(matter.updatedAt)}
        </span>
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
            color: colors.textTertiary,
            fontWeight: 500,
          }}>
            {filtered ? 'No matters match this filter' : 'No matters yet'}
          </div>
          <p style={{
            fontSize: fontSizes.body,
            color: colors.textTertiary,
            margin: 0,
            maxWidth: 320,
          }}>
            {filtered
              ? 'Try a different status filter or clear the selection.'
              : 'Create your first matter to get started.'}
          </p>
        </div>
      </td>
    </tr>
  );
}

// ---------------------------------------------------------------------------
// MatterList page
// ---------------------------------------------------------------------------

export default function MatterList() {
  const navigate = useNavigate();
  const shouldReduce = useReducedMotion();
  const { matterList, setMatterList, filter, setFilter, isLoading, setLoading } = useMattersStore();

  const [activeStatus, setActiveStatus] = useState<MatterStatus | 'All'>('All');
  const [drawerOpen, setDrawerOpen] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      const list = await keel.matters.list({
        status: activeStatus === 'All' ? undefined : [activeStatus],
      });
      setMatterList(list);
    } catch {
      setMatterList([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, [activeStatus]);

  const handleStatusChip = (status: MatterStatus | 'All') => {
    setActiveStatus(status);
    setFilter({ ...filter, status: status === 'All' ? undefined : [status] });
  };

  const listVariants = {
    hidden:  {},
    visible: { transition: shouldReduce ? {} : stagger.normal },
  };

  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
      background: colors.bgPrimary,
    }}>
      {/* ── Page header ─────────────────────────────────────── */}
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
            margin: 0,
            lineHeight: 1.2,
          }}>
            Matters
          </h1>
          <p style={{
            fontSize: fontSizes.label,
            color: colors.textTertiary,
            margin: '4px 0 0',
            fontFamily: fonts.ui,
          }}>
            {isLoading
              ? 'Loading…'
              : `${matterList.length} matter${matterList.length !== 1 ? 's' : ''}`}
          </p>
        </div>

        <motion.button
          onClick={() => setDrawerOpen(true)}
          whileHover={!shouldReduce ? { opacity: 0.88 } : undefined}
          whileTap={!shouldReduce ? { scale: 0.97 } : undefined}
          style={{
            padding: `${spacing[2]} ${spacing[5]}`,
            borderRadius: radius.button,
            border: 'none',
            background: colors.accentPrimary,
            color: '#fff',
            fontFamily: fonts.ui,
            fontSize: fontSizes.body,
            fontWeight: 500,
            cursor: 'pointer',
          }}
        >
          + New Matter
        </motion.button>
      </header>

      {/* ── Status filter bar ───────────────────────────────── */}
      <div style={{
        padding: `${spacing[3]} ${spacing[8]}`,
        borderBottom: `0.5px solid ${colors.border}`,
        display: 'flex',
        gap: spacing[2],
        flexShrink: 0,
      }}>
        {STATUS_CHIPS.map(chip => {
          const active = chip.value === activeStatus;
          return (
            <button
              key={chip.value}
              onClick={() => handleStatusChip(chip.value)}
              style={{
                padding: '4px 12px',
                borderRadius: 20,
                border: active
                  ? `1px solid ${colors.accentPrimary}`
                  : `0.5px solid ${colors.border}`,
                background: active ? `rgba(74,101,128,0.10)` : 'transparent',
                color: active ? colors.accentPrimary : colors.textSecondary,
                fontSize: fontSizes.label,
                fontFamily: fonts.ui,
                fontWeight: active ? 500 : 400,
                cursor: 'pointer',
                transition: shouldReduce ? 'none' : `all ${transition.fast.duration}s`,
              }}
            >
              {chip.label}
            </button>
          );
        })}
      </div>

      {/* ── Matter table ────────────────────────────────────── */}
      <div style={{ flex: 1, overflow: 'auto', padding: `0 ${spacing[8]}` }}>
        <table style={{
          width: '100%',
          borderCollapse: 'collapse',
          tableLayout: 'auto',
        }}>
          <thead>
            <tr style={{ borderBottom: `0.5px solid ${colors.border}` }}>
              <th style={{ width: 4, padding: 0 }} />
              <th style={{
                padding: `${spacing[3]} ${spacing[4]} ${spacing[3]} ${spacing[3]}`,
                textAlign: 'left' as const,
                fontSize: fontSizes.label,
                fontWeight: 500,
                color: colors.textTertiary,
                fontFamily: fonts.ui,
                whiteSpace: 'nowrap',
              }}>
                ID
              </th>
              <th style={{
                padding: `${spacing[3]} ${spacing[4]}`,
                textAlign: 'left' as const,
                fontSize: fontSizes.label,
                fontWeight: 500,
                color: colors.textTertiary,
                fontFamily: fonts.ui,
              }}>
                Matter
              </th>
              <th style={{
                padding: `${spacing[3]} ${spacing[4]}`,
                textAlign: 'left' as const,
                fontSize: fontSizes.label,
                fontWeight: 500,
                color: colors.textTertiary,
                fontFamily: fonts.ui,
                whiteSpace: 'nowrap',
              }}>
                Type
              </th>
              <th style={{
                padding: `${spacing[3]} ${spacing[4]}`,
                textAlign: 'left' as const,
                fontSize: fontSizes.label,
                fontWeight: 500,
                color: colors.textTertiary,
                fontFamily: fonts.ui,
              }}>
                Status
              </th>
              <th style={{
                padding: `${spacing[3]} ${spacing[4]}`,
                textAlign: 'left' as const,
                fontSize: fontSizes.label,
                fontWeight: 500,
                color: colors.textTertiary,
                fontFamily: fonts.ui,
                whiteSpace: 'nowrap',
              }}>
                Next deadline
              </th>
              <th style={{
                padding: `${spacing[3]} ${spacing[5]} ${spacing[3]} ${spacing[4]}`,
                textAlign: 'right' as const,
                fontSize: fontSizes.label,
                fontWeight: 500,
                color: colors.textTertiary,
                fontFamily: fonts.ui,
                whiteSpace: 'nowrap',
              }}>
                Updated
              </th>
            </tr>
          </thead>

          <AnimatePresence mode="wait">
            <motion.tbody
              key={activeStatus}
              variants={listVariants}
              initial="hidden"
              animate="visible"
            >
              {isLoading ? (
                <tr>
                  <td colSpan={7} style={{
                    padding: `${spacing[12]} 0`,
                    textAlign: 'center',
                    color: colors.textTertiary,
                    fontSize: fontSizes.body,
                    fontFamily: fonts.ui,
                  }}>
                    Loading matters…
                  </td>
                </tr>
              ) : matterList.length === 0 ? (
                <EmptyState filtered={activeStatus !== 'All'} />
              ) : (
                matterList.map(m => (
                  <MatterRow
                    key={m.id}
                    matter={m}
                    onClick={() => navigate(`/matters/${m.id}`)}
                  />
                ))
              )}
            </motion.tbody>
          </AnimatePresence>
        </table>
      </div>

      {/* ── New matter drawer ───────────────────────────────── */}
      <NewMatterDrawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        onCreated={load}
      />
    </div>
  );
}
