# Session S13 — 2026-08-05
## B07 — Real Document Metadata Stripping

---

## Summary

Sprint chosen ahead of M5 Step 2 deliberately: B07 is independent of the pending
spec review. Metadata stripping is required by root CLAUDE.md for *every* export
path, not just the portal, so it is correct work regardless of how the M5 review
lands. Step 2 (schema) would need reworking if the spec changes.

`clean_metadata()` had been a pass-through stub since S05, which made a stated
security rule a no-op. It now strips for real and fails closed.

**Result:** cargo test 67 → **86/86**. pnpm build PASS (472 modules, 497kb).
Nothing pushed to main — held on `claude/new-session-dbqe5o` for review.

---

## Files Changed

### New files
- `src-tauri/src/storage/metadata.rs` — the real implementation (19 tests)
- `SESSION-LOG/2026-08-05-S13-metadata-stripping.md` — this file

### Modified files
- `src-tauri/src/storage/vault.rs` — stub removed; pointer comment to the new module
- `src-tauri/src/storage/mod.rs` — registers `metadata`
- `src-tauri/src/commands/documents.rs` — `get_document` no longer strips;
  new `export_document` returns cleaned bytes + report; header invariants rewritten
- `src-tauri/src/lib.rs` — registers `export_document`
- `src-tauri/Cargo.toml` — adds `zip`, `quick-xml`, `lopdf`
- `src-tauri/KEEL-RULES.md` — corrected import path and the internal-vs-export rule
- `specs/module-05-portal.md` — §9.1, §15.3, §17, §18 updated; B07 marked resolved
- `src/lib/ipc-types.ts` — `CleanReport`, `ExportedDocument`
- `src/lib/tauri.ts` — `documents.exportForClient`, doc comments on `get`
- `src/pages/Documents/DocumentList.tsx` — "↓ Client copy" action
- `PROGRESS.md` — B07 resolved, M5 row updated, 5 decisions, session log

---

## What Is Actually Stripped

| Format | Removed |
|---|---|
| PDF | Info dictionary (Author, Creator, Producer, Title, Subject, Keywords), XMP metadata stream, then orphaned objects pruned |
| DOCX / XLSX / PPTX | `docProps/core.xml`, `app.xml`, `custom.xml`; all comment parts; `word/people.xml`; **tracked changes**; comment anchors in the body; dangling relationships to removed parts |
| JPEG | APP1 (EXIF incl. **GPS**, XMP), APP3, APP5–APP15 (IPTC/Photoshop), COM. APP0 (JFIF) and APP2 (ICC) kept — they affect rendering and carry no personal data |
| PNG | `tEXt`, `zTXt`, `iTXt`, `tIME`, `eXIf`, `dSIG` |
| Plain text | Nothing to strip |

**Deliberately refused:** TIFF (metadata lives in the IFD structure; stripping
means rewriting it, and half-doing it would be worse than refusing), legacy
`.doc`/`.xls` OLE compound files, and anything embedded inside another document.

---

## Architecture Decisions

### Fail closed
An unsupported type returns an error, not the original bytes. The old stub meant
a documented rule silently did nothing; the new failure mode is a visible refusal
("Cannot strip metadata from this file type, so it must not be sent to a client.
Convert to PDF and retry") rather than a quiet leak.

### Internal read and client export are now separate commands
This was the subtle part. Making `get_document` strip would have been a
regression: an attorney reviewing a counterparty's draft needs to see its tracked
changes — that is *why* they opened the file. Conflating the two paths either
leaks metadata or destroys the information the attorney wanted.

- `get_document` → raw bytes, internal use, documented as never-send-to-client
- `export_document` → cleaned bytes + report, the only client-facing path

The M5 sync engine must use the latter; noted in the spec.

### Tracked changes are accepted, not rejected
`w:ins` wrappers are unwrapped (insertion becomes plain text); `w:del` subtrees
are removed entirely along with their `w:delText`. That is what "send the clean
copy" means to an attorney. Formatting revisions (`w:rPrChange`, `w:pPrChange`,
and the table/section variants) are dropped wholesale.

