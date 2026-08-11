// Document drafting — the commands the Smart Form Compiler (M9.8) will drive.
//
// Deck asks what templates exist, gets a schema, builds a form from it, and
// sends values back. Keel validates against the same schema and renders.
//
// The attorney never sees LaTeX. They also never see a LaTeX error: a compile
// failure comes back as a plain sentence, with the detail in the log where a
// developer can find it (PRD §9.8, "compile errors are handled silently").

use crate::rbac::{self, Permission};
use crate::services::sync_engine::{self, EntityType, Op};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use crate::services::annexures::{self, Annexure};
use crate::services::latex::{self, Attachment, CompileMode, Field};
use crate::services::layout::{self, DocumentLayout};
use crate::services::templates::{
    self, FieldError, FieldKind, FieldValue, TemplateManifest,
};
use crate::storage::metadata;
use crate::AppState;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// IPC types
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderDocumentInput {
    pub template_id: String,
    /// Attorney-entered values, keyed by field key.
    ///
    /// A value is either what was typed into one input or, for a `list` field,
    /// the rows of a repeating group — see `FieldValue`.
    pub values: HashMap<String, FieldValue>,
    /// Draft for the live preview, Final for anything that leaves the firm.
    pub mode: CompileMode,
    /// Where a Final render is filed. A draft needs none — it is never stored.
    #[serde(default)]
    pub matter_id: Option<String>,
    /// Documents to attach as proof, in the order they should be marked.
    #[serde(default)]
    pub annexures: Vec<AnnexureInput>,
    /// Paper, typeface, spacing, page numbering, letterhead placement.
    ///
    /// Absent means the firm's house format. This is not template content — see
    /// services/layout.rs — so it is not in `values` and no manifest declares it.
    #[serde(default)]
    pub layout: DocumentLayout,
}

/// One file the attorney has attached, in the order they put it in.
///
/// No mark: Persist allocates those from this order. Letting the form carry a
/// mark would let two annexures be called C, and would make reordering a manual
/// renumbering exercise across the printed list and the bundle.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnexureInput {
    /// From `stage_annexure`.
    pub staged_id: String,
    /// What the attorney called it — "Receipt one". Printed in the list on the
    /// document; never on the annexure page itself.
    pub title: String,
}

/// A render either produced a document or has something to say about why not.
///
/// Field errors and a compile failure are different things and Deck shows them
/// differently: the first belongs against an input, the second is a banner.
///
/// `Default` is an empty result: no document, nothing wrong, nothing filed.
/// Every construction site below fills in only what it is saying, so a field
/// added here does not have to be added to five places that do not care.
#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderResult {
    /// The PDF, base64-encoded.
    ///
    /// Not `Vec<u8>`: Tauri serialises a byte vector as a JSON array of
    /// numbers, roughly three to four bytes of JSON per byte of PDF. The live
    /// preview re-renders as the attorney types, so a 250 KB document would
    /// cross the bridge as most of a megabyte each time. Base64 is 1.33x.
    pub pdf_base64: Option<String>,
    #[serde(default)]
    pub field_errors: Vec<FieldError>,
    /// Set when the document could not be produced for a reason that is not a
    /// field. Written for an attorney.
    pub problem: Option<String>,
    /// Set on a Final render that was filed — the vault document id.
    pub document_id: Option<String>,
    /// The marks actually allocated, in order, for the annexures that were
    /// attached.
    ///
    /// Deck shows these against the attorney's list rather than working them
    /// out itself. A second implementation of the marking rules in TypeScript
    /// would eventually disagree with this one, and the attorney would be
    /// looking at a mark the document does not carry.
    #[serde(default)]
    pub annexure_marks: Vec<AnnexureMark>,
}

/// One allocated mark, as it appears on the document.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnexureMark {
    pub staged_id: String,
    /// "A", or "A(ii)".
    pub mark: String,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Every template in the library, for the document-type picker.
#[tauri::command]
pub async fn list_templates() -> Result<Vec<TemplateManifest>, String> {
    let dir = latex::templates_dir().map_err(|e| e.to_string())?;
    templates::list(&dir).map_err(|e| e.to_string())
}

