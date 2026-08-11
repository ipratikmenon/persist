# Session S24 — 2026-08-11
## Letterhead alignment, annexures, and page setup

---

## Summary

Three notes from you on the notice template:

1. The partner names and addresses did not line up with the mark.
2. There should be a way to attach proof — receipts, PDFs, images — with the
   annexure marking handled automatically.
3. Paper size, font, size, bold/italic, page numbers and where the letterhead
   goes should be settings, so the drafter never formats a document by hand.

All three done. The second was built once the long way and then cut down when
you said what you actually wanted: a checkbox, add, name it, choose a file, and
nothing on the annexure page except the mark.

**Result:** cargo test **238/238** (was 204), Deck build PASS. On
`claude/new-session-dbqe5o`.

**On paper size.** You thought the notice was Legal. I measured the one you
sent: it is A4, all nineteen pages. So Legal is a switch and A4 is the default,
matching what the firm actually sends. Say the word if filings should default
the other way — it is one line.

---

## 1. The letterhead

I cropped the header band out of your notice and out of ours at 150 dpi
(`pdftoppm -x 0 -y 0 -W 1240 -H 330`) and compared them rather than adjusting by
eye. Three separate faults:

| What was wrong | Why | Fix |
|---|---|---|
| The partner block was half again as tall as yours | `\\[1pt]` between lines, *on top of* the font's own leading | Leading set once for the block: `\fontsize{7.6}{10}\selectfont` |
| The mark sat level with the first partner's name | Both bands were `[t]`-aligned minipages | Both are now `[c]`-aligned minipages of `\headheight`, so the mark centres against the two blocks however long an email address is |
| The rule between the partners ran the full width | `\rule{\linewidth}` — reads as a section divider | `\hfill\rule{46mm}{0.4pt}`, right-aligned under the first partner |

Also: `newgeometry` now carries `includehead, includefoot`. Without them `top`
and `bottom` are the distance to the *body*, so the header was pinned to the
paper edge and the whole letterhead sat higher than yours.

---

## 2. Annexures

### What a page looks like

The document, and `ANNEXURE A` boxed in the top-right corner. That is all. No
cover sheet, no caption, no firm letterhead printed over a third party's
receipt. A PDF keeps its own pages and its own page numbering — renumbering
someone else's document would misrepresent it.

### The flow

Tick **Attach annexures** → **+ Add annexure** → name it ("Receipt one") →
**Choose file**. The file is combined into the PDF. Reordering rows renumbers
everything.

### How the pieces fit

| Piece | What it does |
|---|---|
| `_shared/persist-annexures.tex` | `\annexurepdf`, `\annexureimage`, `\annexurestamp`, and the list macros |
| `services/annexures.rs` | Allocates marks, classifies files, builds **both** the printed list and the appended pages from one `Vec<Annexure>` |
| `latex::Attachment` + `compile_with` | Stages files into the compile directory under names Keel generates |
| `stage_annexure` / `discard_annexure` | Reads the chosen file, checks its type, strips its metadata, holds it for the session |
| `components/drafting/AnnexureList.tsx` | The checkbox, the rows, the file button |

### Four decisions worth recording

**The list and the pages come from one `Vec`.** If they were built separately
they would eventually disagree — the notice promising an Annexure D that is not
in the bundle. That is exactly the discrepancy opposing counsel is paid to find.
`marks_match_the_pages` fails if they ever stop agreeing.

**The file type is sniffed from the bytes, never from the `mime_type` column.**
A file recorded with the wrong type would otherwise reach the engine as the
wrong `\include*` command and fail the compile of a notice at the moment it was
needed. One unusable file refuses the whole batch, by name: a notice served with
one of its four annexures silently missing is worse than one that would not
build.

**Keel reads the file, not Deck.** The OS dialog gives Deck a path; Deck passes
the path and never touches the bytes. Two reasons: Deck does not read documents
off the filesystem, and the preview re-renders on every pause — resending a 5 MB
scan each time would be the slowest thing in the app. Keel holds the cleaned
bytes in memory keyed by an id, and renders reference the id.

**Metadata is stripped at the moment the file is chosen.** An annexure leaves
the firm inside a document served on the opposite party. The author, the
revision history and the GPS coordinates of the phone that photographed a
receipt stop at `stage_annexure`. Doing it there rather than at render time also
means an unusable file is a sentence in front of the attorney straight away.

---

## 3. Page setup

Everything here is a setting, not a template field. A template says what a legal
notice says; what size the paper is has nothing to do with that, and putting
paper size into nineteen manifests would mean nineteen places to change it.

| Setting | Options |
|---|---|
| Paper | A4 (210 × 297 mm), Legal (8.5 × 14 in) |
| Typeface | Noto Serif, Times New Roman, Palatino, Century Schoolbook, Noto Sans, Helvetica |
| Size | 10–16 pt (6–32 accepted) |
| Line spacing | Single, 1.5, Double |
| Emphasis | Bold throughout, Italic throughout |
| Page numbers | `Page 3 of 19`, `Page 3`, `3`, none — plus which number page one is |
| Letterhead | Every page, First page only, Named pages, None |

`services/layout.rs` compiles the choices to a `persist-layout.tex` staged
beside the template. `_shared/persist-base.tex` reads it and falls back to the
house format for anything unset, so a compile that supplies no layout — a test,
an invoice generated straight from billing — looks exactly as it did.

Nothing in a layout is free text. Every field is an enum or a number, so there
is no string from Deck that reaches the engine as markup.
`nothing_in_a_layout_is_free_text` asserts a font name Deck invented is refused
at deserialisation.