### Detection by magic bytes only
A caller-supplied mime type is not evidence. A `.docx` renamed to `.pdf` is still
cleaned as OOXML. ZIP magic alone is not enough — the container is confirmed by
the presence of `[Content_Types].xml`, so an ordinary `.zip` is refused rather
than silently mangled.

### Export reports what it removed
`CleanReport` names each category ("3 tracked change(s)", "Reviewer comments",
"GPS location data") and the UI shows it after saving. Silent stripping would
give the attorney no chance to notice that a file should not have been sent at
all.

### Dangling relationships are cleaned up
Removing `word/comments.xml` without removing its `<Relationship>` entry makes
Word report a corrupt file. `word/_rels/document.xml.rels` is rewritten to drop
references to deleted parts while leaving every other relationship intact —
covered by a test that asserts `styles.xml` survives.

---

## Test Coverage (19 new)

Notable cases beyond the happy paths:

- `docx_tracked_deletion_is_removed_and_insertion_is_accepted` — asserts the
  deleted figure ("50,000") is gone, the inserted one ("75,000") survives as
  plain text, the author name is gone, and surrounding text is intact
- `nested_deletions_do_not_truncate_the_document` — a `w:del` inside a `w:del`
  must close correctly; getting the depth tracking wrong would silently drop
  everything after it
- `docx_comments_and_anchors_are_removed` — comment body, anchors, and the
  dangling relationship all go; an unrelated relationship stays
- `jpeg_exif_is_removed_but_jfif_and_image_data_kept` — stripping must not
  break rendering
- `jpeg_gps_is_called_out_in_the_report` — GPS gets its own report line
- `pdf_info_dictionary_is_removed` — asserts the cleaned PDF still parses
- `unsupported_type_errors_rather_than_passing_through` — the fail-closed guarantee
- `detects_by_magic_not_by_mime_hint`, `plain_text_needs_valid_utf8`

---

## Dependencies Added

| Crate | Why | Features |
|---|---|---|
| `zip` 2 | OOXML container read/write | `deflate` only |
| `quick-xml` 0.37 | Streaming XML transform for revision/comment removal — regex on XML would be wrong | default |
| `lopdf` 0.34 | PDF object model | `nom_parser` only |

---

## Commands Run

```bash
cargo add zip@2 --no-default-features --features deflate
cargo add quick-xml@0.37
cargo add lopdf@0.34 --no-default-features --features nom_parser

cd src-tauri && cargo test --lib
# 86/86 PASSED (19 new)

pnpm build
# 472 modules, 497kb — PASS
```

---

## Known Issues / Status

| ID | Status | Notes |
|---|---|---|
| B01 | Resolved | S11 |
| B02 | Resolved | S11 |
| B03 | Partial | latex.rs real; runtime needs MacTeX or bundled TeX Live |
| B05 | Resolved | GTK/WebKit deps |
| B06 | Partial | UTC parsing helper exists; other call sites unconverted |
| B07 | **Resolved** | This session |

---

## Not Done / Deferred

- **MatterDetail** still has only the raw "Save" action; the "Client copy" action
  was added to DocumentList (the firm-wide vault) only. Same pattern, small edit.
- **TIFF and legacy .doc/.xls** are refused rather than cleaned — a deliberate
  scope call, documented in the module header.
- **Metadata stripping on upload** is not applied. Only exports are cleaned;
  vaulted originals keep their metadata on purpose, since the firm may need it.
- The pre-existing unused-import warning in `services/latex.rs:10` is untouched.

---

## Next Session Options

1. **M5 Step 2 — Schema** (once the spec is approved): `0009_portal_sync.sql`,
   `server/migrations/0001_mirror.sql`, `0002_rls.sql`, `server/SCHEMA.md`.
   Gate: `test_rls_blocks_cross_client_read`.
2. **Phase 1 M2 remainder** — `cascade_engine.rs`, `abandonment_watcher.rs`,
   `RenewalDashboard.tsx`.
3. **RBAC enforcement + first-launch wizard** — closes out the auth spec.
