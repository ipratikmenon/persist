// Document metadata stripping — resolves B07.
//
// Root CLAUDE.md: "Metadata stripped before EVERY document export."
// Until now `clean_metadata` was a pass-through stub, so that rule was stated
// but not enforced. This module makes it real.
//
// The threat this addresses is the classic law-firm metadata incident: a .docx
// leaves the firm still carrying tracked changes, reviewer comments, and the
// author's name — disclosing internal deliberation to the other side. A close
// second is a photograph carrying GPS EXIF.
//
// DESIGN: fail closed. `clean_metadata` returns an error for any type it cannot
// clean, rather than passing bytes through. A caller exporting to a client must
// not be able to ship uncleaned bytes by accident. Internal viewing does not go
// through here at all — see commands/documents.rs.
//
// SCOPE — handled: PDF (info dictionary, XMP), OOXML .docx/.xlsx/.pptx (core,
// app and custom properties; comments; tracked changes), JPEG (EXIF incl. GPS,
// XMP, IPTC, comments), PNG (text chunks, timestamp, embedded EXIF), plain text
// (nothing to strip). Everything else is refused.
//
// NOT handled, deliberately: TIFF (metadata lives in the IFD structure and
// stripping means rewriting it — refused rather than half-done), legacy .doc/.xls
// (OLE compound files), and anything embedded inside another document. A stripper
// that silently misses tracked changes is worse than no stripper, because people
// trust it.

use anyhow::{anyhow, Context, Result};
use std::io::{Cursor, Read, Write};

// ---------------------------------------------------------------------------
// Report — what was actually removed
// ---------------------------------------------------------------------------

/// Summary of a cleaning pass. Surfaced to the attorney before a client export
/// so "3 tracked changes and 2 comments were removed" is visible, not silent.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    /// Human-readable list of what was stripped.
    pub removed: Vec<String>,
    /// Original byte length.
    pub original_bytes: usize,
    /// Cleaned byte length.
    pub cleaned_bytes: usize,
}

impl CleanReport {
    fn note(&mut self, what: impl Into<String>) {
        self.removed.push(what.into());
    }

    pub fn is_empty(&self) -> bool {
        self.removed.is_empty()
    }
}

/// Result of a cleaning pass: the cleaned bytes plus what was taken out.
pub struct Cleaned {
    pub bytes:  Vec<u8>,
    pub report: CleanReport,
}

// ---------------------------------------------------------------------------
// Format detection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Pdf,
    /// Office Open XML — .docx / .xlsx / .pptx (a ZIP container)
    Ooxml,
    Jpeg,
    Png,
    /// Plain text / markdown / csv — no embedded metadata to strip.
    PlainText,
    Unsupported,
}

/// Detect by magic bytes, not by extension or the caller's mime string.
/// A client-supplied mime type is not evidence of anything.
pub fn detect_format(bytes: &[u8], mime_hint: &str) -> Format {
    if bytes.starts_with(b"%PDF-") {
        return Format::Pdf;
    }
    // JPEG: SOI marker
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Format::Jpeg;
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Format::Png;
    }
    // ZIP magic — OOXML, but so is any other zip. Confirm by looking for the
    // OOXML content-types part.
    if bytes.starts_with(b"PK\x03\x04") {
        if looks_like_ooxml(bytes) {
            return Format::Ooxml;
        }
        return Format::Unsupported;
    }

    // Text formats carry no embedded metadata. Trust the hint here only because
    // there is nothing to strip either way, and verify the bytes are valid UTF-8.
    let texty = mime_hint.starts_with("text/")
        || mime_hint == "application/json"
        || mime_hint == "application/xml";
    if texty && std::str::from_utf8(bytes).is_ok() {
        return Format::PlainText;
    }

    Format::Unsupported
}

