// Annexures — the proof attached to a document that leaves the firm.
//
// A demand notice asserts facts: a sum was paid on a date, a message was sent,
// a receipt was issued. The firm attaches the proof of each to the notice
// itself, so what is served is one document rather than a letter and a pile of
// loose paper whose arrival can later be denied.
//
// WHY THIS IS A MODULE AND NOT A FEW LINES IN THE COMMAND
//
// Two things have to be produced from one list of documents: the list of
// annexures printed at the foot of the notice, and the annexure pages appended
// after it. If those are built separately they will eventually disagree — the
// list promising an Annexure D that is not in the bundle, which is exactly the
// discrepancy opposing counsel is paid to find. Here they are two functions
// over the same `Vec<Annexure>`, and `marks_match_the_pages` fails if that
// stops being true.
//
// MARKING IS NOT THE ATTORNEY'S JOB
//
// Marks are allocated in order: A, B, C. Nothing about a mark is typed by hand,
// so inserting a document in the middle renumbers everything below it and the
// printed list follows.
//
// An annexure page carries the document and its mark, and nothing else. No
// caption, no heading, no firm letterhead — the recipient sees the evidence as
// it was, with only the marking added.

use crate::services::latex::{self, Attachment, Field};
use crate::storage::metadata::{self, Format};
use anyhow::{bail, Result};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// What kind of file an annexure is, once its bytes have been looked at.
///
/// Sniffed, never taken from the `mime_type` column: a document uploaded with
/// the wrong type recorded against it would otherwise reach the engine as the
/// wrong `\include*` command and fail the compile of a notice at the moment it
/// was needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnexureKind {
    /// Included page for page, keeping the source's own pagination.
    Pdf,
    /// Set on a page of ours, centred, with the attorney's description beneath.
    Image,
}

impl AnnexureKind {
    fn extension(self, format: Format) -> &'static str {
        match (self, format) {
            (AnnexureKind::Pdf, _) => "pdf",
            (AnnexureKind::Image, Format::Png) => "png",
            (AnnexureKind::Image, _) => "jpg",
        }
    }
}

/// One annexure, marked and ready to be staged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annexure {
    /// "A", "B", "C" — allocated from the order the attorney put them in.
    pub mark: String,
    /// What the attorney called this. Printed in the list on the notice; never
    /// on the annexure page itself.
    pub title: String,
    /// The name the staged file has in the compile directory.
    pub file_name: String,
    pub kind: AnnexureKind,
}

/// A document on its way to becoming an annexure: the bytes, and what the
/// attorney called it.
pub struct Source {
    /// The file's bytes, already stripped of metadata.
    pub bytes: Vec<u8>,
    /// What the attorney called it — "Receipt one".
    pub title: String,
    /// For the error message when the file is a type that cannot be attached.
    pub filename: String,
}

// ---------------------------------------------------------------------------
// Preparation
// ---------------------------------------------------------------------------

/// Mark, name and classify a list of documents.
///
/// Refuses the whole batch if any one document cannot be attached. A notice
/// served with one of its four annexures silently missing is worse than a
/// notice that would not compile.
pub fn prepare(sources: &[Source]) -> Result<(Vec<Annexure>, Vec<Attachment>)> {
    let mut annexures = Vec::with_capacity(sources.len());
    let mut attachments = Vec::with_capacity(sources.len());

    for (index, source) in sources.iter().enumerate() {
        let format = metadata::detect_format(&source.bytes, "");
        let kind = match format {
            Format::Pdf => AnnexureKind::Pdf,
            Format::Png | Format::Jpeg => AnnexureKind::Image,
            _ => bail!(
                "\"{}\" cannot be attached as an annexure. Annexures must be PDF, \
                 PNG or JPEG files.",
                source.filename
            ),
        };

        // Named from the position, not from the mark or the original filename:
        // it is the one string here that becomes a path, and a counter cannot
        // contain a separator, a space or a character LaTeX reads as markup.
        let file_name = format!("annexure-{:02}.{}", index + 1, kind.extension(format));

        attachments.push(Attachment::new(&file_name, source.bytes.clone())?);
        annexures.push(Annexure {
            mark: letter(index),
            title: source.title.clone(),
            file_name,
            kind,
        });
    }

    Ok((annexures, attachments))
}

