// LaTeX compilation service.
// Compiles .tex templates locally using TeX Live (bundled in the installer).
// NEVER compile LaTeX server-side or via network.
//
// Template variables use the {{KEY}} syntax.
// All substitutions happen before the subprocess is spawned.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{bail, Context};
use tokio::task;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile a LaTeX template with the provided field values.
/// Returns raw PDF bytes — caller decides where to store them.
///
/// # Arguments
/// * `template_id` — filename without extension inside the templates directory (e.g. "invoice")
/// * `field_values` — map of `KEY` → value; replaces `{{KEY}}` in the template
pub async fn compile_latex(
    template_id: &str,
    field_values: &HashMap<String, String>,
) -> anyhow::Result<Vec<u8>> {
    let template_dir = find_templates_dir()?;
    let template_path = template_dir.join(format!("{template_id}.tex"));

    let source = std::fs::read_to_string(&template_path)
        .with_context(|| format!("failed to read template: {}", template_path.display()))?;

    // Substitute all {{KEY}} → value
    let rendered = apply_substitutions(&source, field_values);

    // Spawn blocking so we don't stall Tokio on the subprocess
    let pdf_bytes = task::spawn_blocking(move || {
        run_pdflatex(&rendered)
    })
    .await
    .context("pdflatex thread panicked")??;

    Ok(pdf_bytes)
}

// ---------------------------------------------------------------------------
// Template substitution
// ---------------------------------------------------------------------------

fn apply_substitutions(source: &str, fields: &HashMap<String, String>) -> String {
    let mut out = source.to_owned();
    for (key, value) in fields {
        let placeholder = format!("{{{{{key}}}}}"); // {{KEY}}
        out = out.replace(&placeholder, value);
    }
    out
}

// ---------------------------------------------------------------------------
// pdflatex subprocess
// ---------------------------------------------------------------------------

fn run_pdflatex(latex_source: &str) -> anyhow::Result<Vec<u8>> {
    let pdflatex = find_pdflatex()?;

    // Create a temp directory for this compilation job
    let tmp_dir = tempfile::tempdir().context("failed to create temp dir for pdflatex")?;
    let tex_path = tmp_dir.path().join("invoice.tex");
    let pdf_path = tmp_dir.path().join("invoice.pdf");

    std::fs::write(&tex_path, latex_source)
        .context("failed to write .tex source to temp dir")?;

    // Run pdflatex (non-interactive, halt on error, output to temp dir)
    let output = std::process::Command::new(&pdflatex)
        .args([
            "-interaction=nonstopmode",
            "-halt-on-error",
            "-output-directory",
            tmp_dir.path().to_str().unwrap_or("."),
            tex_path.to_str().unwrap_or("invoice.tex"),
        ])
        .output()
        .with_context(|| format!("failed to spawn pdflatex at {}", pdflatex.display()))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "pdflatex exited with status {:?}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
            output.status.code()
        );
    }

    let pdf_bytes = std::fs::read(&pdf_path)
        .with_context(|| format!("pdflatex succeeded but PDF not found at {}", pdf_path.display()))?;

    Ok(pdf_bytes)
}

// ---------------------------------------------------------------------------
// Locate pdflatex binary
// ---------------------------------------------------------------------------

fn find_pdflatex() -> anyhow::Result<PathBuf> {
    // Ordered preference: bundled sidecar → macOS TeX Live → common Unix paths → PATH
    let candidates = [
        // Tauri sidecar in production (installed alongside the app binary)
        sidecar_pdflatex_path().as_deref().unwrap_or("").to_owned(),
        // macOS TeX Live (installed by MacTeX)
        "/Library/TeX/texbin/pdflatex".to_owned(),
        "/usr/local/texlive/2024/bin/universal-darwin/pdflatex".to_owned(),
        "/usr/local/texlive/2023/bin/universal-darwin/pdflatex".to_owned(),
        // Linux common
        "/usr/bin/pdflatex".to_owned(),
        "/usr/local/bin/pdflatex".to_owned(),
    ];

    for candidate in &candidates {
        if candidate.is_empty() { continue; }
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Ok(p);
        }
    }

    // Last resort: hope it's on PATH
    if let Ok(path_str) = which_pdflatex() {
        return Ok(PathBuf::from(path_str));
    }

    bail!(
        "pdflatex not found. Install TeX Live (macOS: `brew install --cask mactex-no-gui`) \
         or ensure the bundled sidecar is installed."
    )
}