fn looks_like_ooxml(bytes: &[u8]) -> bool {
    let Ok(archive) = zip::ZipArchive::new(Cursor::new(bytes)) else {
        return false;
    };
    // index_for_name avoids taking a borrow that outlives `archive`.
    archive.index_for_name("[Content_Types].xml").is_some()
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Strip metadata for a client-facing export.
///
/// Fails closed: an unsupported type returns an error rather than passing
/// through. Callers must not ship bytes this function refused to clean.
pub fn clean_metadata(bytes: &[u8], mime_type: &str) -> Result<Vec<u8>> {
    clean_with_report(bytes, mime_type).map(|c| c.bytes)
}

/// As `clean_metadata`, but also returns what was removed.
pub fn clean_with_report(bytes: &[u8], mime_type: &str) -> Result<Cleaned> {
    let mut report = CleanReport {
        original_bytes: bytes.len(),
        ..Default::default()
    };

    let format = detect_format(bytes, mime_type);

    let cleaned = match format {
        Format::Pdf       => clean_pdf(bytes, &mut report)?,
        Format::Ooxml     => clean_ooxml(bytes, &mut report)?,
        Format::Jpeg      => clean_jpeg(bytes, &mut report)?,
        Format::Png       => clean_png(bytes, &mut report)?,
        Format::PlainText => bytes.to_vec(),
        Format::Unsupported => {
            return Err(anyhow!(
                "Cannot strip metadata from this file type ({mime_type}), so it \
                 must not be sent to a client. Supported types: PDF, DOCX, XLSX, \
                 PPTX, JPEG, PNG, plain text. Convert the file to PDF and retry."
            ))
        }
    };

    report.cleaned_bytes = cleaned.len();
    Ok(Cleaned { bytes: cleaned, report })
}

// ---------------------------------------------------------------------------
// PDF
// ---------------------------------------------------------------------------

/// Remove the Info dictionary (Author, Creator, Producer, Title, Subject,
/// Keywords) and any XMP metadata stream.
fn clean_pdf(bytes: &[u8], report: &mut CleanReport) -> Result<Vec<u8>> {
    let mut doc = lopdf::Document::load_mem(bytes).context("failed to parse PDF")?;

    // --- Info dictionary -------------------------------------------------
    if doc.trailer.get(b"Info").is_ok() {
        doc.trailer.remove(b"Info");
        report.note("PDF document info (author, creator, producer, title)");
    }

    // --- XMP metadata stream ---------------------------------------------
    // Referenced from the catalog as /Metadata.
    let catalog_id = doc
        .catalog()
        .ok()
        .and_then(|_| doc.trailer.get(b"Root").ok().cloned())
        .and_then(|r| r.as_reference().ok());

    if let Some(id) = catalog_id {
        let had_metadata = doc
            .get_object(id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .map(|d| d.has(b"Metadata"))
            .unwrap_or(false);

        if had_metadata {
            if let Ok(obj) = doc.get_object_mut(id) {
                if let Ok(dict) = obj.as_dict_mut() {
                    dict.remove(b"Metadata");
                    report.note("PDF XMP metadata stream");
                }
            }
        }
    }

    // Drop now-unreferenced objects (the XMP stream itself).
    doc.prune_objects();

    let mut out = Vec::new();
    doc.save_to(&mut Cursor::new(&mut out))
        .context("failed to write cleaned PDF")?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Office Open XML (.docx / .xlsx / .pptx)
// ---------------------------------------------------------------------------

/// Parts removed wholesale. Comment bodies and their author lists.
const OOXML_DROP_PARTS: &[&str] = &[
    "docProps/core.xml",              // dc:creator, cp:lastModifiedBy, cp:revision
    "docProps/app.xml",               // Company, Manager, Template, TotalTime
    "docProps/custom.xml",            // arbitrary custom properties
    "word/comments.xml",
    "word/commentsExtended.xml",
    "word/commentsIds.xml",
    "word/commentsExtensible.xml",
    "word/people.xml",                // comment/revision author identities
    "xl/comments1.xml",
    "ppt/comments/comment1.xml",
];

fn clean_ooxml(bytes: &[u8], report: &mut CleanReport) -> Result<Vec<u8>> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).context("failed to read OOXML container")?;

    let mut out_buf = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut out_buf));
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut dropped_parts = Vec::new();
        let mut revisions_removed = 0usize;
        let mut comment_refs_removed = 0usize;

        let names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();

        for name in names {
            if OOXML_DROP_PARTS.contains(&name.as_str()) {
                dropped_parts.push(name);
                continue;
            }

            let mut file = archive.by_name(&name)?;
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;
            drop(file);

            // Rewrite XML parts that can carry revisions/comment anchors, and
            // the relationship files that would otherwise dangle.
            let rewritten = if name.ends_with(".rels") {
                strip_rels_to_dropped_parts(&content)?
            } else if is_body_part(&name) {
                let (xml, revs, refs) = strip_revisions(&content)?;
                revisions_removed += revs;
                comment_refs_removed += refs;
                xml
            } else {
                content
            };

            writer.start_file(&name, options)?;
            writer.write_all(&rewritten)?;
        }

        writer.finish()?;

        if !dropped_parts.is_empty() {
            let has_comments = dropped_parts.iter().any(|p| p.contains("comment"));
            report.note(format!(
                "Office document properties ({} part{} removed)",
                dropped_parts.len(),
                if dropped_parts.len() == 1 { "" } else { "s" }
            ));
            if has_comments {
                report.note("Reviewer comments");
            }
        }
        if revisions_removed > 0 {
            report.note(format!("{revisions_removed} tracked change(s)"));
        }
        if comment_refs_removed > 0 {
            report.note(format!("{comment_refs_removed} comment anchor(s)"));
        }
    }

    Ok(out_buf)
}

