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

    /// The LaTeX this field will contribute. `pub(crate)` so a caller that
    /// assembles a block can assert on what it built; not public, because
    /// outside Keel a `Field` should only ever be something you construct.
    pub(crate) fn as_str(&self) -> &str {
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
// Attachments
// ---------------------------------------------------------------------------

/// A file staged beside the template for one compilation.
///
/// The annexures of a legal notice are documents out of the vault, and the
/// engine has to be able to open them by name — `\includepdf{annexure-01.pdf}`.
/// They are written into the same temporary directory as the rendered .tex and
/// go away with it, so decrypted client material never lands anywhere
/// persistent.
///
/// The name is checked rather than trusted. It reaches a filesystem path and a
/// LaTeX argument, and the only reason `../../.ssh/id_rsa` is not a working
/// attack here is that [`Attachment::new`] refuses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    name: String,
    bytes: Vec<u8>,
}

impl Attachment {
    pub fn new(name: impl Into<String>, bytes: Vec<u8>) -> anyhow::Result<Self> {
        let name = name.into();

        let shape_is_safe = !name.is_empty()
            && name.len() <= 64
            && !name.starts_with('.')
            && name.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' || c == '.'
            });

        if !shape_is_safe {
            bail!("unsafe attachment name: {name:?}");
        }
        Ok(Attachment { name, bytes })
    }

    /// The filename the template refers to.
    pub fn name(&self) -> &str {
        &self.name
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// What a compilation is for.
///
/// The Smart Form Compiler re-renders on every keystroke-ish change (PRD §9.8
/// asks for a live preview at 1–2s). A preview that is one pass out of date on
/// column widths is fine; a filing is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompileMode {
    /// One pass. For the live preview only — never for a document that leaves
    /// the firm, because `longtable` widths and page references settle on the
    /// second pass.
    Draft,
    /// Two passes. Everything an attorney signs or sends.
    Final,
}

impl CompileMode {
    fn passes(self) -> usize {
        match self {
            CompileMode::Draft => 1,
            CompileMode::Final => PASSES,
        }
    }
}

/// Compile a template to PDF bytes. The caller decides where they are stored.
///
/// `template_id` is the filename without extension inside the templates
/// directory — "invoice" for `invoice.tex`.
pub async fn compile_latex(
    template_id: &str,
    fields: &HashMap<String, Field>,
) -> anyhow::Result<Vec<u8>> {
    compile(template_id, fields, CompileMode::Final).await
}

pub async fn compile(
    template_id: &str,
    fields: &HashMap<String, Field>,
    mode: CompileMode,
) -> anyhow::Result<Vec<u8>> {
    compile_with(template_id, fields, mode, &[]).await
}

/// Compile with files staged beside the template — annexures, exhibits.
///
/// The attachments are written into the compile directory under the names the
/// rendered template refers to. Keel chooses those names; see [`Attachment`].
pub async fn compile_with(
    template_id: &str,
    fields: &HashMap<String, Field>,
    mode: CompileMode,
    attachments: &[Attachment],
) -> anyhow::Result<Vec<u8>> {
    let template_dir = find_templates_dir()?;
    let template_path = template_dir.join(format!("{template_id}.tex"));

    let source = std::fs::read_to_string(&template_path)
        .with_context(|| format!("failed to read template: {}", template_path.display()))?;

    let rendered = render(&source, fields)?;
    run_engine_in(
        template_id,
        &rendered,
        Some(&template_dir),
        mode.passes(),
        attachments,
    )
    .await
}