/// One template's schema. Deck builds the form from this — labels, kinds,
/// validation, conditional visibility and autofill hints all come from here,
/// so adding a template needs no Deck change.
#[tauri::command]
pub async fn get_template(id: String) -> Result<TemplateManifest, String> {
    let dir = latex::templates_dir().map_err(|e| e.to_string())?;
    templates::get(&dir, &id).map_err(|e| e.to_string())
}

/// Validate and render.
///
/// A draft render still validates. Letting the preview show a document that
/// could never be filed would teach the attorney to trust it.
#[tauri::command]
pub async fn render_document(
    input: RenderDocumentInput,
    state: tauri::State<'_, AppState>,
) -> Result<RenderResult, String> {
    // Drafting produces firm instruments on firm letterhead.
    rbac::require(&state, Permission::CreateDeadline).await?;

    let dir = latex::templates_dir().map_err(|e| e.to_string())?;
    let manifest = templates::get(&dir, &input.template_id).map_err(|e| e.to_string())?;

    let field_errors = templates::validate(&manifest, &input.values);
    if !field_errors.is_empty() {
        return Ok(RenderResult { field_errors, ..RenderResult::default() });
    }

    // Annexures are loaded before the compile so a document that cannot be
    // attached is a sentence the attorney can act on, not a LaTeX failure.
    let (annexures, attachments) = match load_annexures(&state, &input.annexures).await {
            Ok(loaded) => loaded,
            Err(problem) => {
                return Ok(RenderResult { problem: Some(problem), ..RenderResult::default() })
            }
        };

    // Refused here rather than at the engine, where a bad size would surface as
    // a LaTeX error instead of a sentence.
    if let Err(e) = input.layout.validate() {
        return Ok(RenderResult { problem: Some(e.to_string()), ..RenderResult::default() });
    }

    // A template that does not declare the annexure keys cannot carry
    // annexures. Binding them anyway would fail the render on an unused key;
    // dropping them silently would produce a notice missing its proof.
    let takes_annexures = ANNEXURE_KEYS
        .iter()
        .all(|key| manifest.fields.iter().any(|spec| spec.key == *key));

    if !annexures.is_empty() && !takes_annexures {
        return Ok(RenderResult {
            problem: Some(format!("{} documents cannot carry annexures.", manifest.name)),
            ..RenderResult::default()
        });
    }

    // The list printed in the notice and the pages appended after it, from one
    // source. `with_computed` overrides anything Deck sent under these keys.
    let mut fields = to_latex_fields(&manifest, &input.values);
    if takes_annexures {
        fields = with_computed(
            fields,
            HashMap::from([
                ("ANNEXURES_BLOCK".to_owned(), annexures::list_block(&annexures)),
                ("ANNEXURE_PAGES".to_owned(), annexures::pages_block(&annexures)),
            ]),
        );
    }

    // The layout is staged like any other file the compile needs;
    // _shared/persist-base.tex reads it by name.
    let mut attachments = attachments;
    match Attachment::new(layout::LAYOUT_FILE, input.layout.to_latex().into_bytes()) {
        Ok(staged) => attachments.push(staged),
        Err(e) => {
            log::error!("could not stage the document layout: {e:#}");
            return Ok(RenderResult {
                problem: Some("The page settings could not be applied.".to_owned()),
                ..RenderResult::default()
            });
        }
    }

    let pdf = match latex::compile_with(&input.template_id, &fields, input.mode, &attachments).await
    {
        Ok(pdf) => pdf,
        Err(e) => {
            // The LaTeX log is for us, not for an attorney (PRD §9.8).
            log::error!("render of '{}' failed: {e:#}", input.template_id);
            return Ok(RenderResult {
                problem: Some(match input.mode {
                    CompileMode::Draft => {
                        "Preview temporarily unavailable — your content is saved.".to_owned()
                    }
                    CompileMode::Final => {
                        "This document could not be generated. The details have been \
                         written to the log for the firm's administrator."
                            .to_owned()
                    }
                }),
                ..RenderResult::default()
            });
        }
    };

    // A draft is never stored. It exists for the length of one preview.
    let document_id = match (input.mode, input.matter_id.as_deref()) {
        (CompileMode::Final, Some(matter_id)) => {
            Some(file_document(&state, matter_id, &manifest, &pdf).await?)
        }
        _ => None,
    };

    Ok(RenderResult {
        pdf_base64: Some(BASE64.encode(&pdf)),
        document_id,
        annexure_marks: input
            .annexures
            .iter()
            .zip(&annexures)
            .map(|(chosen, prepared)| AnnexureMark {
                staged_id: chosen.staged_id.clone(),
                mark: prepared.mark.clone(),
            })
            .collect(),
        ..RenderResult::default()
    })
}

