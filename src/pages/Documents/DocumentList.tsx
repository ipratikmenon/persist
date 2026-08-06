/**
 * DocumentList — All-firm document vault view.
 *
 * Shows every document across all matters in a searchable, filterable list.
 * Documents are encrypted at rest; clicking "Download" decrypts and saves locally.
 * Accessed via /documents
 */
import { useEffect, useState, useCallback } from 'react';
import { useNavigate } from 'react-router';
import { motion, AnimatePresence } from 'motion/react';
import { useReducedMotion } from 'motion/react';
import { save as saveDialog } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import type { DocumentCategory, DocumentMeta, MatterSummary } from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { useAuthStore } from '@/stores/auth';
import { can } from '@/lib/permissions';
import { colors, fonts, fontSizes, radius, spacing, shadows } from '@/design-system/tokens';
import { transition, stagger } from '@/design-system/motion';
import { AnimatedUploadDrawer } from './UploadDrawer';

// ---------------------------------------------------------------------------
// Category chips
// ---------------------------------------------------------------------------

const ALL_CATEGORIES: { value: DocumentCategory | 'All'; label: string }[] = [
  { value: 'All',           label: 'All'          },
  { value: 'Filing',        label: 'Filing'        },
  { value: 'Correspondence',label: 'Correspondence'},
  { value: 'Certificate',   label: 'Certificate'   },
  { value: 'SearchReport',  label: 'Search Report' },
  { value: 'Contract',      label: 'Contract'      },
  { value: 'Invoice',       label: 'Invoice'       },
  { value: 'Other',         label: 'Other'         },
];

const CATEGORY_COLORS: Record<DocumentCategory, string> = {
  Filing:         colors.accentPrimary,
  Correspondence: colors.statusWarning,
  Certificate:    colors.statusClear,
  SearchReport:   '#7B6FAB',
  Contract:       colors.accentSecondary,
  Invoice:        '#5B8A6E',
  Other:          colors.textTertiary,
};

