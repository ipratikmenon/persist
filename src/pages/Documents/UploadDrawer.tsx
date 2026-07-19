/**
 * UploadDrawer — Slide-in drawer for uploading an encrypted document.
 *
 * Opens a native file picker via Tauri dialog plugin.
 * Reads bytes via Tauri FS plugin.
 * Passes bytes + metadata to keel.documents.upload().
 */
import { useState } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { useReducedMotion } from 'motion/react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { readFile } from '@tauri-apps/plugin-fs';
import type { DocumentCategory } from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';
import { panelVariants } from '@/design-system/motion';

// ---------------------------------------------------------------------------
// Category options
// ---------------------------------------------------------------------------

const CATEGORIES: { value: DocumentCategory; label: string }[] = [
  { value: 'Filing',          label: 'Filing'           },
  { value: 'Correspondence',  label: 'Correspondence'   },
  { value: 'Certificate',     label: 'Certificate'      },
  { value: 'SearchReport',    label: 'Search Report'    },
  { value: 'Contract',        label: 'Contract'         },
  { value: 'Invoice',         label: 'Invoice'          },
  { value: 'Other',           label: 'Other'            },
];

function mimeFromFilename(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase() ?? '';
  const map: Record<string, string> = {
    pdf: 'application/pdf',
    doc: 'application/msword',
    docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    xls: 'application/vnd.ms-excel',
    xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    png: 'image/png',
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    txt: 'text/plain',
  };
  return map[ext] ?? 'application/octet-stream';
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface UploadDrawerProps {
  matterId: string;
  onClose: () => void;
  onUploaded: () => void; // callback to refresh parent list
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function UploadDrawer({ matterId, onClose, onUploaded }: UploadDrawerProps) {
  const shouldReduce = useReducedMotion();

  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [filename, setFilename] = useState('');
  const [category, setCategory] = useState<DocumentCategory>('Filing');
  const [description, setDescription] = useState('');
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleFilePick = async () => {
    const result = await openDialog({
      multiple: false,
      directory: false,
    });
    if (typeof result === 'string') {
      setSelectedPath(result);
      // Use the basename as default filename
      const parts = result.replace(/\\/g, '/').split('/');
      setFilename(parts[parts.length - 1] ?? result);
    }
  };

  const handleUpload = async () => {
    if (!selectedPath || !filename.trim()) return;
    setUploading(true);
    setError(null);
    try {
      const rawBytes = await readFile(selectedPath);
      await keel.documents.upload({
        matterId,
        filename: filename.trim(),
        category,
        mimeType: mimeFromFilename(filename),
        description: description.trim() || undefined,
        bytes: Array.from(rawBytes),
      });
      onUploaded();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setUploading(false);
    }
  };

  const canSubmit = selectedPath !== null && filename.trim().length > 0 && !uploading;

  return (
    <>
      {/* Backdrop */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        onClick={onClose}
        style={{
          position: 'fixed', inset: 0,
          background: 'rgba(44,44,42,0.35)',
          zIndex: 40,
        }}
      />

      {/* Drawer */}
      <motion.aside
        variants={shouldReduce ? {} : panelVariants.right}
        initial="hidden"
        animate="visible"
        exit="hidden"
        style={{
          position: 'fixed',
          top: 0, right: 0, bottom: 0,
          width: 400,
          background: colors.bgPrimary,
          borderLeft: `0.5px solid ${colors.border}`,
          boxShadow: '-8px 0 32px rgba(0,0,0,0.12)',
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
          alignItems: 'center',
          justifyContent: 'space-between',
          flexShrink: 0,
        }}>
          <div>
            <h2 style={{
              fontFamily: fonts.display,
              fontSize: fontSizes.displaySm,
              fontWeight: 600,
              color: colors.textPrimary,
              margin: 0,
            }}>
              Upload document
            </h2>
            <p style={{
              fontSize: fontSizes.label,
              color: colors.textTertiary,
              margin: '4px 0 0',
              fontFamily: fonts.ui,
            }}>
              Encrypted at rest · AES-256-GCM
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
          >
            ×
          </button>
        </div>

        {/* Body */}
        <div style={{
          flex: 1,
          overflowY: 'auto',
          padding: spacing[6],
          display: 'flex',
          flexDirection: 'column',
          gap: spacing[5],
        }}>
          {/* File picker */}
          <div>
            <label style={labelStyle}>File</label>
            <button
              onClick={handleFilePick}
              style={{
                width: '100%',
                padding: `${spacing[4]} ${spacing[4]}`,
                borderRadius: radius.button,
                border: `0.5px solid ${selectedPath ? colors.accentPrimary : colors.border}`,
                background: selectedPath ? 'rgba(74,101,128,0.05)' : colors.bgSecondary,
                color: selectedPath ? colors.textPrimary : colors.textTertiary,
                fontFamily: fonts.ui,
                fontSize: fontSizes.body,
                cursor: 'pointer',
                textAlign: 'left' as const,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {selectedPath
                ? selectedPath.replace(/\\/g, '/').split('/').pop()
                : '↑  Choose file…'}
            </button>
          </div>

          {/* Filename (editable) */}
          <div>
            <label style={labelStyle}>Filename</label>
            <input
              value={filename}
              onChange={e => setFilename(e.target.value)}
              placeholder="e.g. TM-Application-Form.pdf"
              style={inputStyle}
            />
          </div>

          {/* Category */}
          <div>
            <label style={labelStyle}>Category</label>
            <select
              value={category}
              onChange={e => setCategory(e.target.value as DocumentCategory)}
              style={{ ...inputStyle, appearance: 'none' as const }}
            >
              {CATEGORIES.map(c => (
                <option key={c.value} value={c.value}>{c.label}</option>
              ))}
            </select>
          </div>

          {/* Description */}
          <div>
            <label style={labelStyle}>Description <OptLabel /></label>
            <input
              value={description}
              onChange={e => setDescription(e.target.value)}
              placeholder="Short note about this document"
              style={inputStyle}
            />
          </div>

          {/* Error */}
          {error && (
            <div style={{
              padding: `${spacing[3]} ${spacing[4]}`,
              background: 'rgba(192,57,43,0.06)',
              border: `0.5px solid rgba(192,57,43,0.25)`,
              borderRadius: radius.card,
              fontSize: fontSizes.label,
              color: colors.statusUrgent,
              fontFamily: fonts.ui,
            }}>
              {error}
            </div>
          )}
        </div>

        {/* Footer */}
        <div style={{
          padding: `${spacing[4]} ${spacing[6]}`,
          borderTop: `0.5px solid ${colors.border}`,
          display: 'flex',
          gap: spacing[3],
          flexShrink: 0,
        }}>
          <button
            onClick={onClose}
            style={{
              flex: 1,
              padding: `${spacing[3]} 0`,
              borderRadius: radius.button,
              border: `0.5px solid ${colors.border}`,
              background: 'transparent',
              color: colors.textSecondary,
              fontFamily: fonts.ui,
              fontSize: fontSizes.body,
              cursor: 'pointer',
            }}
          >
            Cancel
          </button>
          <button
            onClick={handleUpload}
            disabled={!canSubmit}
            style={{
              flex: 2,
              padding: `${spacing[3]} 0`,
              borderRadius: radius.button,
              border: 'none',
              background: canSubmit ? colors.accentPrimary : colors.border,
              color: canSubmit ? '#fff' : colors.textTertiary,
              fontFamily: fonts.ui,
              fontSize: fontSizes.body,
              fontWeight: 500,
              cursor: canSubmit ? 'pointer' : 'default',
            }}
          >
            {uploading ? 'Uploading…' : 'Upload & encrypt'}
          </button>
        </div>
      </motion.aside>
    </>
  );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const labelStyle: React.CSSProperties = {
  display: 'block',
  fontSize: fontSizes.label,
  fontWeight: 500,
  color: colors.textTertiary,
  fontFamily: fonts.ui,
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
  marginBottom: spacing[2],
};

const inputStyle: React.CSSProperties = {
  width: '100%',
  padding: `${spacing[3]} ${spacing[3]}`,
  borderRadius: radius.button,
  border: `0.5px solid ${colors.border}`,
  background: colors.bgSecondary,
  color: colors.textPrimary,
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  outline: 'none',
  boxSizing: 'border-box',
};

function OptLabel() {
  return (
    <span style={{
      fontSize: 10,
      fontFamily: fonts.mono,
      color: colors.textTertiary,
      marginLeft: 6,
      textTransform: 'uppercase' as const,
      letterSpacing: '0.04em',
      fontWeight: 400,
    }}>
      optional
    </span>
  );
}

// ---------------------------------------------------------------------------
// Animated wrapper (for AnimatePresence in parent)
// ---------------------------------------------------------------------------

interface AnimatedUploadDrawerProps extends UploadDrawerProps {
  open: boolean;
}

export function AnimatedUploadDrawer({ open, ...props }: AnimatedUploadDrawerProps) {
  return (
    <AnimatePresence>
      {open && <UploadDrawer {...props} />}
    </AnimatePresence>
  );
}