/// Encrypt a finished document into the vault and record it against the matter.
///
/// Same path an uploaded document takes — a generated filing is a document like
/// any other, and it must be in the vault rather than on disk beside it.
async fn file_document(
    state: &tauri::State<'_, AppState>,
    matter_id: &str,
    manifest: &TemplateManifest,
    pdf: &[u8],
) -> Result<String, String> {
    let session = rbac::require(state, Permission::CreateDeadline).await?;
    let pool = { state.db.lock().await.clone() };

    let doc_id = uuid::Uuid::new_v4().to_string();

    crate::storage::vault::encrypt_to_vault(
        &state.vault_dir,
        &state.vault_key,
        pdf,
        matter_id,
        &doc_id,
    )
    .map_err(|e| format!("Could not store the document securely: {e}"))?;

    let vault_path = state
        .vault_dir
        .join(matter_id)
        .join(format!("{doc_id}.enc"))
        .to_string_lossy()
        .to_string();

    // The version is on the filename so an associate can see which template
    // produced a document without opening the record (PRD §9.8).
    let filename = format!("{}-v{}-{}.pdf", manifest.id, manifest.version, &doc_id[..8]);

    sqlx::query(
        "INSERT INTO documents
         (id, matter_id, filename, category, mime_type, file_size_bytes, vault_path, uploaded_by,
          description)
         VALUES (?, ?, ?, 'Filing', 'application/pdf', ?, ?, ?, ?)",
    )
    .bind(&doc_id)
    .bind(matter_id)
    .bind(&filename)
    .bind(pdf.len() as i64)
    .bind(&vault_path)
    .bind(&session.user_id)
    .bind(format!("{} (template v{})", manifest.name, manifest.version))
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    sync_engine::note_change(&pool, EntityType::Document, &doc_id, Op::Upsert).await;

    Ok(doc_id)
}

// ---------------------------------------------------------------------------
// Annexures
// ---------------------------------------------------------------------------

/// The two computed keys a template must declare before it can carry annexures.
const ANNEXURE_KEYS: [&str; 2] = ["ANNEXURES_BLOCK", "ANNEXURE_PAGES"];

/// The largest single file that can be attached, and the largest total held.
///
/// Staged files sit in memory for the length of a drafting session so that the
/// live preview does not re-read and re-clean them on every keystroke. Memory
/// is the reason there is a limit at all: without one, a scanned bundle dropped
/// in by mistake would sit in the app until it was restarted.
const MAX_ANNEXURE_BYTES: usize = 25 * 1024 * 1024;
const MAX_STAGED_BYTES: usize = 100 * 1024 * 1024;

/// A file the attorney has attached, held until the document is generated.
pub struct StagedAnnexure {
    /// Cleaned bytes — metadata already stripped, type already checked.
    pub bytes: Vec<u8>,
    pub filename: String,
}

/// What Deck gets back after attaching a file.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedAnnexureInfo {
    /// Referred to by later renders instead of resending the bytes.
    pub id: String,
    /// The file's own name, shown against the row.
    pub filename: String,
    pub size_bytes: usize,
}

