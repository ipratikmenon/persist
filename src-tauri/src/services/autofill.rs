// Autofill — filling a field from the record it already lives on.
//
// A manifest declares where a field's value can be found — `"autofill":
// "matter.forum"`, `"ipAsset.applicationNumber"` — and until this, nothing
// read it. The Reply to Examination Report has asked the attorney to re-type
// an application number the matter's own IP asset record already holds, every
// time it has been used, since S20.
//
// WHAT THIS IS NOT
//
// Autofill is a starting point, never an answer the attorney is bound by.
// Every field it touches stays an ordinary, editable input — nothing here
// makes a field read-only or a template placeholder. Getting a class list
// wrong is a form the attorney corrects before generating; a field they could
// not correct would be worse than no autofill at all.
//
// WHAT A SOURCE RESOLVES AGAINST
//
//   client.name                    — the client on the matter
//   matter.responsibleAttorney     — the partner's name, not their id
//   matter.forum                   — free text on the matter record
//   ipAsset.applicationNumber
//   ipAsset.title
//   ipAsset.classes                — Nice/Locarno numbers, joined "3, 5"
//
// An `ipAsset.*` source needs to know *which* IP asset. If the caller does not
// say, and the matter has exactly one, that one is used — a trademark matter
// usually is about one mark. With more than one, or none, every `ipAsset.*`
// field is left unresolved rather than guessed.

use crate::commands::ip_assets::IpAsset;
use crate::db::queries::{ip_assets, matters};
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Resolve every `(field key, autofill source)` pair the manifest declares,
/// against one matter and — where an `ipAsset.*` source appears — one IP
/// asset.
///
/// A source that cannot be resolved is left out of the returned map rather
/// than bound to an empty string. The caller only ever fills a field the
/// attorney has not yet touched, and "we did not attempt this one" is a
/// different fact from "the record's answer is blank" — collapsing them would
/// make a genuinely blank forum indistinguishable from a matter that does not
/// exist.
pub async fn resolve(
    pool: &SqlitePool,
    fields: &[(String, String)],
    matter_id: &str,
    ip_asset_id: Option<&str>,
) -> anyhow::Result<HashMap<String, String>> {
    let mut out = HashMap::new();

    let Some(matter) = matters::get_row(pool, matter_id).await? else {
        // No such matter. `render_document` is where a missing matter is an
        // attorney-facing error; autofill is best-effort and simply offers
        // nothing.
        return Ok(out);
    };

    // Resolved once per call and reused, not once per field: a manifest with
    // three `client.name` fields should not be three queries.
    let mut client_name: Option<Option<String>> = None;
    let mut attorney_name: Option<Option<String>> = None;
    let mut ip_asset: Option<Option<IpAsset>> = None;

    for (key, source) in fields {
        let value = match source.as_str() {
            "client.name" => {
                let name = client_name.get_or_insert_with(|| None);
                if name.is_none() {
                    *name = matters::get_client_row(pool, &matter.client_id)
                        .await?
                        .map(|c| c.name);
                }
                name.clone()
            }
            "matter.responsibleAttorney" => {
                let name = attorney_name.get_or_insert_with(|| None);
                if name.is_none() {
                    *name = matters::responsible_attorney_name(pool, &matter).await?;
                }
                name.clone()
            }
            "matter.forum" => matter.forum.clone(),
            source if source.starts_with("ipAsset.") => {
                let asset = match &ip_asset {
                    Some(cached) => cached,
                    None => {
                        let resolved =
                            resolve_ip_asset(pool, matter_id, ip_asset_id).await?;
                        ip_asset.insert(resolved)
                    }
                };
                asset.as_ref().and_then(|a| ip_asset_field(a, &source["ipAsset.".len()..]))
            }
            // A source the manifest declares that this resolver does not
            // recognise is skipped rather than treated as an error: a newer
            // template describing a source this build predates should still
            // render, just without that one convenience filled in.
            _ => None,
        };

        if let Some(value) = value.filter(|v| !v.is_empty()) {
            out.insert(key.clone(), value);
        }
    }

    Ok(out)
}

