// Page formatting for a generated document.
//
// The drafter should never have to format a document. They choose what it says;
// Persist decides where it sits on the page. This module is the small set of
// choices that are genuinely the attorney's — paper, typeface, size, spacing,
// page numbering, and which pages carry the firm's letterhead — expressed once
// and applied to every template.
//
// NOT TEMPLATE FIELDS
//
// None of this is a `{{PLACEHOLDER}}`. A template describes what a legal notice
// says; how big the paper is has nothing to do with that, and putting paper size
// in nineteen manifests would mean nineteen places to change it. The layout is
// compiled into a `persist-layout.tex` staged beside the template, which
// `_shared/persist-base.tex` reads. A compile with no layout — a test, or a
// caller that predates this — gets the defaults and looks exactly as it did.
//
// THE RUPEE
//
// Only the Noto faces carry ₹ (U+20B9). Every Times, Palatino and Century
// Schoolbook clone in TeX Live silently drops it: no error, a clean compile, and
// a notice demanding "1,04,000" with no currency sign. Measured, not assumed —
// `every_offered_font_can_set_the_rupee` compiles each one and fails on a
// missing glyph. So the preamble routes ₹ through a Noto fallback via
// `newunicodechar`, which catches it in template source and attorney-typed text
// alike, whatever face the document is set in.

use anyhow::{bail, Result};

// ---------------------------------------------------------------------------
// The choices
// ---------------------------------------------------------------------------

/// Paper size.
///
/// The firm's own notices are A4 — measured off the one supplied, all nineteen
/// pages. Legal is here because filings before some forums are made on it, and
/// switching should not mean re-typesetting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Paper {
    A4,
    /// 8.5 x 14 in.
    Legal,
}

impl Paper {
    /// Width and height, as LaTeX lengths.
    ///
    /// Explicit dimensions rather than geometry's named `a4paper`/`legalpaper`
    /// options, because those are *keys* and keyval does not expand a macro in
    /// key position: `\geometry{\persistpaper}` matches nothing, says nothing,
    /// and leaves the document whatever size the class made it. It went out as
    /// A4 with "Legal" selected until a compile test measured the MediaBox.
    /// `papersize={w,h}` takes values, and values are expanded.
    fn dimensions(self) -> (&'static str, &'static str) {
        match self {
            Paper::A4 => ("210mm", "297mm"),
            // 8.5 x 14 in.
            Paper::Legal => ("215.9mm", "355.6mm"),
        }
    }
}

/// The body typeface.
///
/// A closed list, not a free string. Every entry is verified to resolve, to have
/// a real bold and italic, and to set ₹ — and every entry ships with TeX Live or
/// with the installer, so a document that renders on one partner's machine
/// renders on the other's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BodyFont {
    /// The firm's default.
    NotoSerif,
    NotoSans,
    /// Times New Roman metrics — what several Indian forums ask for by name.
    Times,
    /// Palatino.
    Pagella,
    /// Century Schoolbook.
    Schola,
    /// Helvetica metrics.
    Helvetica,
}

impl BodyFont {
    /// The family name fontconfig resolves.
    pub fn family(self) -> &'static str {
        match self {
            BodyFont::NotoSerif => "Noto Serif",
            BodyFont::NotoSans => "Noto Sans",
            BodyFont::Times => "TeX Gyre Termes",
            BodyFont::Pagella => "TeX Gyre Pagella",
            BodyFont::Schola => "TeX Gyre Schola",
            BodyFont::Helvetica => "TeX Gyre Heros",
        }
    }

    /// Every face offered, for the test that compiles each one.
    pub const ALL: [BodyFont; 6] = [
        BodyFont::NotoSerif,
        BodyFont::NotoSans,
        BodyFont::Times,
        BodyFont::Pagella,
        BodyFont::Schola,
        BodyFont::Helvetica,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineSpacing {
    Single,
    OneAndHalf,
    Double,
}

impl LineSpacing {
    fn stretch(self) -> f32 {
        match self {
            LineSpacing::Single => 1.0,
            LineSpacing::OneAndHalf => 1.5,
            LineSpacing::Double => 2.0,
        }
    }
}

/// Which pages carry the firm's identity — the mark, the partners, the
/// registered-office footer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Letterhead {
    /// Every page. A notice is served page by page, and a page without the
    /// firm's mark is a page whose provenance is arguable.
    AllPages,
    /// Page one only; the rest are plain continuation sheets.
    FirstPageOnly,
    /// Named pages. 1-indexed, as printed.
    Pages { pages: Vec<u32> },
    None,
}

