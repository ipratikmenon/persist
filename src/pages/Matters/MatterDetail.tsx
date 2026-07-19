import { useEffect, useState, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router';
import { motion, AnimatePresence } from 'motion/react';
import { useReducedMotion } from 'motion/react';
import { save as saveDialog } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import type { DocumentCategory, DocumentMeta, Matter, MatterStatus } from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { useMattersStore } from '@/stores/matters';
import { MatterStatusBadge } from '@/components/matters/MatterStatusBadge';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';
import { transition } from '@/design-system/motion';
import { AnimatedUploadDrawer } from '@/pages/Documents/UploadDrawer';

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

type Tab = 'overview' | 'parties' | 'documents' | 'deadlines';

const TABS: { id: Tab; label: string }[] = [
  { id: 'overview',  label: 'Overview'  },
  { id: 'parties',   label: 'Parties'   },
  { id: 'documents', label: 'Documents' },
  { id: 'deadlines', label: 'Deadlines' },
];

// ---------------------------------------------------------------------------
// Status transitions — drives the status change menu
// ---------------------------------------------------------------------------

const TRANSITIONS: Record<MatterStatus, MatterStatus[]> = {
  Active:                ['OnHold', 'PendingClientResponse', 'Closed', 'Archived'],
  OnHold:                ['Active', 'Closed', 'Archived'],
  PendingClientResponse: ['Active', 'Closed', 'Archived'],
  Closed:                ['Archived'],
  Archived:              [],
};

const STATUS_LABELS: Record<MatterStatus, string> = {
  Active:                'Active',
  OnHold:                'On Hold',
  PendingClientResponse: 'Pending Client',
  Closed:                'Closed',
  Archived:              'Archived',
};

// ---------------------------------------------------------------------------
// Overview tab — matter metadata grid
// ---------------------------------------------------------------------------

function MetaRow({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div style={{
      display: 'grid',
      gridTemplateColumns: '160px 1fr',
      gap: spacing[4],
      padding: `${spacing[3]} 0`,
      borderBottom: `0.5px solid ${colors.border}`,
    }}>
      <span style={{
        fontSize: fontSizes.label,
        color: colors.textTertiary,
        fontFamily: fonts.ui,
        fontWeight: 500,
        textTransform: 'uppercase' as const,
        letterSpacing: '0.05em',
        paddingTop: 2,
      }}>
        {label}
      </span>
      <span style={{
        fontSize: fontSizes.body,
        color: colors.textPrimary,
        fontFamily: fonts.ui,
        lineHeight: 1.5,
      }}>
        {value ?? <span style={{ color: colors.textTertiary }}>—</span>}
      </span>
    </div>
  );
}

function OverviewTab({ matter }: { matter: Matter }) {
  const opened = new Date(matter.openedDate).toLocaleDateString('en-IN', {
    day: 'numeric', month: 'long', year: 'numeric',
  });
  const targetClose = matter.targetCloseDate
    ? new Date(matter.targetCloseDate).toLocaleDateString('en-IN', {
        day: 'numeric', month: 'long', year: 'numeric',
      })
    : null;

  return (
    <div style={{ maxWidth: 680 }}>
      {/* Core info */}
      <MetaRow label="Type" value={
        matter.subType
          ? `${matter.matterType} · ${matter.subType}`
          : matter.matterType
      } />
      <MetaRow label="Jurisdiction"   value={matter.jurisdiction} />
      <MetaRow label="Forum"          value={matter.forum} />
      <MetaRow label="Opened"         value={opened} />
      <MetaRow label="Target close"   value={targetClose} />
      <MetaRow label="Priority"       value={matter.priority} />

      {/* Tags */}
      {matter.tags.length > 0 && (
        <MetaRow label="Tags" value={
          <div style={{ display: 'flex', gap: spacing[2], flexWrap: 'wrap' }}>
            {matter.tags.map(tag => (
              <span key={tag} style={{
                padding: '2px 8px',
                borderRadius: radius.chip,
                background: `rgba(74,101,128,0.08)`,
                color: colors.accentPrimary,
                fontSize: 11,
                fontFamily: fonts.ui,
              }}>
                {tag}
              </span>
            ))}
          </div>
        } />
      )}

      {/* Client notes */}
      {matter.clientNotes && (
        <div style={{ marginTop: spacing[6] }}>
          <div style={{
            fontSize: fontSizes.label,
            fontWeight: 500,
            color: colors.textTertiary,
            textTransform: 'uppercase' as const,
            letterSpacing: '0.05em',
            marginBottom: spacing[3],
          }}>
            Client notes
          </div>
          <p style={{
            fontFamily: fonts.legal,
            fontSize: fontSizes.legal,
            color: colors.textPrimary,
            lineHeight: 1.7,
            margin: 0,
            padding: spacing[4],
            background: colors.bgSecondary,
            borderRadius: radius.card,
            border: `0.5px solid ${colors.border}`,
          }}>
            {matter.clientNotes}
          </p>
        </div>
      )}

      {/* Internal notes — always last, visually separated */}
      {matter.internalNotes && (
        <div style={{ marginTop: spacing[6] }}>
          <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: spacing[2],
            marginBottom: spacing[3],
          }}>
            <span style={{
              fontSize: fontSizes.label,
              fontWeight: 500,
              color: colors.textTertiary,
              textTransform: 'uppercase' as const,
              letterSpacing: '0.05em',
            }}>
              Internal notes
            </span>
            <span style={{
              fontSize: 10,
              fontFamily: fonts.mono,
              color: colors.statusUrgent,
              letterSpacing: '0.04em',
            }}>
              ATTORNEYS ONLY
            </span>
          </div>
          <p style={{
            fontFamily: fonts.legal,
            fontSize: fontSizes.legal,
            color: colors.textPrimary,
            lineHeight: 1.7,
            margin: 0,
            padding: spacing[4],
            background: 'rgba(192,57,43,0.04)',
            borderRadius: radius.card,
            border: `0.5px solid rgba(192,57,43,0.15)`,
          }}>
            {matter.internalNotes}
          </p>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Parties tab — placeholder until users table is live
// ---------------------------------------------------------------------------

function PartiesTab({ matter }: { matter: Matter }) {
  if (matter.parties.length === 0) {
    return (
      <p style={{ color: colors.textTertiary, fontSize: fontSizes.body, fontFamily: fonts.ui }}>
        No parties assigned yet.
      </p>
    );
  }
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: spacing[3], maxWidth: 480 }}>
      {matter.parties.map(p => (
        <div key={p.userId} style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: `${spacing[3]} ${spacing[4]}`,
          background: colors.bgSecondary,
          borderRadius: radius.card,
          border: `0.5px solid ${colors.border}`,
        }}>
          <div>
            <div style={{ fontSize: fontSizes.body, fontWeight: 500, color: colors.textPrimary }}>
              {p.name || p.userId}
            </div>
            <div style={{ fontSize: fontSizes.label, color: colors.textTertiary, marginTop: 2 }}>
              {p.role}{p.isPrimary ? ' · Responsible attorney' : ''}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Stub tab for deadlines (built in M2 — wired via IPAssetRecord route)
// ---------------------------------------------------------------------------

function DeadlinesTab({ matterId }: { matterId: string }) {
  const navigate = useNavigate();
  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      padding: `${spacing[12]} 0`,
      gap: spacing[3],
      textAlign: 'center',
    }}>
      <div style={{
        fontFamily: fonts.display,
        fontSize: fontSizes.displaySm,
        color: colors.textTertiary,
        fontWeight: 500,
      }}>
        Deadlines
      </div>
      <p style={{
        fontSize: fontSizes.body,
        color: colors.textTertiary,
        margin: 0,
        fontFamily: fonts.ui,
      }}>
        View and manage deadlines for this matter.
      </p>
      <button
        onClick={() => navigate(`/dockets/${matterId}`)}
        style={{
          marginTop: spacing[2],
          padding: `${spacing[2]} ${spacing[5]}`,
          borderRadius: radius.button,
          border: `0.5px solid ${colors.accentPrimary}`,
          background: 'transparent',
          color: colors.accentPrimary,
          fontFamily: fonts.ui,
          fontSize: fontSizes.body,
          cursor: 'pointer',
        }}
      >
        Open docket timeline →
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Documents tab — Phase 1 M3
// ---------------------------------------------------------------------------

const CATEGORY_LABELS: Record<DocumentCategory, string> = {
  Filing: 'Filing', Correspondence: 'Correspondence', Certificate: 'Certificate',
  SearchReport: 'Search Report', Contract: 'Contract', Invoice: 'Invoice', Other: 'Other',
};

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function DocumentsTab({ matterId }: { matterId: string }) {
  const shouldReduce = useReducedMotion();
  const [docs, setDocs] = useState<DocumentMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [uploadOpen, setUploadOpen] = useState(false);
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);

  const loadDocs = useCallback(async () => {
    setLoading(true);
    try {
      setDocs(await keel.documents.list(matterId));
    } catch {
      setDocs([]);
    } finally {
      setLoading(false);
    }
  }, [matterId]);

  useEffect(() => { loadDocs(); }, [loadDocs]);

  const handleDownload = async (doc: DocumentMeta) => {
    if (downloading) return;
    setDownloading(doc.id);
    try {
      const destPath = await saveDialog({ defaultPath: doc.filename });
      if (!destPath) return;
      const bytes = await keel.documents.get(doc.id);
      await writeFile(destPath, new Uint8Array(bytes));
    } finally {
      setDownloading(null);
    }
  };

  const handleDelete = async (doc: DocumentMeta) => {
    if (!window.confirm(`Delete "${doc.filename}"? This cannot be undone.`)) return;
    setDeleting(doc.id);
    try {
      await keel.documents.delete(doc.id);
      setDocs(prev => prev.filter(d => d.id !== doc.id));
    } finally {
      setDeleting(null);
    }
  };

  if (loading) {
    return (
      <div style={{ color: colors.textTertiary, fontFamily: fonts.ui, fontSize: fontSizes.body }}>
        Loading documents…
      </div>
    );
  }

  return (
    <div>
      {/* Upload button */}
      <div style={{ marginBottom: spacing[5], display: 'flex', justifyContent: 'flex-end' }}>
        <button
          onClick={() => setUploadOpen(true)}
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
          + Upload document
        </button>
      </div>

      {/* Empty state */}
      {docs.length === 0 ? (
        <div style={{
          textAlign: 'center',
          padding: `${spacing[10]} 0`,
          color: colors.textTertiary,
          fontFamily: fonts.ui,
          fontSize: fontSizes.body,
        }}>
          No documents yet. Upload the first one above.
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: spacing[2] }}>
          {docs.map(doc => {
            const hovered = hoveredId === doc.id;
            const isDownloading = downloading === doc.id;
            const isDeleting = deleting === doc.id;
            const uploadedDate = new Date(doc.createdAt).toLocaleDateString('en-IN', {
              day: 'numeric', month: 'short', year: 'numeric',
            });

            return (
              <motion.div
                key={doc.id}
                layout
                onHoverStart={() => setHoveredId(doc.id)}
                onHoverEnd={() => setHoveredId(null)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: spacing[4],
                  padding: `${spacing[3]} ${spacing[4]}`,
                  borderRadius: radius.card,
                  border: `0.5px solid ${colors.border}`,
                  background: hovered ? colors.bgSecondary : colors.bgPrimary,
                  transition: shouldReduce ? 'none' : `background ${transition.fast.duration}s`,
                }}
              >
                {/* Filename + meta */}
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{
                    fontSize: fontSizes.cardTitle,
                    fontWeight: 500,
                    color: colors.textPrimary,
                    fontFamily: fonts.ui,
                    marginBottom: 2,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}>
                    {doc.filename}
                  </div>
                  <div style={{
                    fontSize: fontSizes.label,
                    color: colors.textTertiary,
                    fontFamily: fonts.ui,
                    display: 'flex',
                    gap: spacing[3],
                    flexWrap: 'wrap',
                  }}>
                    <span>{CATEGORY_LABELS[doc.category as DocumentCategory] ?? doc.category}</span>
                    <span>·</span>
                    <span>{formatBytes(doc.fileSizeBytes)}</span>
                    <span>·</span>
                    <span>{uploadedDate}</span>
                    {doc.description && (
                      <>
                        <span>·</span>
                        <span style={{
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                          maxWidth: 200,
                        }}>
                          {doc.description}
                        </span>
                      </>
                    )}
                  </div>
                </div>

                {/* Actions */}
                <AnimatePresence>
                  {hovered && (
                    <motion.div
                      initial={{ opacity: 0, scale: 0.92 }}
                      animate={{ opacity: 1, scale: 1 }}
                      exit={{ opacity: 0, scale: 0.92 }}
                      transition={transition.fast}
                      style={{ display: 'flex', gap: spacing[2], flexShrink: 0 }}
                    >
                      <button
                        onClick={() => handleDownload(doc)}
                        disabled={isDownloading}
                        style={docBtnStyle(colors.accentPrimary, isDownloading)}
                      >
                        {isDownloading ? '…' : '↓ Save'}
                      </button>
                      <button
                        onClick={() => handleDelete(doc)}
                        disabled={isDeleting}
                        style={docBtnStyle(colors.statusUrgent, isDeleting)}
                      >
                        {isDeleting ? '…' : 'Delete'}
                      </button>
                    </motion.div>
                  )}
                </AnimatePresence>
              </motion.div>
            );
          })}
        </div>
      )}

      {/* Upload drawer */}
      <AnimatedUploadDrawer
        open={uploadOpen}
        matterId={matterId}
        onClose={() => setUploadOpen(false)}
        onUploaded={loadDocs}
      />
    </div>
  );
}