/// Attach a file to the document being drafted.
///
/// `path` comes from the OS file dialog, so it is somewhere the attorney can
/// already read. Keel opens it rather than Deck, because Deck does not read
/// documents off the filesystem and because the bytes would otherwise cross the
/// IPC bridge twice — once on attach and again on every preview.
///
/// The file is checked and cleaned here, at the moment it is chosen, so an
/// unusable file is a sentence in front of the attorney straight away rather
/// than a failed render ten minutes later.
#[tauri::command]
pub async fn stage_annexure(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<StagedAnnexureInfo, String> {
    rbac::require(&state, Permission::CreateDeadline).await?;

    let path = std::path::PathBuf::from(path);
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "attachment".to_owned());

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| {
            log::error!("could not read annexure {}: {e}", path.display());
            format!("\"{filename}\" could not be opened.")
        })?;

    if bytes.len() > MAX_ANNEXURE_BYTES {
        return Err(format!(
            "\"{filename}\" is larger than {} MB and cannot be attached.",
            MAX_ANNEXURE_BYTES / (1024 * 1024)
        ));
    }

    // The same three types the annexure assembler can lay out. Checked here so
    // the refusal names the file the attorney just picked.
    let format = metadata::detect_format(&bytes, "");
    let mime = match format {
        metadata::Format::Pdf => "application/pdf",
        metadata::Format::Png => "image/png",
        metadata::Format::Jpeg => "image/jpeg",
        _ => {
            return Err(format!(
                "\"{filename}\" cannot be attached. Annexures must be PDF, PNG or \
                 JPEG files."
            ))
        }
    };

    // An annexure leaves the firm inside a document served on the opposite
    // party. The author, the revision history and the GPS coordinates of the
    // phone that photographed a receipt go no further than this line.
    let bytes = metadata::clean_metadata(&bytes, mime).map_err(|e| {
        log::error!("annexure metadata strip failed for {filename}: {e:#}");
        format!("\"{filename}\" could not be cleaned of hidden data, so it has not been attached.")
    })?;

    let mut staged = state.staged_annexures.lock().await;

    let held: usize = staged.values().map(|a: &StagedAnnexure| a.bytes.len()).sum();
    if held + bytes.len() > MAX_STAGED_BYTES {
        return Err(
            "There are too many attachments on this document. Remove one before \
             attaching another."
                .to_owned(),
        );
    }

    let id = uuid::Uuid::new_v4().to_string();
    let info = StagedAnnexureInfo {
        id: id.clone(),
        filename: filename.clone(),
        size_bytes: bytes.len(),
    };
    staged.insert(id, StagedAnnexure { bytes, filename });

    Ok(info)
}

/// Let go of a file the attorney removed from the list.
#[tauri::command]
pub async fn discard_annexure(id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.staged_annexures.lock().await.remove(&id);
    Ok(())
}