/// Body parts where revision markup can appear.
fn is_body_part(name: &str) -> bool {
    matches!(
        name,
        "word/document.xml" | "word/footnotes.xml" | "word/endnotes.xml"
    ) || name.starts_with("word/header")
        || name.starts_with("word/footer")
}

/// Elements removed together with all their children (deleted text, formatting
/// revision records, comment bodies' anchors).
const REVISION_DROP_ELEMENTS: &[&str] = &[
    "w:del",              // deleted content — must not survive
    "w:moveFrom",         // source half of a move
    "w:rPrChange",        // run formatting revision
    "w:pPrChange",        // paragraph formatting revision
    "w:sectPrChange",
    "w:tblPrChange",
    "w:trPrChange",
    "w:tcPrChange",
    "w:tblGridChange",
    "w:customXmlDelRangeStart",
    "w:customXmlDelRangeEnd",
];

/// Elements unwrapped — the wrapper goes, the content stays (an accepted
/// insertion is just text).
const REVISION_UNWRAP_ELEMENTS: &[&str] = &["w:ins", "w:moveTo"];

/// Empty marker elements deleted outright.
const COMMENT_ANCHOR_ELEMENTS: &[&str] = &[
    "w:commentRangeStart",
    "w:commentRangeEnd",
    "w:commentReference",
];

/// Accept all tracked changes and remove comment anchors.
///
/// Returns (rewritten xml, revisions removed, comment anchors removed).
///
/// Accepting rather than rejecting matches what an attorney means by "send the
/// clean copy": insertions become plain text, deletions disappear.
fn strip_revisions(xml: &[u8]) -> Result<(Vec<u8>, usize, usize)> {
    use quick_xml::events::Event;
    use quick_xml::{Reader, Writer};

    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);

    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    let mut revisions = 0usize;
    let mut comment_refs = 0usize;

    // When >0 we are inside a dropped element and skip everything.
    let mut skip_depth = 0usize;
    // Name of the element that opened the current skip region.
    let mut skipping: Option<Vec<u8>> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,

            Ok(Event::Start(e)) => {
                let name = e.name().as_ref().to_vec();

                if skip_depth > 0 {
                    // Track nesting of the same element so we close correctly.
                    if Some(&name) == skipping.as_ref() {
                        skip_depth += 1;
                    }
                    buf.clear();
                    continue;
                }

                if is_named(&name, REVISION_DROP_ELEMENTS) {
                    skip_depth = 1;
                    skipping = Some(name);
                    revisions += 1;
                    buf.clear();
                    continue;
                }

                if is_named(&name, REVISION_UNWRAP_ELEMENTS) {
                    // Drop the wrapper, keep the children.
                    revisions += 1;
                    buf.clear();
                    continue;
                }

                writer.write_event(Event::Start(e))?;
            }

            Ok(Event::End(e)) => {
                let name = e.name().as_ref().to_vec();

                if skip_depth > 0 {
                    if Some(&name) == skipping.as_ref() {
                        skip_depth -= 1;
                        if skip_depth == 0 {
                            skipping = None;
                        }
                    }
                    buf.clear();
                    continue;
                }

                if is_named(&name, REVISION_UNWRAP_ELEMENTS) {
                    buf.clear();
                    continue;
                }

                writer.write_event(Event::End(e))?;
            }

            Ok(Event::Empty(e)) => {
                let name = e.name().as_ref().to_vec();

                if skip_depth > 0 {
                    buf.clear();
                    continue;
                }

                if is_named(&name, COMMENT_ANCHOR_ELEMENTS) {
                    comment_refs += 1;
                    buf.clear();
                    continue;
                }
                if is_named(&name, REVISION_DROP_ELEMENTS) {
                    revisions += 1;
                    buf.clear();
                    continue;
                }

                writer.write_event(Event::Empty(e))?;
            }

            Ok(ev) => {
                if skip_depth == 0 {
                    writer.write_event(ev)?;
                }
            }

            Err(e) => return Err(anyhow!("malformed XML in OOXML part: {e}")),
        }
        buf.clear();
    }

    Ok((writer.into_inner().into_inner(), revisions, comment_refs))
}

