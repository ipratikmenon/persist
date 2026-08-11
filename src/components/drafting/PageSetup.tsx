// Page setup.
//
// The point of this panel is that the drafter never formats a document. They
// choose the paper and the typeface once, here, and Persist lays the whole
// thing out — every page, the letterhead, the numbering — instead of leaving
// margins and headers to be fixed by hand in a word processor afterwards.
//
// None of these are template fields. A template says what a legal notice says;
// what size the paper is has nothing to do with that. Putting paper size into
// nineteen manifests would mean nineteen places to change it.

import { useState } from 'react';
import type {
  BodyFont,
  DocumentLayout,
  Letterhead,
  LineSpacing,
  Margins,
  PageNumbers,
  Paper,
} from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';

interface Props {
  layout: DocumentLayout;
  onChange: (next: DocumentLayout) => void;
}

const PAPERS: Array<[Paper, string]> = [
  ['a4', 'A4 — 210 × 297 mm'],
  ['legal', 'Legal — 8.5 × 14 in'],
];

const FONTS: Array<[BodyFont, string]> = [
  ['notoSerif', 'Noto Serif (firm default)'],
  ['times', 'Times New Roman'],
  ['pagella', 'Palatino'],
  ['schola', 'Century Schoolbook'],
  ['notoSans', 'Noto Sans'],
  ['helvetica', 'Helvetica'],
];

const SPACINGS: Array<[LineSpacing, string]> = [
  ['single', 'Single'],
  ['oneAndHalf', '1.5 lines'],
  ['double', 'Double'],
];

const NUMBERING: Array<[PageNumbers, string]> = [
  ['pageOfTotal', 'Page 3 of 19'],
  ['page', 'Page 3'],
  ['plain', '3'],
  ['none', 'No page numbers'],
];

type LetterheadChoice = Letterhead['type'];

const LETTERHEADS: Array<[LetterheadChoice, string]> = [
  ['allPages', 'Every page'],
  ['firstPageOnly', 'First page only'],
  ['pages', 'Chosen pages…'],
  ['none', 'No letterhead'],
];

const SIZES = [10, 11, 12, 13, 14, 16];

// Four boxes rather than Normal/Narrow/Wide. A direction to leave 40mm on the
// left and 20mm elsewhere — a binding margin, which Indian forums do ask for —
// has no preset, and an attorney holding such a direction should be able to
// type what it says.
const MARGIN_SIDES: Array<[keyof Margins, string]> = [
  ['topMm', 'Top'],
  ['bottomMm', 'Bottom'],
  ['leftMm', 'Left'],
  ['rightMm', 'Right'],
];

