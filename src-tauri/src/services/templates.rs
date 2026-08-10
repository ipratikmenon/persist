// Template registry — the substrate the Smart Form Compiler (M9.8) sits on.
//
// WHY THIS EXISTS
//
// Before this, generating a document meant writing Rust: `generate_invoice_pdf`
// hardcodes twenty-one field names. The PRD lists nineteen templates for the
// initial library — cover letters, examination replies, oppositions,
// affidavits, vakalatnamas. Nineteen bespoke command functions is not a plan,
// and it puts the platform admin's job (add a template when a registry changes
// its format) behind a Rust release.
//
// So a template declares its own fields, in a manifest beside it. Keel reads
// the manifest, Deck builds the form from it, and Keel validates against it
// again on submit. Adding a template is adding two files.
//
// WHY VALIDATION LIVES HERE
//
// Deck's form gives immediate feedback; that is UX. This is the enforcement.
// A malformed TM number reaches the Registry as a defective filing, so the
// check that matters is the one nearest the document.
//
// WHY NOT REGEX
//
// The PRD asks for "TM application number must be numeric and 7 digits". A
// declarative `digits(7)` produces "must be exactly 7 digits"; a regex produces
// "must match ^[0-9]{7}$". Attorneys read these.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// What a template declares about itself. Lives in `<id>.json` beside `<id>.tex`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateManifest {
    pub id: String,
    /// Shown to the attorney when choosing a document type.
    pub name: String,
    /// "IP / Trademark", "Patent", "Litigation", "Corporate", "Billing".
    pub category: String,
    /// Bumped whenever the body changes. An associate can always see which
    /// version produced a document (PRD §9.8).
    pub version: u32,
    /// ISO date the body was last revised.
    pub revised: String,
    /// The partner who approved this version. Templates are firm instruments;
    /// an unapproved one should be visibly unapproved.
    #[serde(default)]
    pub approved_by: Option<String>,
    /// The court or registry format this follows, where one applies.
    #[serde(default)]
    pub authority: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub fields: Vec<FieldSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldSpec {
    /// The `{{KEY}}` this fills. Uppercase, matching the template.
    pub key: String,
    /// The form label. Not the key — "Trade mark application number", not
    /// "TM_NUMBER".
    pub label: String,
    /// Nested rather than flattened: serde cannot combine `flatten` with
    /// `deny_unknown_fields`, and losing that here is the wrong trade. A
    /// manifest with `"requird": false` must fail loudly, not quietly produce
    /// a filing nobody validated.
    pub kind: FieldKind,
    #[serde(default = "default_true")]
    pub required: bool,
    /// Shown under the input. Where a court imposes a limit, say so here.
    #[serde(default)]
    pub help: Option<String>,
    /// Where Deck should pre-fill from, so the attorney does not retype what
    /// the matter record already knows (PRD §9.8 "auto-fill from matter").
    #[serde(default)]
    pub autofill: Option<String>,
    /// Shown only when another field has one of these values. This is the
    /// "conditional fields" behaviour the PRD asks for.
    #[serde(default)]
    pub shown_when: Option<ShownWhen>,
    /// Collected from the attorney but never printed directly — it drives a
    /// `computed` field instead.
    ///
    /// The grounds selector on an examination reply is the example: the choice
    /// decides which standard paragraph Keel assembles into `PRIOR_USE_BLOCK`,
    /// but the word "PriorUse" never appears in the document. Without this flag
    /// the drift check would demand a `{{GROUNDS}}` the template must not have.
    #[serde(default)]
    pub input_only: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShownWhen {
    pub field: String,
    pub equals: Vec<String>,
}

/// What kind of value a field holds, and the rules that go with it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FieldKind {
    Text {
        #[serde(default)]
        max_length: Option<usize>,
    },
    /// Free-text zone. `maxWords` exists because some registries impose one.
    Multiline {
        #[serde(default)]
        max_length: Option<usize>,
        #[serde(default)]
        max_words: Option<usize>,
    },
    /// ISO `YYYY-MM-DD`.
    Date {
        /// Key of another date field this one must not precede — "hearing date
        /// cannot precede filing date" (PRD §9.8).
        #[serde(default)]
        not_before: Option<String>,
    },
    /// Exactly `length` digits. TM applications are 7; patents are 6.
    Digits {
        length: usize,
    },
    Number {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    Select {
        options: Vec<SelectOption>,
    },
    Checkbox,
    /// Filled by Keel, never shown in the form — assembled tables, totals,
    /// firm identity pulled from settings.
    Computed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// One thing wrong with one field. Deck shows these against the inputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldError {
    pub key: String,
    pub label: String,
    /// Written for an attorney, not a developer.
    pub message: String,
}

/// Check submitted values against the manifest.
///
/// Returns every problem, not the first: an attorney fixing one field at a time
/// through five round trips is worse than seeing all five at once.
///
/// A field hidden by its `shownWhen` condition is not required, and a value
/// supplied for it is ignored — otherwise choosing "no likelihood of confusion"
/// would still demand the first-use date that only "prior use" needs.
pub fn validate(
    manifest: &TemplateManifest,
    values: &HashMap<String, String>,
) -> Vec<FieldError> {
    let mut errors = Vec::new();

    for spec in &manifest.fields {
        if matches!(spec.kind, FieldKind::Computed) {
            continue;
        }
        if !is_visible(spec, manifest, values) {
            continue;
        }

        let raw = values.get(&spec.key).map(|s| s.trim()).unwrap_or("");

        if raw.is_empty() {
            if spec.required {
                errors.push(FieldError {
                    key: spec.key.clone(),
                    label: spec.label.clone(),
                    message: format!("{} is required.", spec.label),
                });
            }
            // An empty optional field has nothing left to check.
            continue;
        }

        if let Some(message) = check_value(spec, raw, values) {
            errors.push(FieldError {
                key: spec.key.clone(),
                label: spec.label.clone(),
                message,
            });
        }
    }

    errors
}

/// Is this field shown, given what has been filled in so far?
fn is_visible(
    spec: &FieldSpec,
    manifest: &TemplateManifest,
    values: &HashMap<String, String>,
) -> bool {
    let Some(condition) = &spec.shown_when else {
        return true;
    };

    // A condition pointing at a field that is itself hidden is not satisfied —
    // otherwise a two-deep chain would resurrect a field its parent hid.
    if let Some(parent) = manifest.fields.iter().find(|f| f.key == condition.field) {
        if !is_visible(parent, manifest, values) {
            return false;
        }
    }

    values
        .get(&condition.field)
        .map(|v| condition.equals.contains(v))
        .unwrap_or(false)
}

fn check_value(
    spec: &FieldSpec,
    raw: &str,
    values: &HashMap<String, String>,
) -> Option<String> {
    match &spec.kind {
        FieldKind::Text { max_length } => {
            let limit = (*max_length)?;
            (raw.chars().count() > limit).then(|| {
                format!("{} must be {limit} characters or fewer.", spec.label)
            })
        }

        FieldKind::Multiline { max_length, max_words } => {
            if let Some(limit) = max_length {
                if raw.chars().count() > *limit {
                    return Some(format!(
                        "{} must be {limit} characters or fewer (currently {}).",
                        spec.label,
                        raw.chars().count()
                    ));
                }
            }
            if let Some(limit) = max_words {
                let words = raw.split_whitespace().count();
                if words > *limit {
                    return Some(format!(
                        "{} must be {limit} words or fewer (currently {words}).",
                        spec.label
                    ));
                }
            }
            None
        }

        FieldKind::Date { not_before } => {
            if parse_iso_date(raw).is_none() {
                return Some(format!("{} must be a date, as YYYY-MM-DD.", spec.label));
            }
            let other_key = not_before.as_ref()?;
            let other_raw = values.get(other_key)?.trim();
            let (this, other) = (parse_iso_date(raw)?, parse_iso_date(other_raw)?);
            (this < other).then(|| {
                format!("{} cannot be earlier than {other_raw}.", spec.label)
            })
        }

        FieldKind::Digits { length } => {
            let all_digits = raw.chars().all(|c| c.is_ascii_digit());
            (!all_digits || raw.len() != *length).then(|| {
                format!("{} must be exactly {length} digits.", spec.label)
            })
        }

        FieldKind::Number { min, max } => {
            let Ok(parsed) = raw.parse::<f64>() else {
                return Some(format!("{} must be a number.", spec.label));
            };
            if let Some(m) = min {
                if parsed < *m {
                    return Some(format!("{} cannot be less than {m}.", spec.label));
                }
            }
            if let Some(m) = max {
                if parsed > *m {
                    return Some(format!("{} cannot be more than {m}.", spec.label));
                }
            }
            None
        }

        FieldKind::Select { options } => {
            let known = options.iter().any(|o| o.value == raw);
            (!known).then(|| format!("{} is not one of the available choices.", spec.label))
        }

        FieldKind::Checkbox => {
            matches!(raw, "true" | "false")
                .then_some(())
                .map_or(Some(format!("{} must be ticked or unticked.", spec.label)), |_| None)
        }

        FieldKind::Computed => None,
    }
}

/// Parse `YYYY-MM-DD` into a comparable tuple.
///
/// Comparing the strings would nearly work and fail on a malformed one, so the
/// parse is real. `chrono` would also do, but this keeps the validator free of
/// timezone semantics it has no use for — these are calendar dates on a filing.
fn parse_iso_date(raw: &str) -> Option<(i32, u32, u32)> {
    let mut parts = raw.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Load every manifest in the templates directory, newest-first by category.
///
/// A template whose manifest is malformed is skipped with a loud log rather
/// than taking the whole library down — one bad file must not stop an attorney
/// filing something else today.
pub fn list(templates_dir: &Path) -> Result<Vec<TemplateManifest>> {
    let mut found = Vec::new();

    let entries = std::fs::read_dir(templates_dir)
        .with_context(|| format!("cannot read templates directory {}", templates_dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        match load_manifest(&path) {
            Ok(manifest) => found.push(manifest),
            Err(e) => log::error!("ignoring malformed template manifest {}: {e:#}", path.display()),
        }
    }

    found.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
    Ok(found)
}

pub fn get(templates_dir: &Path, id: &str) -> Result<TemplateManifest> {
    let path = templates_dir.join(format!("{id}.json"));
    load_manifest(&path).with_context(|| format!("no template manifest for '{id}'"))
}

fn load_manifest(path: &Path) -> Result<TemplateManifest> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    let manifest: TemplateManifest = serde_json::from_str(&text)
        .with_context(|| format!("cannot parse {}", path.display()))?;

    // The id is what maps the manifest to its .tex; a mismatch would silently
    // render the wrong document.
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    if manifest.id != stem {
        bail!("manifest id '{}' does not match filename '{stem}'", manifest.id);
    }

    Ok(manifest)
}

/// Check a manifest against the template it describes.
///
/// Drift between the two is the failure this whole module exists to prevent: a
/// field in the manifest that the template ignores is a form asking for
/// something it will not print, and a placeholder in the template that the
/// manifest omits fails the render at the worst moment.
pub fn check_against_template(manifest: &TemplateManifest, tex_source: &str) -> Vec<String> {
    let declared: std::collections::HashSet<&str> =
        manifest.fields.iter().map(|f| f.key.as_str()).collect();
    let used = super::latex::placeholders_in(tex_source);

    let mut problems = Vec::new();

    for key in &used {
        if !declared.contains(key.as_str()) {
            problems.push(format!(
                "template uses {{{{{key}}}}} but the manifest does not declare it"
            ));
        }
    }
    for spec in &manifest.fields {
        // An input-only field is deliberately not printed; it feeds a computed one.
        if spec.input_only || used.contains(&spec.key) {
            continue;
        }
        problems.push(format!(
            "manifest declares {} but the template never uses it              (mark it inputOnly if it drives a computed field)",
            spec.key
        ));
    }

    problems.sort();
    problems
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(key: &str, kind: FieldKind) -> FieldSpec {
        FieldSpec {
            key: key.to_owned(),
            label: key.replace('_', " ").to_lowercase(),
            kind,
            required: true,
            help: None,
            autofill: None,
            shown_when: None,
            input_only: false,
        }
    }

    fn manifest(fields: Vec<FieldSpec>) -> TemplateManifest {
        TemplateManifest {
            id: "t".into(),
            name: "Test".into(),
            category: "Test".into(),
            version: 1,
            revised: "2026-08-10".into(),
            approved_by: None,
            authority: None,
            description: None,
            fields,
        }
    }

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
    }

    // -- required ----------------------------------------------------------

    #[test]
    fn a_missing_required_field_is_reported_with_its_label() {
        let m = manifest(vec![spec("CLIENT_NAME", FieldKind::Text { max_length: None })]);
        let errors = validate(&m, &HashMap::new());

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].key, "CLIENT_NAME");
        assert!(errors[0].message.contains("client name"), "{:?}", errors[0]);
    }

    #[test]
    fn whitespace_does_not_satisfy_a_required_field() {
        let m = manifest(vec![spec("CLIENT_NAME", FieldKind::Text { max_length: None })]);
        assert_eq!(validate(&m, &values(&[("CLIENT_NAME", "   ")])).len(), 1);
    }

    #[test]
    fn every_problem_is_reported_not_just_the_first() {
        // Five round trips to fix five fields is not a form, it is a punishment.
        let m = manifest(vec![
            spec("A", FieldKind::Text { max_length: None }),
            spec("B", FieldKind::Text { max_length: None }),
            spec("C", FieldKind::Text { max_length: None }),
        ]);
        assert_eq!(validate(&m, &HashMap::new()).len(), 3);
    }

    #[test]
    fn computed_fields_are_never_asked_of_the_attorney() {
        let m = manifest(vec![spec("LINE_ITEMS_TABLE", FieldKind::Computed)]);
        assert!(validate(&m, &HashMap::new()).is_empty());
    }

    // -- digits ------------------------------------------------------------

    #[test]
    fn a_tm_number_must_be_seven_digits() {
        let m = manifest(vec![spec("TM_NUMBER", FieldKind::Digits { length: 7 })]);

        assert!(validate(&m, &values(&[("TM_NUMBER", "1234567")])).is_empty());

        for bad in ["123456", "12345678", "12345a7", "12 34567"] {
            let errors = validate(&m, &values(&[("TM_NUMBER", bad)]));
            assert_eq!(errors.len(), 1, "{bad} should have been rejected");
            assert!(errors[0].message.contains("exactly 7 digits"), "{:?}", errors[0]);
        }
    }

    // -- dates -------------------------------------------------------------

    #[test]
    fn a_date_must_be_a_real_iso_date() {
        let m = manifest(vec![spec("FILING_DATE", FieldKind::Date { not_before: None })]);

        assert!(validate(&m, &values(&[("FILING_DATE", "2026-08-10")])).is_empty());
        for bad in ["10-08-2026", "2026-13-01", "2026-08-32", "next Tuesday", "2026-08"] {
            assert_eq!(
                validate(&m, &values(&[("FILING_DATE", bad)])).len(),
                1,
                "{bad} should have been rejected"
            );
        }
    }

    #[test]
    fn a_hearing_cannot_precede_the_filing_it_belongs_to() {
        let m = manifest(vec![
            spec("FILING_DATE", FieldKind::Date { not_before: None }),
            spec("HEARING_DATE", FieldKind::Date { not_before: Some("FILING_DATE".into()) }),
        ]);

        let ok = values(&[("FILING_DATE", "2026-08-01"), ("HEARING_DATE", "2026-09-01")]);
        assert!(validate(&m, &ok).is_empty());

        let backwards = values(&[("FILING_DATE", "2026-09-01"), ("HEARING_DATE", "2026-08-01")]);
        let errors = validate(&m, &backwards);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].key, "HEARING_DATE");
    }

    #[test]
    fn the_same_day_is_allowed() {
        let m = manifest(vec![
            spec("FILING_DATE", FieldKind::Date { not_before: None }),
            spec("HEARING_DATE", FieldKind::Date { not_before: Some("FILING_DATE".into()) }),
        ]);
        let same = values(&[("FILING_DATE", "2026-08-01"), ("HEARING_DATE", "2026-08-01")]);
        assert!(validate(&m, &same).is_empty());
    }

    // -- conditional fields ------------------------------------------------

    #[test]
    fn a_hidden_field_is_not_required() {
        // "Grounds: prior use" reveals a first-use date. Choosing any other
        // ground must not demand it.
        let mut first_use = spec("FIRST_USE_DATE", FieldKind::Date { not_before: None });
        first_use.shown_when = Some(ShownWhen {
            field: "GROUNDS".into(),
            equals: vec!["PriorUse".into()],
        });

        let m = manifest(vec![
            spec("GROUNDS", FieldKind::Select {
                options: vec![
                    SelectOption { value: "PriorUse".into(), label: "Prior use".into() },
                    SelectOption { value: "NoConfusion".into(), label: "No confusion".into() },
                ],
            }),
            first_use,
        ]);

        assert!(validate(&m, &values(&[("GROUNDS", "NoConfusion")])).is_empty());

        let errors = validate(&m, &values(&[("GROUNDS", "PriorUse")]));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].key, "FIRST_USE_DATE");
    }

    #[test]
    fn a_hidden_fields_own_dependents_stay_hidden() {
        // A condition on a hidden field must not resurrect its children.
        let mut middle = spec("MIDDLE", FieldKind::Text { max_length: None });
        middle.shown_when = Some(ShownWhen { field: "TOP".into(), equals: vec!["yes".into()] });

        let mut leaf = spec("LEAF", FieldKind::Text { max_length: None });
        leaf.shown_when = Some(ShownWhen { field: "MIDDLE".into(), equals: vec!["x".into()] });

        let m = manifest(vec![
            spec("TOP", FieldKind::Text { max_length: None }),
            middle,
            leaf,
        ]);

        // TOP is "no", so MIDDLE is hidden. A stale MIDDLE value must not make
        // LEAF required.
        let stale = values(&[("TOP", "no"), ("MIDDLE", "x")]);
        assert!(validate(&m, &stale).is_empty(), "{:?}", validate(&m, &stale));
    }

    // -- limits ------------------------------------------------------------

    #[test]
    fn a_word_limit_is_enforced_where_a_registry_imposes_one() {
        let m = manifest(vec![spec(
            "STATEMENT",
            FieldKind::Multiline { max_length: None, max_words: Some(5) },
        )]);

        assert!(validate(&m, &values(&[("STATEMENT", "one two three four five")])).is_empty());

        let errors = validate(&m, &values(&[("STATEMENT", "one two three four five six")]));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("currently 6"), "{:?}", errors[0]);
    }

    #[test]
    fn a_select_refuses_a_value_that_is_not_on_the_list() {
        let m = manifest(vec![spec("GST_TYPE", FieldKind::Select {
            options: vec![
                SelectOption { value: "Intra".into(), label: "Intra-state".into() },
                SelectOption { value: "Inter".into(), label: "Inter-state".into() },
            ],
        })]);

        assert!(validate(&m, &values(&[("GST_TYPE", "Intra")])).is_empty());
        assert_eq!(validate(&m, &values(&[("GST_TYPE", "Elsewhere")])).len(), 1);
    }

    // -- manifest/template drift -------------------------------------------

    #[test]
    fn drift_between_a_manifest_and_its_template_is_reported_both_ways() {
        let m = manifest(vec![
            spec("USED", FieldKind::Text { max_length: None }),
            spec("DECLARED_BUT_UNUSED", FieldKind::Text { max_length: None }),
        ]);
        let tex = "Hello {{USED}} and {{UNDECLARED}}";

        let problems = check_against_template(&m, tex);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("UNDECLARED")), "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("DECLARED_BUT_UNUSED")), "{problems:?}");
    }

    #[test]
    fn a_manifest_matching_its_template_reports_nothing() {
        let m = manifest(vec![spec("A", FieldKind::Text { max_length: None })]);
        assert!(check_against_template(&m, "x {{A}} y").is_empty());
    }
}