function CategoryPill({ category }: { category: DocumentCategory }) {
  const color = CATEGORY_COLORS[category] ?? colors.textTertiary;
  const labels: Record<DocumentCategory, string> = {
    Filing: 'Filing',
    Correspondence: 'Corr.',
    Certificate: 'Cert.',
    SearchReport: 'Search',
    Contract: 'Contract',
    Invoice: 'Invoice',
    Other: 'Other',
  };
  return (
    <span style={{
      fontSize: 11,
      fontFamily: fonts.mono,
      color,
      letterSpacing: '0.04em',
      background: `${color}15`,
      padding: '2px 7px',
      borderRadius: 4,
      whiteSpace: 'nowrap',
    }}>
      {labels[category]}
    </span>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// ---------------------------------------------------------------------------
// Document row
// ---------------------------------------------------------------------------

const rowVariants = {
  hidden:  { opacity: 0, y: 6 },
  visible: { opacity: 1, y: 0, transition: transition.fast },
};

interface DocRowProps {
  doc: DocumentMeta;
  onDelete: (id: string) => void;
  onMatterClick: (matterId: string) => void;
  matterTitle: string;
}

function DocRow({ doc, onDelete, onMatterClick, matterTitle }: DocRowProps) {
  const shouldReduce = useReducedMotion();
  const [hovered, setHovered] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [sharing, setSharing] = useState(false);
  const [shared, setShared] = useState(doc.isSharedWithClient);

  const role = useAuthStore(s => s.session?.role);
  const canShare  = can(role, 'ShareDocumentWithClient');
  const canDelete = can(role, 'DeleteDocument');

  /**
   * Show or withdraw a document in the client portal.
   * Un-sharing queues a tombstone in Keel, so the mirror row and the stored
   * object are removed — not merely left un-refreshed.
   */
  const toggleShared = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setSharing(true);
    try {
      if (shared) {
        await keel.sharing.unshareDocument(doc.id);
      } else {
        await keel.sharing.shareDocument(doc.id);
      }
      setShared(!shared);
    } catch (err: unknown) {
      window.alert(err instanceof Error ? err.message : String(err));
    } finally {
      setSharing(false);
    }
  };

  const handleDownload = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setDownloading(true);
    try {
      const destPath = await saveDialog({ defaultPath: doc.filename });
      if (!destPath) return;
      const bytes = await keel.documents.get(doc.id);
      await writeFile(destPath, new Uint8Array(bytes));
    } finally {
      setDownloading(false);
    }
  };

  /**
   * Save a copy with metadata stripped, for sending to a client.
   * Keel refuses file types it cannot clean rather than handing back raw bytes,
   * so a refusal here means "do not send this file as-is".
   */
  const handleExport = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setExporting(true);
    try {
      const exported = await keel.documents.exportForClient(doc.id);
      const destPath = await saveDialog({ defaultPath: `clean-${exported.filename}` });
      if (!destPath) return;
      await writeFile(destPath, new Uint8Array(exported.bytes));

      // Tell the attorney what was taken out — silence would hide the point.
      const { removed } = exported.report;
      window.alert(
        removed.length > 0
          ? `Saved a client copy.\n\nRemoved:\n• ${removed.join('\n• ')}`
          : 'Saved a client copy. No metadata was found in this file.',
      );
    } catch (err: unknown) {
      window.alert(err instanceof Error ? err.message : String(err));
    } finally {
      setExporting(false);
    }
  };

  const handleDelete = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!window.confirm(`Delete "${doc.filename}"? This cannot be undone.`)) return;
    setDeleting(true);
    try {
      await keel.documents.delete(doc.id);
      onDelete(doc.id);
    } finally {
      setDeleting(false);
    }
  };

  const uploadedDate = new Date(doc.createdAt).toLocaleDateString('en-IN', {
    day: 'numeric', month: 'short', year: 'numeric',
  });

  return (
    <motion.tr
      variants={rowVariants}
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
      style={{
        background: hovered ? colors.bgSecondary : 'transparent',
        transition: shouldReduce ? 'none' : `background ${transition.fast.duration}s`,
      }}
    >
      {/* Filename + description */}
      <td style={{ padding: `${spacing[3]} ${spacing[5]}`, verticalAlign: 'middle' }}>
        <div style={{
          fontSize: fontSizes.body,
          fontWeight: 500,
          color: colors.textPrimary,
          fontFamily: fonts.ui,
          marginBottom: 2,
        }}>
          {doc.filename}
        </div>
        {doc.description && (
          <div style={{
            fontSize: fontSizes.label,
            color: colors.textTertiary,
            fontFamily: fonts.ui,
            maxWidth: 320,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            {doc.description}
          </div>
        )}
      </td>

      {/* Category */}
      <td style={{ padding: `${spacing[3]} ${spacing[4]}`, verticalAlign: 'middle', whiteSpace: 'nowrap' }}>
        <CategoryPill category={doc.category as DocumentCategory} />
      </td>

      {/* Matter */}
      <td style={{ padding: `${spacing[3]} ${spacing[4]}`, verticalAlign: 'middle' }}>
        <button
          onClick={() => onMatterClick(doc.matterId)}
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            padding: 0,
            textAlign: 'left',
            fontSize: fontSizes.body,
            color: colors.accentPrimary,
            fontFamily: fonts.ui,
            fontWeight: 500,
            textDecoration: hovered ? 'underline' : 'none',
          }}
        >
          {matterTitle || doc.matterId}
        </button>
      </td>

      {/* Size */}
      <td style={{
        padding: `${spacing[3]} ${spacing[4]}`,
        verticalAlign: 'middle',
        whiteSpace: 'nowrap',
        fontSize: fontSizes.label,
        color: colors.textTertiary,
        fontFamily: fonts.mono,
      }}>
        {formatBytes(doc.fileSizeBytes)}
      </td>

      {/* Uploaded */}
      <td style={{
        padding: `${spacing[3]} ${spacing[4]}`,
        verticalAlign: 'middle',
        whiteSpace: 'nowrap',
        fontSize: fontSizes.label,
        color: colors.textTertiary,
        fontFamily: fonts.ui,
      }}>
        {uploadedDate}
      </td>

      {/* Shared badge */}
      <td style={{ padding: `${spacing[3]} ${spacing[4]}`, verticalAlign: 'middle', whiteSpace: 'nowrap' }}>
        {shared && (
          <span style={{
            fontSize: 10,
            fontFamily: fonts.mono,
            color: colors.statusClear,
            letterSpacing: '0.04em',
            textTransform: 'uppercase' as const,
          }}>
            Shared
          </span>
        )}
      </td>

      {/* Actions */}
      <td style={{
        padding: `${spacing[3]} ${spacing[5]} ${spacing[3]} ${spacing[4]}`,
        verticalAlign: 'middle',
        textAlign: 'right' as const,
        whiteSpace: 'nowrap',
      }}>
        <AnimatePresence>
          {hovered && (
            <motion.div
              initial={{ opacity: 0, scale: 0.92 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.92 }}
              transition={transition.fast}
              style={{ display: 'flex', gap: spacing[2], justifyContent: 'flex-end' }}
            >
              <button
                onClick={handleDownload}
                disabled={downloading}
                style={actionBtnStyle(colors.accentPrimary, downloading)}
              >
                {downloading ? '…' : '↓ Save'}
              </button>
              <button
                onClick={handleExport}
                disabled={exporting}
                title="Save a copy with metadata stripped, for sending to a client"
                style={actionBtnStyle(colors.accentSecondary, exporting)}
              >
                {exporting ? '…' : '↓ Client copy'}
              </button>
              {canShare && (
                <button
                  onClick={toggleShared}
                  disabled={sharing}
                  title={shared
                    ? 'Withdraw from the client portal'
                    : 'Show this document in the client portal'}
                  style={actionBtnStyle(
                    shared ? colors.textSecondary : colors.statusClear,
                    sharing,
                  )}
                >
                  {sharing ? '…' : shared ? 'Unshare' : 'Share'}
                </button>
              )}
              {canDelete && (
                <button
                  onClick={handleDelete}
                  disabled={deleting}
                  style={actionBtnStyle(colors.statusUrgent, deleting)}
                >
                  {deleting ? '…' : 'Delete'}
                </button>
              )}
            </motion.div>
          )}
        </AnimatePresence>
      </td>
    </motion.tr>
  );
}

