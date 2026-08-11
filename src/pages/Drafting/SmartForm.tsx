// The Smart Form Compiler — PRD §9.8.
//
// Split screen: a guided form on the left, the finished document on the right.
// The attorney fills fields and reads a PDF. They never see LaTeX, never see a
// compile error, never touch formatting.
//
// The form is built entirely from the template manifest. Nothing in this file
// knows what a trade mark is, so adding a template to the library needs no
// change here.

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router';
import { motion, useReducedMotion } from 'motion/react';
import { keel } from '@/lib/tauri';
import type {
  AnnexureMark,
  DraftAnnexure,
  FieldError,
  TemplateManifest,
} from '@/lib/ipc-types';
import { FormField, isVisible } from '@/components/drafting/FormField';
import { AnnexureList } from '@/components/drafting/AnnexureList';
import { colors, fonts, fontSizes, radius, shadows, spacing } from '@/design-system/tokens';

/** How long to wait after a keystroke before re-rendering the preview.
 *
 *  A compile is ~1.4s warm, so re-rendering on every character would queue
 *  work faster than it drains. 600ms is roughly the pause at the end of a
 *  thought, which is when a preview is actually worth looking at. */
const PREVIEW_DEBOUNCE_MS = 600;

/** The computed fields a template declares when it can carry annexures.
 *
 *  Mirrors ANNEXURE_KEYS in commands/drafting.rs. A template without both is
 *  one Keel will refuse annexures for, so the picker is not offered. */
const ANNEXURE_KEYS = ['ANNEXURES_BLOCK', 'ANNEXURE_PAGES'];

