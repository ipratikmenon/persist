// Page formatting for a generated document.
//
// The drafter should never have to format a document. They choose what it says;
// Persist decides where it sits on the page. This module is the small set of
// choices that are genuinely the attorney's — paper, margins, typeface, size,
// spacing, page numbering, and which pages carry the firm's letterhead —
// expressed once and applied to every template.
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
    /// Width and height in millimetres. One source of truth: the LaTeX lengths
    /// are formatted from these, and `validate` measures the text block against
    /// them to decide whether a set of margins leaves anything to print on.
    fn size_mm(self) -> (f32, f32) {
        match self {
            Paper::A4 => (210.0, 297.0),
            // 8.5 x 14 in.
            Paper::Legal => (215.9, 355.6),
        }
    }

    /// Width and height, as LaTeX lengths.
    ///
    /// Explicit dimensions rather than geometry's named `a4paper`/`legalpaper`
    /// options, because those are *keys* and keyval does not expand a macro in
    /// key position: `\geometry{\persistpaper}` matches nothing, says nothing,
    /// and leaves the document whatever size the class made it. It went out as
    /// A4 with "Legal" selected until a compile test measured the MediaBox.
    /// `papersize={w,h}` takes values, and values are expanded.
    fn dimensions(self) -> (String, String) {
        let (width, height) = self.size_mm();
        (format!("{width}mm"), format!("{height}mm"))
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

/// Page margins, in millimetres, one per side.
///
/// Four numbers rather than named presets. A forum that specifies a margin
/// specifies a measurement — and it is usually the left one, because a filing
/// is bound down that edge and a narrow left margin puts the first character of
/// every line under the stitching. "Normal / Narrow / Wide" cannot answer a
/// direction to leave 40mm on the left and 20mm elsewhere; four fields can.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Margins {
    pub top_mm: f32,
    pub bottom_mm: f32,
    pub left_mm: f32,
    pub right_mm: f32,
}

impl Default for Margins {
    fn default() -> Self {
        // What persist-base.tex has always set, so a caller that says nothing
        // about margins gets the page it got before margins were a choice.
        Margins { top_mm: 20.0, bottom_mm: 20.0, left_mm: 22.0, right_mm: 22.0 }
    }
}

/// The letterhead band sits nearer the paper edge than a plain document's body
/// does. Measured off the firm's own notice: the mark starts 15mm down and the
/// registered-office footer ends 12mm up, against the 20mm of body margin a
/// page without the letterhead carries. Those two insets belong to the artwork
/// rather than to the attorney's margin, so the band keeps them and moves when
/// the margin moves — 30mm of top margin puts the mark at 25mm, not at 15mm.
const LETTERHEAD_TOP_INSET_MM: f32 = 5.0;
const LETTERHEAD_BOTTOM_INSET_MM: f32 = 8.0;

/// Nothing is placed closer than this to the edge of the sheet. Consumer laser
/// printers have an unprintable border of roughly 4-5mm and simply clip what
/// falls inside it, which on a served notice means a page number that is not
/// there.
const PRINTABLE_EDGE_MM: f32 = 5.0;

/// Past this a margin is not a margin, it is a mistake in the box — 100mm is
/// already a third of the way down an A4 sheet.
const MAX_MARGIN_MM: f32 = 100.0;

/// Enough width to set a paragraph in. Below this the justification breaks down
/// into rivers and single-word lines long before the geometry itself fails.
const MIN_TEXT_WIDTH_MM: f32 = 90.0;

/// The letterhead head band is two minipages side by side — 54mm for the mark
/// and 62mm for the partner block. Narrower than their sum and fancyhdr sets
/// them overlapping: the second partner's email prints through the logo, with
/// only an overfull-hbox warning in a log nobody reads.
const LETTERHEAD_BAND_WIDTH_MM: f32 = 54.0 + 62.0;

/// How much of the page height the letterhead consumes before the body starts:
/// a 108pt head band (38.1mm) plus the 10mm headsep under it.
const LETTERHEAD_BAND_HEIGHT_MM: f32 = 48.1;

/// Enough height to be worth printing — about fifteen lines at 12pt.
const MIN_TEXT_HEIGHT_MM: f32 = 60.0;