### First page only is not a running head

`\headheight` is one value for the whole document. It cannot be 108pt on page
one and 14pt after. Running the letterhead as a head in first-page-only mode
would give every continuation sheet a four-centimetre blank band where the
firm's mark used to be.

So that mode does not use a running head at all: the letterhead is set as body
content at the start of page one, and the band shrinks to what the page number
needs. Page two gets an ordinary top margin. Named pages and every-page keep the
running head, because there the mark genuinely has to repeat.

---

## 4. Three faults the compile tests found

None of these would have shown up in a string comparison, and two of them
produce a clean compile and a wrong document.

**Only the Noto faces carry ₹.** Every Times, Palatino and Century Schoolbook
clone in TeX Live drops U+20B9 without a word — no error, no warning, and a
served notice demanding "1,04,000" with no currency sign. Measured across all
six offered faces. The preamble now binds ₹ to a Noto fallback with
`newunicodechar`, which catches it in template source and in attorney-typed text
alike, without either having to know.

**Legal paper produced A4.** `keyval` does not expand a macro in key position,
so `\geometry{\persistpaper}` matched no key, said nothing, and left the
document whatever size the class made it. Now `papersize={w,h}` — values are
expanded, keys are not.

**`\newgeometry` re-derives the paper from the class options.** Even after the
above, a Legal notice still came out A4, because `\applyletterhead` re-ran
geometry and `\newgeometry` discards the papersize set earlier in favour of the
`\documentclass` option. It runs in the preamble, where plain `\geometry` is
allowed and accumulates onto what is already set.

---

## 5. Negative controls

Two guards, each run against a deliberately broken version first.

### The mark's position

The obvious way to stamp each page is a `fancyhdr` page style. I built it that
way, compiled it, and measured:

```
naive (fancyhdr):  ANNEXURE A at yMin=137.24pt
                   receipt's own first line at yMin=144.53pt
```

The mark lands *on* the evidence. The cause is `\applyletterhead`'s 108pt head
band — the annexure page inherits the geometry the firm's own pages need, which
pushes its header roughly 40% down the paper. A shipout picture is positioned
against the paper instead, so it sits 13mm from the edge whatever the body
geometry is.

`the_mark_sits_at_the_top_of_the_page_clear_of_the_content` asserts a position,
not merely that the words are somewhere on the page. It fails at exactly
137.24pt on the naive version.

### Staging

Removing the compile directory from `TEXINPUTS` breaks
`a_staged_file_is_reachable_by_name_from_the_template` — confirmed. The engine's
working directory is the app's, not the compile directory, so that entry is the
only reason `\includepdf{annexure-01.pdf}` resolves at all.

### A claim of mine that the control disproved

I first wrote into `persist-annexures.tex` that the `fancyhdr` version puts the
*following* annexure's letter on the last page of the one before it, because the
header is expanded at shipout. Plausible, and wrong: the control showed the
marks come out right either way. The comment now gives the reason that is
actually true — the position — and points at the test that enforces it.

---

## 6. Files

**New**

```
src-tauri/storage/templates/_shared/persist-annexures.tex
src-tauri/src/services/annexures.rs
src-tauri/src/services/layout.rs
src/components/drafting/AnnexureList.tsx
src/components/drafting/PageSetup.tsx
```

**Changed**

```
src-tauri/storage/templates/_shared/persist-base.tex         reads the layout; rupee fallback; page-number label
src-tauri/storage/templates/_shared/persist-letterhead.tex   alignment, letterhead modes, \geometry not \newgeometry
src-tauri/storage/templates/legal-notice.tex                 {{ANNEXURE_PAGES}}, annexure input
src-tauri/storage/templates/legal-notice.json                ANNEXURE_PAGES declared
src-tauri/src/services/latex.rs                              Attachment, compile_with, TEXINPUTS, engine_available
src-tauri/src/commands/drafting.rs                           stage/discard, load_annexures, AnnexureMark
src-tauri/src/lib.rs                                         staged_annexures on AppState, 2 commands
src/lib/ipc-types.ts, src/lib/tauri.ts                       annexure types + wrappers
src/pages/Drafting/SmartForm.tsx                             wires the picker in
```

---

## 7. Tests

```
cargo test --lib     238 passed   (was 204)
pnpm build           PASS
```

New: 12 in `services::annexures` (8 unit, 4 real compiles read back with
`pdftotext`), 3 in `services::latex::attachment_tests`, 19 in `services::layout`
(9 unit, 10 real compiles measured with `pdfinfo`, `pdftotext` and `pdffonts`).

The compile tests build their own fixtures — a multi-page PDF from the same
engine that will include it, and a hand-built 8×8 greyscale PNG with correct
CRCs and a stored-deflate zlib stream, because XeLaTeX rejects anything less and
there is no image crate in the tree.

---

## 8. Open

- **Staged files are not separately vaulted.** They are held in memory for the
  drafting session. What is retained is the generated document, which contains
  them; a restart loses an unfinished draft's attachments.
- **Repeating groups** (`list` field kind) — still the highest-value thing left
  in M9. Annexures are now a real repeating list in the form, but only because
  they carry a file rather than fields.
- **The boilerplate notice language is still my reconstruction** from the notice
  you sent. It needs your read-through before anything goes out on it.
- **Margins are not yet a setting.** They are the one page-formatting control
  left out; say if you want them and it is a small addition to the same panel.
- Autofill from the matter record; clause libraries; M5 Step 3d; sidecar
  bundling (B03).
