// One input, rendered from a template manifest.
//
// Nothing here knows what a trade mark is. The manifest says a field is seven
// digits, or a date that cannot precede another, or a choice from a list; this
// renders that. Adding a template to the library therefore needs no change to
// Deck — which is the whole point of the registry (PRD §9.8).
//
// A `list` field is the exception to "one input": it is a repeating group, and
// what it renders is a stack of rows with an add button under them. The rows
// are numbered here for the attorney to read, but the number that reaches the
// document is Keel's, taken from the same order — nothing in this file decides
// what a section is called.

import type { FieldRow, FieldSpec, FieldValue } from '@/lib/ipc-types';
import { rowErrorKey, rowsOf, scalarOf } from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';

interface Props {
  spec: FieldSpec;
  value: FieldValue | undefined;
  /** Every field error from the last render, keyed as Keel keyed it. A list
   *  needs the whole map: its own errors are keyed per row and per cell. */
  errors: Record<string, string>;
  onChange: (value: FieldValue) => void;
}

const inputBase: React.CSSProperties = {
  width: '100%',
  boxSizing: 'border-box',
  padding: `${spacing[2]} ${spacing[3]}`,
  borderRadius: radius.input,
  background: colors.bgPrimary,
  color: colors.textPrimary,
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  outline: 'none',
};

export function FormField({ spec, value, errors, onChange }: Props) {
  const error = errors[spec.key];
  const border = `0.5px solid ${error ? colors.statusUrgent : colors.border}`;
  const style = { ...inputBase, border };
  const text = scalarOf(value);

  return (
    <div style={{ marginBottom: spacing[5] }}>
      <label
        htmlFor={spec.key}
        style={{
          display: 'block',
          marginBottom: spacing[2],
          fontFamily: fonts.ui,
          fontSize: fontSizes.body,
          color: colors.textPrimary,
        }}
      >
        {spec.label}
        {!spec.required && (
          <span style={{ color: colors.textTertiary, fontSize: fontSizes.label }}>
            {' '}— optional
          </span>
        )}
      </label>

      {spec.kind.type === 'list' ? (
        <RowList spec={spec} rows={rowsOf(value)} errors={errors} onChange={onChange} />
      ) : (
        renderInput(spec, text, style, (next) => onChange(next))
      )}

      {/* The error replaces the help text rather than stacking under it: two
          lines of small print under one input is how neither gets read. */}
      {error ? (
        <p
          role="alert"
          style={{
            margin: `${spacing[2]} 0 0`,
            fontFamily: fonts.ui,
            fontSize: fontSizes.label,
            color: colors.statusUrgent,
          }}
        >
          {error}
        </p>
      ) : (
        spec.help && (
          <p
            style={{
              margin: `${spacing[2]} 0 0`,
              fontFamily: fonts.ui,
              fontSize: 11,
              color: colors.textTertiary,
            }}
          >
            {spec.help}
          </p>
        )
      )}

      {counter(spec, text)}
    </div>
  );
}

/**
 * The rows of a repeating group.
 *
 * The row's ordinal is shown because it is what the document will print, and
 * because "row 3" is how the error Keel sends back refers to it. Moving a row
 * renumbers everything under it, in the form and on the paper together — the
 * same rule as an annexure mark, for the same reason.
 */
function RowList({
  spec,
  rows,
  errors,
  onChange,
}: {
  spec: FieldSpec;
  rows: FieldRow[];
  errors: Record<string, string>;
  onChange: (rows: FieldRow[]) => void;
}) {
  if (spec.kind.type !== 'list') return null;
  const { itemFields, itemLabel, maxItems } = spec.kind;

  const setCell = (index: number, key: string, next: string) =>
    onChange(rows.map((row, i) => (i === index ? { ...row, [key]: next } : row)));

  const add = () =>
    onChange([...rows, Object.fromEntries(itemFields.map((f) => [f.key, '']))]);

  const remove = (index: number) => onChange(rows.filter((_, i) => i !== index));

  const move = (index: number, by: number) => {
    const target = index + by;
    if (target < 0 || target >= rows.length) return;
    const next = [...rows];
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };

  const full = maxItems != null && rows.length >= maxItems;

  return (
    <div>
      {rows.map((row, index) => (
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
                color: colors.textSecondary,
              }}
            >
              {index + 1}.
            </span>
            <span style={{ flex: 1 }} />
            <RowButton label="Move up" onClick={() => move(index, -1)} disabled={index === 0}>
              ↑
            </RowButton>
            <RowButton
              label="Move down"
              onClick={() => move(index, 1)}
              disabled={index === rows.length - 1}
            >
              ↓
            </RowButton>
            <RowButton label="Remove" onClick={() => remove(index)}>
              ×
            </RowButton>
          </div>

          {itemFields.map((item) => {
            const cellError = errors[rowErrorKey(spec.key, index, item.key)];
            return (
              <div key={item.key} style={{ marginBottom: spacing[3] }}>
                <label
                  htmlFor={`${spec.key}-${index}-${item.key}`}
                  style={{
                    display: 'block',
                    marginBottom: spacing[1],
                    fontFamily: fonts.ui,
                    fontSize: fontSizes.label,
                    color: colors.textSecondary,
                  }}
                >
                  {item.label}
                </label>

                {renderInput(
                  { ...item, key: `${spec.key}-${index}-${item.key}` },
                  row[item.key] ?? '',
                  {
                    ...inputBase,
                    border: `0.5px solid ${
                      cellError ? colors.statusUrgent : colors.border
                    }`,
                  },
                  (next) => setCell(index, item.key, next),
                )}

                {cellError ? (
                  <p
                    role="alert"
                    style={{
                      margin: `${spacing[1]} 0 0`,
                      fontFamily: fonts.ui,
                      fontSize: fontSizes.label,
                      color: colors.statusUrgent,
                    }}
                  >
                    {cellError}
                  </p>
                ) : (
                  item.help && (
                    <p
                      style={{
                        margin: `${spacing[1]} 0 0`,
                        fontFamily: fonts.ui,
                        fontSize: 11,
                        color: colors.textTertiary,
                      }}
                    >
                      {item.help}
                    </p>
                  )
                )}
              </div>
            );
          })}
        </div>
      ))}

      <button
        type="button"
        onClick={add}
        disabled={full}
        style={{
          padding: `${spacing[2]} ${spacing[4]}`,
          borderRadius: radius.button,
          border: `0.5px solid ${colors.border}`,
          background: 'transparent',
          color: full ? colors.textTertiary : colors.textSecondary,
          fontFamily: fonts.ui,
          fontSize: fontSizes.body,
          cursor: full ? 'not-allowed' : 'pointer',
        }}
      >
        + {itemLabel}
      </button>
    </div>
  );
}

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

