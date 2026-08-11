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
//
// WHY A FIELD CAN BE A LIST
//
// A demand notice carries N numbered sections and N payment tranches; the one
// the firm supplied had ten and seven. Neither count is knowable when the
// template is written, so declaring TRANCHE_1..TRANCHE_7 is a guess at the
// eighth. A `list` field declares the shape of one row instead, and Keel
// assembles however many rows the attorney entered into a single block — so the
// template still carries one placeholder and does not care how many there are.

use crate::services::latex::{self, Field};
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
///
/// `rename_all` renames the variants — `Multiline` to `"multiline"`. The fields
/// *inside* a variant need `rename_all_fields`, and without it every
/// `"maxWords"` in the shipped library matched nothing, fell through to
/// `#[serde(default)]` and became `None`: a clean parse, no warning, and a
/// registry word limit that was never once enforced. `deny_unknown_fields`
/// cannot be added here — serde refuses it on an internally tagged enum — so
/// `a_limit_written_in_a_manifest_is_actually_read` stands in for it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
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
    /// A repeating group: the attorney adds as many rows as the matter needs.
    ///
    /// The template sees one placeholder. Keel renders `item_template` once per
    /// row and concatenates the results, so the number of numbered sections in
    /// a notice is a thing the attorney decides rather than a thing the
    /// template guessed.
    List {
        /// The shape of one row. These keys are scoped to the row: they are not
        /// `{{KEY}}`s of the template, and two different lists may both have a
        /// `HEADING`.
        item_fields: Vec<FieldSpec>,
        /// The add-a-row button — "Add a section", "Add a payment".
        item_label: String,
        /// How one row is set. `{{INDEX}}` is the row's 1-based number, and
        /// every other placeholder is an item field key.
        ///
        /// This is markup in a manifest, which is deliberate: the manifest and
        /// the .tex are one template in two files, ship together, and are firm
        /// work product at the same trust level. What is *not* trusted is the
        /// row's values — every one of them goes through `latex::escape` on its
        /// way in, so a heading an attorney typed cannot become markup.
        item_template: String,
        /// Fewest rows the document makes sense with. A notice with no numbered
        /// sections is a letterhead and a signature.
        #[serde(default)]
        min_items: Option<usize>,
        #[serde(default)]
        max_items: Option<usize>,
    },
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

/// One row of a `list` field: the row's own field keys, and what was typed
/// into them.
pub type Row = HashMap<String, String>;

/// What the attorney submitted for one field.
///
/// Untagged rather than tagged, so the wire format is the obvious one — a
/// string for a scalar field, an array of objects for a list — and Deck sends
/// `Record<string, string | Row[]>` without a wrapper object per value.
///
/// A value of the wrong shape is not a deserialisation failure that takes the
/// whole render down with a serde message no attorney can act on; it becomes a
/// field error, next to the field, like every other validation problem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum FieldValue {
    Scalar(String),
    Rows(Vec<Row>),
}

impl FieldValue {
    /// The scalar text, or `""` for a list. A list bound where a scalar was
    /// expected is caught by `validate`; this only decides what gets printed.
    pub fn as_scalar(&self) -> &str {
        match self {
            FieldValue::Scalar(text) => text,
            FieldValue::Rows(_) => "",
        }
    }

    /// The rows, or `None` if this arrived as a scalar.
    pub fn as_rows(&self) -> Option<&[Row]> {
        match self {
            FieldValue::Rows(rows) => Some(rows),
            FieldValue::Scalar(_) => None,
        }
    }
}

impl From<&str> for FieldValue {
    fn from(text: &str) -> Self {
        FieldValue::Scalar(text.to_owned())
    }
}