function actionBtnStyle(color: string, disabled: boolean): React.CSSProperties {
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
            color: filtered ? colors.textTertiary : colors.accentPrimary,
            fontWeight: 500,
          }}>
            {filtered ? 'No documents in this category' : 'Vault is empty'}
          </div>
          <p style={{
            fontSize: fontSizes.body,
            color: colors.textTertiary,
            margin: 0,
            maxWidth: 320,
            fontFamily: fonts.ui,
          }}>
            {filtered
              ? 'Try switching to "All" to see all documents.'
              : 'Upload your first document from a matter page.'}
          </p>
        </div>
      </td>
    </tr>
  );
}

// ---------------------------------------------------------------------------
// DocumentList page
// ---------------------------------------------------------------------------

export default function DocumentList() {
  const navigate = useNavigate();
  const shouldReduce = useReducedMotion();

  const [docs, setDocs] = useState<DocumentMeta[]>([]);
  const [matters, setMatters] = useState<MatterSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeCategory, setActiveCategory] = useState<DocumentCategory | 'All'>('All');
  const [uploadOpen, setUploadOpen] = useState(false);
  const [uploadMatterId, setUploadMatterId] = useState<string | null>(null);

  const loadAll = useCallback(async () => {
    setLoading(true);
    try {
      // Load all matters then collect all docs
      const allMatters = await keel.matters.list({});
      setMatters(allMatters);

      const docArrays = await Promise.all(
        allMatters.map(m => keel.documents.list(m.id))
      );
      setDocs(docArrays.flat());
    } catch {
      setDocs([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadAll(); }, [loadAll]);

  const handleDelete = (id: string) => setDocs(prev => prev.filter(d => d.id !== id));

  const filtered = activeCategory === 'All'
    ? docs
    : docs.filter(d => d.category === activeCategory);

  const counts: Record<string, number> = {};
  for (const d of docs) counts[d.category] = (counts[d.category] ?? 0) + 1;

  const matterTitles = Object.fromEntries(matters.map(m => [m.id, m.title]));

  const listVariants = {
    hidden:  {},
    visible: { transition: shouldReduce ? {} : stagger.normal },
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* ── Header ─────────────────────────────────────────── */}
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
            Document Vault
          </h1>
          <p style={{
            fontSize: fontSizes.label,
            color: colors.textTertiary,
            margin: 0,
            fontFamily: fonts.ui,
          }}>
            {loading
              ? 'Loading…'
              : `${docs.length} document${docs.length !== 1 ? 's' : ''} · AES-256 encrypted at rest`}
          </p>
        </div>

        <div style={{ display: 'flex', gap: spacing[3] }}>
          <button
            onClick={() => {
              // Use first matter if available, else do nothing (upload from matter page)
              if (matters.length > 0) {
                setUploadMatterId(matters[0].id);
                setUploadOpen(true);
              }
            }}
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
            + Upload
          </button>
        </div>
      </header>

      {/* ── Category filter bar ─────────────────────────────── */}
      <div style={{
        padding: `${spacing[3]} ${spacing[8]}`,
        borderBottom: `0.5px solid ${colors.border}`,
        display: 'flex',
        gap: spacing[2],
        flexWrap: 'wrap',
        flexShrink: 0,
      }}>
        {ALL_CATEGORIES.map(chip => {
          const active = chip.value === activeCategory;
          const count = chip.value === 'All' ? docs.length : (counts[chip.value] ?? 0);
          return (
            <button
              key={chip.value}
              onClick={() => setActiveCategory(chip.value)}
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

      {/* ── Table ───────────────────────────────────────────── */}
      <div style={{ flex: 1, overflow: 'auto', padding: `0 ${spacing[8]}` }}>
        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
          <thead>
            <tr style={{ borderBottom: `0.5px solid ${colors.border}` }}>
              {['Document', 'Category', 'Matter', 'Size', 'Uploaded', '', ''].map((h, i) => (
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
              key={activeCategory}
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
                    Loading vault…
                  </td>
                </tr>
              ) : filtered.length === 0 ? (
                <EmptyState filtered={activeCategory !== 'All'} />
              ) : (
                filtered.map(doc => (
                  <DocRow
                    key={doc.id}
                    doc={doc}
                    matterTitle={matterTitles[doc.matterId] ?? doc.matterId}
                    onDelete={handleDelete}
                    onMatterClick={(id) => navigate(`/matters/${id}`)}
                  />
                ))
              )}
            </motion.tbody>
          </AnimatePresence>
        </table>
      </div>

      {/* Upload drawer */}
      <AnimatedUploadDrawer
        open={uploadOpen && uploadMatterId !== null}
        matterId={uploadMatterId ?? ''}
        onClose={() => setUploadOpen(false)}
        onUploaded={loadAll}
      />
    </div>
  );
}

// keep shadows import from triggering unused warning
void shadows;