// ---------------------------------------------------------------------------
// The shipped library
//
// These run against the real templates directory. A manifest and its template
// drifting apart is the failure this module exists to prevent, and it is not a
// hypothetical: it happens the first time someone edits one and not the other.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod library_tests {
    use super::*;

    fn templates_dir() -> PathBuf {
        for candidate in ["storage/templates", "src-tauri/storage/templates"] {
            let p = PathBuf::from(candidate);
            if p.is_dir() {
                return p;
            }
        }
        panic!("templates directory not found from {:?}", std::env::current_dir());
    }

    #[test]
    fn every_shipped_template_has_a_manifest_that_matches_it() {
        let dir = templates_dir();
        let manifests = list(&dir).expect("the library must load");
        assert!(!manifests.is_empty(), "no templates found in {}", dir.display());

        for manifest in &manifests {
            let tex_path = dir.join(format!("{}.tex", manifest.id));
            let source = std::fs::read_to_string(&tex_path)
                .unwrap_or_else(|e| panic!("{} has a manifest but no template: {e}", manifest.id));

            let problems = check_against_template(manifest, &source);
            assert!(
                problems.is_empty(),
                "{} drifted from its manifest:\n  {}",
                manifest.id,
                problems.join("\n  ")
            );
        }
    }

    /// A .tex with no manifest is invisible to the form compiler — it would
    /// simply never appear in the template picker, which is a silent failure.
    #[test]
    fn every_shipped_template_is_declared() {
        let dir = templates_dir();
        let declared: std::collections::HashSet<String> =
            list(&dir).unwrap().into_iter().map(|m| m.id).collect();

        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("tex") {
                continue;
            }
            let stem = path.file_stem().unwrap().to_str().unwrap().to_owned();
            assert!(
                declared.contains(&stem),
                "{stem}.tex has no {stem}.json manifest, so it can never be offered"
            );
        }
    }

    #[test]
    fn the_invoice_manifest_asks_the_attorney_for_nothing() {
        // Every invoice field comes from the invoice record. If one ever becomes
        // attorney-entered that is a deliberate decision, not a drift.
        let manifest = get(&templates_dir(), "invoice").unwrap();
        for field in &manifest.fields {
            assert!(
                matches!(field.kind, FieldKind::Computed),
                "{} is not computed — the invoice form would now have inputs",
                field.key
            );
        }
    }
}