/// 0 -> A, 25 -> Z, 26 -> AA. A notice with twenty-six annexures is unusual;
/// one that silently reused a letter would be a problem.
fn letter(mut index: usize) -> String {
    let mut out = Vec::new();
    loop {
        out.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).expect("ASCII letters")
}

// ---------------------------------------------------------------------------
// The two blocks
// ---------------------------------------------------------------------------

/// The list printed at the foot of the notice.
///
/// Empty when there is nothing attached — including the heading, so a notice
/// with no annexures does not print "Enclosures / Annexures:" over blank paper.
pub fn list_block(annexures: &[Annexure]) -> Field {
    if annexures.is_empty() {
        return Field::raw("");
    }

    let mut out = String::from("\\annexurelistheading\n");
    for annexure in annexures {
        out.push_str(&format!(
            "\\annexureentry{{{}}}{{{}}}\n",
            latex::escape(&annexure.mark),
            latex::escape(&annexure.title)
        ));
    }
    Field::raw(out)
}

/// The annexure pages themselves, appended after the notice.
pub fn pages_block(annexures: &[Annexure]) -> Field {
    let mut out = String::new();
    for annexure in annexures {
        // The file name is generated here and matches [a-z0-9.-]; the mark is
        // escaped on principle, being the only other thing interpolated.
        let mark = latex::escape(&annexure.mark);
        out.push_str(&match annexure.kind {
            // A PDF keeps its own pages, and its own page numbering: renumbering
            // someone else's document would misrepresent it.
            AnnexureKind::Pdf => {
                format!("\\annexurepdf{{{mark}}}{{{}}}\n", annexure.file_name)
            }
            AnnexureKind::Image => {
                format!("\\annexureimage{{{mark}}}{{{}}}\n", annexure.file_name)
            }
        });
    }
    Field::raw(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Smallest thing `detect_format` will call a PDF.
    fn pdf(marker: &str) -> Vec<u8> {
        format!("%PDF-1.4\n{marker}\n%%EOF\n").into_bytes()
    }

    fn png() -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        bytes.extend_from_slice(&[0; 32]);
        bytes
    }

    fn source(title: &str, bytes: Vec<u8>) -> Source {
        Source { bytes, title: title.to_owned(), filename: format!("{title}.bin") }
    }

    #[test]
    fn marks_run_in_order_without_the_attorney_typing_one() {
        let sources = vec![
            source("Receipt", pdf("a")),
            source("Statement", pdf("b")),
            source("Screenshot", png()),
        ];
        let (annexures, _) = prepare(&sources).unwrap();
        let marks: Vec<&str> = annexures.iter().map(|a| a.mark.as_str()).collect();
        assert_eq!(marks, ["A", "B", "C"]);
    }

    #[test]
    fn the_kind_comes_from_the_bytes() {
        let sources = vec![
            source("Receipt", pdf("a")),
            source("Screenshot", png()),
        ];
        let (annexures, attachments) = prepare(&sources).unwrap();

        assert_eq!(annexures[0].kind, AnnexureKind::Pdf);
        assert_eq!(annexures[1].kind, AnnexureKind::Image);
        assert_eq!(attachments[0].name(), "annexure-01.pdf");
        assert_eq!(attachments[1].name(), "annexure-02.png");
    }

    /// Serving a notice whose Annexure C is quietly absent is worse than a
    /// notice that would not build.
    #[test]
    fn one_unattachable_document_refuses_the_whole_batch() {
        let sources = vec![
            source("Receipt", pdf("a")),
            Source {
                bytes: b"PK\x03\x04 not really a document".to_vec(),
                title: "Spreadsheet".into(),
                filename: "ledger.xlsx".into(),
            },
        ];
        let err = prepare(&sources).unwrap_err().to_string();
        assert!(err.contains("ledger.xlsx"), "the attorney must be told which one: {err}");
        assert!(err.contains("PDF"), "and what is acceptable: {err}");
    }

    /// The list and the pages are built from one vec, and this is the assertion
    /// that says so: same marks, same order, same count.
    #[test]
    fn marks_match_the_pages() {
        let sources = vec![
            source("Receipt", pdf("a")),
            source("Transfer", pdf("b")),
            source("Screenshot", png()),
        ];
        let (annexures, attachments) = prepare(&sources).unwrap();

        let list = list_block(&annexures);
        let pages = pages_block(&annexures);
        let (list, pages) = (list.as_str(), pages.as_str());

        for annexure in &annexures {
            assert!(list.contains(&annexure.mark), "{} missing from the list", annexure.mark);
            assert!(pages.contains(&annexure.mark), "{} missing from the pages", annexure.mark);
        }
        assert_eq!(attachments.len(), annexures.len());
        assert_eq!(pages.matches("annexure-").count(), 3);
    }

    #[test]
    fn nothing_attached_prints_no_heading() {
        assert_eq!(list_block(&[]), Field::raw(""));
        assert_eq!(pages_block(&[]), Field::raw(""));
    }

    /// A title is the attorney's prose and reaches the document as LaTeX.
    #[test]
    fn a_title_is_escaped_into_the_list() {
        let sources = vec![source("Invoice for M/s Tata & Sons — 100% paid", pdf("a"))];
        let (annexures, _) = prepare(&sources).unwrap();

        let list = list_block(&annexures);
        let list = list.as_str();
        assert!(list.contains(r"Tata \& Sons"), "unescaped ampersand: {list}");
        assert!(list.contains(r"100\%"), "unescaped percent: {list}");
    }

    /// The annexure page carries the document and its mark, and nothing else.
    /// The title belongs in the list on the notice, not printed over evidence.
    #[test]
    fn a_title_never_reaches_the_annexure_pages() {
        let sources = vec![
            source("Receipt one", pdf("a")),
            source("Screenshot two", png()),
        ];
        let (annexures, _) = prepare(&sources).unwrap();

        let pages = pages_block(&annexures);
        let pages = pages.as_str();
        assert!(!pages.contains("Receipt one"), "a caption reached a PDF page: {pages}");
        assert!(!pages.contains("Screenshot two"), "a caption reached an image page: {pages}");
        assert!(pages.contains("{A}") && pages.contains("{B}"), "marks missing: {pages}");
    }

    #[test]
    fn letters_carry_past_z() {
        assert_eq!(letter(0), "A");
        assert_eq!(letter(25), "Z");
        assert_eq!(letter(26), "AA");
        assert_eq!(letter(27), "AB");
        assert_eq!(letter(51), "AZ");
        assert_eq!(letter(52), "BA");
    }

}

