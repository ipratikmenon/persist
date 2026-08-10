// LaTeX compilation service.
//
// Compiles .tex templates locally. NEVER server-side, NEVER over the network —
// a draft in progress is privileged material and does not leave the machine.
//
// ENGINE: XeLaTeX, not pdfLaTeX
//
// An Indian IP practice needs ₹ on every invoice and Devanagari on Hindi
// filings. pdfLaTeX cannot typeset either without per-glyph workarounds; it
// fails outright on U+20B9. XeLaTeX with a Unicode font handles both natively,
// so templates can contain the characters the work actually requires.
//
// ESCAPING: not optional, and not the caller's job to remember
//
// Every value arrives as a `Field`, which is either `text` (escaped on the way
// in) or `raw` (LaTeX the caller has built deliberately). There is no way to
// pass a bare string, so "forgot to escape this one field" is not a mistake the
// API allows — which is exactly the bug this replaced.
//
// FAILURE POSTURE
//
// A placeholder the caller never filled is an error, not a `{{CLIENT_ADDRESS}}`
// printed into a document that goes to a client. Compilation runs under a
// timeout with shell escape disabled, so a pathological template cannot hang
// the app or reach the filesystem.

use anyhow::{bail, Context};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Long enough for a heavy document with a large table, short enough that a
/// runaway template surfaces as an error rather than a hung app.
const COMPILE_TIMEOUT: Duration = Duration::from_secs(60);

/// XeLaTeX resolves cross-references, `longtable` column widths and page
/// numbers on the second pass. One pass silently produces a misaligned
/// document, which is worse than a failure because it looks finished.
const PASSES: usize = 2;

// ---------------------------------------------------------------------------
// Field values
// ---------------------------------------------------------------------------

/// A value bound to a `{{KEY}}` placeholder.
///
/// Construct with [`Field::text`] for anything a human typed, or [`Field::raw`]
/// for LaTeX the caller assembled itself. The distinction is deliberately
/// visible at the call site: `raw` is where a template injection would have to
/// come from, so it should be greppable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field(String);

impl Field {
    /// Untrusted text — a client name, an address, an attorney's note.
    /// Escaped so no character in it can be read as markup.
    pub fn text(value: impl AsRef<str>) -> Self {
        Field(escape(value.as_ref()))
    }