function docBtnStyle(color: string, disabled: boolean): React.CSSProperties {
  return {
    padding: '4px 12px',
    borderRadius: radius.button,
    border: `0.5px solid ${color}`,
    background: 'transparent',
    color,
    fontSize: fontSizes.label,
    fontFamily: fonts.ui,
    cursor: disabled ? 'not-allowed' : 'pointer',
    opacity: disabled ? 0.5 : 1,
    whiteSpace: 'nowrap' as const,
  };
}

// ---------------------------------------------------------------------------
// MatterDetail page
// ---------------------------------------------------------------------------

export default function MatterDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const shouldReduce = useReducedMotion();
  const { setActiveMatter } = useMattersStore();

  const [matter, setMatter] = useState<Matter | null>(null);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<Tab>('overview');
  const [statusMenuOpen, setStatusMenuOpen] = useState(false);
  const [transitioning, setTransitioning] = useState(false);

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    keel.matters.get(id)
      .then(m => { setMatter(m); setActiveMatter(m); })
      .catch(() => navigate('/matters'))
      .finally(() => setLoading(false));
    return () => setActiveMatter(null);
  }, [id]);

  const handleStatusChange = async (to: MatterStatus) => {
    if (!matter || transitioning) return;
    setStatusMenuOpen(false);
    setTransitioning(true);
    try {
      const updated = await keel.matters.updateStatus(matter.id, to);
      setMatter(updated);
    } finally {
      setTransitioning(false);
    }
  };

  if (loading) {
    return (
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        height: '100%',
        color: colors.textTertiary,
        fontFamily: fonts.ui,
        fontSize: fontSizes.body,
      }}>
        Loading matter…
      </div>
    );
  }

  if (!matter) return null;

  const allowedTransitions = TRANSITIONS[matter.status as MatterStatus] ?? [];

  const tabContent = {
    overview:  <OverviewTab matter={matter} />,
    parties:   <PartiesTab matter={matter} />,
    documents: <DocumentsTab matterId={matter.id} />,
    deadlines: <DeadlinesTab matterId={matter.id} />,
  };

  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
      overflow: 'hidden',
    }}>
      {/* ── Matter header ───────────────────────────────────── */}
      <header style={{
        padding: `${spacing[5]} ${spacing[8]} ${spacing[4]}`,
        borderBottom: `0.5px solid ${colors.border}`,
        flexShrink: 0,
      }}>
        {/* Back + ID row */}
        <div style={{
          display: 'flex',
          alignItems: 'center',
          gap: spacing[4],
          marginBottom: spacing[4],
        }}>
          <button
            onClick={() => navigate('/matters')}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: colors.textTertiary,
              fontSize: fontSizes.body,
              fontFamily: fonts.ui,
              padding: 0,
              display: 'flex',
              alignItems: 'center',
              gap: spacing[1],
            }}
          >
            ← Matters
          </button>
          <span style={{
            fontFamily: fonts.mono,
            fontSize: fontSizes.label,
            color: colors.textTertiary,
            letterSpacing: '0.02em',
            padding: '2px 8px',
            background: colors.bgSecondary,
            borderRadius: radius.chip,
            border: `0.5px solid ${colors.border}`,
          }}>
            {matter.id}
          </span>
        </div>

        {/* Title */}
        <h1 style={{
          fontFamily: fonts.display,
          fontSize: fontSizes.displayLg,
          fontWeight: 600,
          color: colors.textPrimary,
          margin: '0 0 10px',
          lineHeight: 1.25,
        }}>
          {matter.title}
        </h1>

        {/* Meta bar: status | priority | attorney */}
        <div style={{
          display: 'flex',
          alignItems: 'center',
          gap: spacing[3],
          flexWrap: 'wrap',
        }}>
          {/* Status with dropdown */}
          <div style={{ position: 'relative' }}>
            <button
              onClick={() => allowedTransitions.length > 0 && setStatusMenuOpen(v => !v)}
              disabled={transitioning || allowedTransitions.length === 0}
              style={{
                background: 'none',
                border: 'none',
                cursor: allowedTransitions.length > 0 ? 'pointer' : 'default',
                padding: 0,
                display: 'flex',
                alignItems: 'center',
                gap: 4,
              }}
            >
              <AnimatePresence mode="wait">
                <motion.span
                  key={matter.status}
                  initial={shouldReduce ? {} : { opacity: 0, scale: 0.92 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={shouldReduce ? {} : { opacity: 0, scale: 0.92 }}
                  transition={transition.fast}
                >
                  <MatterStatusBadge status={matter.status as MatterStatus} />
                </motion.span>
              </AnimatePresence>
              {allowedTransitions.length > 0 && (
                <span style={{ fontSize: 10, color: colors.textTertiary }}>▾</span>
              )}
            </button>

            {/* Status dropdown */}
            <AnimatePresence>
              {statusMenuOpen && (
                <motion.div
                  initial={shouldReduce ? { opacity: 0 } : { opacity: 0, y: -4 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -4 }}
                  transition={transition.fast}
                  style={{
                    position: 'absolute',
                    top: '100%',
                    left: 0,
                    marginTop: 6,
                    background: colors.bgPrimary,
                    border: `0.5px solid ${colors.border}`,
                    borderRadius: radius.card,
                    boxShadow: '0 4px 16px rgba(0,0,0,0.10)',
                    zIndex: 20,
                    minWidth: 160,
                    overflow: 'hidden',
                  }}
                >
                  {allowedTransitions.map(s => (
                    <button
                      key={s}
                      onClick={() => handleStatusChange(s)}
                      style={{
                        width: '100%',
                        textAlign: 'left',
                        padding: `${spacing[3]} ${spacing[4]}`,
                        border: 'none',
                        background: 'transparent',
                        cursor: 'pointer',
                        fontSize: fontSizes.body,
                        fontFamily: fonts.ui,
                        color: colors.textPrimary,
                      }}
                      onMouseEnter={e => (e.currentTarget.style.background = colors.bgSecondary)}
                      onMouseLeave={e => (e.currentTarget.style.background = 'transparent')}
                    >
                      → {STATUS_LABELS[s]}
                    </button>
                  ))}
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          <span style={{
            fontSize: fontSizes.label,
            color: colors.textSecondary,
            fontFamily: fonts.ui,
            padding: '2px 8px',
            borderRadius: radius.chip,
            background: colors.bgSecondary,
          }}>
            {matter.priority}
          </span>

          {matter.parties.find(p => p.isPrimary) && (
            <span style={{
              fontSize: fontSizes.label,
              color: colors.textSecondary,
              fontFamily: fonts.ui,
            }}>
              {matter.parties.find(p => p.isPrimary)?.name || 'Assigned'}
            </span>
          )}
        </div>
      </header>

      {/* ── Tab bar ─────────────────────────────────────────── */}
      <div style={{
        display: 'flex',
        borderBottom: `0.5px solid ${colors.border}`,
        padding: `0 ${spacing[8]}`,
        flexShrink: 0,
      }}>
        {TABS.map(tab => {
          const active = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              style={{
                padding: `${spacing[3]} ${spacing[1]}`,
                marginRight: spacing[6],
                border: 'none',
                borderBottom: active
                  ? `2px solid ${colors.accentPrimary}`
                  : '2px solid transparent',
                background: 'transparent',
                cursor: 'pointer',
                fontSize: fontSizes.body,
                fontFamily: fonts.ui,
                fontWeight: active ? 500 : 400,
                color: active ? colors.accentPrimary : colors.textSecondary,
                transition: shouldReduce ? 'none' : `color ${transition.fast.duration}s`,
              }}
            >
              {tab.label}
            </button>
          );
        })}
      </div>

      {/* ── Tab content ─────────────────────────────────────── */}
      <div style={{
        flex: 1,
        overflow: 'auto',
        padding: `${spacing[6]} ${spacing[8]}`,
      }}>
        <AnimatePresence mode="wait">
          <motion.div
            key={activeTab}
            initial={shouldReduce ? { opacity: 0 } : { opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={shouldReduce ? { opacity: 0 } : { opacity: 0, y: -4 }}
            transition={transition.fast}
          >
            {tabContent[activeTab]}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}
