# Session S20 — 2026-08-10
## Prerequisites for M9 — the Drafting Suite

---

## Summary

S19 made the LaTeX pipeline produce a PDF. This session made it produce more
than one *kind* of PDF without writing Rust each time.

The problem: `generate_invoice_pdf` hardcodes twenty-one field names. The PRD
lists **nineteen** templates for the Smart Form Compiler's initial library —
cover letters, examination replies, oppositions, affidavits, vakalatnamas.
Nineteen bespoke command functions is not a plan, and it puts the platform
admin's job (update a template when a registry changes its format) behind a Rust
release.

**Result:** cargo test **203/203** (was 174), pnpm build PASS, and the repo's
first CI workflow. On `claude/new-session-dbqe5o`.

---

## What was built

### Template registry — `services/templates.rs`

A template is now two files: `<id>.tex` and `<id>.json`. The manifest declares
every field — key, label, kind, validation, conditional visibility, and where
Deck should autofill from. Deck builds the form from it; Keel validates against
the same schema on submit. Adding a template is adding two files.

Field kinds: `text`, `multiline` (with word limits, because registries impose
them), `date` (with cross-field `notBefore`), `digits` (exact length),
`number`, `select`, `checkbox`, `computed`.

**Validation is declarative rather than regex.** The PRD asks for "TM
application number must be numeric and 7 digits". `digits(7)` produces "must be
exactly 7 digits"; a regex produces "must match `^[0-9]{7}$`". Attorneys read
these.

Every problem is returned, not the first — five round trips to fix five fields
is a punishment, not a form.

### Shared preamble — `_shared/persist-base.tex`

Fonts, colours (mirroring the Deck design tokens), geometry, column types,
letterhead and footer macros. Nineteen templates each carrying their own copy
would be nineteen places to change when the letterhead does, and nineteen
chances to miss one. Resolved via `TEXINPUTS`, since the rendered `.tex` is
written to a temp directory.

### Draft and Final compile modes

`Draft` is one pass, for the live preview the PRD wants at 1–2s. `Final` is two,
for anything an attorney signs or sends — `longtable` settles column widths on
the second pass, so a one-pass filing is quietly misaligned.

### Engine warm-up

Measured here: cold XeLaTeX **14.2s**, warm **1.4s**. The difference is the font
cache. That cost is unavoidable once per machine, but paying it while an
attorney watches a blank preview pane is the wrong moment, so `latex::warm_up()`
is spawned at startup. It swallows its errors — the app must still start on a
machine with no TeX Live and say so when a document is actually requested.

### Drafting commands — `commands/drafting.rs`

`list_templates`, `get_template`, `render_document`. A compile failure reaches
the attorney as *"Preview temporarily unavailable — your content is saved"*,
with the LaTeX log going to the log where a developer can find it (PRD §9.8).

Everything an attorney typed goes in as `Field::text`. A value sent from Deck
under a `computed` key is escaped like any other, so Deck cannot inject LaTeX by
naming a field — there is a test for exactly that.

### A second real template

Reply to Examination Report (Trade Marks Registry), with a conditional
prior-use ground. It compiles, and it shares the letterhead with the invoice.
That is the proof the machinery generalises past the one document it was
extracted from.

### CI — the repo had none

Four jobs: Keel with TeX Live, Deck, the portal against real PostgreSQL, the RLS
gate, and the sync server.

`PERSIST_REQUIRE_LATEX=1` turns a silently skipped compilation test into a
failure. The first version grepped the test output for the skip notice, which
would never have worked — cargo captures test stdout, so the notice never
reaches the log. Verified the guard fires by pointing the engine at a
nonexistent path.

---

## Bugs the new drift test found in my own code

**`placeholders_in` mis-parsed a placeholder inside LaTeX braces.** Templates
write `\textbf{{{FIRM_GSTIN}}}` — a brace wrapping a placeholder. The scanner
anchored on the first `{{`, read the key as `{FIRM_GSTIN`, rejected it, and
found nothing. Consequences beyond the drift check: `render` uses the same
function to detect unfilled placeholders, so `{{{CLIENT_ADDRESS}}}` left
unfilled would have printed into a client's PDF — the exact failure S19 added
that check to prevent. Now scans one brace at a time.

**The drift check could not express a collected-but-not-printed field.** The
grounds selector on an examination reply drives which standard paragraph Keel
assembles into `PRIOR_USE_BLOCK`; the word "PriorUse" never appears in the
document. The check demanded a `{{GROUNDS}}` the template must not contain.
Added `inputOnly` rather than loosening the check — loosening it would have lost
the guarantee for every other field.

Also: serde cannot combine `#[serde(flatten)]` with `deny_unknown_fields`, so
the first manifest failed to parse with "unknown field `kind`". Nesting the kind
object costs a little JSON verbosity and keeps strictness — a manifest with
`"requird": false` must fail loudly rather than quietly produce a filing nobody
validated.

---

## Commands Run

```bash
cargo test --lib                      # 203/203
PERSIST_REQUIRE_LATEX=1 cargo test --lib
pnpm build                            # PASS

# Timing, which set the design:
#   cold xelatex  14.163s
#   warm xelatex   1.423s
```

Both new templates were rendered and inspected as PDFs, not only asserted on.

### Negative control

```bash
PERSIST_REQUIRE_LATEX=1 PERSIST_LATEX_ENGINE=/nonexistent cargo test --lib compile_tests
→ "PERSIST_REQUIRE_LATEX is set but no xelatex/lualatex was found —
   the compilation tests would have skipped silently"
```

---

## Not Done / Deferred

- **Sidecar bundling in `tauri.conf.json`** (B03). The installer must ship
  xelatex and the Noto families. It cannot be verified from here — it needs a
  real macOS/Windows build machine.
- **The Smart Form Compiler UI itself.** That is M9. Deck now has everything it
  needs to build a form from a schema; nothing has been built in Deck yet.
- **Clause libraries.** `PRIOR_USE_BLOCK` is assembled by the caller today.
  Pre-approved paragraphs selected by checkbox (PRD §9.8) want their own store.
- **Seventeen more templates.** Two exist. The machinery is what this sprint was
  for; the library is content, and content wants the firm's actual precedents.
- **AI proofreading, comparison, precedent library** — M9.2, 9.3, 9.6, all still
  Phase 4.

---

## Next Session Options

1. **M9 — the Smart Form Compiler UI.** Split-screen form and live preview in
   Deck, driven by the manifests. The backend is ready.
2. **M5 Step 4** — the portal frontend.
3. **Sidecar bundling**, which needs a real build machine.
