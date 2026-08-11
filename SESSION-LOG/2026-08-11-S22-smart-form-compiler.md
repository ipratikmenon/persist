# Session S22 — 2026-08-11
## Ownership, and Module 9.8 — the Smart Form Compiler

---

## Summary

Two things: the ownership record you asked for, and M9.8 — the half of the
drafting suite that an attorney actually touches.

S20 built the template registry so that adding a document type is adding two
files rather than writing Rust. This session is the proof of that claim:
`components/drafting/FormField.tsx` renders a form from a manifest, and nothing
in Deck knows what a trade mark is.

**Result:** cargo test **203/203**, Deck build PASS, 16 screenshots. On
`claude/new-session-dbqe5o`.

---

## Ownership

`COPYRIGHT.md` records it plainly: **Persist is proprietary software and the
exclusive property of Persistas & Partners.** Not open-source, no licence
granted, express or implied.

The templates get their own paragraph. They encode the firm's precedents,
drafting conventions and accumulated practice before the Indian registries —
that is professional work product, not configuration, and it is the part most
likely to be mistaken for something generic.

There is also a confidentiality note. Source, database, vault and logs can each
reveal the existence or subject matter of a client engagement, so anyone with
access is bound by the firm's obligations whether or not they are an advocate.

Carried into the places it actually shows:

| Where | What |
|---|---|
| `tauri.conf.json` | `copyright`, `publisher` — installer dialogs, macOS Get Info, Windows file properties |
| `src-tauri/Cargo.toml`, `server/Cargo.toml` | `authors`, `license = "UNLICENSED"`, `publish = false` |
| `package.json` ×2 | `private`, `license`, `author` |
| `CLAUDE.md` | So the next session reads it before touching anything |

`productName` was already `Persist` and the identifier already
`com.persistaspartners.persist`.

---

## M9.8 — the Smart Form Compiler

Split screen. Guided form on the left, the finished document on the right. The
attorney fills fields and reads a PDF; they never see LaTeX, never see a compile
error, never touch formatting (PRD §9.8).

### The form is generated, not written

`FormField.tsx` renders `text`, `multiline`, `date`, `digits`, `number`,
`select` and `checkbox` from the manifest. It reads labels, help text,
conditional visibility and word limits from the same file Keel validates
against.

There is no trade-mark-specific code in Deck. The examination reply's seven-digit
application number, its registry dropdown, and its prior-use date that appears
only when prior use is the ground relied upon — all of that is in
`tm-examination-reply.json`. The seventeen remaining templates need no Deck
change at all.

Two details worth their lines:

- **Digits are filtered as they are typed** rather than flagged afterwards. The
  field cannot hold anything else, so accepting a letter only to reject it
  wastes the attorney's attention.
- **The word counter is live.** A registry limit discovered on submit means
  rewriting; discovered while writing it means editing.

### The preview

Debounced at 600ms. A warm compile is ~1.4s, so rendering on every keystroke
would queue work faster than it drains — 600ms is roughly the pause at the end
of a thought, which is when a preview is worth looking at.

Three things that would otherwise bite:

- **Stale renders are discarded by sequence number.** An older, slower compile
  finishing after a newer one must not overwrite it.
- **Blob URLs are revoked.** The preview replaces one on every render; leaking
  one per keystroke would accumulate the whole drafting session in memory.
- **The previous document stays on screen while the next compiles**, with a
  quiet "Updating…" marker rather than a spinner over the page. Blanking the
  pane on every pause would make the preview unreadable.

The preview only starts once every visible required field has something in it.
Before that the render would fail validation on every keystroke and the attorney
would watch a form fill with errors they are on their way to fixing.

### Generate

Files the PDF into the vault and records it against the matter, with the
template version in the filename so an associate can see which version produced
a document without opening the record.

### Base64, not a byte vector

`render_document` returned `Vec<u8>`, which Tauri serialises as a JSON array of
numbers — roughly three to four bytes of JSON per byte of PDF. The live preview
re-renders as the attorney types, so a 250 KB document would have crossed the
bridge as most of a megabyte each time. Base64 is 1.33x.

---

## What the screenshot harness caught

The first capture showed every field labelled "— optional". Not an app bug: the
manifests on disk omit `required` where it takes its default, serde fills that
in on the way out of Keel, and my mock was serving the raw JSON. A mock that is
wrong in the same way twice is worse than no mock, so it now applies the same
defaults serde does.

Also noted: Chromium's PDF viewer does not paint into a headless screenshot. The
viewer chrome renders and the blob URL resolves, so the pane works — it simply
cannot be captured this way. Not chased.

---

## Not Done / Deferred

- **Autofill from the matter record.** The manifests declare `autofill` sources
  (`matter.responsibleAttorney`, `ipAsset.applicationNumber`) and nothing reads
  them yet. This is the PRD's "zero retyping" promise and it is the next piece
  of M9.8.
- **Clause libraries** — pre-approved paragraphs selected by checkbox, injected
  into a computed field. `PRIOR_USE_BLOCK` is assembled by the caller today.
- **Seventeen more templates.** Content, and it wants the firm's real
  precedents rather than my invention of what a vakalatnama says.
- **M9.2, 9.3, 9.6** — AI proofreading, document comparison, precedent library.
- **M5 Step 3d**, **sidecar bundling (B03)**.

---

## Next Session Options

1. **Finish M9.8** — autofill from the matter record, then clause libraries.
   Both make the form materially faster to use.
2. **M5 Step 3d** — object storage and email, so documents actually flow to
   clients rather than queueing.
3. **Template library** — sit with the firm's precedents and add real templates.
