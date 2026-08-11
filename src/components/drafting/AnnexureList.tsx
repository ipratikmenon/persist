// Attaching proof to a document.
//
// The whole interaction is: tick the box, add an annexure, name it, choose a
// file. The file is combined into the PDF as it is — no cover sheet, no
// caption, nothing printed over it except the mark in the corner.
//
// The attorney does not choose the mark. Persist allocates A, B, C from this
// order, so moving a row renumbers the printed list and the stamp on every
// attached page together. The marks shown here come back from Keel after a
// render rather than being worked out again in TypeScript, because two
// implementations of the same rule eventually disagree and the attorney would
// be reading a mark the document does not carry.

import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { keel } from '@/lib/tauri';
import type { AnnexureMark, DraftAnnexure } from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';

interface Props {
  annexures: DraftAnnexure[];
  /** Marks from the last successful render, keyed by staged id. */
  marks: AnnexureMark[];
  onChange: (next: DraftAnnexure[]) => void;
}

export function AnnexureList({ annexures, marks, onChange }: Props) {
  // The section is off until asked for. Most notices carry no annexures, and an
  // empty attachment list on every draft is a thing to ignore rather than use.
  const [enabled, setEnabled] = useState(annexures.length > 0);
  const [busy, setBusy] = useState<number | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const markFor = (stagedId: string | null) =>
    stagedId ? marks.find((m) => m.stagedId === stagedId)?.mark : undefined;

  const update = (index: number, patch: Partial<DraftAnnexure>) =>
    onChange(annexures.map((row, i) => (i === index ? { ...row, ...patch } : row)));

  const remove = (index: number) => {
    const row = annexures[index];
    if (row.stagedId) void keel.drafting.discardAnnexure(row.stagedId);
    onChange(annexures.filter((_, i) => i !== index));
  };

  const move = (index: number, by: number) => {
    const target = index + by;
    if (target < 0 || target >= annexures.length) return;
    const next = [...annexures];
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };

  /** Pick a file and hand it to Keel, which reads, checks and cleans it. */
  const attach = async (index: number) => {
    setProblem(null);
    const picked = await open({
      multiple: false,
      filters: [{ name: 'Annexure', extensions: ['pdf', 'png', 'jpg', 'jpeg'] }],
    });
    if (typeof picked !== 'string') return;

    setBusy(index);
    try {
      const staged = await keel.drafting.stageAnnexure(picked);
      // Replacing a file leaves the old one held in Keel until it is discarded.
      const previous = annexures[index].stagedId;
      if (previous) void keel.drafting.discardAnnexure(previous);
      update(index, { stagedId: staged.id, filename: staged.filename });
    } catch (e) {
      setProblem(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <section style={{ marginTop: spacing[5] }}>
      <label
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: spacing[2],
          fontFamily: fonts.ui,
          fontSize: fontSizes.cardTitle,
          fontWeight: 500,
          color: colors.textPrimary,
          cursor: 'pointer',
        }}
      >
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => {
            setEnabled(e.target.checked);
            if (!e.target.checked) {
              annexures.forEach((row) => {
                if (row.stagedId) void keel.drafting.discardAnnexure(row.stagedId);
              });
              onChange([]);
            }
          }}
        />
        Attach annexures
      </label>

      {enabled && (
        <>
          <p
            style={{
              margin: `${spacing[1]} 0 ${spacing[4]}`,
              fontFamily: fonts.ui,
              fontSize: fontSizes.label,
              color: colors.textSecondary,
            }}
          >
            Combined into the PDF in this order and marked automatically. PDF, PNG
            or JPEG.
          </p>

          {annexures.map((row, index) => {
            const mark = markFor(row.stagedId);
            return (
              <div
                key={index}
                style={{
                  border: `0.5px solid ${colors.border}`,
                  borderRadius: radius.card,
                  background: colors.bgPrimary,
                  padding: spacing[4],
                  marginBottom: spacing[3],
                }}
              >
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: spacing[3],
                    marginBottom: spacing[3],
                  }}
                >
                  <span
                    style={{
                      fontFamily: fonts.mono,
                      fontSize: fontSizes.mono,
                      color: mark ? colors.textPrimary : colors.textTertiary,
                    }}
                  >
                    {mark ? `Annexure-${mark}` : `${index + 1}.`}
                  </span>
                  <span style={{ flex: 1 }} />
                  <RowButton label="Move up" onClick={() => move(index, -1)} disabled={index === 0}>
                    ↑
                  </RowButton>
                  <RowButton
                    label="Move down"
                    onClick={() => move(index, 1)}
                    disabled={index === annexures.length - 1}
                  >
                    ↓
                  </RowButton>
                  <RowButton label="Remove" onClick={() => remove(index)}>
                    ×
                  </RowButton>
                </div>

                <input
                  value={row.title}
                  onChange={(e) => update(index, { title: e.target.value })}
                  placeholder="Name this annexure — e.g. Receipt one"
                  style={inputStyle}
                />

                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: spacing[3],
                    marginTop: spacing[3],
                  }}
                >
                  <button
                    type="button"
                    onClick={() => void attach(index)}
                    disabled={busy === index}
                    style={{
                      padding: `${spacing[1]} ${spacing[3]}`,
                      borderRadius: radius.button,
                      border: `0.5px solid ${colors.accentPrimary}`,
                      background: 'transparent',
                      color: colors.accentPrimary,
                      fontFamily: fonts.ui,
                      fontSize: fontSizes.label,
                      cursor: busy === index ? 'progress' : 'pointer',
                    }}
                  >
                    {busy === index
                      ? 'Attaching…'
                      : row.filename
                        ? 'Replace file'
                        : 'Choose file'}
                  </button>
                  <span
                    style={{
                      fontFamily: fonts.ui,
                      fontSize: fontSizes.label,
                      color: row.filename ? colors.textSecondary : colors.textTertiary,
                      overflowWrap: 'anywhere',
                    }}
                  >
                    {row.filename ?? 'No file attached yet'}
                  </span>
                </div>
              </div>
            );
          })}

          <button
            type="button"
            onClick={() => onChange([...annexures, { title: '', stagedId: null, filename: null }])}
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
            + Add annexure
          </button>

          {problem && (
            <p
              role="alert"
              style={{
                margin: `${spacing[3]} 0 0`,
                fontFamily: fonts.ui,
                fontSize: fontSizes.label,
                color: colors.statusUrgent,
              }}
            >
              {problem}
            </p>
          )}
        </>
      )}
    </section>
  );
}

const inputStyle: React.CSSProperties = {
  width: '100%',
  boxSizing: 'border-box',
  padding: spacing[2],
  borderRadius: radius.input,
  border: `0.5px solid ${colors.border}`,
  background: colors.bgPrimary,
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  color: colors.textPrimary,
};

function RowButton({
  label,
  onClick,
  disabled,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      disabled={disabled}
      style={{
        width: 24,
        height: 24,
        padding: 0,
        borderRadius: radius.input,
        border: `0.5px solid ${colors.border}`,
        background: 'transparent',
        color: disabled ? colors.textTertiary : colors.textSecondary,
        fontFamily: fonts.ui,
        fontSize: fontSizes.body,
        lineHeight: 1,
        cursor: disabled ? 'not-allowed' : 'pointer',
      }}
    >
      {children}
    </button>
  );
}