export function SmartForm() {
  const { templateId = '' } = useParams();
  const [params] = useSearchParams();
  const matterId = params.get('matter') ?? undefined;
  const navigate = useNavigate();
  const shouldReduce = useReducedMotion();

  const [manifest, setManifest] = useState<TemplateManifest | null>(null);
  const [values, setValues] = useState<Record<string, string>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [pdfUrl, setPdfUrl] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);
  const [saved, setSaved] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [annexures, setAnnexures] = useState<DraftAnnexure[]>([]);
  const [annexureMarks, setAnnexureMarks] = useState<AnnexureMark[]>([]);

  // Guards against an older, slower render overwriting a newer one.
  const renderSeq = useRef(0);

  useEffect(() => {
    keel.drafting
      .getTemplate(templateId)
      .then(setManifest)
      .catch((e) => setLoadError(e instanceof Error ? e.message : String(e)));
  }, [templateId]);

  // A blob URL is a document held in memory. Releasing it matters here: the
  // preview replaces it on every render, so leaking one per keystroke would
  // accumulate the whole drafting session.
  useEffect(() => () => { if (pdfUrl) URL.revokeObjectURL(pdfUrl); }, [pdfUrl]);

  const visibleFields = useMemo(() => {
    if (!manifest) return [];
    return manifest.fields.filter(
      (f) => f.kind.type !== 'computed' && isVisible(f, manifest.fields, values),
    );
  }, [manifest, values]);

  const takesAnnexures = useMemo(
    () =>
      !!manifest &&
      ANNEXURE_KEYS.every((key) => manifest.fields.some((f) => f.key === key)),
    [manifest],
  );

  // Only the rows that actually have a file behind them.
  const attachable = useMemo(
    () =>
      annexures
        .filter((a): a is DraftAnnexure & { stagedId: string } => !!a.stagedId)
        .map((a) => ({ stagedId: a.stagedId, title: a.title })),
    [annexures],
  );

  const missingRequired = useMemo(
    () => visibleFields.filter((f) => f.required && !(values[f.key] ?? '').trim()),
    [visibleFields, values],
  );

  const render = useCallback(
    async (mode: 'draft' | 'final') => {
      if (!manifest) return;
      const seq = ++renderSeq.current;

      if (mode === 'draft') setPreviewing(true);
      else setGenerating(true);
      setProblem(null);

      try {
        const result = await keel.drafting.render({
          templateId: manifest.id,
          values,
          mode,
          matterId: mode === 'final' ? matterId : undefined,
          // A row with no file yet is a name the attorney is still typing, not
          // an annexure. Sending it would mark a gap in the bundle.
          annexures: attachable,
        });

        // A render that finished after a newer one started is stale.
        if (seq !== renderSeq.current) return;

        setErrors(
          Object.fromEntries(result.fieldErrors.map((e: FieldError) => [e.key, e.message])),
        );

        if (result.pdfBase64) {
          const bytes = Uint8Array.from(atob(result.pdfBase64), (c) => c.charCodeAt(0));
          const url = URL.createObjectURL(new Blob([bytes], { type: 'application/pdf' }));
          setPdfUrl((previous) => {
            if (previous) URL.revokeObjectURL(previous);
            return url;
          });
        }

        setProblem(result.problem);
        setAnnexureMarks(result.annexureMarks);
        if (mode === 'final' && result.documentId) setSaved(result.documentId);
      } catch (e) {
        if (seq === renderSeq.current) {
          setProblem(e instanceof Error ? e.message : String(e));
        }
      } finally {
        if (seq === renderSeq.current) {
          setPreviewing(false);
          setGenerating(false);
        }
      }
    },
    [manifest, values, matterId, attachable],
  );

  // Live preview. Only once every visible required field has something in it —
  // before that the render would fail validation on every keystroke and the
  // attorney would watch a form fill up with errors they are on their way to
  // fixing.
  useEffect(() => {
    if (!manifest || missingRequired.length > 0) return;
    const timer = setTimeout(() => render('draft'), PREVIEW_DEBOUNCE_MS);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [values, attachable, manifest, missingRequired.length]);

  if (loadError) {
    return <Banner tone="error" message={loadError} />;
  }
  if (!manifest) {
    return <div style={{ color: colors.textTertiary, fontFamily: fonts.ui }}>Loading…</div>;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <header style={{ marginBottom: spacing[5] }}>
        <button
          onClick={() => navigate('/drafting')}
          style={{
            background: 'none',
            border: 'none',
            padding: 0,
            cursor: 'pointer',
            fontFamily: fonts.ui,
            fontSize: fontSizes.label,
            color: colors.accentPrimary,
          }}
        >
          ← All documents
        </button>

        <h1
          style={{
            margin: `${spacing[3]} 0 0`,
            fontFamily: fonts.display,
            fontSize: fontSizes.displayLg,
            fontWeight: 600,
            color: colors.textPrimary,
          }}
        >
          {manifest.name}
        </h1>

        <div
          style={{
            display: 'flex',
            gap: spacing[4],
            marginTop: spacing[2],
            fontFamily: fonts.ui,
            fontSize: fontSizes.label,
            color: colors.textSecondary,
          }}
        >
          <span>Version {manifest.version}</span>
          <span>Revised {manifest.revised}</span>
          {manifest.authority && <span>{manifest.authority}</span>}
        </div>
      </header>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'minmax(340px, 1fr) minmax(420px, 1.15fr)',
          gap: spacing[6],
          alignItems: 'start',
          flex: 1,
          minHeight: 0,
        }}
      >
        {/* ---- Left: the guided form ---- */}
        <section
          style={{
            background: colors.bgSecondary,
            border: `0.5px solid ${colors.border}`,
            borderRadius: radius.card,
            boxShadow: shadows.card,
            padding: spacing[6],
            maxHeight: '100%',
            overflowY: 'auto',
          }}
        >
          {visibleFields.map((spec) => (
            <FormField
              key={spec.key}
              spec={spec}
              value={values[spec.key] ?? ''}
              error={errors[spec.key]}
              onChange={(v) => setValues((prev) => ({ ...prev, [spec.key]: v }))}
            />
          ))}

          {takesAnnexures && (
            <AnnexureList
              annexures={annexures}
              marks={annexureMarks}
              onChange={setAnnexures}
            />
          )}

          <div
            style={{
              borderTop: `0.5px solid ${colors.border}`,
              paddingTop: spacing[5],
              marginTop: spacing[5],
            }}
          >
            {saved ? (
              <Banner
                tone="ok"
                message={
                  matterId
                    ? 'Generated and filed to the matter.'
                    : 'Generated. Not filed — no matter was selected.'
                }
              />
            ) : (
              <>
                <motion.button
                  onClick={() => render('final')}
                  disabled={generating || missingRequired.length > 0}
                  whileHover={
                    !shouldReduce && !generating && missingRequired.length === 0
                      ? { opacity: 0.88 }
                      : undefined
                  }
                  style={{
                    padding: `${spacing[2]} ${spacing[5]}`,
                    borderRadius: radius.button,
                    border: 'none',
                    background: colors.accentPrimary,
                    color: '#fff',
                    fontFamily: fonts.ui,
                    fontSize: fontSizes.body,
                    fontWeight: 500,
                    opacity: generating || missingRequired.length > 0 ? 0.5 : 1,
                    cursor:
                      generating || missingRequired.length > 0 ? 'not-allowed' : 'pointer',
                  }}
                >
                  {generating ? 'Generating…' : 'Generate PDF'}
                </motion.button>

                {missingRequired.length > 0 && (
                  <p
                    style={{
                      margin: `${spacing[3]} 0 0`,
                      fontFamily: fonts.ui,
                      fontSize: fontSizes.label,
                      color: colors.textSecondary,
                    }}
                  >
                    Still needed: {missingRequired.map((f) => f.label).join(', ')}.
                  </p>
                )}
              </>
            )}
          </div>
        </section>

        {/* ---- Right: the document ---- */}
        <section
          style={{
            position: 'relative',
            background: colors.bgSecondary,
            border: `0.5px solid ${colors.border}`,
            borderRadius: radius.card,
            boxShadow: shadows.card,
            overflow: 'hidden',
            height: '100%',
            minHeight: 520,
          }}
        >
          {pdfUrl ? (
            <iframe
              title={`${manifest.name} preview`}
              src={pdfUrl}
              style={{ width: '100%', height: '100%', border: 'none' }}
            />
          ) : (
            <div
              style={{
                display: 'grid',
                placeItems: 'center',
                height: '100%',
                padding: spacing[8],
                textAlign: 'center',
                fontFamily: fonts.ui,
                fontSize: fontSizes.body,
                color: colors.textTertiary,
              }}
            >
              {problem
                ? problem
                : missingRequired.length > 0
                  ? 'The document appears here once the required fields are filled.'
                  : 'Preparing the document…'}
            </div>
          )}

          {/* A quiet marker rather than a spinner over the document: the
              previous render stays readable while the next one compiles. */}
          {previewing && pdfUrl && (
            <div
              style={{
                position: 'absolute',
                top: spacing[3],
                right: spacing[3],
                padding: `2px ${spacing[3]}`,
                borderRadius: radius.chip,
                background: colors.bgPrimary,
                border: `0.5px solid ${colors.border}`,
                fontFamily: fonts.ui,
                fontSize: 11,
                color: colors.textSecondary,
              }}
            >
              Updating…
            </div>
          )}
        </section>
      </div>

      {problem && pdfUrl && (
        <div style={{ marginTop: spacing[4] }}>
          <Banner tone="error" message={problem} />
        </div>
      )}
    </div>
  );
}

function Banner({ tone, message }: { tone: 'error' | 'ok'; message: string }) {
  const colour = tone === 'error' ? colors.statusUrgent : colors.statusClear;
  return (
    <div
      role={tone === 'error' ? 'alert' : 'status'}
      style={{
        padding: spacing[3],
        borderRadius: radius.input,
        background: tone === 'error' ? 'rgba(192,57,43,0.08)' : 'rgba(74,124,89,0.09)',
        color: colour,
        fontFamily: fonts.ui,
        fontSize: fontSizes.body,
      }}
    >
      {message}
    </div>
  );
}