    /// Pre-built LaTeX, such as assembled table rows.
    ///
    /// The caller is asserting this is valid markup. Anything interpolated into
    /// it must have been through [`escape`] first.
    pub fn raw(latex: impl Into<String>) -> Self {
        Field(latex.into())
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

/// Escape every character LaTeX treats specially.
///
/// The backslash is handled first and mapped to `\textbackslash{}` — NOT to
/// `\\`, which is a line break. A previous version made that substitution and
/// silently split "In re Bajaj\Auto" across two lines in a compiled invoice:
/// clean compile, no warning, wrong document. The braces introduced by the
/// replacement text are added after the brace rules run, so they survive.
pub fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str(r"\textbackslash{}"),
            '{' => out.push_str(r"\{"),
            '}' => out.push_str(r"\}"),
            '&' => out.push_str(r"\&"),
            '%' => out.push_str(r"\%"),
            '$' => out.push_str(r"\$"),
            '#' => out.push_str(r"\#"),
            '_' => out.push_str(r"\_"),
            '~' => out.push_str(r"\textasciitilde{}"),
            '^' => out.push_str(r"\textasciicircum{}"),
            _ => out.push(ch),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile a template to PDF bytes. The caller decides where they are stored.
///
/// `template_id` is the filename without extension inside the templates
/// directory — "invoice" for `invoice.tex`.
pub async fn compile_latex(
    template_id: &str,
    fields: &HashMap<String, Field>,
) -> anyhow::Result<Vec<u8>> {
    let template_dir = find_templates_dir()?;
    let template_path = template_dir.join(format!("{template_id}.tex"));

    let source = std::fs::read_to_string(&template_path)
        .with_context(|| format!("failed to read template: {}", template_path.display()))?;

    let rendered = render(&source, fields)?;
    run_engine(template_id, &rendered).await
}

// ---------------------------------------------------------------------------
// Template rendering
// ---------------------------------------------------------------------------

/// Substitute `{{KEY}}` placeholders, then refuse to proceed if any remain.
///
/// Leaving an unfilled placeholder in place is how `{{CLIENT_ADDRESS}}` ends up
/// printed in a PDF sent to a client. A template referring to a key the caller
/// does not supply is a bug in one of the two, and it should stop the build.
fn render(source: &str, fields: &HashMap<String, Field>) -> anyhow::Result<String> {
    let mut out = source.to_owned();
    for (key, value) in fields {
        out = out.replace(&format!("{{{{{key}}}}}"), value.as_str());
    }

    let mut unfilled = unfilled_placeholders(&out);
    if !unfilled.is_empty() {
        unfilled.sort();
        unfilled.dedup();
        bail!(
            "template has placeholders no value was supplied for: {}",
            unfilled.join(", ")
        );
    }

    Ok(out)
}

/// Find `{{KEY}}` occurrences outside LaTeX comments.
///
/// Comment lines are skipped because templates document their own variables in
/// a `%` header, and that documentation must not fail the build it describes.
fn unfilled_placeholders(source: &str) -> Vec<String> {
    let mut found = Vec::new();

    for line in source.lines() {
        let code = match line.find('%') {
            // An escaped `\%` is a literal percent, not a comment.
            Some(i) if !line[..i].ends_with('\\') => &line[..i],
            Some(_) | None => line,
        };

        let mut rest = code;
        while let Some(start) = rest.find("{{") {
            let after = &rest[start + 2..];
            let Some(end) = after.find("}}") else { break };
            let key = &after[..end];
            if !key.is_empty()
                && key.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
            {
                found.push(key.to_owned());
            }
            rest = &after[end + 2..];
        }
    }

    found
}

// ---------------------------------------------------------------------------
// Engine subprocess
// ---------------------------------------------------------------------------

async fn run_engine(job_name: &str, source: &str) -> anyhow::Result<Vec<u8>> {
    let engine = find_engine()?;

    let tmp_dir = tempfile::tempdir().context("failed to create temp dir for LaTeX")?;
    // Named after the template so a failure log is identifiable, and sanitised
    // because the name reaches a command line.
    let stem = sanitise_job_name(job_name);
    let tex_path = tmp_dir.path().join(format!("{stem}.tex"));
    let pdf_path = tmp_dir.path().join(format!("{stem}.pdf"));

    std::fs::write(&tex_path, source).context("failed to write .tex source")?;

    let mut last_log = String::new();
    for pass in 1..=PASSES {
        let child = tokio::process::Command::new(&engine)
            .args([
                "-interaction=nonstopmode",
                "-halt-on-error",
                // Templates are firm assets today and AI-assembled tomorrow.
                // Neither needs to run programs, so neither may.
                "-no-shell-escape",
                "-output-directory",
                tmp_dir.path().to_str().context("temp dir path is not UTF-8")?,
                tex_path.to_str().context("tex path is not UTF-8")?,
            ])
            // The engine must not inherit a terminal; without this a template
            // that asks a question waits for an answer nobody can give.
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("failed to spawn {}", engine.display()))?;

        let output = match tokio::time::timeout(COMPILE_TIMEOUT, child.wait_with_output()).await {
            Ok(result) => result.context("LaTeX engine failed while running")?,
            Err(_) => {
                // kill_on_drop reaps it; -halt-on-error cannot catch a loop that
                // never errors, so the timeout is the only thing that can.
                bail!(
                    "LaTeX compilation exceeded {}s and was stopped",
                    COMPILE_TIMEOUT.as_secs()
                );
            }
        };

        last_log = String::from_utf8_lossy(&output.stdout).into_owned();

        if !output.status.success() {
            bail!(
                "LaTeX engine failed on pass {pass} of {PASSES}:\n{}",
                first_error(&last_log)
            );
        }
    }

    std::fs::read(&pdf_path).with_context(|| {
        format!(
            "LaTeX reported success but produced no PDF at {}:\n{}",
            pdf_path.display(),
            first_error(&last_log)
        )
    })
}

/// Pull the actual error out of a LaTeX log.
///
/// A full log is thousands of lines and the useful part is the `!` line and the
/// `l.NNN` that follows it. Surfacing the whole thing to an attorney is the
/// same as surfacing nothing.
fn first_error(log: &str) -> String {
    let lines: Vec<&str> = log
        .lines()
        .filter(|l| l.starts_with('!') || l.starts_with("l."))
        .take(6)
        .collect();

    if lines.is_empty() {
        // No recognisable error: show the tail, which is where it usually is.
        return log.lines().rev().take(15).collect::<Vec<_>>().join("\n");
    }
    lines.join("\n")
}

/// Keep a job name to characters that are safe as a filename and on a command
/// line. Template ids are internal today, but this is the one value from the
/// caller that becomes a path.
fn sanitise_job_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if cleaned.is_empty() {
        "document".to_owned()
    } else {
        cleaned
    }
}

