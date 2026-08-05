// Date helpers for values coming from Keel.
//
// SQLite's datetime('now') produces 'YYYY-MM-DD HH:MM:SS' in UTC, with no zone
// suffix. JavaScript parses that shape as *local* time, which silently shifts the
// value by the machine's offset — in IST that is 5h30m, enough to make an 8-hour
// session look expired the moment it is created.
//
// Always route Keel DATETIME strings through parseKeelDateTime().
// Plain DATE values ('YYYY-MM-DD') are already parsed as UTC midnight by JS and
// do not need this.

/**
 * Parse a Keel DATETIME string ('YYYY-MM-DD HH:MM:SS', UTC) into a Date.
 * Also accepts ISO-8601 strings that already carry a zone, so it is safe to use
 * on any timestamp Keel returns. Returns null for empty or unparseable input.
 */
export function parseKeelDateTime(value: string | null | undefined): Date | null {
  if (!value) return null;

  const trimmed = value.trim();
  if (!trimmed) return null;

  // Already zone-qualified (ends in Z or ±HH:MM) — hand it straight to Date.
  const hasZone = /(?:Z|[+-]\d{2}:?\d{2})$/.test(trimmed);
  const normalised = hasZone ? trimmed : `${trimmed.replace(' ', 'T')}Z`;

  const date = new Date(normalised);
  return Number.isNaN(date.getTime()) ? null : date;
}

/**
 * Milliseconds until the given Keel DATETIME. Negative once it has passed.
 * Returns null when the value cannot be parsed.
 */
export function msUntil(value: string | null | undefined): number | null {
  const date = parseKeelDateTime(value);
  return date === null ? null : date.getTime() - Date.now();
}
