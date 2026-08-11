// One input, rendered from a template manifest.
//
// Nothing here knows what a trade mark is. The manifest says a field is seven
// digits, or a date that cannot precede another, or a choice from a list; this
// renders that. Adding a template to the library therefore needs no change to
// Deck — which is the whole point of the registry (PRD §9.8).

import type { FieldSpec } from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';

interface Props {
  spec: FieldSpec;
  value: string;
  error?: string;
  onChange: (value: string) => void;
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

export function FormField({ spec, value, error, onChange }: Props) {
  const border = `0.5px solid ${error ? colors.statusUrgent : colors.border}`;
  const style = { ...inputBase, border };

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

      {renderInput(spec, value, style, onChange)}

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

      {counter(spec, value)}
    </div>
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
  values: Record<string, string>,
): boolean {
  if (!spec.shownWhen) return true;

  const parent = all.find((f) => f.key === spec.shownWhen!.field);
  if (parent && !isVisible(parent, all, values)) return false;

  return spec.shownWhen.equals.includes(values[spec.shownWhen.field] ?? '');
}