// ---------------------------------------------------------------------------
// Locating the engine
// ---------------------------------------------------------------------------

/// Engines in order of preference. LuaLaTeX is an acceptable substitute — it is
/// also Unicode-native and reads the same `fontspec` templates.
const ENGINES: [&str; 2] = ["xelatex", "lualatex"];

fn find_engine() -> anyhow::Result<PathBuf> {
    if let Ok(explicit) = std::env::var("PERSIST_LATEX_ENGINE") {
        let p = PathBuf::from(&explicit);
        if p.exists() {
            return Ok(p);
        }
        bail!("PERSIST_LATEX_ENGINE points at {explicit}, which does not exist");
    }

    for engine in ENGINES {
        // Bundled sidecar first: the installer ships TeX Live, and the firm's
        // machine may have an unrelated or older TeX on PATH.
        if let Some(p) = sidecar_path(engine) {
            return Ok(p);
        }
        for dir in [
            "/Library/TeX/texbin",              // macOS, MacTeX
            "/usr/local/texlive/2025/bin/universal-darwin",
            "/usr/local/texlive/2024/bin/universal-darwin",
            "/usr/bin",
            "/usr/local/bin",
        ] {
            let p = Path::new(dir).join(engine);
            if p.exists() {
                return Ok(p);
            }
        }
        if let Some(p) = on_path(engine) {
            return Ok(p);
        }
    }

    bail!(
        "No Unicode LaTeX engine found. Persist needs xelatex or lualatex \
         (macOS: `brew install --cask mactex-no-gui`; Debian/Ubuntu: \
         `apt install texlive-xetex fonts-noto-core`), or the bundled TeX Live \
         sidecar from the installer."
    )
}

fn on_path(binary: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(binary))
        .find(|p| p.is_file())
}

fn sidecar_path(binary: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bin_dir = exe.parent()?;
    [
        bin_dir.join(binary),
        bin_dir.join("../resources").join(binary),
        bin_dir.join("../Frameworks").join(binary),
    ]
    .into_iter()
    .find(|p| p.exists())
}

// ---------------------------------------------------------------------------
// Locating templates
// ---------------------------------------------------------------------------