/// Turn the attorney's list into marked, staged annexures.
///
/// Errors are written for an attorney and come back as `problem`, because each
/// one is something they can act on.
async fn load_annexures(
    state: &tauri::State<'_, AppState>,
    inputs: &[AnnexureInput],
) -> Result<(Vec<Annexure>, Vec<Attachment>), String> {
    if inputs.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let staged = state.staged_annexures.lock().await;
    let mut sources = Vec::with_capacity(inputs.len());

    for input in inputs {
        let Some(file) = staged.get(&input.staged_id) else {
            // The only way here is a restart, or a discard that raced a render.
            return Err(
                "One of the attachments is no longer available. Attach it again.".to_owned()
            );
        };

        sources.push(annexures::Source {
            bytes: file.bytes.clone(),
            title: input.title.clone(),
            filename: file.filename.clone(),
        });
    }

    annexures::prepare(&sources).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Turn submitted values into escaped template fields.
///
/// Everything an attorney typed is `Field::text` — escaped, without exception.
/// A `computed` field is the only thing that may carry markup, and only because
/// Keel assembled it; a value arriving from Deck under a computed key is
/// escaped like any other, so Deck cannot inject LaTeX by naming a field.
///
/// A `list` field is the one place a value from Deck becomes a `Field::raw`,
/// and the markup in it is the manifest's `itemTemplate` rather than anything
/// that crossed the bridge: `templates::assemble` escapes every cell it
/// interpolates. The rows decide how many blocks there are and what they say,
/// never what shape they take.
///
/// A field hidden by its condition, or simply absent, is bound to an empty
/// string rather than left out: `render` refuses on unfilled placeholders, and
/// an optional field the attorney skipped is not an error.
fn to_latex_fields(
    manifest: &TemplateManifest,
    values: &HashMap<String, FieldValue>,
) -> HashMap<String, Field> {
    manifest
        .fields
        .iter()
        .filter(|spec| !spec.input_only)
        .map(|spec| {
            let value = values.get(&spec.key);
            let field = match spec.kind {
                FieldKind::List { .. } => {
                    let rows = value.and_then(FieldValue::as_rows).unwrap_or(&[]);
                    templates::assemble(spec, rows)
                }
                _ => Field::text(value.map(FieldValue::as_scalar).unwrap_or("")),
            };
            (spec.key.clone(), field)
        })
        .collect()
}

/// Fields a caller inside Keel supplies as assembled LaTeX.
///
/// Kept separate from `to_latex_fields` so the escaped path stays the default
/// and the unescaped one is something you have to reach for.
#[allow(dead_code)]
pub fn with_computed(
    mut fields: HashMap<String, Field>,
    computed: HashMap<String, Field>,
) -> HashMap<String, Field> {
    fields.extend(computed);
    fields
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::templates::{FieldSpec, Row, ShownWhen};

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, FieldValue> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), FieldValue::from(*v))).collect()
    }

    fn spec(key: &str, kind: FieldKind) -> FieldSpec {
        FieldSpec {
            key: key.to_owned(),
            label: key.to_owned(),
            kind,
            required: false,
            help: None,
            autofill: None,
            shown_when: None,
            input_only: false,
        }
    }

    fn manifest(fields: Vec<FieldSpec>) -> TemplateManifest {
        TemplateManifest {
            id: "t".into(),
            name: "T".into(),
            category: "C".into(),
            version: 1,
            revised: "2026-08-10".into(),
            approved_by: None,
            authority: None,
            description: None,
            fields,
        }
    }

    #[test]
    fn attorney_text_is_escaped_on_its_way_into_the_template() {
        let m = manifest(vec![spec("NAME", FieldKind::Text { max_length: None })]);

        let fields = to_latex_fields(&m, &values(&[("NAME", "Tata & Sons")]));
        assert_eq!(fields["NAME"], Field::text("Tata & Sons"));
    }

    /// Deck must not be able to inject LaTeX by sending a value for a field the
    /// manifest calls computed.
    #[test]
    fn a_value_sent_for_a_computed_field_is_still_escaped() {
        let m = manifest(vec![spec("BLOCK", FieldKind::Computed)]);

        let fields = to_latex_fields(&m, &values(&[("BLOCK", r"\input{/etc/passwd}")]));
        assert_eq!(fields["BLOCK"], Field::text(r"\input{/etc/passwd}"));
        assert_ne!(fields["BLOCK"], Field::raw(r"\input{/etc/passwd}"));
    }

    /// Every placeholder the template will look for must be bound, or `render`
    /// refuses — including the optional ones nobody filled.
    #[test]
    fn an_absent_optional_field_is_bound_to_empty_not_omitted() {
        let m = manifest(vec![
            spec("FILLED", FieldKind::Text { max_length: None }),
            spec("SKIPPED", FieldKind::Text { max_length: None }),
        ]);

        let fields = to_latex_fields(&m, &values(&[("FILLED", "x")]));
        assert!(fields.contains_key("SKIPPED"), "an unbound key fails the render");
        assert_eq!(fields["SKIPPED"], Field::text(""));
    }

    /// An input-only field drives a computed one and has no placeholder of its
    /// own; binding it would leave a stray key the template never uses.
    #[test]
    fn input_only_fields_are_not_bound_to_the_template() {
        let mut grounds = spec("GROUNDS", FieldKind::Text { max_length: None });
        grounds.input_only = true;

        let m = manifest(vec![grounds, spec("BLOCK", FieldKind::Computed)]);

        let fields = to_latex_fields(&m, &values(&[("GROUNDS", "PriorUse")]));
        assert!(!fields.contains_key("GROUNDS"));
        assert!(fields.contains_key("BLOCK"));
    }

    // -- repeating groups ---------------------------------------------------

    fn sections_spec() -> FieldSpec {
        spec(
            "SECTIONS_BLOCK",
            FieldKind::List {
                item_fields: vec![
                    spec("HEADING", FieldKind::Text { max_length: None }),
                    spec("BODY", FieldKind::Multiline { max_length: None, max_words: None }),
                ],
                item_label: "Add a section".into(),
                item_template: "\\noticesection{{{INDEX}}}{{{HEADING}}}\n{{BODY}}\n".into(),
                min_items: None,
                max_items: None,
            },
        )
    }

    fn rows(entries: &[(&str, &str)]) -> FieldValue {
        FieldValue::Rows(
            entries
                .iter()
                .map(|(heading, body)| {
                    [
                        ("HEADING".to_owned(), (*heading).to_owned()),
                        ("BODY".to_owned(), (*body).to_owned()),
                    ]
                    .into_iter()
                    .collect::<Row>()
                })
                .collect(),
        )
    }

    /// A list is the one place a value from Deck becomes a `Field::raw`. The
    /// markup in it comes from the manifest; the attorney's words are escaped
    /// into it.
    #[test]
    fn a_lists_rows_become_one_assembled_block() {
        let m = manifest(vec![sections_spec()]);
        let submitted: HashMap<String, FieldValue> = [(
            "SECTIONS_BLOCK".to_owned(),
            rows(&[("Background", "That M/s Tata & Sons paid."), ("Demand", "That you refund.")]),
        )]
        .into_iter()
        .collect();

        let fields = to_latex_fields(&m, &submitted);
        let block = fields["SECTIONS_BLOCK"].as_str();

        assert!(block.contains("\\noticesection{1}{Background}"), "{block}");
        assert!(block.contains("\\noticesection{2}{Demand}"), "{block}");
        assert!(block.contains(r"Tata \& Sons"), "a row reached LaTeX unescaped: {block}");
    }

    /// Deck cannot get raw LaTeX into a document by sending a list field a
    /// string: what makes the block markup is the manifest's row template, and
    /// a scalar has no rows to run it over.
    #[test]
    fn a_scalar_sent_for_a_list_field_produces_nothing_rather_than_markup() {
        let m = manifest(vec![sections_spec()]);
        let submitted = values(&[("SECTIONS_BLOCK", r"\input{/etc/passwd}")]);

        let fields = to_latex_fields(&m, &submitted);
        assert_eq!(fields["SECTIONS_BLOCK"], Field::raw(String::new()));
    }

    /// `render` refuses on an unfilled placeholder, so a notice with no
    /// sections yet still has to bind `{{SECTIONS_BLOCK}}`.
    #[test]
    fn a_list_with_no_rows_still_binds_its_placeholder() {
        let m = manifest(vec![sections_spec()]);

        let fields = to_latex_fields(&m, &HashMap::new());
        assert!(fields.contains_key("SECTIONS_BLOCK"), "an unbound key fails the render");
        assert_eq!(fields["SECTIONS_BLOCK"], Field::raw(String::new()));
    }

    #[test]
    fn a_hidden_field_still_gets_bound_so_the_template_can_reference_it() {
        // The field is not required while hidden, but its placeholder is still
        // in the template and must resolve to something.
        let mut conditional = spec("FIRST_USE", FieldKind::Text { max_length: None });
        conditional.shown_when =
            Some(ShownWhen { field: "GROUNDS".into(), equals: vec!["PriorUse".into()] });

        let m = manifest(vec![conditional]);
        let fields = to_latex_fields(&m, &HashMap::new());
        assert_eq!(fields["FIRST_USE"], Field::text(""));
    }
}
