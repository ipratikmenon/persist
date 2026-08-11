# Session S24 — 2026-08-11
## Letterhead alignment, and annexures

---

## Summary

Two notes from you on the notice template:

1. The partner names and addresses did not line up with the mark.
2. There should be a way to attach proof — receipts, PDFs, images — with the
   annexure marking handled automatically.

Both done. The second was built once the long way and then cut down when you
said what you actually wanted: a checkbox, add, name it, choose a file, and
nothing on the annexure page except the mark.

**Result:** cargo test **219/219** (was 204), Deck build PASS. Nothing pushed to
`main` — all of it is on `claude/new-session-dbqe5o`, as you asked.

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

## 3. Negative controls

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

## 4. Files

**New**

```
src-tauri/storage/templates/_shared/persist-annexures.tex
src-tauri/src/services/annexures.rs
src/components/drafting/AnnexureList.tsx
```

**Changed**

```
src-tauri/storage/templates/_shared/persist-letterhead.tex   alignment + \annexure* list macros
src-tauri/storage/templates/legal-notice.tex                 {{ANNEXURE_PAGES}}, annexure input
src-tauri/storage/templates/legal-notice.json                ANNEXURE_PAGES declared
src-tauri/src/services/latex.rs                              Attachment, compile_with, TEXINPUTS, engine_available
src-tauri/src/commands/drafting.rs                           stage/discard, load_annexures, AnnexureMark
src-tauri/src/lib.rs                                         staged_annexures on AppState, 2 commands
src/lib/ipc-types.ts, src/lib/tauri.ts                       annexure types + wrappers
src/pages/Drafting/SmartForm.tsx                             wires the picker in
```

---

## 5. Tests

```
cargo test --lib     219 passed   (was 204)
pnpm build           PASS
```

New: 12 in `services::annexures` (8 unit, 4 real compiles reading the PDF back
with `pdftotext`), 3 in `services::latex::attachment_tests`.

The compile tests build their own fixtures — a multi-page PDF from the same
engine that will include it, and a hand-built 8×8 greyscale PNG with correct
CRCs and a stored-deflate zlib stream, because XeLaTeX rejects anything less and
there is no image crate in the tree.

---

## 6. Open

- **Staged files are not separately vaulted.** They are held in memory for the
  drafting session. What is retained is the generated document, which contains
  them; a restart loses an unfinished draft's attachments.
- **Repeating groups** (`list` field kind) — still the highest-value thing left
  in M9. Annexures are now a real repeating list in the form, but only because
  they carry a file rather than fields.
- **The boilerplate notice language is still my reconstruction** from the notice
  you sent. It needs your read-through before anything goes out on it.
- Autofill from the matter record; clause libraries; M5 Step 3d; sidecar
  bundling (B03).