export function PageSetup({ layout, onChange }: Props) {
  const [open, setOpen] = useState(false);
  const set = (patch: Partial<DocumentLayout>) => onChange({ ...layout, ...patch });

  // The page list is edited as text, so a half-typed "1, " does not become a
  // layout Keel would reject on every keystroke.
  const [pageText, setPageText] = useState(
    layout.letterhead.type === 'pages' ? layout.letterhead.pages.join(', ') : '',
  );

  const commitPages = (text: string) => {
    setPageText(text);
    const pages = text
      .split(',')
      .map((part) => Number.parseInt(part.trim(), 10))
      .filter((n) => Number.isFinite(n) && n > 0);
    if (pages.length > 0) set({ letterhead: { type: 'pages', pages } });
  };

  // Taken as typed rather than clamped on each keystroke: clamping would turn
  // the "4" on the way to "40" into the smallest margin allowed, and the box
  // would fight the person typing in it. Keel refuses the ones that cannot be
  // printed, in a sentence.
  const setMargin = (side: keyof Margins, typed: string) => {
    const mm = Number(typed);
    if (!Number.isFinite(mm)) return;
    set({ margins: { ...layout.margins, [side]: mm } });
  };

  const chooseLetterhead = (choice: LetterheadChoice) => {
    if (choice === 'pages') {
      commitPages(pageText || '1');
      return;
    }
    set({ letterhead: { type: choice } });
  };

  return (
    <section
      style={{
        marginTop: spacing[5],
        border: `0.5px solid ${colors.border}`,
        borderRadius: radius.card,
        background: colors.bgSecondary,
      }}
    >
      <button
        type="button"
        onClick={() => setOpen(!open)}
        aria-expanded={open}
        style={{
          width: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: spacing[4],
          border: 'none',
          background: 'transparent',
          cursor: 'pointer',
          fontFamily: fonts.ui,
          fontSize: fontSizes.cardTitle,
          fontWeight: 500,
          color: colors.textPrimary,
        }}
      >
        <span>Page setup</span>
        <span
          style={{
            fontFamily: fonts.ui,
            fontSize: fontSizes.label,
            fontWeight: 400,
            color: colors.textSecondary,
          }}
        >
          {summarise(layout)} {open ? '▴' : '▾'}
        </span>
      </button>

      {open && (
        <div style={{ padding: `0 ${spacing[4]} ${spacing[4]}` }}>
          <Row>
            <Choice
              label="Paper"
              value={layout.paper}
              options={PAPERS}
              onChange={(paper) => set({ paper })}
            />
            <Choice
              label="Typeface"
              value={layout.font}
              options={FONTS}
              onChange={(font) => set({ font })}
            />
          </Row>

          <Row>
            {MARGIN_SIDES.map(([side, label]) => (
              <Field key={side} label={`${label} margin`}>
                <input
                  type="number"
                  min={5}
                  max={100}
                  step={1}
                  value={layout.margins[side]}
                  onChange={(e) => setMargin(side, e.target.value)}
                  style={controlStyle}
                />
              </Field>
            ))}
          </Row>
          <p style={hintStyle}>
            Margins in millimetres, measured from the edge of the sheet. A left
            margin wider than the others is what a forum means when it asks for
            a margin for binding.
          </p>

          <Row>
            <Choice
              label="Size"
              value={String(layout.fontSizePt)}
              options={SIZES.map((n) => [String(n), `${n} pt`] as [string, string])}
              onChange={(v) => set({ fontSizePt: Number(v) })}
            />
            <Choice
              label="Line spacing"
              value={layout.lineSpacing}
              options={SPACINGS}
              onChange={(lineSpacing) => set({ lineSpacing })}
            />
          </Row>

          <Row>
            <Choice
              label="Page numbers"
              value={layout.pageNumbers}
              options={NUMBERING}
              onChange={(pageNumbers) => set({ pageNumbers })}
            />
            <Field label="First page numbered">
              <input
                type="number"
                min={1}
                value={layout.pageNumberStart}
                disabled={layout.pageNumbers === 'none'}
                onChange={(e) =>
                  set({ pageNumberStart: Math.max(1, Number(e.target.value) || 1) })
                }
                style={controlStyle}
              />
            </Field>
          </Row>

          <Row>
            <Choice
              label="Letterhead on"
              value={layout.letterhead.type}
              options={LETTERHEADS}
              onChange={chooseLetterhead}
            />
            {layout.letterhead.type === 'pages' ? (
              <Field label="Which pages">
                <input
                  value={pageText}
                  onChange={(e) => commitPages(e.target.value)}
                  placeholder="1, 5, 9"
                  style={controlStyle}
                />
              </Field>
            ) : (
              <div style={{ flex: 1 }} />
            )}
          </Row>

          <div style={{ display: 'flex', gap: spacing[5], marginTop: spacing[4] }}>
            <Toggle
              label="Bold throughout"
              checked={layout.bold}
              onChange={(bold) => set({ bold })}
            />
            <Toggle
              label="Italic throughout"
              checked={layout.italic}
              onChange={(italic) => set({ italic })}
            />
          </div>

          <p style={hintStyle}>
            First page only sets the letterhead at the top of page one and leaves
            the rest as plain continuation sheets — with a normal top margin, not
            a blank band.
          </p>
        </div>
      )}
    </section>
  );
}

/** The one-line state shown when the panel is shut. */
function summarise(layout: DocumentLayout): string {
  const paper = layout.paper === 'legal' ? 'Legal' : 'A4';
  const face = FONTS.find(([value]) => value === layout.font)?.[1].split(' (')[0] ?? '';
  return `${paper} · ${face} ${layout.fontSizePt}pt`;
}

// ---------------------------------------------------------------------------

function Row({ children }: { children: React.ReactNode }) {
  return (
    <div style={{ display: 'flex', gap: spacing[4], marginTop: spacing[3] }}>{children}</div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label style={{ flex: 1, display: 'block' }}>
      <span
        style={{
          display: 'block',
          fontFamily: fonts.ui,
          fontSize: fontSizes.label,
          color: colors.textSecondary,
          marginBottom: spacing[1],
        }}
      >
        {label}
      </span>
      {children}
    </label>
  );
}

function Choice<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: Array<[T, string]>;
  onChange: (value: T) => void;
}) {
  return (
    <Field label={label}>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as T)}
        style={{ ...controlStyle, cursor: 'pointer' }}
      >
        {options.map(([optionValue, optionLabel]) => (
          <option key={optionValue} value={optionValue}>
            {optionLabel}
          </option>
        ))}
      </select>
    </Field>
  );
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: spacing[2],
        fontFamily: fonts.ui,
        fontSize: fontSizes.body,
        color: colors.textPrimary,
        cursor: 'pointer',
      }}
    >
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      {label}
    </label>
  );
}

const hintStyle: React.CSSProperties = {
  margin: `${spacing[4]} 0 0`,
  fontFamily: fonts.ui,
  fontSize: fontSizes.label,
  color: colors.textTertiary,
};

const controlStyle: React.CSSProperties = {
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