/// What the page number looks like, if there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PageNumbers {
    None,
    /// `3`
    Plain,
    /// `Page 3`
    Page,
    /// `Page 3 of 19` — the firm's own footer, and the one a served document
    /// should carry: it says on its face that nothing was removed.
    PageOfTotal,
}

/// How a document is laid out. Everything here has a default that reproduces
/// the firm's house format, so a caller that sets nothing gets what it had.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DocumentLayout {
    pub paper: Paper,
    pub font: BodyFont,
    /// Points. Indian forums commonly ask for 14.
    pub font_size_pt: f32,
    pub line_spacing: LineSpacing,
    /// Set the whole body bold. Off by default, and it should stay off — this
    /// exists because a document occasionally has to be produced that way, not
    /// because it is ever good typography.
    pub bold: bool,
    pub italic: bool,
    pub letterhead: Letterhead,
    pub page_numbers: PageNumbers,
    /// What the first page is numbered. A notice filed as part of a larger
    /// paper-book starts at the page it starts at.
    pub page_number_start: u32,
}

impl Default for DocumentLayout {
    fn default() -> Self {
        DocumentLayout {
            paper: Paper::A4,
            font: BodyFont::NotoSerif,
            font_size_pt: 12.0,
            line_spacing: LineSpacing::Single,
            bold: false,
            italic: false,
            letterhead: Letterhead::AllPages,
            page_numbers: PageNumbers::PageOfTotal,
            page_number_start: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Compiling it to LaTeX
// ---------------------------------------------------------------------------

/// The name the generated file is staged under. `_shared/persist-base.tex`
/// reads exactly this.
pub const LAYOUT_FILE: &str = "persist-layout.tex";

impl DocumentLayout {
    /// Reject a layout that would produce a document nobody wants before it
    /// reaches the engine, where the failure would be a LaTeX error instead of
    /// a sentence.
    pub fn validate(&self) -> Result<()> {
        if !(6.0..=32.0).contains(&self.font_size_pt) {
            bail!("Font size must be between 6 and 32 points.");
        }
        if self.page_number_start < 1 {
            bail!("The first page number must be 1 or more.");
        }
        if let Letterhead::Pages { pages } = &self.letterhead {
            if pages.is_empty() {
                bail!("Choose at least one page for the letterhead, or turn it off.");
            }
            if pages.iter().any(|p| *p == 0) {
                bail!("Pages are numbered from 1.");
            }
        }
        Ok(())
    }

    /// The generated preamble fragment.
    ///
    /// Definitions only — no packages are loaded here. `persist-base.tex` reads
    /// this first and then does the loading, so the order in which fontspec,
    /// geometry and scrextend see their arguments stays in one file.
    pub fn to_latex(&self) -> String {
        let mut out = String::from(
            "% Generated per render by services/layout.rs. Not a checked-in file.\n\
             % _shared/persist-base.tex reads this, then falls back to its own\n\
             % defaults for anything left unset.\n",
        );

        let (width, height) = self.paper.dimensions();
        out.push_str(&format!("\\def\\persistpaperwidth{{{width}}}\n"));
        out.push_str(&format!("\\def\\persistpaperheight{{{height}}}\n"));
        out.push_str(&format!("\\def\\persistmainfont{{{}}}\n", self.font.family()));

        // scrextend wants both numbers. Leading is the type size times the
        // chosen spacing times 1.2 — the ratio LaTeX itself uses for "single".
        let size = self.font_size_pt;
        let leading = size * 1.2 * self.line_spacing.stretch();
        out.push_str(&format!("\\def\\persistfontsize{{{size:.1}pt}}\n"));
        out.push_str(&format!("\\def\\persistleading{{{leading:.1}pt}}\n"));
        out.push_str(&format!(
            "\\def\\persiststretch{{{:.2}}}\n",
            self.line_spacing.stretch()
        ));

        let mut body_style = String::new();
        if self.bold {
            body_style.push_str("\\bfseries");
        }
        if self.italic {
            body_style.push_str("\\itshape");
        }
        out.push_str(&format!("\\def\\persistbodystyle{{{body_style}}}\n"));

        out.push_str(&format!(
            "\\def\\persistpagenumbers{{{}}}\n",
            match self.page_numbers {
                PageNumbers::None => "none",
                PageNumbers::Plain => "plain",
                PageNumbers::Page => "page",
                PageNumbers::PageOfTotal => "pageoftotal",
            }
        ));
        // setcounter would fight \pagenumbering; an offset added at print time
        // leaves \value{page} meaning "sheet number", which is what the
        // letterhead-on-page test compares against.
        out.push_str(&format!(
            "\\def\\persistpageoffset{{{}}}\n",
            self.page_number_start as i64 - 1
        ));

        out.push_str(&self.letterhead_latex());
        out
    }

    /// `\persistheadtest` sets `\ifpersistshowhead` for the page being shipped.
    ///
    /// Emitted as generated code rather than parsed in TeX: a comma list would
    /// have to be walked at shipout, and the disjunction is three lines here and
    /// a small horror there.
    fn letterhead_latex(&self) -> String {
        let mode = match self.letterhead {
            Letterhead::AllPages => "all",
            Letterhead::FirstPageOnly => "first",
            Letterhead::Pages { .. } => "pages",
            Letterhead::None => "none",
        };

        let mut out = format!("\\def\\persistletterheadmode{{{mode}}}\n");
        out.push_str("\\newcommand{\\persistheadtest}{%\n");

        match &self.letterhead {
            Letterhead::AllPages => out.push_str("  \\persistshowheadtrue\n"),
            Letterhead::None => out.push_str("  \\persistshowheadfalse\n"),
            // In `first` mode the letterhead is set as body content on page one
            // rather than as a running head — see persist-letterhead.tex — so
            // the running head is never wanted.
            Letterhead::FirstPageOnly => out.push_str("  \\persistshowheadfalse\n"),
            Letterhead::Pages { pages } => {
                out.push_str("  \\persistshowheadfalse\n");
                let mut seen = pages.clone();
                seen.sort_unstable();
                seen.dedup();
                for page in seen {
                    out.push_str(&format!(
                        "  \\ifnum\\value{{page}}={page}\\relax\\persistshowheadtrue\\fi\n"
                    ));
                }
            }
        }

        out.push_str("}\n");
        out
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_firms_house_format() {
        let layout = DocumentLayout::default();
        let tex = layout.to_latex();

        assert!(tex.contains("\\def\\persistpaperwidth{210mm}"), "{tex}");
        assert!(tex.contains("\\def\\persistpaperheight{297mm}"), "{tex}");
        assert!(tex.contains("\\def\\persistmainfont{Noto Serif}"), "{tex}");
        assert!(tex.contains("\\def\\persistfontsize{12.0pt}"), "{tex}");
        assert!(tex.contains("\\def\\persistpagenumbers{pageoftotal}"), "{tex}");
        assert!(tex.contains("\\persistshowheadtrue"), "{tex}");
        assert!(tex.contains("\\def\\persistbodystyle{}"), "{tex}");
    }

    #[test]
    fn leading_follows_the_size_and_the_spacing() {
        let layout = DocumentLayout {
            font_size_pt: 14.0,
            line_spacing: LineSpacing::OneAndHalf,
            ..Default::default()
        };
        // 14 x 1.2 x 1.5
        assert!(layout.to_latex().contains("\\def\\persistleading{25.2pt}"));
    }

    #[test]
    fn named_pages_become_one_test_each_sorted_and_deduplicated() {
        let layout = DocumentLayout {
            letterhead: Letterhead::Pages { pages: vec![3, 1, 3] },
            ..Default::default()
        };
        let tex = layout.to_latex();

        assert_eq!(tex.matches("\\persistshowheadtrue").count(), 2, "{tex}");
        let first = tex.find("=1\\relax").expect("page 1 missing");
        let third = tex.find("=3\\relax").expect("page 3 missing");
        assert!(first < third, "pages should be emitted in order: {tex}");
    }

    /// `first` puts the letterhead in the body, not in a running head — so the
    /// continuation sheets get a normal top margin instead of a blank band the
    /// height of the firm's mark.
    #[test]
    fn first_page_only_does_not_use_a_running_head() {
        let tex = DocumentLayout {
            letterhead: Letterhead::FirstPageOnly,
            ..Default::default()
        }
        .to_latex();

        assert!(tex.contains("\\def\\persistletterheadmode{first}"), "{tex}");
        assert!(!tex.contains("\\persistshowheadtrue"), "{tex}");
    }

    #[test]
    fn a_starting_page_number_becomes_an_offset() {
        let tex = DocumentLayout { page_number_start: 7, ..Default::default() }.to_latex();
        assert!(tex.contains("\\def\\persistpageoffset{6}"), "{tex}");
    }

    #[test]
    fn body_style_carries_both_switches() {
        let tex = DocumentLayout { bold: true, italic: true, ..Default::default() }.to_latex();
        assert!(tex.contains("\\def\\persistbodystyle{\\bfseries\\itshape}"), "{tex}");
    }

    #[test]
    fn an_unusable_layout_is_refused_with_something_an_attorney_can_act_on() {
        let too_small = DocumentLayout { font_size_pt: 2.0, ..Default::default() };
        assert!(too_small.validate().unwrap_err().to_string().contains("between 6 and 32"));

        let no_pages = DocumentLayout {
            letterhead: Letterhead::Pages { pages: Vec::new() },
            ..Default::default()
        };
        assert!(no_pages.validate().unwrap_err().to_string().contains("at least one page"));

        let page_zero = DocumentLayout {
            letterhead: Letterhead::Pages { pages: vec![0] },
            ..Default::default()
        };
        assert!(page_zero.validate().unwrap_err().to_string().contains("numbered from 1"));

        assert!(DocumentLayout::default().validate().is_ok());
    }

    /// The layout arrives from Deck. Nothing in it may be free text that reaches
    /// the engine as markup — every field is an enum or a number.
    #[test]
    fn nothing_in_a_layout_is_free_text() {
        let json = r#"{
            "paper": "legal",
            "font": "times",
            "fontSizePt": 14,
            "lineSpacing": "oneAndHalf",
            "bold": false,
            "italic": false,
            "letterhead": { "type": "firstPageOnly" },
            "pageNumbers": "page",
            "pageNumberStart": 1
        }"#;
        let layout: DocumentLayout = serde_json::from_str(json).unwrap();
        assert_eq!(layout.paper, Paper::Legal);
        assert_eq!(layout.font, BodyFont::Times);

        // A font name Deck invented is not a font this can be set in.
        let injected = r#"{ "font": "Noto Serif}\\input{/etc/passwd}{" }"#;
        assert!(serde_json::from_str::<DocumentLayout>(injected).is_err());
    }

    #[test]
    fn an_absent_field_takes_the_default() {
        let layout: DocumentLayout = serde_json::from_str(r#"{ "paper": "legal" }"#).unwrap();
        assert_eq!(layout.paper, Paper::Legal);
        assert_eq!(layout.font, BodyFont::NotoSerif);
        assert_eq!(layout.font_size_pt, 12.0);
    }
}

// ---------------------------------------------------------------------------
// Compilation tests
// ---------------------------------------------------------------------------
//
// The unit tests above prove the right LaTeX is generated. They cannot prove the
// engine does anything with it — a paper size that geometry ignores, a font that
// resolves to something else, a page number that prints when it was turned off.
// These compile the real notice and measure the result.

#[cfg(test)]
mod compile_tests {
    use super::*;
    use crate::services::latex::{self, CompileMode, Field};
    use std::collections::HashMap;

    /// The notice, with nothing attached. The letterhead needs its own fields
    /// and the template refuses to render with any placeholder unfilled.
    fn notice_fields() -> HashMap<String, Field> {
        let mut fields: HashMap<String, Field> = [
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
        .map(|(k, v)| (k.to_owned(), Field::text(v)))
        .collect();

        fields.insert("NOTICE_DATE".into(), Field::raw("14\\textsuperscript{th} July 2026"));
        fields.insert(
            "RECIPIENT_ADDRESS_BLOCK".into(),
            Field::raw("House No. 460/21,\\\\\nLucknow -- 226003"),
        );
        // Long enough to run onto a second page, so page-two behaviour is real.
        let mut sections = String::new();
        for n in 1..=6 {
            sections.push_str(&format!(
                "\\noticesection{{{n}}}{{Background}}\n\\begin{{noticebody}}\n\
                 That you received a sum from my client and have not repaid it, \
                 despite repeated requests made in person, over the telephone and \
                 in writing, over a period of several months.\n\
                 \\end{{noticebody}}\n"
            ));
        }
        fields.insert("SECTIONS_BLOCK".into(), Field::raw(sections));
        fields.insert(
            "SIGNATORY_BLOCK".into(),
            Field::raw("Sree Lakshmi Menon\\\\\nD/6361/2020\\\\\nAdvocates"),
        );
        fields.insert("ANNEXURES_BLOCK".into(), Field::raw(""));
        fields.insert("ANNEXURE_PAGES".into(), Field::raw(""));
        fields
    }

    async fn compile(layout: &DocumentLayout) -> Vec<u8> {
        let staged = latex::Attachment::new(LAYOUT_FILE, layout.to_latex().into_bytes()).unwrap();
        latex::compile_with(
            "legal-notice",
            &notice_fields(),
            CompileMode::Final,
            std::slice::from_ref(&staged),
        )
        .await
        .expect("the notice must compile under every layout offered")
    }

    fn pdfinfo(pdf: &[u8], flags: &[&str]) -> String {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.pdf");
        std::fs::write(&path, pdf).unwrap();
        let out = std::process::Command::new("pdfinfo")
            .args(flags)
            .arg(&path)
            .output()
            .expect("pdfinfo (poppler-utils) is needed to measure the page");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// Page size in points. Parsed rather than string-matched: xdvipdfmx writes
    /// a fractional MediaBox, so pdfinfo prints "595.276 x 841.89 pts (A4)" and
    /// an assertion on "595 x 842" never matches anything.
    fn page_size(pdf: &[u8]) -> (f32, f32) {
        let info = pdfinfo(pdf, &[]);
        let line = info
            .lines()
            .find(|l| l.starts_with("Page size:"))
            .unwrap_or_else(|| panic!("no page size in:\n{info}"));

        let mut numbers = line
            .split_whitespace()
            .filter_map(|token| token.parse::<f32>().ok());
        let width = numbers.next().expect("width");
        let height = numbers.next().expect("height");
        (width, height)
    }

    fn page_count(pdf: &[u8]) -> u32 {
        let info = pdfinfo(pdf, &[]);
        info.lines()
            .find(|l| l.starts_with("Pages:"))
            .and_then(|l| l.split_whitespace().last()?.parse().ok())
            .unwrap_or_else(|| panic!("no page count in:\n{info}"))
    }

    fn pdf_text(pdf: &[u8]) -> String {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.pdf");
        std::fs::write(&path, pdf).unwrap();
        let out = std::process::Command::new("pdftotext")
            .arg(&path)
            .arg("-")
            .output()
            .expect("pdftotext is needed to read the document back");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn embedded_fonts(pdf: &[u8]) -> String {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.pdf");
        std::fs::write(&path, pdf).unwrap();
        let out = std::process::Command::new("pdffonts")
            .arg(&path)
            .output()
            .expect("pdffonts is needed to check which face was used");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    // --- paper ---------------------------------------------------------------

    #[tokio::test]
    async fn a4_and_legal_produce_the_sizes_they_name() {
        if !latex::engine_available() {
            return;
        }

        let close = |got: f32, want: f32| (got - want).abs() < 1.0;

        let (w, h) = page_size(&compile(&DocumentLayout::default()).await);
        assert!(close(w, 595.3) && close(h, 841.9), "A4 is 595x842pt, got {w}x{h}");

        // 8.5 x 14in at 72pt/in.
        let legal = DocumentLayout { paper: Paper::Legal, ..Default::default() };
        let (w, h) = page_size(&compile(&legal).await);
        assert!(close(w, 612.0) && close(h, 1008.0), "Legal is 612x1008pt, got {w}x{h}");
    }

    // --- fonts ---------------------------------------------------------------

    /// THE ONE THAT MATTERS.
    ///
    /// Every Times, Palatino and Century Schoolbook face in TeX Live drops ₹
    /// without a word: clean compile, no warning, and a demand for "1,04,000"
    /// with no currency sign in a document that has already been served. The
    /// preamble routes the glyph through a Noto fallback; this compiles the real
    /// subject line in each offered face and fails on any missing character.
    #[tokio::test]
    async fn every_offered_font_can_set_the_rupee() {
        if !latex::engine_available() {
            return;
        }

        for font in BodyFont::ALL {
            let layout = DocumentLayout { font, ..Default::default() };
            let pdf = compile(&layout).await;
            let text = pdf_text(&pdf);
            assert!(
                text.contains("₹1,04,000"),
                "{} lost the rupee sign — the subject line reads: {}",
                font.family(),
                text.lines().find(|l| l.contains("1,04,000")).unwrap_or("<not found>")
            );
        }
    }

    #[tokio::test]
    async fn the_chosen_face_is_the_one_embedded() {
        if !latex::engine_available() {
            return;
        }

        let times = DocumentLayout { font: BodyFont::Times, ..Default::default() };
        let fonts = embedded_fonts(&compile(&times).await);
        assert!(fonts.contains("Termes"), "Times was asked for; embedded:\n{fonts}");

        let fonts = embedded_fonts(&compile(&DocumentLayout::default()).await);
        assert!(fonts.contains("NotoSerif"), "the default is Noto Serif; embedded:\n{fonts}");
    }

    /// Size and spacing are the reason a filing is accepted or returned, so they
    /// have to do something measurable rather than merely compile.
    #[tokio::test]
    async fn a_bigger_face_and_wider_spacing_take_more_pages() {
        if !latex::engine_available() {
            return;
        }

        let base = page_count(&compile(&DocumentLayout::default()).await);

        let big = DocumentLayout { font_size_pt: 16.0, ..Default::default() };
        assert!(page_count(&compile(&big).await) > base, "16pt should run longer than {base}");

        let spaced = DocumentLayout { line_spacing: LineSpacing::Double, ..Default::default() };
        assert!(
            page_count(&compile(&spaced).await) > base,
            "double spacing should run longer than {base}"
        );
    }

    // --- page numbers --------------------------------------------------------

    #[tokio::test]
    async fn page_numbering_is_what_was_asked_for() {
        if !latex::engine_available() {
            return;
        }

        let off = DocumentLayout { page_numbers: PageNumbers::None, ..Default::default() };
        let text = pdf_text(&compile(&off).await);
        assert!(!text.contains("Page 1"), "numbering was turned off:\n{text}");

        let plain = DocumentLayout { page_numbers: PageNumbers::Plain, ..Default::default() };
        let text = pdf_text(&compile(&plain).await);
        assert!(!text.contains("Page 1"), "plain means the digit alone:\n{text}");

        let of_total = pdf_text(&compile(&DocumentLayout::default()).await);
        assert!(of_total.contains("Page 1 of "), "the house footer:\n{of_total}");
    }

    /// A notice bound into a larger paper-book starts at the page it starts at.
    #[tokio::test]
    async fn a_document_can_start_at_a_page_other_than_one() {
        if !latex::engine_available() {
            return;
        }

        let layout = DocumentLayout { page_number_start: 7, ..Default::default() };
        let text = pdf_text(&compile(&layout).await);

        assert!(text.contains("Page 7 of "), "should open at 7:\n{text}");
        assert!(!text.contains("Page 1 of "), "nothing should be page 1:\n{text}");
    }

    // --- letterhead placement ------------------------------------------------

    #[tokio::test]
    async fn the_letterhead_can_be_put_on_the_first_page_only() {
        if !latex::engine_available() {
            return;
        }

        let all = pdf_text(&compile(&DocumentLayout::default()).await);
        let repeats = all.matches("Kajal Thakur").count();
        assert!(repeats >= 2, "the default repeats the mark on every page: {repeats}");

        let first = DocumentLayout {
            letterhead: Letterhead::FirstPageOnly,
            ..Default::default()
        };
        let text = pdf_text(&compile(&first).await);
        assert_eq!(
            text.matches("Kajal Thakur").count(),
            1,
            "first-page-only means once:\n{text}"
        );
        // The continuation sheets still carry a page number.
        assert!(text.contains("Page 2 of "), "page two lost its number:\n{text}");
    }

    #[tokio::test]
    async fn the_letterhead_can_be_turned_off_entirely() {
        if !latex::engine_available() {
            return;
        }

        let layout = DocumentLayout { letterhead: Letterhead::None, ..Default::default() };
        let text = pdf_text(&compile(&layout).await);

        assert!(!text.contains("Kajal Thakur"), "no letterhead was asked for:\n{text}");
        assert!(!text.contains("Regd. Office"), "nor the office footer:\n{text}");
    }

    /// Named pages. The test asks for the mark on page two and nowhere else,
    /// which is the case a page-list gets used for.
    #[tokio::test]
    async fn the_letterhead_can_be_put_on_named_pages() {
        if !latex::engine_available() {
            return;
        }

        let layout = DocumentLayout {
            letterhead: Letterhead::Pages { pages: vec![2] },
            ..Default::default()
        };
        let pdf = compile(&layout).await;
        let text = pdf_text(&pdf);
        let pages: Vec<&str> = text.trim_end_matches('\u{c}').split('\u{c}').collect();

        assert!(pages.len() >= 2, "the fixture must run to two pages");
        assert!(!pages[0].contains("Kajal Thakur"), "page one should be plain:\n{}", pages[0]);
        assert!(pages[1].contains("Kajal Thakur"), "page two should carry it:\n{}", pages[1]);
    }

    // --- bold and italic -----------------------------------------------------

    /// A document-wide bold or italic is not something a PDF records as a flag,
    /// and the body face alone does not prove it: the notice already contains
    /// bold and italic runs of its own.
    ///
    /// What does prove it is the *combination*. The template sets one sentence
    /// with \textit and many with \textbf. Turn the body bold and the italic
    /// sentence becomes bold-italic; turn the body italic and the bold ones do.
    /// Neither face appears at all in the default document, so its presence is
    /// only explicable by the switch having reached the body.
    #[tokio::test]
    async fn a_document_wide_bold_or_italic_reaches_the_body() {
        if !latex::engine_available() {
            return;
        }

        let plain = embedded_fonts(&compile(&DocumentLayout::default()).await);
        assert!(
            !plain.contains("BoldItalic"),
            "the unstyled notice has no bold-italic to begin with:\n{plain}"
        );

        let bold = DocumentLayout { bold: true, ..Default::default() };
        let bolded = embedded_fonts(&compile(&bold).await);
        assert!(
            bolded.contains("BoldItalic"),
            "a bold body should have turned the italic sentence bold-italic:\n{bolded}"
        );

        let italic = DocumentLayout { italic: true, ..Default::default() };
        let italicised = embedded_fonts(&compile(&italic).await);
        assert!(
            italicised.contains("BoldItalic"),
            "an italic body should have turned the bold runs bold-italic:\n{italicised}"
        );
    }
}
