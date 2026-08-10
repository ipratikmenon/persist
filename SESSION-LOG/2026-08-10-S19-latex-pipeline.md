# Session S19 — 2026-08-10
## The LaTeX Pipeline — Repair

---

## Summary

You asked how the drafting/LaTeX/template layer handles. The answer was that it
did not: invoice PDF generation had **never produced a PDF**, and could not have.
Three independent faults, each fatal on its own.

None was catchable by the tests that existed, because all three tests asserted on
strings and none ran the engine — and pdfLaTeX was not installed in this
container, so the pipeline had never once executed end to end.

**Result:** cargo test **174/174** (was 170), now including four tests that
compile the real template with hostile values. On `claude/new-session-dbqe5o`;
nothing pushed to main.

---

## What was broken

**1. The firm's own name broke compilation.** `firm_settings.firm_name` defaults
to `'Persistas & Partners'` (`0006_billing.sql:9`). `&` is LaTeX's alignment
character:

```
! Missing } inserted.
l.72 ...RGE\bfseries\color{accentBlue} Persistas &
!  ==> Fatal error occurred, no output PDF file produced!
```

A `latex_escape` existed, but was applied to exactly one field —
`line_item.description` — out of twenty-one. `CLIENT_NAME`, `CLIENT_ADDRESS`,
`FIRM_*` and `NOTES` all went in raw.

**2. The rupee sign broke compilation.** The template hardcodes ₹ in two column
headers and the code prefixes every rate and amount with it. pdfLaTeX cannot
typeset U+20B9:

```
! LaTeX Error: Unicode character ₹ (U+20B9)
```

Still fatal after fixing #1.

**3. The documents row violated a foreign key.** `generate_invoice_pdf` bound
`inv.client_id` into `documents.matter_id`, which is
`NOT NULL REFERENCES matters(id)`. The code said so:
`// Use client_id as matter_id placeholder until linked`. sqlx sets
`PRAGMA foreign_keys=ON` by default (`sqlx-sqlite/src/options/mod.rs:185`), so
the insert fails — *after* the PDF has been encrypted into the vault, leaving an
orphaned file behind.

**4. A silent one.** `latex_escape` mapped `\` to `\\`. In LaTeX `\\` is a line
break, not an escaped backslash. Compiled and confirmed:

```
input:  "See In re Bajaj\Auto, para 12"
output: "See In re Bajaj"
        "Auto, para 12"
```

Clean compile, no warning, wrong legal document — the worst failure mode of the
four, because nothing signals it.

---

## What changed

### Engine: XeLaTeX, not pdfLaTeX

`fontspec` with Noto Serif. ₹ works; so does Devanagari, declared as a font
family so a Hindi party name or title renders instead of dropping to tofu. That
second one is not speculative — an Indian IP practice files in Hindi, and
switching engines after M9 has built a template library would be far more
expensive than switching now with one template in place.

### Escaping is no longer something to remember

```rust
pub async fn compile_latex(template_id: &str, fields: &HashMap<String, Field>)
```

`Field::text(...)` escapes on the way in. `Field::raw(...)` does not, and says
so. A bare `String` no longer compiles — which is what surfaced the problem: the
type error listed every call site that had been silently trusting the caller.

`raw` is deliberately greppable: it is the only place a template injection can
originate, and there is exactly one use of it (assembled table rows), where each
interpolated value goes through `latex::escape` first.

The old `latex_escape` is **deleted**, not deprecated. Leaving a broken escaper
in the codebase is how it gets used again.

### Failing instead of shipping

- An unfilled `{{PLACEHOLDER}}` now fails the render. The previous behaviour left
  it in place, and a unit test asserted that was correct — it is not, it reaches
  the client. Placeholders inside `%` comments are ignored, so a template's own
  documentation of its variables does not fail the build it describes.
- Two compilation passes. `longtable` settles column widths on the second; one
  pass silently produces a misaligned document, which is worse than an error
  because it looks finished.
- 60-second timeout with `kill_on_drop`. `-halt-on-error` cannot catch a template
  that loops without erroring; only a timeout can.
- `-no-shell-escape`, stdin closed, job name sanitised before it reaches a path.
  All cheap now, and all necessary once M9/M30 accept AI-assembled templates.
- LaTeX's actual error is extracted from the log. A thousand-line dump shown to
  an attorney is the same as showing nothing.

### Indian digit grouping

`₹1,55,760.00`, not `₹155760.00`. It prints directly beside `amount_in_words`,
which already says "One Lakh Fifty Five Thousand" — grouped the Western way the
two read as contradicting each other on a GST invoice.

---

## Bugs Found and Fixed

- **B10** — invoice PDF generation had never worked (the three faults above).
  Now covered by tests that compile.
- **B03** downgraded further: the engine is resolved at runtime with a clear
  error naming the install command. `tauri.conf.json` sidecar bundling remains.
- **A test-isolation bug I introduced.** The runaway-template test set
  `PERSIST_TEMPLATES_DIR`, which is process-global, and raced the other
  compilation test under the parallel runner — it passed alone and failed in the
  suite. Rewritten to drive `run_engine` with inline source, so no global state
  is touched at all.

---

## Commands Run

```bash
apt-get install -y texlive-latex-base texlive-latex-recommended \
                   texlive-latex-extra texlive-fonts-recommended \
                   texlive-xetex fonts-noto-core poppler-utils

cargo test --lib                 # 174/174, ~60s (the compile tests are real)
cargo build --lib                # 1 pre-existing warning, unrelated
```

### Negative controls

Both original faults were reproduced against the new test before it was trusted:

| Break | Result |
|---|---|
| `Field::text` stops escaping | `! Missing } inserted.` — FAILED |
| Template reverted to `fontenc`/`inputenc`, engine forced to pdflatex | `! LaTeX Error: Unicode character ₹ (U+20B9)` — FAILED |

Restored, suite green.

Rendered output was also inspected as a PDF, not just asserted on: two-page
`longtable` with a repeating header, ₹ throughout, ampersands in both the firm
and client names, GST breakdown, and the words matching the figure.

---

## Not Done / Deferred

- **Sidecar bundling in `tauri.conf.json`** (B03). The installer must ship
  xelatex and the Noto families; TeX Live adds roughly 80 MB, or considerably
  less with a trimmed scheme.
- **No CI workflow exists in this repo.** When one is added it needs
  `texlive-xetex fonts-noto-core`, or the compilation tests will skip — which is
  precisely how these faults survived.
- **The drafting suite itself (M9) is untouched.** This sprint repaired the
  substrate it will sit on; the template engine, precedent library and smart
  forms are still Phase 4.
- **Only one template exists.** `invoice.tex`. Everything above is what makes
  adding the second one safe.

---

## Next Session Options

1. **M5 Step 4** — the portal frontend.
2. **M5 Step 3d** — object storage + email delivery.
3. **Sidecar bundling + a CI workflow**, so the compilation tests actually run
   somewhere other than a developer's machine.