impl From<String> for FieldValue {
    fn from(text: String) -> Self {
        FieldValue::Scalar(text)
    }
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
    values: &HashMap<String, FieldValue>,
) -> Vec<FieldError> {
    let mut errors = Vec::new();

    for spec in &manifest.fields {
        if matches!(spec.kind, FieldKind::Computed) {
            continue;
        }
        if !is_visible(spec, manifest, values) {
            continue;
        }

        if let FieldKind::List { .. } = spec.kind {
            check_list(spec, values.get(&spec.key), &mut errors);
            continue;
        }

        let raw = values.get(&spec.key).map(|v| v.as_scalar().trim()).unwrap_or("");

        if raw.is_empty() {
            // A list sent for a scalar field reads as empty, and "is required"
            // would send the attorney looking for something they filled in.
            if values.get(&spec.key).is_some_and(|v| v.as_rows().is_some()) {
                errors.push(FieldError {
                    key: spec.key.clone(),
                    label: spec.label.clone(),
                    message: format!("{} takes a single value, not a list.", spec.label),
                });
                continue;
            }
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

        if let Some(message) = check_value(spec, raw, &|key| scalar_of(values, key)) {
            errors.push(FieldError {
                key: spec.key.clone(),
                label: spec.label.clone(),
                message,
            });
        }
    }

    errors
}

/// Check one repeating group: how many rows, and then every row.
///
/// A row's errors are keyed so Deck can put each one against the input it
/// belongs to, and worded so an attorney reading the message alone still knows
/// which row is wrong — "Numbered sections, row 3: Heading is required."
/// Reporting "Heading is required" against a form with ten headings is not a
/// message, it is a search.
fn check_list(spec: &FieldSpec, value: Option<&FieldValue>, errors: &mut Vec<FieldError>) {
    let FieldKind::List { item_fields, min_items, max_items, .. } = &spec.kind else {
        return;
    };

    let rows: &[Row] = match value {
        Some(value) => match value.as_rows() {
            Some(rows) => rows,
            None => {
                errors.push(FieldError {
                    key: spec.key.clone(),
                    label: spec.label.clone(),
                    message: format!("{} must be a list of rows.", spec.label),
                });
                return;
            }
        },
        // Nothing sent at all is an empty list, not a broken one — the attorney
        // has simply not added a row yet.
        None => &[],
    };

    // `required` is the floor `minItems` refines: a required list needs a row.
    let minimum = min_items.unwrap_or(if spec.required { 1 } else { 0 });
    if rows.len() < minimum {
        errors.push(FieldError {
            key: spec.key.clone(),
            label: spec.label.clone(),
            message: match minimum {
                1 => format!("{} needs at least one entry.", spec.label),
                n => format!("{} needs at least {n} entries.", spec.label),
            },
        });
    }
    if let Some(limit) = max_items {
        if rows.len() > *limit {
            errors.push(FieldError {
                key: spec.key.clone(),
                label: spec.label.clone(),
                message: format!(
                    "{} can have at most {limit} entries (currently {}).",
                    spec.label,
                    rows.len()
                ),
            });
        }
    }

    for (index, row) in rows.iter().enumerate() {
        for item in item_fields {
            let raw = row.get(&item.key).map(|s| s.trim()).unwrap_or("");

            let message = if raw.is_empty() {
                if !item.required {
                    continue;
                }
                format!("{} is required.", item.label)
            } else {
                // A row is checked against itself: a `notBefore` inside a
                // payment schedule means the date in that row, not a date in
                // some other row.
                match check_value(item, raw, &|key| row.get(key).map(String::as_str).unwrap_or("")) {
                    Some(message) => message,
                    None => continue,
                }
            };

            errors.push(FieldError {
                key: row_error_key(&spec.key, index, &item.key),
                label: item.label.clone(),
                message: format!("{}, row {}: {message}", spec.label, index + 1),
            });
        }
    }
}

/// How a row's field error is keyed — `SECTIONS_BLOCK[2].HEADING`.
///
/// Deck mirrors this in FormField.tsx to put the message against the right
/// input. It is one shape in two places, so it is a function here rather than
/// a `format!` at each site.
pub fn row_error_key(list_key: &str, index: usize, item_key: &str) -> String {
    format!("{list_key}[{index}].{item_key}")
}

/// The scalar text of another field, for a cross-field rule like `notBefore`.
fn scalar_of<'a>(values: &'a HashMap<String, FieldValue>, key: &str) -> &'a str {
    values.get(key).map(FieldValue::as_scalar).unwrap_or("")
}

/// Is this field shown, given what has been filled in so far?
fn is_visible(
    spec: &FieldSpec,
    manifest: &TemplateManifest,
    values: &HashMap<String, FieldValue>,
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
        .map(|v| condition.equals.iter().any(|option| option == v.as_scalar()))
        .unwrap_or(false)
}