impl Margins {
    /// Where the letterhead band goes: the same margin the attorney chose, less
    /// the inset the artwork was measured at, and never off the printable
    /// sheet. `\applyletterhead` re-runs `\geometry` with these.
    fn head_top_mm(self) -> f32 {
        (self.top_mm - LETTERHEAD_TOP_INSET_MM).max(PRINTABLE_EDGE_MM)
    }

    fn head_bottom_mm(self) -> f32 {
        (self.bottom_mm - LETTERHEAD_BOTTOM_INSET_MM).max(PRINTABLE_EDGE_MM)
    }

    fn each_side(self) -> [(&'static str, f32); 4] {
        [
            ("top", self.top_mm),
            ("bottom", self.bottom_mm),
            ("left", self.left_mm),
            ("right", self.right_mm),
        ]
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
    pub margins: Margins,
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
            margins: Margins::default(),
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
        self.validate_margins()?;
        Ok(())
    }

    /// Margins are refused here, in sentences, rather than at the engine, where
    /// too little width is an overfull box in a log and too little height is a
    /// document that silently runs to twice the pages.
    fn validate_margins(&self) -> Result<()> {
        for (side, mm) in self.margins.each_side() {
            if !(PRINTABLE_EDGE_MM..=MAX_MARGIN_MM).contains(&mm) {
                bail!(
                    "The {side} margin must be between {PRINTABLE_EDGE_MM:.0} and \
                     {MAX_MARGIN_MM:.0} millimetres. A printer cannot put ink closer \
                     than about {PRINTABLE_EDGE_MM:.0}mm to the edge of the sheet."
                );
            }
        }

        let (paper_width, paper_height) = self.paper.size_mm();
        let shows_letterhead = !matches!(self.letterhead, Letterhead::None);

        let width = paper_width - self.margins.left_mm - self.margins.right_mm;
        if shows_letterhead && width < LETTERHEAD_BAND_WIDTH_MM {
            bail!(
                "The letterhead needs {LETTERHEAD_BAND_WIDTH_MM:.0}mm across the page — \
                 the firm's mark and the partners' details sit side by side — and these \
                 margins leave {width:.0}mm. Narrow the left or right margin, or take \
                 the letterhead off this document."
            );
        }
        if width < MIN_TEXT_WIDTH_MM {
            bail!(
                "These margins leave only {width:.0}mm of text across the page, which is \
                 too narrow to set a paragraph in. Narrow the left or right margin."
            );
        }

        // Conservative on a letterhead document: the band is hung from
        // `head_top_mm`, which is above `top_mm`, so the real body is a little
        // taller than this. Refusing slightly early is the right side to err on.
        let height = paper_height - self.margins.top_mm - self.margins.bottom_mm;
        let band = if shows_letterhead { LETTERHEAD_BAND_HEIGHT_MM } else { 0.0 };
        if height - band < MIN_TEXT_HEIGHT_MM {
            let after = if shows_letterhead { ", once the letterhead band is set" } else { "" };
            bail!(
                "These margins leave only {:.0}mm down the page{after}, which is not \
                 enough to print on. Reduce the top or bottom margin.",
                height - band
            );
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

        // Two sets of margins from one choice. The body block is set by
        // persist-base; a letterhead document then re-runs \geometry in
        // \applyletterhead to make room for the bands, and if that second run
        // did not read these the margin would apply to an invoice and be
        // ignored on the notice the attorney actually set it for.
        let m = self.margins;
        out.push_str(&format!("\\def\\persistmargintop{{{:.1}mm}}\n", m.top_mm));
        out.push_str(&format!("\\def\\persistmarginbottom{{{:.1}mm}}\n", m.bottom_mm));
        out.push_str(&format!("\\def\\persistmarginleft{{{:.1}mm}}\n", m.left_mm));
        out.push_str(&format!("\\def\\persistmarginright{{{:.1}mm}}\n", m.right_mm));
        out.push_str(&format!("\\def\\persistheadmargintop{{{:.1}mm}}\n", m.head_top_mm()));
        out.push_str(&format!(
            "\\def\\persistheadmarginbottom{{{:.1}mm}}\n",
            m.head_bottom_mm()
        ));

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
        assert!(tex.contains("\\def\\persistmarginleft{22.0mm}"), "{tex}");
        assert!(tex.contains("\\def\\persistmargintop{20.0mm}"), "{tex}");
        // The band as the firm's own notice measures it, unchanged.
        assert!(tex.contains("\\def\\persistheadmargintop{15.0mm}"), "{tex}");
        assert!(tex.contains("\\def\\persistheadmarginbottom{12.0mm}"), "{tex}");
    }

    /// A margin has to reach both `\geometry` calls. persist-base sets the body
    /// block; `\applyletterhead` re-runs geometry to make room for the bands and
    /// overrides it, so a left margin that is only emitted for the body applies
    /// to an invoice and is silently dropped on a notice.
    #[test]
    fn a_margin_is_emitted_for_the_letterhead_bands_as_well_as_the_body() {
        let tex = DocumentLayout {
            margins: Margins { top_mm: 30.0, bottom_mm: 40.0, left_mm: 45.0, right_mm: 18.0 },
            ..Default::default()
        }
        .to_latex();

        assert!(tex.contains("\\def\\persistmargintop{30.0mm}"), "{tex}");
        assert!(tex.contains("\\def\\persistmarginbottom{40.0mm}"), "{tex}");
        assert!(tex.contains("\\def\\persistmarginleft{45.0mm}"), "{tex}");
        assert!(tex.contains("\\def\\persistmarginright{18.0mm}"), "{tex}");
        // Left and right are the same number in both places — a band is as wide
        // as the text it sits over. Top and bottom carry the artwork's inset.
        assert!(tex.contains("\\def\\persistheadmargintop{25.0mm}"), "{tex}");
        assert!(tex.contains("\\def\\persistheadmarginbottom{32.0mm}"), "{tex}");
    }

    /// The inset would put the band off the sheet at the narrowest margin the
    /// attorney is allowed to ask for, and a printer would clip it.
    #[test]
    fn the_letterhead_band_never_leaves_the_printable_sheet() {
        let tex = DocumentLayout {
            margins: Margins { top_mm: 6.0, bottom_mm: 6.0, ..Default::default() },
            ..Default::default()
        }
        .to_latex();

        assert!(tex.contains("\\def\\persistheadmargintop{5.0mm}"), "{tex}");
        assert!(tex.contains("\\def\\persistheadmarginbottom{5.0mm}"), "{tex}");
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

    #[test]
    fn margins_that_would_leave_nothing_to_print_on_are_refused_in_sentences() {
        let refuse = |margins: Margins, letterhead: Letterhead, paper: Paper| {
            DocumentLayout { margins, letterhead, paper, ..Default::default() }
                .validate()
                .expect_err("should have been refused")
                .to_string()
        };

        let off_the_sheet = Margins { left_mm: 2.0, ..Default::default() };
        assert!(refuse(off_the_sheet, Letterhead::AllPages, Paper::A4)
            .contains("The left margin must be between 5 and 100 millimetres"));

        // A4 is 210mm wide, so this leaves 110mm — enough to set text in, but
        // the mark and the partner block need 116mm side by side.
        let squeezes_the_band = Margins { left_mm: 50.0, right_mm: 50.0, ..Default::default() };
        let message = refuse(squeezes_the_band, Letterhead::AllPages, Paper::A4);
        assert!(message.contains("The letterhead needs 116mm"), "{message}");
        assert!(message.contains("leave 110mm"), "{message}");
        // The same page without the letterhead is a document, not an error.
        assert!(DocumentLayout {
            margins: squeezes_the_band,
            letterhead: Letterhead::None,
            ..Default::default()
        }
        .validate()
        .is_ok());

        let no_width = Margins { left_mm: 70.0, right_mm: 70.0, ..Default::default() };
        assert!(refuse(no_width, Letterhead::None, Paper::A4)
            .contains("only 70mm of text across the page"));

        // 97mm of page left on A4, of which the letterhead band takes 48.1mm.
        let no_height = Margins { top_mm: 100.0, bottom_mm: 100.0, ..Default::default() };
        let message = refuse(no_height, Letterhead::AllPages, Paper::A4);
        assert!(message.contains("once the letterhead band is set"), "{message}");
        // Legal is 58.6mm taller, and the same margins fit on it.
        assert!(DocumentLayout {
            margins: no_height,
            paper: Paper::Legal,
            ..Default::default()
        }
        .validate()
        .is_ok());
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
        assert_eq!(layout.margins, Margins::default());

        // A forum that asks for a binding margin asks for one side of it, and
        // Deck should not have to restate the other three to say so.
        let bound: DocumentLayout =
            serde_json::from_str(r#"{ "margins": { "leftMm": 40 } }"#).unwrap();
        assert_eq!(bound.margins.left_mm, 40.0);
        assert_eq!(bound.margins.right_mm, 22.0);
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
        // A notice with no schedule of payments. The placeholder still has to be
        // bound: `render` refuses to compile on an unfilled one.
        fields.insert("PAYMENTS_BLOCK".into(), Field::raw(""));
        fields.insert("ANNEXURES_BLOCK".into(), Field::raw(""));
        fields.insert("ANNEXURE_PAGES".into(), Field::raw(""));
        fields
    }

    /// The examination reply. It takes `_shared/persist-base` and stops there —
    /// no letterhead, so no second `\geometry` — which makes it the only shipped
    /// document that measures the base geometry on its own.
    fn reply_fields() -> HashMap<String, Field> {
        [
            ("FIRM_NAME", "Persistas & Partners"),
            ("FIRM_ADDRESS", "80-A, Pocket-A, Mayuri Enclave, Delhi - 110096"),
            ("FIRM_CONTACT", "persistas.pnp@outlook.com"),
            ("REPLY_DATE", "14 July 2026"),
            ("REGISTRY_OFFICE", "Delhi"),
            ("TM_NUMBER", "5642178"),
            ("TM_MARK", "PETALVEDA"),
            ("TM_CLASS", "3"),
            ("APPLICANT_NAME", "Petalveda Scents Private Limited"),
            ("EXAM_REPORT_DATE", "2 May 2026"),
            ("ATTORNEY_NAME", "Sree Lakshmi Menon"),
            (
                "SUBMISSIONS",
                "The Applicant's mark is inherently distinctive in relation to the \
                 goods applied for, and has been in continuous and uninterrupted use \
                 since 2019 in the course of trade throughout India.",
            ),
            (
                "GROUNDS_TEXT",
                "The objection under Section 11(1) is misconceived: the cited mark \
                 covers dissimilar goods, is registered in a different class, and \
                 has not been shown to be in use.",
            ),
            (
                "PRIOR_USE_BLOCK",
                "The Applicant further relies on prior use since 12 March 2019.",
            ),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), Field::text(v)))
        .collect()
    }

    async fn compile(layout: &DocumentLayout) -> Vec<u8> {
        compile_template("legal-notice", &notice_fields(), layout).await
    }

    async fn compile_template(
        template: &str,
        fields: &HashMap<String, Field>,
        layout: &DocumentLayout,
    ) -> Vec<u8> {
        let staged = latex::Attachment::new(LAYOUT_FILE, layout.to_latex().into_bytes()).unwrap();
        latex::compile_with(
            template,
            fields,
            CompileMode::Final,
            std::slice::from_ref(&staged),
        )
        .await
        .expect("the document must compile under every layout offered")
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

    /// Where the ink actually is on page one, in PDF points from the top-left
    /// corner of the sheet.
    struct TextBox {
        left: f32,
        right: f32,
        top: f32,
    }

    /// Measure page one: the leftmost, rightmost and highest word on it.
    ///
    /// `pdftotext -bbox` gives a box per word as XHTML. Word boxes rather than
    /// the page's own crop or geometry's idea of the text area, because a margin
    /// is only real if it moved ink — geometry can be set to anything in the
    /// preamble and a second \geometry call further down can quietly undo it.
    /// That is exactly what \applyletterhead does.
    fn text_box(pdf: &[u8]) -> TextBox {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.pdf");
        std::fs::write(&path, pdf).unwrap();
        let out = std::process::Command::new("pdftotext")
            .args(["-bbox", "-f", "1", "-l", "1"])
            .arg(&path)
            .arg("-")
            .output()
            .expect("pdftotext is needed to measure where the text sits");
        let xml = String::from_utf8_lossy(&out.stdout).into_owned();

        let coordinate = |line: &str, attribute: &str| -> Option<f32> {
            let rest = line.split_once(attribute)?.1;
            rest.split('"').nth(1)?.parse().ok()
        };

        let words: Vec<&str> = xml
            .lines()
            .filter(|l| l.trim_start().starts_with("<word "))
            .collect();
        assert!(!words.is_empty(), "no words on page one:\n{xml}");

        let least = |attribute| {
            words
                .iter()
                .filter_map(|l| coordinate(l, attribute))
                .fold(f32::MAX, f32::min)
        };

        TextBox {
            left: least("xMin="),
            top: least("yMin="),
            right: words
                .iter()
                .filter_map(|l| coordinate(l, "xMax="))
                .fold(f32::MIN, f32::max),
        }
    }

    /// Millimetres as PDF points, for comparing against a measured box.
    fn mm(value: f32) -> f32 {
        value * 72.0 / 25.4
    }

    /// Within a millimetre. A glyph's box starts at its left side bearing
    /// rather than exactly on the margin, so an exact comparison would fail on
    /// a document that is otherwise correct.
    fn within_a_millimetre(got: f32, want: f32, what: &str) {
        assert!(
            (got - want).abs() < mm(1.0),
            "{what}: measured {:.1}mm, expected {:.1}mm",
            got * 25.4 / 72.0,
            want * 25.4 / 72.0
        );
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

    // --- margins -------------------------------------------------------------

    /// The base geometry, on a document that has no letterhead over it. The
    /// examination reply sets a 15cm table, so the margins here stay wide
    /// enough for it — a narrower page would overfull the table into the right
    /// margin and the measurement would be of the table, not of the geometry.
    #[tokio::test]
    async fn a_margin_moves_the_text_on_a_page_without_the_letterhead() {
        if !latex::engine_available() {
            return;
        }

        let reply = |layout: DocumentLayout| async move {
            text_box(&compile_template("tm-examination-reply", &reply_fields(), &layout).await)
        };

        let house = reply(DocumentLayout::default()).await;
        within_a_millimetre(house.left, mm(22.0), "the house left margin");
        within_a_millimetre(house.right, mm(210.0 - 22.0), "the house right margin");

        let bound = DocumentLayout {
            margins: Margins { left_mm: 40.0, right_mm: 15.0, top_mm: 32.0, ..Default::default() },
            ..Default::default()
        };
        let moved = reply(bound).await;
        within_a_millimetre(moved.left, mm(40.0), "a binding left margin");
        within_a_millimetre(moved.right, mm(210.0 - 15.0), "a narrower right margin");
        // The top is measured as a movement rather than against 32mm: the first
        // line here is the firm's name at \LARGE, whose ascender rises above the
        // text area by however much it exceeds \topskip — measured at 2.2mm,
        // which is a property of the face, not of the margin.
        within_a_millimetre(moved.top - house.top, mm(12.0), "a deeper top margin");
    }

    /// THE ONE THAT MATTERS.
    ///
    /// `\applyletterhead` re-runs `\geometry` to make room for the head band,
    /// and that second call is the one the page ends up with. Hardcode the
    /// margins there — as it did until this test — and a margin set in page
    /// setup applies to an invoice and is silently thrown away on the notice it
    /// was set for, which is the document that had the filing requirement.
    ///
    /// So this measures the same thing as the test above on a document that
    /// carries the firm's letterhead, where nothing about the generated LaTeX
    /// looks any different.
    #[tokio::test]
    async fn a_margin_reaches_a_document_on_letterhead_too() {
        if !latex::engine_available() {
            return;
        }

        let house = text_box(&compile(&DocumentLayout::default()).await);
        within_a_millimetre(house.left, mm(22.0), "the house left margin, on letterhead");
        within_a_millimetre(house.right, mm(210.0 - 22.0), "the house right margin, on letterhead");

        let bound = DocumentLayout {
            margins: Margins { left_mm: 45.0, right_mm: 16.0, top_mm: 35.0, ..Default::default() },
            ..Default::default()
        };
        let moved = text_box(&compile(&bound).await);
        within_a_millimetre(moved.left, mm(45.0), "a binding left margin, on letterhead");
        within_a_millimetre(moved.right, mm(210.0 - 16.0), "a narrower right margin, on letterhead");

        // The band moves with the margin, keeping the 5mm the firm's own notice
        // hangs it above the body — so 20mm to 35mm of top margin drops the
        // partner block, which is the highest thing on the page, by exactly 15.
        within_a_millimetre(moved.top - house.top, mm(15.0), "the letterhead band's drop");
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