fn find_templates_dir() -> anyhow::Result<PathBuf> {
    if let Ok(dir) = std::env::var("PERSIST_TEMPLATES_DIR") {
        let p = PathBuf::from(&dir);
        if p.is_dir() {
            return Ok(p);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            for candidate in [
                bin_dir.join("resources/templates"),
                bin_dir.join("../resources/templates"),
                bin_dir.join("../Resources/templates"), // macOS .app bundle
            ] {
                if candidate.is_dir() {
                    return Ok(candidate);
                }
            }
        }
    }

    // Development: cargo runs tests with the CWD at src-tauri/.
    for candidate in [
        PathBuf::from("storage/templates"),
        PathBuf::from("src-tauri/storage/templates"),
    ] {
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    bail!(
        "LaTeX templates directory not found. Set PERSIST_TEMPLATES_DIR or place \
         templates in src-tauri/storage/templates/."
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(pairs: &[(&str, Field)]) -> HashMap<String, Field> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect()
    }

    // -- escaping ----------------------------------------------------------

    /// The bug this function was rewritten for. `\\` is a line break in LaTeX,
    /// so the old escape silently split a citation across two lines.
    #[test]
    fn a_backslash_becomes_textbackslash_not_a_line_break() {
        let out = escape(r"In re Bajaj\Auto");
        assert_eq!(out, r"In re Bajaj\textbackslash{}Auto");
        assert!(!out.contains(r"\\"), "a line break reached the document: {out}");
    }

    #[test]
    fn every_latex_special_character_is_escaped() {
        // If a character is special and missing here, the assertion below fails
        // rather than the omission being discovered in a client's PDF.
        for ch in ['\\', '{', '}', '&', '%', '$', '#', '_', '~', '^'] {
            let escaped = escape(&ch.to_string());
            assert_ne!(escaped, ch.to_string(), "{ch:?} was passed through unescaped");
            assert!(escaped.starts_with('\\'), "{ch:?} produced {escaped:?}");
        }
    }

    #[test]
    fn the_firms_own_name_survives_escaping() {
        // 'Persistas & Partners' is the seeded firm_name, and the ampersand in
        // it broke every invoice before this.
        assert_eq!(escape("Persistas & Partners"), r"Persistas \& Partners");
    }

    #[test]
    fn escaping_leaves_ordinary_text_and_unicode_alone() {
        assert_eq!(escape("Tata Sons — ₹23,600.00"), "Tata Sons — ₹23,600.00");
        assert_eq!(escape("व्यापार चिह्न"), "व्यापार चिह्न");
    }

    #[test]
    fn escaping_is_not_double_applied() {
        // Field::text escapes once. Escaping an already-escaped value would
        // print literal backslashes to the client.
        let once = Field::text("A & B");
        assert_eq!(once.as_str(), r"A \& B");
    }

    // -- rendering ---------------------------------------------------------

    #[test]
    fn text_fields_are_escaped_on_the_way_in() {
        let out = render(
            "Client: {{CLIENT}}",
            &fields(&[("CLIENT", Field::text("Tata & Sons"))]),
        )
        .unwrap();
        assert_eq!(out, r"Client: Tata \& Sons");
    }

    #[test]
    fn raw_fields_pass_through_untouched() {
        let out = render(
            "{{ROWS}}",
            &fields(&[("ROWS", Field::raw(r"A & B \\"))]),
        )
        .unwrap();
        assert_eq!(out, r"A & B \\");
    }

    /// The old behaviour printed `{{NOT_SET}}` into the PDF, and a test asserted
    /// that was correct. It is not: it reaches the client.
    #[test]
    fn an_unfilled_placeholder_fails_the_render() {
        let err = render(
            "Firm: {{FIRM_NAME}}, Missing: {{CLIENT_ADDRESS}}",
            &fields(&[("FIRM_NAME", Field::text("Persistas"))]),
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("CLIENT_ADDRESS"), "error must name the key: {err}");
    }

    #[test]
    fn a_placeholder_documented_in_a_comment_does_not_fail_the_render() {
        // Templates document their own variables in a `%` header. That must not
        // fail the build it is describing.
        let out = render(
            "% Variables use {{KEY}} syntax\nFirm: {{FIRM_NAME}}",
            &fields(&[("FIRM_NAME", Field::text("Persistas"))]),
        )
        .unwrap();
        assert!(out.contains("Persistas"));
    }

    #[test]
    fn an_escaped_percent_does_not_start_a_comment() {
        // `100\% within limitation {{MISSING}}` is code, not a comment, so the
        // unfilled placeholder after it must still be caught.
        let err = render(r"Fee 100\% then {{MISSING}}", &HashMap::new()).unwrap_err();
        assert!(err.to_string().contains("MISSING"), "{err}");
    }

    #[test]
    fn a_value_containing_a_placeholder_is_not_re_expanded() {
        // A client who names their company "{{TOTAL}}" must not be able to read
        // another field, and must not fail the render either.
        let out = render(
            "Client: {{CLIENT}} Total: {{TOTAL}}",
            &fields(&[
                ("CLIENT", Field::text("{{TOTAL}} Ltd")),
                ("TOTAL", Field::text("23600.00")),
            ]),
        )
        .unwrap();

        assert!(out.contains(r"\{\{TOTAL\}\} Ltd"), "injected placeholder expanded: {out}");
        assert!(out.contains("23600.00"));
    }

    // -- job names ---------------------------------------------------------

    #[test]
    fn a_job_name_cannot_escape_the_temp_directory() {
        assert_eq!(sanitise_job_name("../../etc/passwd"), "etcpasswd");
        assert_eq!(sanitise_job_name("in voice; rm -rf /"), "invoicerm-rf");
        assert_eq!(sanitise_job_name(""), "document");
        assert_eq!(sanitise_job_name("invoice"), "invoice");
    }

    // -- log handling ------------------------------------------------------

    #[test]
    fn the_reported_error_is_the_latex_error_not_the_whole_log() {
        let log = "\
This is XeTeX, Version 3.141592653
(./invoice.tex
! LaTeX Error: Unicode character ₹ (U+20B9)
l.137 \\color{white}\\bfseries Rate (₹
Some other noise
";
        let reported = first_error(log);
        assert!(reported.contains("U+20B9"), "{reported}");
        assert!(reported.contains("l.137"), "{reported}");
        assert!(!reported.contains("XeTeX, Version"), "the banner is noise: {reported}");
    }
}

// ---------------------------------------------------------------------------
// Compilation tests
//
// These run the real engine against the real template. Nothing above this point
// would have caught the three faults that made invoice generation impossible:
// the firm's own ampersand, the rupee sign, and a template that never compiled
// in CI because no engine was installed. String assertions cannot find those.
//
// Skipped when no engine is present, so `cargo test` stays green on a machine
// without TeX Live — but the skip is loud.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod compile_tests {
    use super::*;

    fn engine_available() -> bool {
        if find_engine().is_ok() {
            return true;
        }
        eprintln!("SKIPPING LaTeX compilation tests: no xelatex/lualatex found");
        false
    }

    /// Everything `generate_invoice_pdf` supplies, with values chosen to be
    /// hostile: the firm's real name, a real client name with an ampersand,
    /// rupee amounts, a percent sign, and Devanagari.
    fn invoice_fields() -> HashMap<String, Field> {
        let rows = (0..30)
            .map(|i| {
                format!(
                    "TM{i} & {} & 2.50 & ₹8000.00 & ₹20000.00 \\\\",
                    escape("Filing of TM application & response to FER")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        [
            ("INVOICE_ID", Field::text("INV-2026-0042")),
            ("INVOICE_DATE", Field::text("2026-08-01")),
            ("DUE_DATE", Field::text("2026-08-31")),
            ("FIRM_NAME", Field::text("Persistas & Partners")),
            ("FIRM_GSTIN", Field::text("07AABCU9603R1ZM")),
            ("FIRM_PAN", Field::text("AABCU9603R")),
            ("FIRM_ADDRESS", Field::text("B-12, Greater Kailash-I, New Delhi 110048")),
            ("FIRM_BANK", Field::text("HDFC Bank — A/C: 50200012345678 IFSC: HDFC0000123")),
            ("CLIENT_NAME", Field::text("Tata & Sons Pvt Ltd")),
            ("CLIENT_GSTIN", Field::text("27AAACT2727Q1ZW")),
            ("CLIENT_ADDRESS", Field::text("Bombay House, 24 Homi Mody St, Mumbai 400001")),
            ("LINE_ITEMS_TABLE", Field::raw(rows)),
            ("SUBTOTAL", Field::text("600000.00")),
            ("CGST_AMOUNT", Field::text("54000.00")),
            ("SGST_AMOUNT", Field::text("54000.00")),
            ("IGST_AMOUNT", Field::text("0.00")),
            ("TOTAL", Field::text("708000.00")),
            ("AMOUNT_IN_WORDS", Field::text("Rupees Seven Lakh Eight Thousand Only")),
            ("SAC_CODE", Field::text("998212")),
            ("GST_TYPE", Field::text("Intra")),
            ("NOTES", Field::text("Opposition filed 100% within limitation; ₹4,500 per class. व्यापार चिह्न")),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect()
    }

    /// The whole point. Before this sprint the seeded firm name alone made every
    /// invoice fail to compile.
    #[tokio::test]
    async fn the_invoice_template_compiles_with_hostile_real_world_values() {
        if !engine_available() {
            return;
        }

        let pdf = compile_latex("invoice", &invoice_fields())
            .await
            .expect("the invoice template must compile");

        assert!(pdf.starts_with(b"%PDF-"), "output is not a PDF");
        assert!(pdf.len() > 5_000, "PDF is implausibly small: {} bytes", pdf.len());
    }

    /// A missing value must stop the job, not print `{{CLIENT_ADDRESS}}` into a
    /// document that goes to a client.
    #[tokio::test]
    async fn a_missing_field_fails_before_the_engine_runs() {
        if !engine_available() {
            return;
        }

        let mut fields = invoice_fields();
        fields.remove("CLIENT_ADDRESS");

        let err = compile_latex("invoice", &fields).await.unwrap_err().to_string();
        assert!(err.contains("CLIENT_ADDRESS"), "error must name the key: {err}");
    }

    /// A client name containing LaTeX must produce a PDF, not a compile error and
    /// not an injection.
    #[tokio::test]
    async fn a_client_name_full_of_latex_still_compiles() {
        if !engine_available() {
            return;
        }

        let mut fields = invoice_fields();
        fields.insert(
            "CLIENT_NAME".into(),
            Field::text(r"\input{/etc/passwd} 100% {\bf x} $y^2$ & Co #1 ~ _z_"),
        );

        let pdf = compile_latex("invoice", &fields)
            .await
            .expect("escaped input must compile");
        assert!(pdf.starts_with(b"%PDF-"));
    }

    /// A template that loops forever must be stopped, not left running.
    ///
    /// Drives `run_engine` with inline source rather than pointing
    /// `PERSIST_TEMPLATES_DIR` at a fixture: that variable is process-global, so
    /// setting it here raced the test above and made it fail intermittently.
    #[tokio::test]
    async fn a_runaway_template_is_stopped_by_the_timeout() {
        if !engine_available() {
            return;
        }

        // Compiles happily and never terminates: exactly what -halt-on-error
        // cannot catch, because nothing ever errors.
        let source = "\\documentclass{article}\\begin{document}\n\
                      \\newcount\\n \\loop \\advance\\n by 1 \\ifnum\\n>0 \\repeat\n\
                      \\end{document}\n";

        let started = std::time::Instant::now();
        let err = run_engine("runaway", source).await.unwrap_err();

        assert!(
            started.elapsed() < COMPILE_TIMEOUT + Duration::from_secs(20),
            "the timeout did not stop it: ran for {:?}",
            started.elapsed()
        );
        assert!(
            err.to_string().contains("exceeded") || err.to_string().contains("failed"),
            "unexpected error: {err}"
        );
    }
}