/// The IP asset an `ipAsset.*` source resolves against: the one named, or the
/// matter's only one.
async fn resolve_ip_asset(
    pool: &SqlitePool,
    matter_id: &str,
    ip_asset_id: Option<&str>,
) -> anyhow::Result<Option<IpAsset>> {
    if let Some(id) = ip_asset_id {
        return ip_assets::get(pool, id).await;
    }

    let mut assets = ip_assets::list_by_matter(pool, matter_id).await?;
    if assets.len() == 1 {
        return Ok(Some(assets.remove(0)));
    }
    // Zero assets: nothing to offer. More than one: guessing which mark the
    // attorney means would be worse than leaving the fields blank.
    Ok(None)
}

fn ip_asset_field(asset: &IpAsset, field: &str) -> Option<String> {
    match field {
        "applicationNumber" => asset.application_number.clone(),
        "title" => Some(asset.title.clone()),
        "classes" => {
            if asset.classes.is_empty() {
                None
            } else {
                Some(asset.classes.iter().map(i64::to_string).collect::<Vec<_>>().join(", "))
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ip_assets::CreateIpAssetInput;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        pool
    }

    async fn seed_matter(pool: &SqlitePool, forum: Option<&str>, attorney: Option<&str>) {
        sqlx::query("INSERT INTO clients (id, name) VALUES ('c-1', 'Petalveda Naturals Pvt Ltd')")
            .execute(pool)
            .await
            .unwrap();

        if let Some(user_id) = attorney {
            sqlx::query(
                "INSERT INTO users (id, name, email, role, password_hash)
                 VALUES (?1, 'Sree Lakshmi Menon', ?1 || '@persist.in', 'Partner', 'x')",
            )
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        }

        sqlx::query(
            "INSERT INTO matters (id, client_id, title, matter_type, forum,
                                   responsible_partner_id, opened_date)
             VALUES ('M-1', 'c-1', 'PETALVEDA', 'Trademark', ?, ?, date('now'))",
        )
        .bind(forum)
        .bind(attorney)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn seed_ip_asset(pool: &SqlitePool, title: &str, application_number: &str, classes: &[i64]) {
        let id = format!("asset-{title}");
        ip_assets::create(
            pool,
            &id,
            CreateIpAssetInput {
                matter_id: "M-1".into(),
                asset_type: "Trademark".into(),
                title: title.into(),
                application_number: Some(application_number.into()),
                registration_number: None,
                filing_date: None,
                priority_date: None,
                grant_date: None,
                registration_date: None,
                expiry_date: None,
                applicant_entity_type: None,
                jurisdiction: None,
                classes: Some(classes.to_vec()),
                status: None,
                notes: None,
            },
        )
        .await
        .unwrap();
    }

    fn declared(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[tokio::test]
    async fn client_name_resolves_from_the_matter() {
        let pool = test_pool().await;
        seed_matter(&pool, None, None).await;

        let out = resolve(&pool, &declared(&[("APPLICANT_NAME", "client.name")]), "M-1", None)
            .await
            .unwrap();

        assert_eq!(out.get("APPLICANT_NAME").map(String::as_str), Some("Petalveda Naturals Pvt Ltd"));
    }

    #[tokio::test]
    async fn the_responsible_attorney_resolves_to_a_name_not_an_id() {
        let pool = test_pool().await;
        seed_matter(&pool, None, Some("user-slm")).await;

        let out = resolve(
            &pool,
            &declared(&[("ATTORNEY_NAME", "matter.responsibleAttorney")]),
            "M-1",
            None,
        )
        .await
        .unwrap();

        assert_eq!(out.get("ATTORNEY_NAME").map(String::as_str), Some("Sree Lakshmi Menon"));
    }

    #[tokio::test]
    async fn a_matter_with_no_responsible_partner_resolves_to_nothing() {
        let pool = test_pool().await;
        seed_matter(&pool, None, None).await;

        let out = resolve(
            &pool,
            &declared(&[("ATTORNEY_NAME", "matter.responsibleAttorney")]),
            "M-1",
            None,
        )
        .await
        .unwrap();

        assert!(!out.contains_key("ATTORNEY_NAME"));
    }

    #[tokio::test]
    async fn forum_comes_straight_off_the_matter() {
        let pool = test_pool().await;
        seed_matter(&pool, Some("Delhi"), None).await;

        let out =
            resolve(&pool, &declared(&[("REGISTRY_OFFICE", "matter.forum")]), "M-1", None)
                .await
                .unwrap();

        assert_eq!(out.get("REGISTRY_OFFICE").map(String::as_str), Some("Delhi"));
    }

    /// The common case a trademark matter is in: one matter, one mark. No
    /// asset id needed for the attorney to get it filled in.
    #[tokio::test]
    async fn a_matters_single_ip_asset_is_used_without_being_named() {
        let pool = test_pool().await;
        seed_matter(&pool, None, None).await;
        seed_ip_asset(&pool, "PETALVEDA", "5544121", &[3, 5]).await;

        let out = resolve(
            &pool,
            &declared(&[
                ("TM_MARK", "ipAsset.title"),
                ("TM_NUMBER", "ipAsset.applicationNumber"),
                ("TM_CLASS", "ipAsset.classes"),
            ]),
            "M-1",
            None,
        )
        .await
        .unwrap();

        assert_eq!(out.get("TM_MARK").map(String::as_str), Some("PETALVEDA"));
        assert_eq!(out.get("TM_NUMBER").map(String::as_str), Some("5544121"));
        assert_eq!(out.get("TM_CLASS").map(String::as_str), Some("3, 5"));
    }

    /// Guessing which of several marks the attorney means would be worse than
    /// leaving the fields for them to fill in by hand.
    #[tokio::test]
    async fn ip_asset_fields_are_left_blank_when_the_matter_has_more_than_one() {
        let pool = test_pool().await;
        seed_matter(&pool, None, None).await;
        seed_ip_asset(&pool, "PETALVEDA", "5544121", &[3, 5]).await;
        seed_ip_asset(&pool, "PETALVEDA DEVICE", "5544122", &[3]).await;

        let out = resolve(&pool, &declared(&[("TM_MARK", "ipAsset.title")]), "M-1", None)
            .await
            .unwrap();

        assert!(!out.contains_key("TM_MARK"));
    }

    #[tokio::test]
    async fn a_named_ip_asset_is_used_even_among_several() {
        let pool = test_pool().await;
        seed_matter(&pool, None, None).await;
        seed_ip_asset(&pool, "PETALVEDA", "5544121", &[3, 5]).await;
        seed_ip_asset(&pool, "PETALVEDA DEVICE", "5544122", &[3]).await;

        let device_id: String =
            sqlx::query_scalar("SELECT id FROM ip_assets WHERE title = 'PETALVEDA DEVICE'")
                .fetch_one(&pool)
                .await
                .unwrap();

        let out = resolve(
            &pool,
            &declared(&[("TM_MARK", "ipAsset.title")]),
            "M-1",
            Some(&device_id),
        )
        .await
        .unwrap();

        // Naming the second mark explicitly overrides the "only one" default —
        // the matter has two, and the caller said which.
        assert_eq!(out.get("TM_MARK").map(String::as_str), Some("PETALVEDA DEVICE"));
    }

    #[tokio::test]
    async fn an_empty_class_list_is_not_autofilled_as_an_empty_string() {
        let pool = test_pool().await;
        seed_matter(&pool, None, None).await;
        seed_ip_asset(&pool, "PETALVEDA", "5544121", &[]).await;

        let out = resolve(&pool, &declared(&[("TM_CLASS", "ipAsset.classes")]), "M-1", None)
            .await
            .unwrap();

        assert!(!out.contains_key("TM_CLASS"), "an empty list should not overwrite the field with nothing");
    }

    /// A column can hold an empty string rather than NULL — SQLite does not
    /// stop that. The class-list case above is guarded earlier, in
    /// `ip_asset_field`; this is the guard for every other source, where the
    /// database value comes straight through as `Some("")`.
    #[tokio::test]
    async fn a_forum_stored_as_an_empty_string_is_not_autofilled_either() {
        let pool = test_pool().await;
        seed_matter(&pool, Some(""), None).await;

        let out =
            resolve(&pool, &declared(&[("REGISTRY_OFFICE", "matter.forum")]), "M-1", None)
                .await
                .unwrap();

        assert!(!out.contains_key("REGISTRY_OFFICE"));
    }

    #[tokio::test]
    async fn a_matter_that_does_not_exist_resolves_to_an_empty_map_not_an_error() {
        let pool = test_pool().await;
        let out =
            resolve(&pool, &declared(&[("APPLICANT_NAME", "client.name")]), "M-404", None)
                .await
                .unwrap();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn an_unrecognised_source_is_skipped_not_fatal() {
        let pool = test_pool().await;
        seed_matter(&pool, None, None).await;

        let out = resolve(
            &pool,
            &declared(&[("SOMETHING", "matter.somethingFutureVersionsMightAdd")]),
            "M-1",
            None,
        )
        .await
        .unwrap();

        assert!(out.is_empty());
    }
}