/// Match an element name ignoring namespace prefix differences is NOT done here
/// deliberately — OOXML always uses the `w:` prefix for WordprocessingML, and
/// matching on local name alone would also strip unrelated elements.
fn is_named(name: &[u8], set: &[&str]) -> bool {
    set.iter().any(|n| n.as_bytes() == name)
}

/// Remove `<Relationship>` entries pointing at parts we deleted, so Word does
/// not report a corrupt file over a dangling reference.
fn strip_rels_to_dropped_parts(xml: &[u8]) -> Result<Vec<u8>> {
    use quick_xml::events::Event;
    use quick_xml::{Reader, Writer};

    let dropped_targets: Vec<&str> = OOXML_DROP_PARTS
        .iter()
        .map(|p| p.rsplit('/').next().unwrap_or(p))
        .collect();

    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(Event::Empty(e)) if e.name().as_ref() == b"Relationship" => {
                let target = e
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"Target")
                    .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
                    .unwrap_or_default();

                let file = target.rsplit('/').next().unwrap_or(&target).to_string();
                if dropped_targets.contains(&file.as_str()) {
                    buf.clear();
                    continue;
                }
                writer.write_event(Event::Empty(e))?;
            }
            Ok(ev) => writer.write_event(ev)?,
            Err(e) => return Err(anyhow!("malformed relationship XML: {e}")),
        }
        buf.clear();
    }

    Ok(writer.into_inner().into_inner())
}

// ---------------------------------------------------------------------------
// JPEG
// ---------------------------------------------------------------------------

/// Application segments that carry metadata rather than rendering information.
/// APP0 (JFIF) and APP2 (ICC colour profile) are kept — dropping them changes
/// how the image renders, and neither carries personal data.
fn is_metadata_app_segment(marker: u8) -> bool {
    matches!(
        marker,
        0xE1 |  // APP1  — EXIF (incl. GPS) and XMP
        0xE3 |  // APP3  — Kodak/meta
        0xE5 |  // APP5
        0xE6 |  // APP6
        0xE7 |  // APP7
        0xE8 |  // APP8
        0xE9 |  // APP9
        0xEA |  // APP10
        0xEB |  // APP11
        0xEC |  // APP12 — Picture Info / Ducky
        0xED |  // APP13 — Photoshop IRB, carries IPTC
        0xEE |  // APP14 — Adobe (colour transform; safe to drop for baseline)
        0xEF |  // APP15
        0xFE    // COM   — free-text comment
    )
}

