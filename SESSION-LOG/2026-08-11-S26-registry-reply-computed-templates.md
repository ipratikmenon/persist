# Session S26 — 2026-08-11
## The reply to the Registry, and a computed field that declares itself

---

## Summary

Picked up the top of the open list: `tm-examination-reply` had the same
blank-letterhead defect the notice had. It turned out to have three blank
computed fields, not one, and fixing the third properly meant giving a manifest
a way to declare its own conditional paragraph.

Separately, rebuilding this container's toolchain surfaced that **CI could not
have run the page-setup font tests at all**.

**Result:** cargo test **311/311** with `PERSIST_REQUIRE_LATEX=1` (was 301),
Deck build PASS. On `claude/new-session-dbqe5o`.

---

## 1. Three blank fields, not one

`tm-examination-reply` goes to the Trade Marks Registry over the firm's name. It
declared FIRM_NAME, FIRM_ADDRESS and PRIOR_USE_BLOCK as computed, and nothing
filled any of them.

I wrote the test before the fix, and it failed the way the defect predicts:

```
no registered address in the letterhead:
Persistas & Partners persistas.pnp@outlook.com Date: 2026-08-11 ...
Yours faithfully, For Persistas & Partners ...
```

That output is from partway through — FIRM_NAME had just started resolving. Run
against the code as it stood at the start of the session, the header was empty
and the signature read "For " with nothing after it.

PRIOR_USE_BLOCK is worse in kind: it is the paragraph pleading prior use and
acquired distinctiveness. A reply relying on prior use as its ground **did not
plead it** — the ground was named in GROUNDS_TEXT and the paragraph that makes
the case for it was absent.

---

## 2. A computed field can now carry its own template

FIRM_NAME and FIRM_ADDRESS belong in `services/firm.rs` with the rest of the
firm's identity. PRIOR_USE_BLOCK does not — it is one document's paragraph,
built from that document's inputs. Hardcoding it in Rust would mean a Rust
change for every conditional paragraph in nineteen templates, against the stated
goal that adding a template is two files.

So `FieldKind::Computed` gained an optional `template`:

```json
{
  "key": "PRIOR_USE_BLOCK",
  "kind": {
    "type": "computed",
    "template": "\\noindent\nThe Applicant has been using the mark {{TM_MARK}} openly, continuously and in the course of trade since {{FIRST_USE_DATE}}, ..."
  },
  "shownWhen": { "field": "GROUNDS", "equals": ["PriorUse"] }
}
```

`assemble_computed` renders it. Design points, each with a reason:

| Decision | Why |
|---|---|
| Only manifest-declared keys are substitutable | A value Deck invents cannot reach the page by naming a placeholder |
| Every substituted value goes through `latex::escape` | The template is firm work product; what goes into it is the attorney's prose |
| Hidden assembles to an **empty block**, not to nothing | `render` refuses to compile on an unbound placeholder — the paragraph disappears, the document still builds |
| No template means Keel fills it | Returning an empty block would overwrite the firm's identity and the annexure blocks with nothing |
| Lists are not substitutable | A row set has no scalar text; a template that wants one should be a list |

Pairing the template with the `shownWhen` that already governed FIRST_USE_DATE
means the paragraph and the date it cites are under one condition and cannot
come apart.

Drift checking extends to it — a template that names an undeclared field, refers
to itself, or has no placeholders at all (a constant belongs in the .tex) is
reported against the shipped library.

---

## 3. Two addresses, kept apart on purpose

`firm_address` came with billing and is the block printed on a tax invoice. The
two office lines came with the letterhead and are the registered office. A firm
may legitimately invoice from one address and be registered at another, so
neither is derived from the other.

But a fresh install had `firm_address` unset, which is how the reply came out
with a blank address. The office lines now stand in when the invoice address is
empty. One behaviour, documented at the point it happens, and no third copy of
the same fact.

---

## 4. CI could not have run the font tests

This container lost its toolchain between sessions — GTK dev packages, TeX Live,
poppler and `node_modules` were all gone, and `cargo test --lib` failed on
`gdk-3.0` that had been fine an hour earlier. Rebuilding it from the package
list in `.github/workflows/ci.yml` is what exposed the gap: **that list is not
sufficient to run the suite.**

- The **TeX Gyre faces** — Times, Palatino, Century Schoolbook, Helvetica, which
  page setup offers by name — come from `fonts-texgyre`, which was not
  installed. On the runner, `every_offered_font_can_set_the_rupee` would fail
  with `! Package fontspec Error: The font "TeX Gyre Termes" cannot be found.`
  That test exists precisely because those faces drop ₹ silently.
- **`poppler-utils`** was not installed either, and every compile test in
  `annexures.rs`, `layout.rs`, `templates.rs` and `firm.rs` measures its PDF
  with `pdftotext`, `pdfinfo` or `pdffonts`.

Both are now installed by the workflow, and it asserts each face resolves and
each binary exists before running anything. A missing font is not a skip — it is
a failure, and it should say so on the line that installs it.

---

## 5. Negative controls

| Control | Result |
|---|---|
| Removed `latex::escape` from `assemble_computed` | 2 tests failed. `unescaped: For Tata & Sons, 100% owned.` and — the one that matters — `a value was re-substituted: secret and secret`, where a value containing `{{B}}` earned a second substitution |
| Pointed PRIOR_USE_BLOCK's template at `{{DATE_OF_FIRST_USE}}` | The shipped-library drift test failed: `PRIOR_USE_BLOCK's template uses {{DATE_OF_FIRST_USE}} but the manifest does not declare it` |

The escaping is what closes the substitution off: an escaped value contains no
`{{`, so a cell cannot name another placeholder. Without it, that is a working
injection into a document going to a registry.

---

## 6. Files

```
src-tauri/src/services/templates.rs   Computed { template }, assemble_computed, drift check, 8 tests
src-tauri/src/services/firm.rs        FIRM_NAME + FIRM_ADDRESS with the office-line fallback, 2 compile tests
src-tauri/src/commands/drafting.rs    binds manifest-declared computed templates
storage/templates/tm-examination-reply.json   PRIOR_USE_BLOCK gets its template and condition
src/lib/ipc-types.ts                  the computed kind can carry a template
.github/workflows/ci.yml              fonts-texgyre, poppler-utils, and guards for both
```

---

## 7. Open

- Payment tranches on the notice are still a `computed` block with nothing
  filling it. The `list` kind supports them; which fields a tranche has is a
  decision about your document.
- No autofill of a list row from the matter record.
- Staged annexures are held in memory, not separately vaulted.
- The boilerplate notice language is still my reconstruction from the notice you
  sent, and has not had your read-through.
