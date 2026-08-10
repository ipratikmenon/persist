// Document drafting — the commands the Smart Form Compiler (M9.8) will drive.
//
// Deck asks what templates exist, gets a schema, builds a form from it, and
// sends values back. Keel validates against the same schema and renders.
//
// The attorney never sees LaTeX. They also never see a LaTeX error: a compile
// failure comes back as a plain sentence, with the detail in the log where a
// developer can find it (PRD §9.8, "compile errors are handled silently").

use crate::rbac::{self, Permission};
use crate::services::latex::{self, CompileMode, Field};
use crate::services::templates::{self, FieldError, FieldKind, TemplateManifest};
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
    pub values: HashMap<String, String>,
    /// Draft for the live preview, Final for anything that leaves the firm.
    pub mode: CompileMode,
}

/// A render either produced a document or has something to say about why not.
///
/// Field errors and a compile failure are different things and Deck shows them
/// differently: the first belongs against an input, the second is a banner.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderResult {
    /// PDF bytes, when there are any.
    pub pdf: Option<Vec<u8>>,
    #[serde(default)]
    pub field_errors: Vec<FieldError>,
    /// Set when the document could not be produced for a reason that is not a
    /// field. Written for an attorney.
    pub problem: Option<String>,
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
        return Ok(RenderResult { pdf: None, field_errors, problem: None });
    }

    let fields = to_latex_fields(&manifest, &input.values);

    match latex::compile(&input.template_id, &fields, input.mode).await {
        Ok(pdf) => Ok(RenderResult { pdf: Some(pdf), field_errors: Vec::new(), problem: None }),
        Err(e) => {
            // The LaTeX log is for us, not for an attorney.
            log::error!("render of '{}' failed: {e:#}", input.template_id);
            Ok(RenderResult {
                pdf: None,
                field_errors: Vec::new(),
                problem: Some(match input.mode {
                    CompileMode::Draft => {
                        "Preview temporarily unavailable — your content is saved.".to_owned()
                    }
                    CompileMode::Final => {
                        "This document could not be generated. The firm's administrator \
                         has been sent the details."
                            .to_owned()
                    }
                }),
            })
        }
    }
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
/// A field hidden by its condition, or simply absent, is bound to an empty
/// string rather than left out: `render` refuses on unfilled placeholders, and
/// an optional field the attorney skipped is not an error.
fn to_latex_fields(
    manifest: &TemplateManifest,
    values: &HashMap<String, String>,
) -> HashMap<String, Field> {
    manifest
        .fields
        .iter()
        .filter(|spec| !spec.input_only)
        .map(|spec| {
            let raw = values.get(&spec.key).map(String::as_str).unwrap_or("");
            (spec.key.clone(), Field::text(raw))
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
    use crate::services::templates::{FieldSpec, ShownWhen};

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
        let values: HashMap<String, String> =
            [("NAME".to_owned(), "Tata & Sons".to_owned())].into_iter().collect();

        let fields = to_latex_fields(&m, &values);
        assert_eq!(fields["NAME"], Field::text("Tata & Sons"));
    }

    /// Deck must not be able to inject LaTeX by sending a value for a field the
    /// manifest calls computed.
    #[test]
    fn a_value_sent_for_a_computed_field_is_still_escaped() {
        let m = manifest(vec![spec("BLOCK", FieldKind::Computed)]);
        let values: HashMap<String, String> =
            [("BLOCK".to_owned(), r"\input{/etc/passwd}".to_owned())].into_iter().collect();

        let fields = to_latex_fields(&m, &values);
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
        let values: HashMap<String, String> =
            [("FILLED".to_owned(), "x".to_owned())].into_iter().collect();

        let fields = to_latex_fields(&m, &values);
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
        let values: HashMap<String, String> =
            [("GROUNDS".to_owned(), "PriorUse".to_owned())].into_iter().collect();

        let fields = to_latex_fields(&m, &values);
        assert!(!fields.contains_key("GROUNDS"));
        assert!(fields.contains_key("BLOCK"));
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