function renderInput(
  spec: FieldSpec,
  value: string,
  style: React.CSSProperties,
  onChange: (v: string) => void,
) {
  const kind = spec.kind;

  switch (kind.type) {
    case 'multiline':
      return (
        <textarea
          id={spec.key}
          rows={6}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          style={{ ...style, resize: 'vertical', lineHeight: 1.55 }}
        />
      );

    case 'date':
      return (
        <input
          id={spec.key}
          type="date"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          style={style}
        />
      );

    case 'digits':
      return (
        <input
          id={spec.key}
          inputMode="numeric"
          maxLength={kind.length}
          value={value}
          // Non-digits are dropped as they are typed rather than flagged after
          // the fact — the field cannot hold anything else, so letting one in
          // only to reject it wastes the attorney's attention.
          onChange={(e) => onChange(e.target.value.replace(/\D/g, ''))}
          style={{ ...style, fontFamily: fonts.mono, letterSpacing: '0.08em' }}
        />
      );

    case 'number':
      return (
        <input
          id={spec.key}
          type="number"
          min={kind.min ?? undefined}
          max={kind.max ?? undefined}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          style={style}
        />
      );

    case 'select':
      return (
        <select
          id={spec.key}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          style={style}
        >
          <option value="">Choose…</option>
          {kind.options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      );

    case 'checkbox':
      return (
        <label
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: spacing[2],
            fontFamily: fonts.ui,
            fontSize: fontSizes.body,
            color: colors.textSecondary,
          }}
        >
          <input
            id={spec.key}
            type="checkbox"
            checked={value === 'true'}
            onChange={(e) => onChange(String(e.target.checked))}
          />
          Yes
        </label>
      );

    case 'list':
      // Handled by RowList, which has the rows. A list nested inside a row is
      // refused by the manifest check in services/templates.rs, so this is
      // unreachable for any template the library ships.
      return null;

    case 'computed':
      // Assembled by Keel. It should never reach this component; if it does,
      // showing nothing is better than showing an input that does nothing.
      return null;

    case 'text':
    default:
      return (
        <input
          id={spec.key}
          type="text"
          maxLength={kind.type === 'text' ? (kind.maxLength ?? undefined) : undefined}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          style={style}
        />
      );
  }
}

/** Live count where a registry imposes a limit — shown before it is exceeded,
 *  since a limit discovered on submit means rewriting rather than editing. */
function counter(spec: FieldSpec, value: string) {
  if (spec.kind.type !== 'multiline') return null;
  const { maxWords, maxLength } = spec.kind;
  if (!maxWords && !maxLength) return null;

  const used = maxWords ? value.trim().split(/\s+/).filter(Boolean).length : value.length;
  const limit = maxWords ?? maxLength!;
  const unit = maxWords ? 'words' : 'characters';
  const over = used > limit;

  return (
    <p
      style={{
        margin: `${spacing[1]} 0 0`,
        textAlign: 'right',
        fontFamily: fonts.ui,
        fontSize: 11,
        color: over ? colors.statusUrgent : colors.textTertiary,
      }}
    >
      {used} / {limit} {unit}
    </p>
  );
}

/** Whether a field is shown, given what is filled in so far.
 *
 *  Mirrors `is_visible` in services/templates.rs — including that a condition
 *  pointing at a hidden field is not satisfied, so a stale value cannot
 *  resurrect a field its parent hid. Keel is the enforcer; this is what stops
 *  the attorney being asked in the first place. */
export function isVisible(
  spec: FieldSpec,
  all: FieldSpec[],
  values: Record<string, FieldValue>,
): boolean {
  if (!spec.shownWhen) return true;

  const parent = all.find((f) => f.key === spec.shownWhen!.field);
  if (parent && !isVisible(parent, all, values)) return false;

  // A list can be hidden by a condition, but it cannot be the condition: what
  // would "equals" mean against a set of rows? Reading a list as '' means such
  // a condition is simply never satisfied, which is the safe direction — the
  // dependent field stays hidden rather than appearing on a comparison nobody
  // defined. The manifest check refuses the reverse (a condition inside a row).
  return spec.shownWhen.equals.includes(scalarOf(values[spec.shownWhen.field]));
}