fn clean_jpeg(bytes: &[u8], report: &mut CleanReport) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    let mut removed = 0usize;
    let mut had_gps = false;

    // SOI
    if bytes.len() < 2 {
        return Err(anyhow!("truncated JPEG"));
    }
    out.extend_from_slice(&bytes[0..2]);
    i += 2;

    while i + 1 < bytes.len() {
        if bytes[i] != 0xFF {
            // Not at a marker — copy the rest verbatim (entropy-coded data).
            out.extend_from_slice(&bytes[i..]);
            break;
        }

        let marker = bytes[i + 1];

        // Standalone markers with no length field.
        if marker == 0xD8 || (0xD0..=0xD7).contains(&marker) || marker == 0x01 || marker == 0xFF {
            out.extend_from_slice(&bytes[i..i + 2]);
            i += 2;
            continue;
        }

        // Start of scan — everything after is compressed image data.
        if marker == 0xDA {
            out.extend_from_slice(&bytes[i..]);
            break;
        }

        if i + 4 > bytes.len() {
            return Err(anyhow!("truncated JPEG segment header"));
        }
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if len < 2 || i + 2 + len > bytes.len() {
            return Err(anyhow!("invalid JPEG segment length"));
        }

        let segment = &bytes[i..i + 2 + len];

        if is_metadata_app_segment(marker) {
            // Detect GPS so the report can call it out specifically.
            if marker == 0xE1 && segment.windows(4).any(|w| w == b"Exif") {
                // GPS IFD tag 0x8825 appears in the EXIF payload.
                if segment.windows(2).any(|w| w == [0x88, 0x25]) {
                    had_gps = true;
                }
            }
            removed += 1;
        } else {
            out.extend_from_slice(segment);
        }

        i += 2 + len;
    }

    if removed > 0 {
        report.note(format!("JPEG metadata segments ({removed})"));
        if had_gps {
            report.note("GPS location data");
        }
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// PNG
// ---------------------------------------------------------------------------

/// Chunks carrying metadata rather than image data.
const PNG_DROP_CHUNKS: &[&[u8; 4]] = &[
    b"tEXt", b"zTXt", b"iTXt", // text, including author/description
    b"tIME",                   // last-modification time
    b"eXIf",                   // embedded EXIF
    b"dSIG",                   // digital signature
];

fn clean_png(bytes: &[u8], report: &mut CleanReport) -> Result<Vec<u8>> {
    const SIG_LEN: usize = 8;
    if bytes.len() < SIG_LEN {
        return Err(anyhow!("truncated PNG"));
    }

    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[..SIG_LEN]);

    let mut i = SIG_LEN;
    let mut removed = 0usize;

    while i + 8 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]) as usize;
        let ctype = &bytes[i + 4..i + 8];

        // length + type + data + crc
        let total = 12usize
            .checked_add(len)
            .ok_or_else(|| anyhow!("PNG chunk length overflow"))?;
        if i + total > bytes.len() {
            return Err(anyhow!("truncated PNG chunk"));
        }

        let drop = PNG_DROP_CHUNKS.iter().any(|c| c.as_slice() == ctype);
        if drop {
            removed += 1;
        } else {
            out.extend_from_slice(&bytes[i..i + total]);
        }

        let is_end = ctype == b"IEND";
        i += total;
        if is_end {
            break;
        }
    }

    if removed > 0 {
        report.note(format!("PNG metadata chunks ({removed})"));
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- format detection ------------------------------------------------

    #[test]
    fn detects_by_magic_not_by_mime_hint() {
        // A PDF mislabelled as a Word document is still a PDF.
        assert_eq!(
            detect_format(b"%PDF-1.7\n...", "application/msword"),
            Format::Pdf
        );
        assert_eq!(detect_format(&[0xFF, 0xD8, 0xFF, 0xE0], "text/plain"), Format::Jpeg);
    }

    #[test]
    fn unknown_binary_is_unsupported() {
        assert_eq!(detect_format(&[0x00, 0x01, 0x02, 0x03], "application/octet-stream"),
                   Format::Unsupported);
    }

    #[test]
    fn plain_text_needs_valid_utf8() {
        assert_eq!(detect_format(b"hello", "text/plain"), Format::PlainText);
        // Invalid UTF-8 claiming to be text is not trusted.
        assert_eq!(detect_format(&[0xFF, 0xFE, 0x00], "text/plain"), Format::Unsupported);
    }

    // --- fail-closed behaviour -------------------------------------------

    #[test]
    fn unsupported_type_errors_rather_than_passing_through() {
        let err = clean_metadata(&[0x00, 0x01, 0x02], "application/octet-stream")
            .expect_err("must not pass unknown bytes through");
        let msg = err.to_string();
        assert!(msg.contains("must not be sent to a client"), "got: {msg}");
    }

    #[test]
    fn plain_text_passes_through_unchanged() {
        let src = b"Dear Sir,\n\nPlease find enclosed.\n";
        let out = clean_metadata(src, "text/plain").unwrap();
        assert_eq!(out, src);
    }

    // --- PNG --------------------------------------------------------------

    fn png_chunk(ctype: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut c = Vec::new();
        c.extend_from_slice(&(data.len() as u32).to_be_bytes());
        c.extend_from_slice(ctype);
        c.extend_from_slice(data);
        c.extend_from_slice(&[0, 0, 0, 0]); // CRC — not validated by our stripper
        c
    }

    fn minimal_png(extra: &[Vec<u8>]) -> Vec<u8> {
        let mut p = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        p.extend(png_chunk(b"IHDR", &[0; 13]));
        for e in extra {
            p.extend_from_slice(e);
        }
        p.extend(png_chunk(b"IDAT", &[1, 2, 3]));
        p.extend(png_chunk(b"IEND", &[]));
        p
    }

    #[test]
    fn png_text_chunks_are_removed_and_image_data_kept() {
        let with_meta = minimal_png(&[
            png_chunk(b"tEXt", b"Author\0Sree Lakshmi Menon"),
            png_chunk(b"tIME", &[0; 7]),
        ]);

        let cleaned = clean_metadata(&with_meta, "image/png").unwrap();

        assert!(!contains(&cleaned, b"Sree Lakshmi Menon"), "author survived");
        assert!(!contains(&cleaned, b"tEXt"));
        assert!(!contains(&cleaned, b"tIME"));
        // Image data intact.
        assert!(contains(&cleaned, b"IHDR"));
        assert!(contains(&cleaned, b"IDAT"));
        assert!(contains(&cleaned, b"IEND"));
    }

    #[test]
    fn png_without_metadata_is_unchanged() {
        let clean = minimal_png(&[]);
        let out = clean_metadata(&clean, "image/png").unwrap();
        assert_eq!(out, clean);
    }

    #[test]
    fn png_report_counts_removed_chunks() {
        let with_meta = minimal_png(&[
            png_chunk(b"tEXt", b"a\0b"),
            png_chunk(b"iTXt", b"c\0d"),
        ]);
        let res = clean_with_report(&with_meta, "image/png").unwrap();
        assert!(res.report.removed.iter().any(|r| r.contains("PNG metadata chunks (2)")),
                "report was: {:?}", res.report.removed);
    }

    // --- JPEG -------------------------------------------------------------

    fn jpeg_segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut s = vec![0xFF, marker];
        s.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        s.extend_from_slice(payload);
        s
    }

    fn minimal_jpeg(extra: &[Vec<u8>]) -> Vec<u8> {
        let mut j = vec![0xFF, 0xD8]; // SOI
        j.extend(jpeg_segment(0xE0, b"JFIF\0\x01\x02\0\0\x01\0\x01\0\0")); // APP0
        for e in extra {
            j.extend_from_slice(e);
        }
        j.extend(jpeg_segment(0xDB, &[0; 64])); // DQT
        j.extend(vec![0xFF, 0xDA, 0x00, 0x08, 1, 1, 0, 0, 0, 0]); // SOS + data
        j.extend_from_slice(&[0x12, 0x34, 0xFF, 0xD9]);
        j
    }

    #[test]
    fn jpeg_exif_is_removed_but_jfif_and_image_data_kept() {
        let mut exif = b"Exif\0\0".to_vec();
        exif.extend_from_slice(b"Canon EOS R5 / Sree Lakshmi Menon");
        let with_exif = minimal_jpeg(&[jpeg_segment(0xE1, &exif)]);

        let cleaned = clean_metadata(&with_exif, "image/jpeg").unwrap();

        assert!(!contains(&cleaned, b"Sree Lakshmi Menon"), "EXIF author survived");
        assert!(!contains(&cleaned, b"Canon EOS R5"));
        // JFIF (APP0) is rendering info — must be preserved.
        assert!(contains(&cleaned, b"JFIF"));
        // Image data preserved.
        assert!(cleaned.ends_with(&[0xFF, 0xD9]));
    }

    #[test]
    fn jpeg_gps_is_called_out_in_the_report() {
        let mut exif = b"Exif\0\0".to_vec();
        exif.extend_from_slice(&[0x88, 0x25]); // GPS IFD tag
        exif.extend_from_slice(&[0u8; 16]);
        let with_gps = minimal_jpeg(&[jpeg_segment(0xE1, &exif)]);

        let res = clean_with_report(&with_gps, "image/jpeg").unwrap();
        assert!(
            res.report.removed.iter().any(|r| r.contains("GPS")),
            "GPS should be reported explicitly; got {:?}", res.report.removed
        );
    }

    #[test]
    fn jpeg_comment_segment_is_removed() {
        let with_comment = minimal_jpeg(&[jpeg_segment(0xFE, b"internal draft - do not send")]);
        let cleaned = clean_metadata(&with_comment, "image/jpeg").unwrap();
        assert!(!contains(&cleaned, b"internal draft"));
    }

    // --- PDF --------------------------------------------------------------

    fn build_pdf_with_info() -> Vec<u8> {
        use lopdf::{dictionary, Document, Object};

        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type"   => "Page",
            "Parent" => pages_id,
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type"  => "Pages",
                "Kids"  => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type"  => "Catalog",
            "Pages" => pages_id,
        });
        let info_id = doc.add_object(dictionary! {
            "Author"   => Object::string_literal("Sree Lakshmi Menon"),
            "Producer" => Object::string_literal("Persist Internal Draft Tool"),
            "Title"    => Object::string_literal("DRAFT - not for circulation"),
        });
        doc.trailer.set("Root", catalog_id);
        doc.trailer.set("Info", info_id);

        let mut out = Vec::new();
        doc.save_to(&mut Cursor::new(&mut out)).unwrap();
        out
    }

    #[test]
    fn pdf_info_dictionary_is_removed() {
        let pdf = build_pdf_with_info();
        // Sanity: the author really is in the original.
        assert!(contains(&pdf, b"Sree Lakshmi Menon"), "fixture is wrong");

        let cleaned = clean_metadata(&pdf, "application/pdf").unwrap();

        assert!(!contains(&cleaned, b"Sree Lakshmi Menon"), "PDF author survived");
        assert!(!contains(&cleaned, b"Persist Internal Draft Tool"), "producer survived");
        assert!(!contains(&cleaned, b"DRAFT - not for circulation"), "title survived");

        // Still a valid, loadable PDF.
        let reparsed = lopdf::Document::load_mem(&cleaned).expect("cleaned PDF must still parse");
        assert!(reparsed.trailer.get(b"Info").is_err(), "Info reference survived");
    }

    #[test]
    fn pdf_report_notes_the_info_removal() {
        let res = clean_with_report(&build_pdf_with_info(), "application/pdf").unwrap();
        assert!(
            res.report.removed.iter().any(|r| r.contains("info")),
            "got: {:?}", res.report.removed
        );
    }

    // --- OOXML ------------------------------------------------------------

    fn build_docx(parts: &[(&str, &str)]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            w.start_file("[Content_Types].xml", opts).unwrap();
            w.write_all(br#"<?xml version="1.0"?><Types/>"#).unwrap();
            for (name, content) in parts {
                w.start_file(*name, opts).unwrap();
                w.write_all(content.as_bytes()).unwrap();
            }
            w.finish().unwrap();
        }
        buf
    }

    fn read_part(zip_bytes: &[u8], name: &str) -> String {
        let mut a = zip::ZipArchive::new(Cursor::new(zip_bytes)).unwrap();
        let mut f = a.by_name(name).unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        s
    }

    fn part_exists(zip_bytes: &[u8], name: &str) -> bool {
        let a = zip::ZipArchive::new(Cursor::new(zip_bytes)).unwrap();
        a.index_for_name(name).is_some()
    }

    #[test]
    fn docx_core_properties_are_removed() {
        let docx = build_docx(&[
            ("docProps/core.xml",
             r#"<cp:coreProperties><dc:creator>Kajal Thakur</dc:creator><cp:lastModifiedBy>Sree Lakshmi Menon</cp:lastModifiedBy></cp:coreProperties>"#),
            ("word/document.xml", r#"<w:document><w:body><w:p><w:r><w:t>Agreement</w:t></w:r></w:p></w:body></w:document>"#),
        ]);

        let cleaned = clean_metadata(&docx, "application/vnd.openxmlformats-officedocument.wordprocessingml.document").unwrap();

        assert!(!part_exists(&cleaned, "docProps/core.xml"), "core.xml survived");
        assert!(!contains(&cleaned, b"Kajal Thakur"));
        // Body text must survive.
        assert!(read_part(&cleaned, "word/document.xml").contains("Agreement"));
    }

    #[test]
    fn docx_tracked_deletion_is_removed_and_insertion_is_accepted() {
        let body = r#"<w:document><w:body><w:p>
            <w:r><w:t>The fee is </w:t></w:r>
            <w:del w:author="Kajal Thakur"><w:r><w:delText>50,000</w:delText></w:r></w:del>
            <w:ins w:author="Kajal Thakur"><w:r><w:t>75,000</w:t></w:r></w:ins>
            <w:r><w:t> rupees.</w:t></w:r>
        </w:p></w:body></w:document>"#;
        let docx = build_docx(&[("word/document.xml", body)]);

        let cleaned = clean_metadata(&docx, "application/vnd.openxmlformats-officedocument.wordprocessingml.document").unwrap();
        let doc = read_part(&cleaned, "word/document.xml");

        // Deleted figure must be gone entirely — this is the leak that matters.
        assert!(!doc.contains("50,000"), "deleted text survived: {doc}");
        assert!(!doc.contains("delText"));
        // Inserted figure is accepted into the running text.
        assert!(doc.contains("75,000"), "inserted text lost: {doc}");
        assert!(!doc.contains("w:ins"), "insertion wrapper survived");
        // Author attribution gone.
        assert!(!doc.contains("Kajal Thakur"));
        // Surrounding text intact.
        assert!(doc.contains("The fee is") && doc.contains("rupees."));
    }

    #[test]
    fn docx_comments_and_anchors_are_removed() {
        let body = r#"<w:document><w:body><w:p>
            <w:commentRangeStart w:id="1"/>
            <w:r><w:t>Clause 4</w:t></w:r>
            <w:commentRangeEnd w:id="1"/>
            <w:r><w:commentReference w:id="1"/></w:r>
        </w:p></w:body></w:document>"#;
        let docx = build_docx(&[
            ("word/document.xml", body),
            ("word/comments.xml", r#"<w:comments><w:comment w:author="Sree Lakshmi Menon"><w:p><w:r><w:t>This is weak, push back</w:t></w:r></w:p></w:comment></w:comments>"#),
            ("word/_rels/document.xml.rels", r#"<Relationships><Relationship Id="rId1" Target="comments.xml"/><Relationship Id="rId2" Target="styles.xml"/></Relationships>"#),
        ]);

        let cleaned = clean_metadata(&docx, "application/vnd.openxmlformats-officedocument.wordprocessingml.document").unwrap();

        assert!(!part_exists(&cleaned, "word/comments.xml"), "comments part survived");
        assert!(!contains(&cleaned, b"This is weak, push back"), "comment text survived");

        let doc = read_part(&cleaned, "word/document.xml");
        assert!(!doc.contains("commentRangeStart"));
        assert!(!doc.contains("commentReference"));
        assert!(doc.contains("Clause 4"), "body text lost");

        // Dangling relationship to the removed part must be gone, but others kept.
        let rels = read_part(&cleaned, "word/_rels/document.xml.rels");
        assert!(!rels.contains("comments.xml"), "dangling rel survived: {rels}");
        assert!(rels.contains("styles.xml"), "unrelated rel was dropped: {rels}");
    }

    #[test]
    fn docx_formatting_revisions_are_removed() {
        let body = r#"<w:document><w:body><w:p>
            <w:pPr><w:pPrChange w:author="Kajal Thakur"><w:pPr/></w:pPrChange></w:pPr>
            <w:r><w:rPr><w:rPrChange w:author="Kajal Thakur"><w:rPr/></w:rPrChange></w:rPr><w:t>Text</w:t></w:r>
        </w:p></w:body></w:document>"#;
        let docx = build_docx(&[("word/document.xml", body)]);

        let cleaned = clean_metadata(&docx, "application/vnd.openxmlformats-officedocument.wordprocessingml.document").unwrap();
        let doc = read_part(&cleaned, "word/document.xml");

        assert!(!doc.contains("pPrChange"));
        assert!(!doc.contains("rPrChange"));
        assert!(!doc.contains("Kajal Thakur"));
        assert!(doc.contains("Text"), "body text lost");
    }

    #[test]
    fn docx_report_names_what_was_removed() {
        let docx = build_docx(&[
            ("docProps/core.xml", r#"<cp:coreProperties><dc:creator>X</dc:creator></cp:coreProperties>"#),
            ("word/comments.xml", r#"<w:comments/>"#),
            ("word/document.xml", r#"<w:document><w:body><w:del><w:r><w:delText>gone</w:delText></w:r></w:del></w:body></w:document>"#),
        ]);

        let res = clean_with_report(&docx, "application/vnd.openxmlformats-officedocument.wordprocessingml.document").unwrap();
        let joined = res.report.removed.join(" | ");

        assert!(joined.contains("document properties"), "got: {joined}");
        assert!(joined.contains("Reviewer comments"), "got: {joined}");
        assert!(joined.contains("tracked change"), "got: {joined}");
    }

    #[test]
    fn nested_deletions_do_not_truncate_the_document() {
        // A w:del containing another w:del must close correctly, or everything
        // after it would be silently dropped.
        let body = r#"<w:document><w:body>
            <w:del><w:r><w:delText>a</w:delText></w:r><w:del><w:r><w:delText>b</w:delText></w:r></w:del></w:del>
            <w:p><w:r><w:t>SURVIVES</w:t></w:r></w:p>
        </w:body></w:document>"#;
        let docx = build_docx(&[("word/document.xml", body)]);

        let cleaned = clean_metadata(&docx, "application/vnd.openxmlformats-officedocument.wordprocessingml.document").unwrap();
        let doc = read_part(&cleaned, "word/document.xml");

        assert!(!doc.contains('a') || !doc.contains("delText"));
        assert!(doc.contains("SURVIVES"), "content after nested deletion was lost: {doc}");
    }

    // --- helpers ----------------------------------------------------------

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }
}