// ---------------------------------------------------------------------------
// Compilation tests
// ---------------------------------------------------------------------------
//
// The unit tests above prove the marks are allocated correctly. They cannot
// prove the bundle exists: `\includepdf` of an unreachable file, a scale that
// puts the mark over the content, a stamp that lags a page — none of those are
// visible from a string comparison. These compile the real notice with a real
// PDF and a real image attached, and read the result back.

#[cfg(test)]
mod compile_tests {
    use super::*;
    use crate::services::latex::{self, CompileMode};
    use std::collections::HashMap;

    // --- fixtures -----------------------------------------------------------

    /// A genuine multi-page PDF, produced by the same engine that will later
    /// include it. Hand-rolling PDF bytes would test our fixture, not pdfpages.
    async fn receipt_pdf(pages: usize) -> Vec<u8> {
        // Joined rather than appended, so there is no trailing \newpage and the
        // fixture has exactly the page count it claims — a blank last page would
        // make the "every page is marked" counts silently wrong.
        let body: String = (1..=pages)
            .map(|p| format!("Receipt page {p} of {pages}.\n"))
            .collect::<Vec<_>>()
            .join("\\newpage\n");
        let source = format!(
            "\\documentclass{{article}}\n\\usepackage[margin=25mm]{{geometry}}\n\
             \\begin{{document}}\n{body}\\end{{document}}\n"
        );
        latex::run_engine("fixture-receipt", &source)
            .await
            .expect("the fixture PDF must compile")
    }