/// `other` resolves a sibling value for the cross-field rules. At the top level
/// that is another field of the form; inside a list it is another cell of the
/// same row.
fn check_value<'v>(
    spec: &FieldSpec,
    raw: &str,
    other: &dyn Fn(&str) -> &'v str,
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
            let other_raw = other(other_key).trim();
            let (this, earliest) = (parse_iso_date(raw)?, parse_iso_date(other_raw)?);
            (this < earliest).then(|| {
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

        // Handled by `check_list`, which has the rows. A list nested inside a
        // list is refused by `check_against_template`, so this is unreachable
        // for any manifest the library ships.
        FieldKind::List { .. } => None,

        FieldKind::Computed => None,
    }
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Turn the rows of a list field into the single block the template expects.
///
/// Every value interpolated here goes through `latex::escape` first — the row
/// is the attorney's prose and reaches a LaTeX document, so this is the same
/// rule as everywhere else, applied at the one place a `Field::raw` is built
/// out of typed input. Escaping also closes the substitution off: an escaped
/// value cannot contain `{{`, so a row value that reads `{{CLIENT_NAME}}`
/// prints those characters rather than becoming another field's value.
///
/// `{{INDEX}}` is generated from the position and never read from the row, for
/// the same reason an annexure mark is not typed: reordering must renumber.
///
/// An empty list assembles to an empty block rather than to nothing, because
/// `latex::render` refuses to compile a template with an unfilled placeholder —
/// a notice with no numbered sections must still bind `{{SECTIONS_BLOCK}}`.
pub fn assemble(spec: &FieldSpec, rows: &[Row]) -> Field {
    let FieldKind::List { item_fields, item_template, .. } = &spec.kind else {
        return Field::raw(String::new());
    };

    let mut out = String::new();
    for (index, row) in rows.iter().enumerate() {
        let mut block = item_template.replace("{{INDEX}}", &(index + 1).to_string());
        for item in item_fields {
            let value = row.get(&item.key).map(String::as_str).unwrap_or("");
            block = block.replace(&format!("{{{{{}}}}}", item.key), &latex::escape(value));
        }
        // A key in the row that the manifest does not declare is dropped here
        // rather than printed: Deck cannot add a column to a document by
        // inventing one.
        out.push_str(&block);
    }
    Field::raw(out)
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

    for spec in &manifest.fields {
        problems.extend(check_list_shape(spec));
    }

    problems.sort();
    problems
}

/// A list field's own internal drift: the row template against the row's
/// fields.
///
/// The same failure one level down. A `{{AMOUNT}}` in the row template that no
/// item field declares would reach `latex::render` as an unfilled placeholder
/// and refuse the compile of a notice at the moment it was wanted; an item
/// field the row template never uses is a question the form asks and the
/// document ignores.
fn check_list_shape(spec: &FieldSpec) -> Vec<String> {
    let FieldKind::List { item_fields, item_template, min_items, max_items, .. } = &spec.kind
    else {
        return Vec::new();
    };

    let mut problems = Vec::new();
    let key = &spec.key;

    if item_fields.is_empty() {
        problems.push(format!("{key} is a list with no itemFields, so a row has nothing in it"));
    }

    let used = latex::placeholders_in(item_template);
    let declared: std::collections::HashSet<&str> =
        item_fields.iter().map(|f| f.key.as_str()).collect();

    for placeholder in &used {
        // INDEX is Keel's, not a field.
        if placeholder != "INDEX" && !declared.contains(placeholder.as_str()) {
            problems.push(format!(
                "{key}'s itemTemplate uses {{{{{placeholder}}}}} but no item field declares it"
            ));
        }
    }

    for item in item_fields {
        if !used.contains(&item.key) {
            problems.push(format!(
                "{key} collects {} but its itemTemplate never prints it",
                item.key
            ));
        }
        // A row is a flat set of inputs. Nesting a list inside one, or a
        // computed cell, or per-cell conditional visibility, are all things the
        // form and the assembler would each have to grow a second time; none of
        // the firm's documents needs them, and a manifest that quietly declared
        // one would produce a row that is not what it says.
        match item.kind {
            FieldKind::List { .. } => {
                problems.push(format!("{key}.{} is a list inside a list", item.key))
            }
            FieldKind::Computed => problems.push(format!(
                "{key}.{} is computed — a row is filled in by the attorney",
                item.key
            )),
            _ => {}
        }
        if item.shown_when.is_some() {
            problems.push(format!(
                "{key}.{} has shownWhen — conditional visibility applies to the \
                 list as a whole, not to a cell inside a row",
                item.key
            ));
        }
        if item.input_only {
            problems.push(format!(
                "{key}.{} is inputOnly, which means nothing inside a row — a row \
                 is only ever printed through the itemTemplate",
                item.key
            ));
        }
    }

    if let (Some(min), Some(max)) = (min_items, max_items) {
        if min > max {
            problems.push(format!("{key} has minItems {min} above maxItems {max}"));
        }
    }

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

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, FieldValue> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), FieldValue::from(*v))).collect()
    }

    /// One list field, with two item fields and a row template that uses both.
    fn list_spec(key: &str, min: Option<usize>, max: Option<usize>) -> FieldSpec {
        let mut heading = spec("HEADING", FieldKind::Text { max_length: Some(160) });
        heading.label = "Heading".into();
        let mut body = spec("BODY", FieldKind::Multiline { max_length: None, max_words: None });
        body.label = "Text".into();

        let mut list = spec(
            key,
            FieldKind::List {
                item_fields: vec![heading, body],
                item_label: "Add a section".into(),
                item_template:
                    "\\noticesection{{{INDEX}}}{{{HEADING}}}\n\\begin{noticebody}\n{{BODY}}\n\\end{noticebody}\n"
                        .into(),
                min_items: min,
                max_items: max,
            },
        );
        list.label = "Numbered sections".into();
        list
    }

    fn rows(entries: &[&[(&str, &str)]]) -> FieldValue {
        FieldValue::Rows(
            entries
                .iter()
                .map(|row| {
                    row.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect::<Row>()
                })
                .collect(),
        )
    }

    fn with_rows(key: &str, entries: &[&[(&str, &str)]]) -> HashMap<String, FieldValue> {
        [(key.to_owned(), rows(entries))].into_iter().collect()
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

    // -- repeating groups: how many rows ------------------------------------

    #[test]
    fn a_required_list_needs_at_least_one_row() {
        // A notice with no numbered sections is a letterhead and a signature.
        let m = manifest(vec![list_spec("SECTIONS", None, None)]);

        let errors = validate(&m, &HashMap::new());
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].key, "SECTIONS");
        assert!(errors[0].message.contains("at least one"), "{:?}", errors[0]);

        let one = with_rows("SECTIONS", &[&[("HEADING", "Background"), ("BODY", "That...")]]);
        assert!(validate(&m, &one).is_empty(), "{:?}", validate(&m, &one));
    }

    #[test]
    fn an_optional_list_is_content_with_no_rows_at_all() {
        let mut list = list_spec("PAYMENTS", None, None);
        list.required = false;

        let m = manifest(vec![list]);
        assert!(validate(&m, &HashMap::new()).is_empty());
        assert!(validate(&m, &with_rows("PAYMENTS", &[])).is_empty());
    }

    #[test]
    fn min_and_max_items_are_both_enforced() {
        let m = manifest(vec![list_spec("SECTIONS", Some(2), Some(3))]);
        let row: &[(&str, &str)] = &[("HEADING", "H"), ("BODY", "B")];

        let one = with_rows("SECTIONS", &[row]);
        let errors = validate(&m, &one);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].message.contains("at least 2"), "{:?}", errors[0]);

        assert!(validate(&m, &with_rows("SECTIONS", &[row, row])).is_empty());
        assert!(validate(&m, &with_rows("SECTIONS", &[row, row, row])).is_empty());

        let four = with_rows("SECTIONS", &[row, row, row, row]);
        let errors = validate(&m, &four);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].message.contains("at most 3"), "{:?}", errors[0]);
        assert!(errors[0].message.contains("currently 4"), "{:?}", errors[0]);
    }

    // -- repeating groups: what is in a row ---------------------------------

    /// The error has to say which row. "Heading is required" against a form
    /// with ten headings is not a message, it is a search.
    #[test]
    fn a_row_error_names_the_row_it_belongs_to() {
        let m = manifest(vec![list_spec("SECTIONS", None, None)]);
        let submitted = with_rows(
            "SECTIONS",
            &[
                &[("HEADING", "Background"), ("BODY", "That the parties met.")],
                &[("HEADING", "Demand"), ("BODY", "That you shall pay.")],
                &[("BODY", "That you have not paid.")],
            ],
        );

        let errors = validate(&m, &submitted);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].key, "SECTIONS[2].HEADING");
        assert_eq!(errors[0].key, row_error_key("SECTIONS", 2, "HEADING"));
        assert!(errors[0].message.contains("row 3"), "{:?}", errors[0]);
        assert!(errors[0].message.contains("Heading"), "{:?}", errors[0]);
    }

    #[test]
    fn every_bad_row_is_reported_not_just_the_first() {
        let m = manifest(vec![list_spec("SECTIONS", None, None)]);
        let submitted = with_rows(
            "SECTIONS",
            &[&[("HEADING", "One")], &[("BODY", "Two")], &[("HEADING", ""), ("BODY", "")]],
        );

        // Row 1 has no body, row 2 no heading, row 3 neither.
        let errors = validate(&m, &submitted);
        let keys: Vec<&str> = errors.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "SECTIONS[0].BODY",
                "SECTIONS[1].HEADING",
                "SECTIONS[2].HEADING",
                "SECTIONS[2].BODY"
            ]
        );
    }

    /// A cell is validated by the same rules as any other field of that kind.
    #[test]
    fn an_item_fields_own_validation_applies_inside_a_row() {
        let mut amount = spec("AMOUNT", FieldKind::Number { min: Some(1.0), max: None });
        amount.label = "Amount".into();
        let mut paid = spec("PAID_ON", FieldKind::Date { not_before: None });
        paid.label = "Paid on".into();

        let mut list = spec(
            "PAYMENTS",
            FieldKind::List {
                item_fields: vec![amount, paid],
                item_label: "Add a payment".into(),
                item_template: "{{INDEX}} & {{AMOUNT}} & {{PAID_ON}}".into(),
                min_items: None,
                max_items: None,
            },
        );
        list.label = "Payments".into();

        let m = manifest(vec![list]);
        let submitted = with_rows(
            "PAYMENTS",
            &[
                &[("AMOUNT", "50000"), ("PAID_ON", "2026-02-11")],
                &[("AMOUNT", "not a number"), ("PAID_ON", "11/02/2026")],
            ],
        );

        let errors = validate(&m, &submitted);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].message.contains("must be a number"), "{:?}", errors[0]);
        assert!(errors[1].message.contains("YYYY-MM-DD"), "{:?}", errors[1]);
        assert!(errors.iter().all(|e| e.message.contains("row 2")), "{errors:?}");
    }

    /// A `notBefore` inside a row means the date in that row. Reading it from
    /// the top-level form would compare a payment against a filing date that
    /// has nothing to do with it.
    #[test]
    fn a_cross_field_rule_inside_a_row_reads_that_rows_own_cells() {
        let mut from = spec("FROM", FieldKind::Date { not_before: None });
        from.label = "From".into();
        let mut to = spec("TO", FieldKind::Date { not_before: Some("FROM".into()) });
        to.label = "To".into();

        let mut list = spec(
            "PERIODS",
            FieldKind::List {
                item_fields: vec![from, to],
                item_label: "Add a period".into(),
                item_template: "{{INDEX}} {{FROM}} {{TO}}".into(),
                min_items: None,
                max_items: None,
            },
        );
        list.label = "Periods".into();
        let m = manifest(vec![list]);

        // Row 2's TO precedes row 2's FROM, but follows row 1's.
        let submitted = with_rows(
            "PERIODS",
            &[
                &[("FROM", "2026-01-01"), ("TO", "2026-02-01")],
                &[("FROM", "2026-06-01"), ("TO", "2026-03-01")],
            ],
        );

        let errors = validate(&m, &submitted);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].key, "PERIODS[1].TO");
        assert!(errors[0].message.contains("2026-06-01"), "{:?}", errors[0]);
    }

    #[test]
    fn a_list_hidden_by_its_condition_is_not_asked_for() {
        let mut list = list_spec("SCHEDULE", Some(1), None);
        list.shown_when =
            Some(ShownWhen { field: "TERMS".into(), equals: vec!["Instalments".into()] });

        let m = manifest(vec![
            spec("TERMS", FieldKind::Text { max_length: None }),
            list,
        ]);

        assert!(validate(&m, &values(&[("TERMS", "Lump sum")])).is_empty());
        assert_eq!(validate(&m, &values(&[("TERMS", "Instalments")])).len(), 1);
    }

    // -- repeating groups: the wrong shape ----------------------------------

    #[test]
    fn a_scalar_sent_for_a_list_is_a_field_error_not_a_broken_render() {
        let m = manifest(vec![list_spec("SECTIONS", None, None)]);
        let errors = validate(&m, &values(&[("SECTIONS", "one, two, three")]));

        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].message.contains("list of rows"), "{:?}", errors[0]);
    }

    #[test]
    fn rows_sent_for_a_scalar_field_are_reported_as_the_wrong_shape() {
        let m = manifest(vec![spec("CLIENT_NAME", FieldKind::Text { max_length: None })]);
        let submitted = with_rows("CLIENT_NAME", &[&[("HEADING", "x")]]);

        let errors = validate(&m, &submitted);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].message.contains("single value"), "{:?}", errors[0]);
    }

    /// A limit written in a manifest has to arrive in the kind that enforces
    /// it.
    ///
    /// It did not. `rename_all` on an enum renames the variants; the fields
    /// *inside* a struct variant need `rename_all_fields`, which was missing —
    /// so `"maxWords": 1500` on the examination reply matched nothing, fell to
    /// `#[serde(default)]`, and became `None`. Clean parse, no warning, and a
    /// registry word limit that was never once enforced. Found while adding
    /// `itemFields`, which failed loudly only because it has no default.
    #[test]
    fn a_limit_written_in_a_manifest_is_actually_read() {
        let spec: FieldSpec = serde_json::from_str(
            r#"{"key":"STATEMENT","label":"Statement",
                "kind":{"type":"multiline","maxWords":1500,"maxLength":9000}}"#,
        )
        .expect("the shape the shipped manifests are written in");

        let FieldKind::Multiline { max_words, max_length } = spec.kind else {
            panic!("not a multiline");
        };
        assert_eq!(max_words, Some(1500));
        assert_eq!(max_length, Some(9000));
    }

    #[test]
    fn a_date_rule_written_in_a_manifest_is_actually_read() {
        let spec: FieldSpec = serde_json::from_str(
            r#"{"key":"HEARING","label":"Hearing",
                "kind":{"type":"date","notBefore":"FILING_DATE"}}"#,
        )
        .unwrap();

        let FieldKind::Date { not_before } = spec.kind else { panic!("not a date") };
        assert_eq!(not_before.as_deref(), Some("FILING_DATE"));
    }

    /// The wire format is a string or an array of objects, with no wrapper.
    #[test]
    fn a_value_deserialises_from_the_obvious_json() {
        let parsed: HashMap<String, FieldValue> = serde_json::from_str(
            r#"{"CLIENT_NAME": "Nikhil", "SECTIONS": [{"HEADING": "Background"}]}"#,
        )
        .expect("the shape Deck sends must parse");

        assert_eq!(parsed["CLIENT_NAME"].as_scalar(), "Nikhil");
        assert_eq!(parsed["SECTIONS"].as_rows().unwrap().len(), 1);
        assert!(parsed["CLIENT_NAME"].as_rows().is_none());
        assert_eq!(parsed["SECTIONS"].as_scalar(), "");
    }

    // -- repeating groups: assembling the block ------------------------------

    #[test]
    fn rows_are_numbered_from_their_position() {
        let list = list_spec("SECTIONS", None, None);
        let assembled = assemble(
            &list,
            rows(&[
                &[("HEADING", "Background"), ("BODY", "First.")],
                &[("HEADING", "Demand"), ("BODY", "Second.")],
                &[("HEADING", "Costs"), ("BODY", "Third.")],
            ])
            .as_rows()
            .unwrap(),
        );

        let out = assembled.as_str();
        assert_eq!(out.matches("\\noticesection").count(), 3, "{out}");
        assert!(out.contains("\\noticesection{1}{Background}"), "{out}");
        assert!(out.contains("\\noticesection{2}{Demand}"), "{out}");
        assert!(out.contains("\\noticesection{3}{Costs}"), "{out}");

        // In the order the attorney put them in, not whatever order a map has.
        let positions: Vec<usize> = ["Background", "Demand", "Costs"]
            .iter()
            .map(|h| out.find(h).expect("every heading is present"))
            .collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]), "sections out of order: {out}");
    }

    /// The row is the attorney's prose, and it reaches a LaTeX document. This
    /// is the assertion that says the one `Field::raw` built out of typed input
    /// escapes every cell it interpolates.
    #[test]
    fn every_cell_of_a_row_is_escaped_into_the_block() {
        let list = list_spec("SECTIONS", None, None);
        let assembled = assemble(
            &list,
            rows(&[&[
                ("HEADING", "Payment to M/s Tata & Sons — 100% due"),
                ("BODY", r"You wrote \input{/etc/passwd} and $x_1$ #3 ~ ^"),
            ]])
            .as_rows()
            .unwrap(),
        );

        let out = assembled.as_str();
        assert!(out.contains(r"Tata \& Sons"), "unescaped ampersand: {out}");
        assert!(out.contains(r"100\%"), "unescaped percent: {out}");
        assert!(out.contains(r"\textbackslash{}input"), "a backslash survived: {out}");
        assert!(!out.contains(r"\input{/etc/passwd}"), "a command got through: {out}");
        assert!(out.contains(r"\$x\_1\$"), "unescaped maths: {out}");
        assert!(out.contains(r"\#3"), "unescaped hash: {out}");
        assert!(out.contains(r"\textasciitilde{}"), "unescaped tilde: {out}");
        assert!(out.contains(r"\textasciicircum{}"), "unescaped caret: {out}");

        // The macros the template's own itemTemplate supplies are still there.
        assert!(out.contains("\\begin{noticebody}"), "{out}");
    }

    /// Escaping also closes the substitution off: a cell cannot name another
    /// placeholder, because an escaped value has no `{{` left in it.
    #[test]
    fn a_cell_cannot_smuggle_in_another_placeholder() {
        let list = list_spec("SECTIONS", None, None);
        let assembled = assemble(
            &list,
            rows(&[&[("HEADING", "{{CLIENT_NAME}}"), ("BODY", "{{INDEX}}")]]).as_rows().unwrap(),
        );

        let out = assembled.as_str();
        assert!(!out.contains("{{CLIENT_NAME}}"), "a live placeholder reached the block: {out}");
        assert!(!out.contains("{{INDEX}}"), "a live placeholder reached the block: {out}");
        assert!(out.contains(r"\{\{CLIENT\_NAME\}\}"), "it should print as typed: {out}");
    }

    /// The manifest decides what a row contains. A key Deck invented is not a
    /// column of the document.
    #[test]
    fn a_key_the_manifest_never_declared_is_dropped() {
        let list = list_spec("SECTIONS", None, None);
        let assembled = assemble(
            &list,
            rows(&[&[("HEADING", "Background"), ("BODY", "Text."), ("SMUGGLED", "surprise")]])
                .as_rows()
                .unwrap(),
        );

        assert!(!assembled.as_str().contains("surprise"), "{}", assembled.as_str());
    }

    /// `render` refuses to compile a template with an unfilled placeholder, so
    /// an empty list has to bind its placeholder to something.
    #[test]
    fn an_empty_list_assembles_to_an_empty_block_not_to_nothing() {
        let list = list_spec("SECTIONS", None, None);
        assert_eq!(assemble(&list, &[]), Field::raw(String::new()));
    }

    // -- repeating groups: manifest shape ------------------------------------

    #[test]
    fn a_row_template_and_its_item_fields_are_checked_for_drift_too() {
        let mut list = list_spec("SECTIONS", None, None);
        if let FieldKind::List { item_template, .. } = &mut list.kind {
            *item_template = "\\noticesection{{{INDEX}}}{{{HEADING}}} {{AMOUNT}}".into();
        }

        let problems = check_against_template(&manifest(vec![list]), "{{SECTIONS}}");
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(
            problems.iter().any(|p| p.contains("AMOUNT") && p.contains("no item field")),
            "{problems:?}"
        );
        assert!(
            problems.iter().any(|p| p.contains("BODY") && p.contains("never prints it")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_list_inside_a_list_is_refused() {
        let inner = list_spec("INNER", None, None);
        let mut outer = spec(
            "OUTER",
            FieldKind::List {
                item_fields: vec![inner],
                item_label: "Add".into(),
                item_template: "{{INDEX}} {{INNER}}".into(),
                min_items: None,
                max_items: None,
            },
        );
        outer.label = "Outer".into();

        let problems = check_against_template(&manifest(vec![outer]), "{{OUTER}}");
        assert!(problems.iter().any(|p| p.contains("list inside a list")), "{problems:?}");
    }

    #[test]
    fn a_cell_cannot_be_computed_or_conditional_or_input_only() {
        let mut computed = spec("TOTAL", FieldKind::Computed);
        computed.shown_when = Some(ShownWhen { field: "HEADING".into(), equals: vec!["x".into()] });
        computed.input_only = true;

        let mut list = spec(
            "ROWS",
            FieldKind::List {
                item_fields: vec![computed],
                item_label: "Add".into(),
                item_template: "{{INDEX}} {{TOTAL}}".into(),
                min_items: None,
                max_items: None,
            },
        );
        list.label = "Rows".into();

        let problems = check_against_template(&manifest(vec![list]), "{{ROWS}}");
        assert!(problems.iter().any(|p| p.contains("is computed")), "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("shownWhen")), "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("inputOnly")), "{problems:?}");
    }

    #[test]
    fn a_list_that_asks_for_more_than_it_allows_is_refused() {
        let list = list_spec("SECTIONS", Some(4), Some(2));
        let problems = check_against_template(&manifest(vec![list]), "{{SECTIONS}}");
        assert!(problems.iter().any(|p| p.contains("minItems 4 above maxItems 2")), "{problems:?}");
    }

    #[test]
    fn a_well_formed_list_reports_nothing() {
        let list = list_spec("SECTIONS", Some(1), Some(20));
        assert!(check_against_template(&manifest(vec![list]), "{{SECTIONS}}").is_empty());
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

    /// The numbered sections of a notice are the attorney's to add, not a
    /// count the template guessed. If this ever goes back to `computed`, the
    /// form has stopped being able to write a notice.
    #[test]
    fn the_legal_notice_collects_its_numbered_sections_as_a_list() {
        let manifest = get(&templates_dir(), "legal-notice").unwrap();
        let sections = manifest
            .fields
            .iter()
            .find(|f| f.key == "SECTIONS_BLOCK")
            .expect("the notice must still have its numbered sections");

        let FieldKind::List { item_fields, min_items, .. } = &sections.kind else {
            panic!("SECTIONS_BLOCK is {:?}, not a list", sections.kind);
        };
        assert_eq!(min_items, &Some(1), "a notice with no sections is a signature on paper");

        let keys: Vec<&str> = item_fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, ["HEADING", "BODY"]);
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

// ---------------------------------------------------------------------------
// Compilation tests
//
// The tests above prove the block is assembled and escaped. They cannot prove
// an attorney's sections reach the paper: a row template that hands
// \noticesection the wrong number of arguments, a noticebody that swallows the
// section after it, a heading that compiles to nothing — none of that is
// visible from a string comparison. These compile the real notice from the real
// manifest and read the result back.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod compile_tests {
    use super::*;
    use crate::services::latex::CompileMode;

    /// Everything the notice needs apart from its sections.
    ///
    /// Every declared key is bound, then the ones worth reading are filled in:
    /// a field added to the letterhead therefore does not fail this test with
    /// an unfilled placeholder, which is not what it is here to catch.
    fn notice_fields(manifest: &TemplateManifest) -> HashMap<String, Field> {
        let known: HashMap<&str, &str> = [
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
            ("NOTICE_DATE", "14 July 2026"),
            ("RECIPIENT_NAME", "Mr. Mohammed Danish"),
            ("RECIPIENT_ADDRESS_BLOCK", "House No. 460/21, Lucknow - 226003"),
            ("MODE_OF_SERVICE", "THROUGH SPEED POST/ WHATSAPP"),
            ("SUBJECT", "DEMAND FOR REFUND OF ₹1,04,000/- WITH INTEREST"),
            ("SALUTATION", "Sir"),
            ("CLIENT_NAME", "Mr. Nikhil Prabhakar"),
            ("CLIENT_DESCRIPTION", "son of P. Prabhakaran"),
            ("CLIENT_ADDRESS", "A-004, Mangal Apartment, New Delhi-110096"),
            ("SIGNATORY_BLOCK", "Sree Lakshmi Menon, D/6361/2020"),
        ]
        .into_iter()
        .collect();

        manifest
            .fields
            .iter()
            .filter(|spec| !spec.input_only)
            .map(|spec| {
                let value = known.get(spec.key.as_str()).copied().unwrap_or("");
                (spec.key.clone(), Field::text(value))
            })
            .collect()
    }

    fn row(heading: &str, body: &str) -> Row {
        [("HEADING".to_owned(), heading.to_owned()), ("BODY".to_owned(), body.to_owned())]
            .into_iter()
            .collect()
    }

    /// Three sections of a short notice, as an attorney would type them.
    fn three_sections() -> Vec<Row> {
        vec![
            row(
                "Background",
                "That my client and you entered into an arrangement on 11 February 2026 \
                 for the purchase of a motor vehicle, and that a sum was paid by my \
                 client to you in part performance thereof.",
            ),
            row(
                "Breach",
                "That you have failed and neglected to deliver the said vehicle or to \
                 refund the said sum, despite repeated requests made by my client both \
                 orally and in writing.",
            ),
            row(
                "Demand",
                "That you are hereby called upon to refund the said sum together with \
                 interest thereon within fifteen days of the receipt of this notice, \
                 failing which my client shall be constrained to initiate appropriate \
                 proceedings against you at your risk as to costs and consequences.",
            ),
        ]
    }

    async fn compile_notice(rows: &[Row]) -> Vec<u8> {
        let dir = latex::templates_dir().expect("templates directory");
        let manifest = get(&dir, "legal-notice").expect("the notice manifest must load");
        let sections =
            manifest.fields.iter().find(|f| f.key == "SECTIONS_BLOCK").expect("SECTIONS_BLOCK");

        let mut fields = notice_fields(&manifest);
        fields.insert("SECTIONS_BLOCK".into(), assemble(sections, rows));

        latex::compile("legal-notice", &fields, CompileMode::Final)
            .await
            .expect("the notice must compile")
    }

    /// The whole point of the change: an attorney adds three sections and the
    /// served notice carries three numbered sections, in order, with their own
    /// text under each.
    #[tokio::test]
    async fn a_notice_carries_the_sections_the_attorney_entered_in_order() {
        if !latex::engine_available() {
            return;
        }

        let text = pdf_text(&compile_notice(&three_sections()).await);

        // Numbered by position, with the number beside the heading on the page
        // rather than merely somewhere in the document.
        for (number, heading) in [("1.", "Background"), ("2.", "Breach"), ("3.", "Demand")] {
            let line = text
                .lines()
                .find(|line| line.contains(heading))
                .unwrap_or_else(|| panic!("'{heading}' never reached the paper:\n{text}"));
            assert!(
                line.trim_start().starts_with(number),
                "section '{heading}' is not numbered {number}: {line:?}"
            );
        }

        // The body of each section, not just its heading. Against the squashed
        // text, because a justified paragraph breaks a phrase across lines
        // wherever the measure happens to fall.
        let flat = squash(&text);
        assert!(flat.contains("part performance thereof"), "section 1's text is missing:\n{text}");
        assert!(flat.contains("orally and in writing"), "section 2's text is missing:\n{text}");
        assert!(flat.contains("costs and consequences"), "section 3's text is missing:\n{text}");

        // In the order they were entered.
        let order: Vec<usize> = ["Background", "Breach", "Demand"]
            .iter()
            .map(|h| text.find(h).expect("present"))
            .collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]), "the sections are out of order:\n{text}");
    }

    /// What an attorney types is text, all the way to the paper. An ampersand
    /// in a company name has broken this firm's own documents before (see the
    /// note on `escape`), and a row is the newest place it could do so again.
    #[tokio::test]
    async fn hostile_text_in_a_section_prints_as_typed() {
        if !latex::engine_available() {
            return;
        }

        let hostile = row(
            "Dealings with M/s Tata & Sons",
            concat!(
                r"That 100% of the sum was paid, that the reference was 26_A#4, ",
                r"and that the file named \input{/etc/passwd} was never provided."
            ),
        );

        let text = pdf_text(&compile_notice(&[hostile]).await);
        let flat = squash(&text);

        assert!(flat.contains("Tata & Sons"), "the ampersand did not print:\n{text}");
        assert!(flat.contains("100%"), "the percent did not print:\n{text}");
        assert!(flat.contains("26_A#4"), "the underscore or hash did not print:\n{text}");
        assert!(
            flat.contains(r"\input{/etc/passwd}"),
            "a command should have printed as characters:\n{text}"
        );
    }

    /// Ten sections, because the notice the firm supplied had ten. The count is
    /// exactly what could not be expressed before this field kind existed.
    #[tokio::test]
    async fn ten_sections_compile_and_are_numbered_to_ten() {
        if !latex::engine_available() {
            return;
        }

        let rows: Vec<Row> = (1..=10)
            .map(|n| {
                row(
                    &format!("Section heading {n}"),
                    &format!("That this is the body of paragraph {n}."),
                )
            })
            .collect();

        let text = pdf_text(&compile_notice(&rows).await);

        for n in 1..=10 {
            assert!(text.contains(&format!("Section heading {n}")), "section {n} missing:\n{text}");
        }
        let ten = text.lines().find(|l| l.contains("Section heading 10")).expect("section 10");
        assert!(ten.trim_start().starts_with("10."), "section 10 is misnumbered: {ten:?}");
    }

    /// A notice being drafted has no sections yet, and the preview still has to
    /// compile — an empty list must bind its placeholder.
    #[tokio::test]
    async fn a_notice_with_no_sections_still_compiles() {
        if !latex::engine_available() {
            return;
        }

        let text = pdf_text(&compile_notice(&[]).await);
        assert!(text.contains("Yours Sincerely"), "the notice itself is missing:\n{text}");
    }

    /// `pdftotext` is in poppler-utils, which CI installs alongside TeX Live.
    /// Without it these tests would assert on a byte count, which proves the
    /// engine ran and nothing about what it produced.
    fn pdf_text(pdf: &[u8]) -> String {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("out.pdf");
        std::fs::write(&path, pdf).expect("write pdf");

        let output = std::process::Command::new("pdftotext")
            .arg("-layout")
            .arg(&path)
            .arg("-")
            .output()
            .expect("pdftotext (poppler-utils) is needed to read the notice back");

        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// One space between words, so an assertion on a phrase is not really an
    /// assertion about where the line broke.
    fn squash(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}