/// Try `which pdflatex` to find it on PATH.
fn which_pdflatex() -> anyhow::Result<String> {
    let output = std::process::Command::new("which")
        .arg("pdflatex")
        .output()?;
    if output.status.success() {
        let path = String::from_utf8(output.stdout)?.trim().to_owned();
        if !path.is_empty() {
            return Ok(path);
        }
    }
    bail!("which pdflatex returned no result")
}

/// Path to a pdflatex sidecar bundled with the Tauri installer.
/// Returns None if we can't locate it (dev mode, wrong platform, etc.)
fn sidecar_pdflatex_path() -> Option<String> {
    // In production the Tauri installer places sidecars next to the main binary.
    // The pattern: <binary_dir>/pdflatex or <binary_dir>/../resources/pdflatex
    let exe = std::env::current_exe().ok()?;
    let bin_dir = exe.parent()?;

    let candidates = [
        bin_dir.join("pdflatex"),
        bin_dir.join("../resources/pdflatex"),
        bin_dir.join("../Frameworks/pdflatex"),
    ];

    for p in &candidates {
        if p.exists() {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Locate templates directory
// ---------------------------------------------------------------------------

fn find_templates_dir() -> anyhow::Result<PathBuf> {
    // 1. Explicit override (CI, tests, dev)
    if let Ok(dir) = std::env::var("PERSIST_TEMPLATES_DIR") {
        let p = PathBuf::from(&dir);
        if p.is_dir() {
            return Ok(p);
        }
    }

    // 2. Next to the binary (production: Tauri resources)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            let candidates = [
                bin_dir.join("resources/templates"),
                bin_dir.join("../resources/templates"),
                bin_dir.join("../Resources/templates"), // macOS .app bundle
            ];
            for p in &candidates {
                if p.is_dir() {
                    return Ok(p.to_owned());
                }
            }
        }
    }

    // 3. Development fallback — relative to the workspace root
    // When running `cargo test` the CWD is src-tauri/
    let dev_candidates = [
        PathBuf::from("storage/templates"),
        PathBuf::from("src-tauri/storage/templates"),
    ];
    for p in &dev_candidates {
        if p.is_dir() {
            return Ok(p.to_owned());
        }
    }

    bail!(
        "LaTeX templates directory not found. \
         Set PERSIST_TEMPLATES_DIR or place templates in src-tauri/storage/templates/."
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_all_placeholders() {
        let source = "Hello {{NAME}}, your invoice is {{INVOICE_ID}}. Total: {{TOTAL}}.";
        let mut fields = HashMap::new();
        fields.insert("NAME".to_owned(), "Acme Corp".to_owned());
        fields.insert("INVOICE_ID".to_owned(), "INV-2026-0001".to_owned());
        fields.insert("TOTAL".to_owned(), "18,000.00".to_owned());

        let result = apply_substitutions(source, &fields);
        assert_eq!(result, "Hello Acme Corp, your invoice is INV-2026-0001. Total: 18,000.00.");
    }

    #[test]
    fn missing_key_leaves_placeholder_intact() {
        let source = "Firm: {{FIRM_NAME}}, Missing: {{NOT_SET}}";
        let mut fields = HashMap::new();
        fields.insert("FIRM_NAME".to_owned(), "Persistas & Partners".to_owned());

        let result = apply_substitutions(source, &fields);
        assert_eq!(result, "Firm: Persistas & Partners, Missing: {{NOT_SET}}");
    }

    #[test]
    fn empty_fields_map_returns_source_unchanged() {
        let source = "Invoice {{INVOICE_ID}} dated {{INVOICE_DATE}}";
        let fields = HashMap::new();
        let result = apply_substitutions(source, &fields);
        assert_eq!(result, source);
    }
}