    /// A valid 8x8 greyscale PNG: correct CRCs, and a zlib stream of stored
    /// (uncompressed) deflate blocks. XeLaTeX rejects anything less.
    fn screenshot_png() -> Vec<u8> {
        const W: usize = 8;
        const H: usize = 8;

        // One filter byte per row, then the row's pixels.
        let mut raw = Vec::with_capacity(H * (W + 1));
        for y in 0..H {
            raw.push(0u8); // filter: none
            for x in 0..W {
                raw.push(if (x + y) % 2 == 0 { 0x20 } else { 0xE0 });
            }
        }

        let mut zlib = vec![0x78, 0x01];
        zlib.push(0x01); // final block, stored
        zlib.extend_from_slice(&(raw.len() as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
        zlib.extend_from_slice(&raw);
        zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&(W as u32).to_be_bytes());
        ihdr.extend_from_slice(&(H as u32).to_be_bytes());
        ihdr.extend_from_slice(&[8, 0, 0, 0, 0]); // 8-bit greyscale, no interlace

        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend(chunk(b"IHDR", &ihdr));
        png.extend(chunk(b"IDAT", &zlib));
        png.extend(chunk(b"IEND", &[]));
        png
    }

    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = (data.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);

        let mut crc_over = kind.to_vec();
        crc_over.extend_from_slice(data);
        out.extend_from_slice(&crc32(&crc_over).to_be_bytes());
        out
    }

    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for byte in bytes {
            crc ^= *byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
            }
        }
        !crc
    }

    fn adler32(bytes: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for byte in bytes {
            a = (a + *byte as u32) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }

    /// Everything the legal notice needs apart from its annexures.
    fn notice_fields() -> HashMap<String, latex::Field> {
        let mut fields: HashMap<String, latex::Field> = [
            ("PARTNER_ONE_NAME", "Sreelakshmi Menon"),
            ("PARTNER_ONE_ROLE", "Advocate & Partner"),
            ("PARTNER_ONE_PHONE", "+91 99535 31789"),
            ("PARTNER_ONE_EMAIL", "sreelakshmimenon.pnp@outlook.com"),
            ("PARTNER_TWO_NAME", "Kajal Thakur"),
            ("PARTNER_TWO_ROLE", "Advocate & Partner"),
            ("PARTNER_TWO_PHONE", "+91 93153 67642"),
            ("PARTNER_TWO_EMAIL", "kajalthakur.pnp@outlook.com"),
            ("FIRM_WEBSITE", "www.persistas.com"),
            ("FIRM_CONTACT", "persistas.pnp@outlook.com"),
            ("FIRM_OFFICE_LINE_ONE", "80-A, Pocket-A, Mayuri Enclave,"),
            ("FIRM_OFFICE_LINE_TWO", "Mayur Vihar Phase-III, Delhi - 110096"),
            ("RECIPIENT_NAME", "Mr. Mohammed Danish"),
            ("MODE_OF_SERVICE", "THROUGH SPEED POST/ WHATSAPP"),
            ("SUBJECT", "DEMAND FOR REFUND OF ₹1,04,000/- WITH INTEREST"),
            ("SALUTATION", "Sir"),
            ("CLIENT_NAME", "Mr. Nikhil Prabhakar"),
            ("CLIENT_DESCRIPTION", "son of P. Prabhakaran"),
            ("CLIENT_ADDRESS", "A-004, Mangal Apartment, New Delhi-110096"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), latex::Field::text(v)))
        .collect();

        fields.insert("NOTICE_DATE".into(), latex::Field::raw("14\\textsuperscript{th} July 2026"));
        fields.insert(
            "RECIPIENT_ADDRESS_BLOCK".into(),
            latex::Field::raw("House No. 460/21,\\\\\nLucknow -- 226003"),
        );
        fields.insert(
            "SECTIONS_BLOCK".into(),
            latex::Field::raw(
                "\\noticesection{1}{Background}\n\\begin{noticebody}\n\
                 That you received a sum from my client and have not repaid it.\n\
                 \\end{noticebody}\n",
            ),
        );
        fields.insert(
            "SIGNATORY_BLOCK".into(),
            latex::Field::raw("Sree Lakshmi Menon\\\\\nD/6361/2020\\\\\nAdvocates"),
        );
        fields
    }

    async fn compile_notice(annexures: &[Annexure], attachments: &[Attachment]) -> Vec<u8> {
        let mut fields = notice_fields();
        fields.insert("ANNEXURES_BLOCK".into(), list_block(annexures));
        fields.insert("ANNEXURE_PAGES".into(), pages_block(annexures));

        latex::compile_with("legal-notice", &fields, CompileMode::Final, attachments)
            .await
            .expect("the notice must compile with its annexures")
    }

    // --- the tests ----------------------------------------------------------

    /// The whole point: a notice, its proof, and every page of that proof
    /// marked — without an attorney typing a single mark.
    #[tokio::test]
    async fn a_notice_carries_its_annexures_and_every_page_is_marked() {
        if !latex::engine_available() {
            return;
        }

        let sources = vec![
            Source {
                bytes: receipt_pdf(3).await,
                title: "Receipt dated 14 July 2026".into(),
                filename: "receipt.pdf".into(),
            },
            Source {
                bytes: screenshot_png(),
                title: "Screenshot of the payment confirmation".into(),
                filename: "screenshot.png".into(),
            },
            Source {
                bytes: receipt_pdf(2).await,
                title: "Bank statement".into(),
                filename: "statement.pdf".into(),
            },
        ];

        let (annexures, attachments) = prepare(&sources).unwrap();
        let pdf = compile_notice(&annexures, &attachments).await;

        let text = pdf_text(&pdf);

        // The list, on the notice itself.
        assert!(text.contains("Enclosures / Annexures"), "no annexure list:\n{text}");
        assert!(text.contains("Receipt dated 14 July 2026"), "title missing:\n{text}");

        // Three annexures, six annexure pages (3 + 1 + 2), each stamped.
        assert_eq!(text.matches("ANNEXURE A").count(), 3, "in:\n{text}");
        assert_eq!(text.matches("ANNEXURE B").count(), 1, "in:\n{text}");
        assert_eq!(text.matches("ANNEXURE C").count(), 2, "in:\n{text}");

        // The included pages are really there, in order.
        assert!(text.contains("Receipt page 3 of 3"));
        assert!(text.contains("Receipt page 2 of 2"));
    }

    /// The mark must sit at the top of the paper, clear of whatever the
    /// annexure itself says.
    ///
    /// The obvious implementation — a fancyhdr page style — inherits the 108pt
    /// head band \applyletterhead needs for the firm's own pages, which pushes
    /// the mark down onto the first line of the document being marked. Measured
    /// that way: mark at y=137pt, receipt text at y=144pt, overlapping. So this
    /// asserts a position, not merely that the words are somewhere on the page.
    #[tokio::test]
    async fn the_mark_sits_at_the_top_of_the_page_clear_of_the_content() {
        if !latex::engine_available() {
            return;
        }

        let sources = vec![Source {
            bytes: receipt_pdf(2).await,
            title: "Receipt".into(),
            filename: "receipt.pdf".into(),
        }];

        let (annexures, attachments) = prepare(&sources).unwrap();
        let pdf = compile_notice(&annexures, &attachments).await;

        // A4 is 842pt tall. 60pt is about 21mm from the top edge — above any
        // plausible first line of a scanned or typeset page.
        const TOP_BAND: f32 = 60.0;

        let marks = word_positions(&pdf, "ANNEXURE");
        assert_eq!(marks.len(), 2, "expected one mark per annexure page");
        for (page, y) in marks {
            assert!(
                y < TOP_BAND,
                "the mark on page {page} is {y}pt down the page — it is sitting in \
                 the annexure's own content, not above it"
            );
        }
    }

    /// Marks stay with their own annexure. Cheap to get wrong the moment the
    /// stamp is made to depend on state that outlives the page.
    #[tokio::test]
    async fn the_last_page_of_an_annexure_does_not_carry_the_next_ones_mark() {
        if !latex::engine_available() {
            return;
        }

        let sources = vec![
            Source {
                bytes: receipt_pdf(2).await,
                title: "First".into(),
                filename: "first.pdf".into(),
            },
            Source {
                bytes: receipt_pdf(2).await,
                title: "Second".into(),
                filename: "second.pdf".into(),
            },
        ];

        let (annexures, attachments) = prepare(&sources).unwrap();
        let pdf = compile_notice(&annexures, &attachments).await;

        // Walk the annexure pages from the back: the last four pages are
        // A, A, B, B in that order and nothing else.
        let pages = pdf_pages(&pdf);
        let tail: Vec<&String> = pages.iter().rev().take(4).rev().collect();

        assert!(tail[0].contains("ANNEXURE A"), "page -4: {}", tail[0]);
        assert!(tail[1].contains("ANNEXURE A"), "page -3 took B's mark: {}", tail[1]);
        assert!(tail[2].contains("ANNEXURE B"), "page -2: {}", tail[2]);
        assert!(tail[3].contains("ANNEXURE B"), "page -1: {}", tail[3]);
    }

    /// A notice with nothing attached must not print an empty heading, and must
    /// still compile — most notices carry no annexures at all.
    #[tokio::test]
    async fn a_notice_with_no_annexures_prints_no_heading() {
        if !latex::engine_available() {
            return;
        }

        let pdf = compile_notice(&[], &[]).await;
        let text = pdf_text(&pdf);

        assert!(!text.contains("Enclosures"), "an empty heading was printed:\n{text}");
        assert!(!text.contains("ANNEXURE"), "in:\n{text}");
        assert!(text.contains("Yours Sincerely"), "the notice itself is missing:\n{text}");
    }

    // --- reading the result back --------------------------------------------

    /// `pdftotext` is in poppler-utils, which CI installs alongside TeX Live.
    /// Without it these tests would assert on a byte count, which proves the
    /// engine ran and nothing about what it produced.
    fn pdftotext(pdf: &[u8], args: &[&str]) -> String {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("out.pdf");
        std::fs::write(&path, pdf).expect("write pdf");

        let output = std::process::Command::new("pdftotext")
            .args(args)
            .arg(&path)
            .arg("-")
            .output()
            .expect("pdftotext (poppler-utils) is needed to check the annexure bundle");

        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn pdf_text(pdf: &[u8]) -> String {
        pdftotext(pdf, &[])
    }

    /// Every occurrence of `word`, as (page number, distance from the top of
    /// the paper in points).
    ///
    /// Read out of pdftotext's `-bbox` XML by hand rather than with a parser:
    /// the shape is one `<word>` element per line with the attributes in a
    /// fixed order, and a dependency for four lines of string work is worse.
    fn word_positions(pdf: &[u8], word: &str) -> Vec<(usize, f32)> {
        let xml = pdftotext(pdf, &["-bbox"]);
        let mut page = 0usize;
        let mut found = Vec::new();

        for line in xml.lines() {
            let line = line.trim();
            if line.starts_with("<page ") {
                page += 1;
            }
            if !line.ends_with(&format!(">{word}</word>")) {
                continue;
            }
            let Some(rest) = line.split("yMin=\"").nth(1) else { continue };
            let Some(value) = rest.split('"').next() else { continue };
            if let Ok(y) = value.parse::<f32>() {
                found.push((page, y));
            }
        }
        found
    }

    /// The text of each page, in order.
    ///
    /// pdftotext writes a form feed *after* every page including the last, so a
    /// plain split leaves an empty trailing element and shifts anything counted
    /// from the back by one.
    fn pdf_pages(pdf: &[u8]) -> Vec<String> {
        let text = pdf_text(pdf);
        text.strip_suffix('\u{c}')
            .unwrap_or(&text)
            .split('\u{c}')
            .map(str::to_owned)
            .collect()
    }
}