/// Compile a trivial document so the font cache is built before an attorney is
/// waiting on it.
///
/// Measured on this machine: a cold XeLaTeX run takes ~14s while it builds the
/// font cache, and ~1.4s warm. That cost lands once per machine, but landing it
/// on the first real document — an attorney watching a blank preview pane —
/// is the wrong time. Call it at startup, off the critical path.
///
/// Failure is logged and swallowed: the app must start on a machine with no
/// TeX Live, and say so when a document is actually requested.
pub async fn warm_up() {
    const PROBE: &str = "\\documentclass{article}\n\
                         \\usepackage{fontspec}\n\
                         \\setmainfont{Noto Serif}\n\
                         \\begin{document}₹\\end{document}\n";

    let started = std::time::Instant::now();
    match run_engine_in("warmup", PROBE, None, 1, &[]).await {
        Ok(_) => log::info!("LaTeX engine warm after {:?}", started.elapsed()),
        Err(e) => log::warn!("LaTeX warm-up skipped: {e:#}"),
    }
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

    let mut unfilled = placeholders_in(&out);
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

/// Every `{{KEY}}` a template refers to, outside LaTeX comments.
///
/// Used by the template registry to check a manifest against the template it
/// describes, and by `render` to find the ones nobody filled.
///
/// Comment lines are skipped because templates document their own variables in
/// a `%` header, and that documentation must not fail the build it describes.
pub fn placeholders_in(source: &str) -> Vec<String> {
    let mut found = Vec::new();

    for line in source.lines() {
        let code = match line.find('%') {
            // An escaped `\%` is a literal percent, not a comment.
            Some(i) if !line[..i].ends_with('\\') => &line[..i],
            Some(_) | None => line,
        };

        let bytes = code.as_bytes();
        let mut i = 0;
        while i + 4 <= bytes.len() {
            if !(bytes[i] == b'{' && bytes[i + 1] == b'{') {
                i += 1;
                continue;
            }

            // Templates legitimately write `\textbf{{{KEY}}}` — a LaTeX brace
            // wrapping a placeholder. Anchoring on the first `{{` would read the
            // key as `{KEY` and miss it, so advance one brace at a time rather
            // than jumping past the whole run.
            let rest = &code[i + 2..];
            let Some(end) = rest.find("}}") else { break };
            let key = &rest[..end];

            if is_placeholder_key(key) {
                found.push(key.to_owned());
                i += 2 + end + 2;
            } else {
                i += 1;
            }
        }
    }

    found
}

/// Placeholder keys are SHOUTY_SNAKE. Anything else between braces is LaTeX.
fn is_placeholder_key(key: &str) -> bool {
    !key.is_empty()
        && key.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
}

// ---------------------------------------------------------------------------
// Engine subprocess
// ---------------------------------------------------------------------------

/// Compile inline source. `search_dir` is prepended to TEXINPUTS so a template
/// can `\input{_shared/persist-base}` — the .tex itself is written to a temp
/// directory, so without this the shared preamble is unreachable.
async fn run_engine_in(
    job_name: &str,
    source: &str,
    search_dir: Option<&Path>,
    passes: usize,
    attachments: &[Attachment],
) -> anyhow::Result<Vec<u8>> {
    let engine = find_engine()?;

    let tmp_dir = tempfile::tempdir().context("failed to create temp dir for LaTeX")?;
    // Named after the template so a failure log is identifiable, and sanitised
    // because the name reaches a command line.
    let stem = sanitise_job_name(job_name);
    let tex_path = tmp_dir.path().join(format!("{stem}.tex"));
    let pdf_path = tmp_dir.path().join(format!("{stem}.pdf"));

    std::fs::write(&tex_path, source).context("failed to write .tex source")?;

    for attachment in attachments {
        // `Attachment::new` has already refused anything with a separator in it,
        // so this join cannot leave the temp directory.
        std::fs::write(tmp_dir.path().join(&attachment.name), &attachment.bytes)
            .with_context(|| format!("failed to stage attachment {}", attachment.name))?;
    }

    let mut last_log = String::new();
    for pass in 1..=passes {
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
            .envs(texinputs(tmp_dir.path(), search_dir))
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
                "LaTeX engine failed on pass {pass} of {passes}:\n{}",
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


/// TEXINPUTS for the child: the compile directory, then the shared templates.
///
/// The compile directory comes first and is always present, because it is where
/// staged attachments live. kpathsea resolves `\includegraphics` and
/// `\includepdf` filenames through TEXINPUTS too, and the engine's working
/// directory is the app's, not the compile directory — so without this an
/// annexure is a "file not found" even though it was written moments earlier.
///
/// The trailing empty entry is significant to kpathsea: it means "then the
/// normal search path", so adding ours does not cut the engine off from its own
/// packages.
fn texinputs(compile_dir: &Path, search_dir: Option<&Path>) -> Vec<(String, String)> {
    let separator = if cfg!(windows) { ";" } else { ":" };
    let existing = std::env::var("TEXINPUTS").unwrap_or_default();

    let mut path = format!("{}{separator}", compile_dir.display());
    if let Some(dir) = search_dir {
        path.push_str(&format!("{}{separator}", dir.display()));
    }
    path.push_str(&existing);
    path.push_str(separator);

    Vec::from([("TEXINPUTS".to_owned(), path)])
}

/// Is there an engine to test against?
///
/// Skipping keeps `cargo test` green on a machine with no TeX Live. But a suite
/// that skips its most important tests reports success while proving nothing —
/// which is exactly how three fatal faults reached a branch. CI sets
/// `PERSIST_REQUIRE_LATEX=1`, and then a missing engine is a failure.
///
/// `pub(crate)`: every module whose output ends up in a compiled document needs
/// this same guard, and a second copy of it would drift.
#[cfg(test)]
pub(crate) fn engine_available() -> bool {
    if find_engine().is_ok() {
        return true;
    }
    assert!(
        std::env::var("PERSIST_REQUIRE_LATEX").is_err(),
        "PERSIST_REQUIRE_LATEX is set but no xelatex/lualatex was found — \
         the compilation tests would have skipped silently"
    );
    eprintln!("SKIPPING LaTeX compilation tests: no xelatex/lualatex found");
    false
}

/// Compile inline source with no shared-template directory. Used by tests —
/// including tests in other modules that need a genuine PDF to work on.
#[cfg(test)]
pub(crate) async fn run_engine(job_name: &str, source: &str) -> anyhow::Result<Vec<u8>> {
    run_engine_in(job_name, source, None, 1, &[]).await
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

/// Where the template library lives. Public so the drafting commands can
/// enumerate it without duplicating the search order.
pub fn templates_dir() -> anyhow::Result<PathBuf> {
    find_templates_dir()
}

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

    /// Templates write `\textbf{{{KEY}}}` — a LaTeX brace around a placeholder.
    /// The scanner used to anchor on the first `{{`, read the key as `{KEY`,
    /// and find nothing. `render` therefore could not tell that such a
    /// placeholder was unfilled, and it would have printed into the PDF.
    #[test]
    fn a_placeholder_wrapped_in_latex_braces_is_still_found() {
        assert_eq!(placeholders_in(r"\textbf{{{FIRM_GSTIN}}}"), vec!["FIRM_GSTIN"]);
        assert_eq!(placeholders_in(r"\textit{{{AMOUNT_IN_WORDS}}}"), vec!["AMOUNT_IN_WORDS"]);
        assert_eq!(
            placeholders_in(r"\firmfooter{{{A}}}{{{B}}}{{{C}}}"),
            vec!["A", "B", "C"],
        );
    }

    #[test]
    fn an_unfilled_placeholder_inside_latex_braces_fails_the_render() {
        let err = render(r"\textbf{{{MISSING}}}", &HashMap::new()).unwrap_err();
        assert!(err.to_string().contains("MISSING"), "{err}");
    }

    #[test]
    fn latex_group_braces_are_not_mistaken_for_placeholders() {
        assert!(placeholders_in(r"{\bfseries x} {{lowercase}} {\color{red} y}").is_empty());
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

#[cfg(test)]
mod second_template_tests {
    use super::*;

    /// The registry only earns its keep if a template other than the invoice
    /// actually renders. This is the second one, and the one the Smart Form
    /// Compiler's first real form will drive.
    #[tokio::test]
    async fn the_examination_reply_compiles() {
        if !engine_available() {
            return;
        }

        let prior_use = format!(
            "\\persistsection{{Prior use}}\n\\noindent The Applicant has used the mark \
             continuously in the course of trade since {}.",
            escape("2019-04-01")
        );

        let fields: HashMap<String, Field> = [
            ("FIRM_NAME", Field::text("Persistas & Partners")),
            ("FIRM_ADDRESS", Field::text("B-12, Greater Kailash-I, New Delhi 110048")),
            ("FIRM_CONTACT", Field::text("+91 11 4000 1234 — mail@persistas.in")),
            ("ATTORNEY_NAME", Field::text("Sree Lakshmi Menon")),
            ("REPLY_DATE", Field::text("2026-08-10")),
            ("REGISTRY_OFFICE", Field::text("Delhi")),
            ("TM_NUMBER", Field::text("5544121")),
            ("TM_MARK", Field::text("PETALVEDA")),
            ("TM_CLASS", Field::text("3, 5")),
            ("APPLICANT_NAME", Field::text("Tata & Sons Pvt Ltd")),
            ("EXAM_REPORT_DATE", Field::text("2026-06-15")),
            (
                "SUBMISSIONS",
                // 100% and the ampersand are the two characters that broke the
                // invoice; a second template must survive them too.
                Field::text(
                    "The objection under s.11(1) is respectfully denied. The cited mark \
                     covers goods in class 30 and is 100% distinct in trade channels. \
                     The Applicant's mark is used on ayurvedic preparations & cosmetics.",
                ),
            ),
            ("GROUNDS_TEXT", Field::text("Prior use since 2019; no likelihood of confusion.")),
            ("PRIOR_USE_BLOCK", Field::raw(prior_use)),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();

        let pdf = compile_latex("tm-examination-reply", &fields)
            .await
            .expect("the examination reply must compile");

        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf.len() > 3_000, "implausibly small: {} bytes", pdf.len());
    }
}

#[cfg(test)]
mod mode_tests {
    use super::*;

    #[test]
    fn a_draft_is_one_pass_and_a_final_is_two() {
        // The distinction is the whole point: a preview may lag on column
        // widths, a filing may not.
        assert_eq!(CompileMode::Draft.passes(), 1);
        assert_eq!(CompileMode::Final.passes(), PASSES);
        assert!(CompileMode::Final.passes() > CompileMode::Draft.passes());
    }

    /// The warm-up must not take the app down on a machine with no TeX Live.
    #[tokio::test]
    async fn warm_up_never_panics_or_propagates() {
        warm_up().await;
    }

    #[tokio::test]
    async fn a_draft_and_a_final_both_produce_a_pdf() {
        if !engine_available() {
            return;
        }

        let fields: HashMap<String, Field> =
            [("NAME".to_owned(), Field::text("Tata & Sons"))].into_iter().collect();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("modes.tex"),
            "\\documentclass{article}\\usepackage{fontspec}\\setmainfont{Noto Serif}\
             \\begin{document}{{NAME}} ₹100\\end{document}",
        )
        .unwrap();

        let source = std::fs::read_to_string(dir.path().join("modes.tex")).unwrap();
        let rendered = render(&source, &fields).unwrap();

        for mode in [CompileMode::Draft, CompileMode::Final] {
            let pdf = run_engine_in("modes", &rendered, None, mode.passes(), &[])
                .await
                .unwrap_or_else(|e| panic!("{mode:?} failed: {e:#}"));
            assert!(pdf.starts_with(b"%PDF-"), "{mode:?} produced no PDF");
        }
    }
}

#[cfg(test)]
mod attachment_tests {
    use super::*;

    /// The attachment name reaches `tmp_dir.join(...)`. A name that can climb
    /// out of the temp directory would let a template's annexure overwrite
    /// anything the app can write.
    #[test]
    fn a_name_that_could_escape_the_compile_directory_is_refused() {
        for bad in [
            "../../etc/passwd",
            "..",
            "sub/dir.pdf",
            "sub\\dir.pdf",
            ".hidden",
            "",
            "annexure 01.pdf",   // a space ends the LaTeX argument
            "annexure{01}.pdf",  // braces do the same
            "ANNEXURE-01.PDF",   // uppercase is not in the generated shape
        ] {
            assert!(
                Attachment::new(bad, Vec::new()).is_err(),
                "{bad:?} should not be an acceptable attachment name"
            );
        }
    }

    #[test]
    fn the_names_keel_generates_are_accepted() {
        for good in ["annexure-01.pdf", "annexure-12.png", "annexure-03.jpg"] {
            assert!(Attachment::new(good, Vec::new()).is_ok(), "{good} was refused");
        }
    }

    /// The engine's working directory is the app's, not the compile directory,
    /// so a staged file is only reachable because TEXINPUTS says so. Without
    /// the compile directory on that path every annexure is "file not found".
    #[tokio::test]
    async fn a_staged_file_is_reachable_by_name_from_the_template() {
        if !engine_available() {
            return;
        }

        let attachment = Attachment::new(
            "staged-note.tex",
            b"Staged and found.\n".to_vec(),
        )
        .unwrap();

        let source = "\\documentclass{article}\n\
                      \\begin{document}\n\
                      \\input{staged-note}\n\
                      \\end{document}\n";

        let pdf = run_engine_in("staged", source, None, 1, std::slice::from_ref(&attachment))
            .await
            .expect("a staged file must be reachable by name");
        assert!(pdf.starts_with(b"%PDF-"));
    }
}

#[cfg(test)]
mod letterhead_tests {
    use super::*;

    /// The letterhead carries the firm's mark and the registered office on every
    /// page of everything it sends. It has to compile, and it has to compile
    /// with the characters the firm's own details contain.
    #[tokio::test]
    async fn the_legal_notice_compiles_on_the_firm_letterhead() {
        if !engine_available() {
            return;
        }

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
            // The characters that have broken this pipeline before: an
            // ampersand, a percent, and the rupee sign.
            ("SUBJECT", "DEMAND FOR REFUND OF ₹1,04,000/- WITH INTEREST @24% PER ANNUM, COSTS & CHARGES"),
            ("SALUTATION", "Sir"),
            ("CLIENT_NAME", "Mr. Nikhil Prabhakar"),
            ("CLIENT_DESCRIPTION", "son of P. Prabhakaran"),
            ("CLIENT_ADDRESS", "A-004, Mangal Apartment, Vasundhra Enclave, New Delhi-110096"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), Field::text(v)))
        .collect();

        // The blocks Keel assembles. Multi-line and multi-row content is LaTeX
        // by necessity — everything interpolated into it is escaped first.
        fields.insert(
            "NOTICE_DATE".into(),
            Field::raw("14\\textsuperscript{th} July 2026"),
        );
        fields.insert(
            "RECIPIENT_ADDRESS_BLOCK".into(),
            Field::raw(format!("{}\\\\\n{}", escape("House No. 460/21,"), escape("Lucknow – 226003"))),
        );
        fields.insert(
            "SECTIONS_BLOCK".into(),
            Field::raw(format!(
                "\\noticesection{{1}}{{{}}}\n\\begin{{noticebody}}\n{}\n\\end{{noticebody}}\n",
                escape("Background & Circumstances"),
                escape("That during 2025 you represented yourself as associated with a lender, 100% falsely.")
            )),
        );
        fields.insert(
            "SIGNATORY_BLOCK".into(),
            Field::raw("Sree Lakshmi Menon\\\\\nD/6361/2020\\\\\nAdvocates"),
        );
        // No annexures: the two blocks a notice carries them in are empty, and
        // the document must be complete without them. Attaching them is covered
        // in services/annexures.rs, which compiles the same template with real
        // files staged beside it.
        fields.insert("ANNEXURES_BLOCK".into(), Field::raw(""));
        fields.insert("ANNEXURE_PAGES".into(), Field::raw(""));

        let pdf = compile_latex("legal-notice", &fields)
            .await
            .expect("the legal notice must compile");

        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf.len() > 40_000, "the logo should make this substantial: {}", pdf.len());
    }
}
