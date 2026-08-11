// The document-type picker.
//
// Templates are grouped by practice area, because that is how an attorney looks
// for one — "I need a TM thing" long before "I need the examination reply".
// Version and revision date are shown on the card: an associate should be able
// to see which version they are about to use without opening it (PRD §9.8).

import { useEffect, useState } from 'react';
import { Link, useSearchParams } from 'react-router';
import { motion, useReducedMotion } from 'motion/react';
import { keel } from '@/lib/tauri';
import type { TemplateManifest } from '@/lib/ipc-types';
import { colors, fonts, fontSizes, radius, shadows, spacing } from '@/design-system/tokens';

export function DraftingHome() {
  const [params] = useSearchParams();
  const matterId = params.get('matter');
  const shouldReduce = useReducedMotion();

  const [templates, setTemplates] = useState<TemplateManifest[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    keel.drafting
      .listTemplates()
      .then(setTemplates)
      .catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  if (error) {
    return (
      <p style={{ fontFamily: fonts.ui, fontSize: fontSizes.body, color: colors.statusUrgent }}>
        {error}
      </p>
    );
  }
  if (!templates) {
    return <p style={{ fontFamily: fonts.ui, color: colors.textTertiary }}>Loading…</p>;
  }

  // Billing templates are driven from an invoice record, not from a form, so
  // offering one here would open a form with nothing an attorney can fill.
  const draftable = templates.filter(
    (t) => !t.fields.every((f) => f.kind.type === 'computed'),
  );

  const byCategory = draftable.reduce<Record<string, TemplateManifest[]>>((acc, t) => {
    (acc[t.category] ??= []).push(t);
    return acc;
  }, {});

  return (
    <>
      <header style={{ marginBottom: spacing[6] }}>
        <h1
          style={{
            margin: 0,
            fontFamily: fonts.display,
            fontSize: fontSizes.displayLg,
            fontWeight: 600,
            color: colors.textPrimary,
          }}
        >
          Draft a document
        </h1>
        <p
          style={{
            margin: `${spacing[2]} 0 0`,
            fontFamily: fonts.ui,
            fontSize: fontSizes.body,
            color: colors.textSecondary,
          }}
        >
          Fill a form; the formatting is the template&rsquo;s problem.
          {matterId && ` Filing to ${matterId}.`}
        </p>
      </header>

      {draftable.length === 0 ? (
        <p style={{ fontFamily: fonts.ui, color: colors.textTertiary }}>
          No templates in the library yet.
        </p>
      ) : (
        Object.entries(byCategory).map(([category, group]) => (
          <section key={category} style={{ marginBottom: spacing[8] }}>
            <div
              style={{
                fontFamily: fonts.ui,
                fontSize: fontSizes.label,
                fontWeight: 600,
                letterSpacing: '0.06em',
                textTransform: 'uppercase',
                color: colors.textSecondary,
                marginBottom: spacing[3],
              }}
            >
              {category}
            </div>

            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
                gap: spacing[3],
              }}
            >
              {group.map((template) => (
                <Link
                  key={template.id}
                  to={`/drafting/${encodeURIComponent(template.id)}${
                    matterId ? `?matter=${encodeURIComponent(matterId)}` : ''
                  }`}
                  style={{ textDecoration: 'none' }}
                >
                  <motion.div
                    whileHover={shouldReduce ? undefined : { y: -2 }}
                    style={{
                      background: colors.bgSecondary,
                      border: `0.5px solid ${colors.border}`,
                      borderRadius: radius.card,
                      boxShadow: shadows.card,
                      padding: spacing[5],
                      height: '100%',
                    }}
                  >
                    <div
                      style={{
                        fontFamily: fonts.ui,
                        fontSize: fontSizes.cardTitle,
                        fontWeight: 500,
                        color: colors.textPrimary,
                      }}
                    >
                      {template.name}
                    </div>

                    {template.description && (
                      <p
                        style={{
                          margin: `${spacing[2]} 0 0`,
                          fontFamily: fonts.ui,
                          fontSize: fontSizes.label,
                          color: colors.textSecondary,
                          lineHeight: 1.5,
                        }}
                      >
                        {template.description}
                      </p>
                    )}

                    <div
                      style={{
                        display: 'flex',
                        gap: spacing[3],
                        marginTop: spacing[4],
                        fontFamily: fonts.ui,
                        fontSize: 11,
                        color: colors.textTertiary,
                      }}
                    >
                      <span>v{template.version}</span>
                      <span>{template.revised}</span>
                      {/* An unapproved template should look unapproved. */}
                      {!template.approvedBy && (
                        <span style={{ color: colors.statusWarning }}>Not yet approved</span>
                      )}
                    </div>
                  </motion.div>
                </Link>
              ))}
            </div>
          </section>
        ))
      )}
    </>
  );
}
