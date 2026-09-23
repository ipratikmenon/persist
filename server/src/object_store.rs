// A minimal S3-compatible client (AWS Signature Version 4, path-style).
//
// Hetzner Object Storage speaks the S3 API. `portal/backend/app/object_store.py`
// hand-rolls the same signing scheme for the portal's quarantine-bucket PUT;
// this is its Rust twin, used by the shared-document and pending-upload
// routes in main.rs (spec §12: POST/DELETE /sync/documents/{key},
// GET /sync/uploads/{id}/object).
//
// WHAT IS NOT HERE: a live Hetzner bucket. `from_env` returns `Ok(None)` when
// no S3_* variable is set at all, and the three document-storage routes then
// answer 503 — sync of matters, deadlines and the rest of the mirror keeps
// working with no bucket configured, per root CLAUDE.md's "a firm with no
// server keeps working exactly as today."

use anyhow::{bail, Context, Result};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::time::Duration;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct ObjectStore {
    endpoint_url: String,
    bucket: String,
    region: String,
    access_key_id: String,
    secret_access_key: String,
    client: reqwest::Client,
}

impl ObjectStore {
    /// `Ok(None)` when no S3_* variable is set — object storage is an
    /// optional feature of this server, not a startup requirement (unlike
    /// DATABASE_URL and SYNC_CLIENT_TOKEN, which the server refuses to run
    /// without). `Err` when SOME but not all five are set: a half-configured
    /// bucket that looks live and silently isn't is worse than refusing to
    /// start.
    pub fn from_env() -> Result<Option<Self>> {
        let endpoint_url = std::env::var("S3_ENDPOINT_URL").ok();
        let bucket = std::env::var("S3_BUCKET").ok();
        let region = std::env::var("S3_REGION").ok();
        let access_key_id = std::env::var("S3_ACCESS_KEY_ID").ok();
        let secret_access_key = std::env::var("S3_SECRET_ACCESS_KEY").ok();

        let present = [&endpoint_url, &bucket, &region, &access_key_id, &secret_access_key]
            .iter()
            .filter(|v| v.is_some())
            .count();
        if present == 0 {
            return Ok(None);
        }
        if present != 5 {
            bail!(
                "object storage is partially configured — S3_ENDPOINT_URL, S3_BUCKET, \
                 S3_REGION, S3_ACCESS_KEY_ID and S3_SECRET_ACCESS_KEY must all be set, \
                 or none of them"
            );
        }

        Ok(Some(Self {
            endpoint_url: endpoint_url.unwrap(),
            bucket: bucket.unwrap(),
            region: region.unwrap(),
            access_key_id: access_key_id.unwrap(),
            secret_access_key: secret_access_key.unwrap(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .context("building the object storage HTTP client")?,
        }))
    }

    /// Builds a store from explicit values, bypassing `from_env` entirely.
    /// For tests only — `main.rs`'s route-level tests need an `ObjectStore`
    /// pointed at a per-test fake bucket, and going through process-global
    /// env vars would race against every other test that also touches them.
    #[cfg(test)]
    pub(crate) fn new_for_test(endpoint_url: &str, bucket: &str) -> Self {
        Self {
            endpoint_url: endpoint_url.to_string(),
            bucket: bucket.to_string(),
            region: "test-region".to_string(),
            access_key_id: "test-access-key-id".to_string(),
            secret_access_key: "test-secret-access-key".to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn canonical_path(&self, key: &str) -> String {
        let mut segments = vec![self.bucket.as_str()];
        segments.extend(key.split('/'));
        let encoded: Vec<String> = segments.iter().map(|s| uri_encode(s)).collect();
        format!("/{}", encoded.join("/"))
    }

    fn signed_headers(
        &self,
        method: &str,
        path: &str,
        payload: &[u8],
        extra: &[(&str, &str)],
    ) -> Result<Vec<(String, String)>> {
        let now = Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();
        let host = self
            .endpoint_url
            .split("://")
            .nth(1)
            .unwrap_or(&self.endpoint_url)
            .trim_end_matches('/')
            .to_string();
        let payload_hash = hex::encode(Sha256::digest(payload));

        let mut to_sign: Vec<(String, String)> = vec![
            ("host".into(), host.clone()),
            ("x-amz-content-sha256".into(), payload_hash.clone()),
            ("x-amz-date".into(), amz_date.clone()),
        ];
        for (k, v) in extra {
            to_sign.push((k.to_string(), v.to_string()));
        }
        to_sign.sort_by(|a, b| a.0.cmp(&b.0));

        let signed_header_names = to_sign.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>().join(";");
        let canonical_headers: String =
            to_sign.iter().map(|(k, v)| format!("{k}:{v}\n")).collect();
        let canonical_request = format!(
            "{method}\n{path}\n\n{canonical_headers}\n{signed_header_names}\n{payload_hash}"
        );

        let credential_scope = format!("{date_stamp}/{}/s3/aws4_request", self.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
            hex::encode(Sha256::digest(canonical_request.as_bytes()))
        );

        let signing_key = self.derive_signing_key(&date_stamp)?;
        let mut mac = HmacSha256::new_from_slice(&signing_key).context("hmac key")?;
        mac.update(string_to_sign.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{credential_scope}, SignedHeaders={signed_header_names}, Signature={signature}",
            self.access_key_id
        );

        let mut headers = to_sign;
        headers.push(("authorization".into(), authorization));
        Ok(headers)
    }

    fn derive_signing_key(&self, date_stamp: &str) -> Result<Vec<u8>> {
        fn hmac_bytes(key: &[u8], msg: &[u8]) -> Result<Vec<u8>> {
            let mut mac = HmacSha256::new_from_slice(key).context("hmac key")?;
            mac.update(msg);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        let k_date = hmac_bytes(format!("AWS4{}", self.secret_access_key).as_bytes(), date_stamp.as_bytes())?;
        let k_region = hmac_bytes(&k_date, self.region.as_bytes())?;
        let k_service = hmac_bytes(&k_region, b"s3")?;
        hmac_bytes(&k_service, b"aws4_request")
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.endpoint_url.trim_end_matches('/'), path)
    }

    pub async fn put(&self, key: &str, body: Vec<u8>, content_type: &str) -> Result<()> {
        let path = self.canonical_path(key);
        let headers = self.signed_headers(
            "PUT",
            &path,
            &body,
            &[("content-type", content_type)],
        )?;

        let mut req = self.client.put(self.url(&path)).body(body);
        for (k, v) in headers {
            req = req.header(k, v);
        }
        let resp = req.send().await.context("PUT to object storage")?;
        if !resp.status().is_success() {
            bail!("object storage refused the write (HTTP {})", resp.status());
        }
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        let path = self.canonical_path(key);
        let headers = self.signed_headers("DELETE", &path, b"", &[])?;

        let mut req = self.client.delete(self.url(&path));
        for (k, v) in headers {
            req = req.header(k, v);
        }
        let resp = req.send().await.context("DELETE from object storage")?;
        // A key that is already gone is not a failure for an unshare/cleanup
        // caller — deleting twice must be safe to retry.
        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
            bail!("object storage refused the delete (HTTP {})", resp.status());
        }
        Ok(())
    }

    /// `Ok(None)` for a missing key — the caller turns that into its own 404,
    /// not an error.
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let path = self.canonical_path(key);
        let headers = self.signed_headers("GET", &path, b"", &[])?;

        let mut req = self.client.get(self.url(&path));
        for (k, v) in headers {
            req = req.header(k, v);
        }
        let resp = req.send().await.context("GET from object storage")?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            bail!("object storage refused the read (HTTP {})", resp.status());
        }
        Ok(Some(resp.bytes().await.context("reading object body")?.to_vec()))
    }
}

/// RFC 3986 unreserved characters pass through; everything else is
/// percent-encoded with uppercase hex, matching what AWS's canonical URI
/// requires and what a path-style S3 endpoint expects on the wire.
fn uri_encode(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Bytes,
        extract::State,
        http::{HeaderMap, StatusCode},
        routing::put,
        Router,
    };
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Recorder {
        objects: Arc<Mutex<std::collections::HashMap<String, Vec<u8>>>>,
        rejected: Arc<Mutex<Vec<String>>>,
    }

    const TEST_SECRET: &str = "test-secret-access-key";
    const TEST_ACCESS_KEY: &str = "test-access-key-id";
    const TEST_REGION: &str = "eu-central-test";
    const TEST_BUCKET: &str = "persist-test-bucket";

    /// Recomputes the SigV4 signature the same way a real bucket would, from
    /// the headers actually received — not merely checking one is present.
    fn expected_signature(method: &str, path: &str, headers: &HeaderMap, body: &[u8]) -> String {
        let get = |name: &str| headers.get(name).unwrap().to_str().unwrap().to_string();
        let amz_date = get("x-amz-date");
        let date_stamp = amz_date[..8].to_string();
        let payload_hash = hex::encode(Sha256::digest(body));

        let names = ["content-type", "host", "x-amz-content-sha256", "x-amz-date"];
        let present: Vec<&str> = names.iter().copied().filter(|n| headers.contains_key(*n)).collect();
        let canonical_headers: String =
            present.iter().map(|n| format!("{n}:{}\n", get(n))).collect();
        let signed = present.join(";");
        let canonical_request = format!("{method}\n{path}\n\n{canonical_headers}\n{signed}\n{payload_hash}");
        let credential_scope = format!("{date_stamp}/{TEST_REGION}/s3/aws4_request");
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
            hex::encode(Sha256::digest(canonical_request.as_bytes()))
        );

        fn hb(key: &[u8], msg: &[u8]) -> Vec<u8> {
            let mut mac = HmacSha256::new_from_slice(key).unwrap();
            mac.update(msg);
            mac.finalize().into_bytes().to_vec()
        }
        let k_date = hb(format!("AWS4{TEST_SECRET}").as_bytes(), date_stamp.as_bytes());
        let k_region = hb(&k_date, TEST_REGION.as_bytes());
        let k_service = hb(&k_region, b"s3");
        let k_signing = hb(&k_service, b"aws4_request");
        hex::encode(hb(&k_signing, string_to_sign.as_bytes()))
    }

    fn verify(method: &str, path: &str, headers: &HeaderMap, body: &[u8]) -> bool {
        let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) else {
            return false;
        };
        if !auth.starts_with("AWS4-HMAC-SHA256 ") {
            return false;
        }
        let credential = auth.split("Credential=").nth(1).unwrap_or("").split(',').next().unwrap_or("");
        if !credential.starts_with(&format!("{TEST_ACCESS_KEY}/")) {
            return false;
        }
        let presented = auth.split("Signature=").nth(1).unwrap_or("").trim();
        let expected = expected_signature(method, path, headers, body);
        presented == expected
    }

    // The signature covers the *wire* path, percent-encoding and all — a
    // real bucket verifies against what was literally in the request line,
    // not a decoded-and-reassembled version of it. Path() gives the decoded
    // key (fine for indexing the in-memory store); OriginalUri gives the raw
    // path the signature actually has to match.

    async fn put_handler(
        State(rec): State<Recorder>,
        axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
        axum::extract::Path((_bucket, key)): axum::extract::Path<(String, String)>,
        headers: HeaderMap,
        body: Bytes,
    ) -> StatusCode {
        if !verify("PUT", uri.path(), &headers, &body) {
            rec.rejected.lock().unwrap().push(key);
            return StatusCode::FORBIDDEN;
        }
        rec.objects.lock().unwrap().insert(key, body.to_vec());
        StatusCode::OK
    }

    async fn delete_handler(
        State(rec): State<Recorder>,
        axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
        axum::extract::Path((_bucket, key)): axum::extract::Path<(String, String)>,
        headers: HeaderMap,
    ) -> StatusCode {
        if !verify("DELETE", uri.path(), &headers, b"") {
            return StatusCode::FORBIDDEN;
        }
        let existed = rec.objects.lock().unwrap().remove(&key).is_some();
        if existed { StatusCode::NO_CONTENT } else { StatusCode::NOT_FOUND }
    }

    async fn get_handler(
        State(rec): State<Recorder>,
        axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
        axum::extract::Path((_bucket, key)): axum::extract::Path<(String, String)>,
        headers: HeaderMap,
    ) -> Result<Vec<u8>, StatusCode> {
        if !verify("GET", uri.path(), &headers, b"") {
            return Err(StatusCode::FORBIDDEN);
        }
        rec.objects.lock().unwrap().get(&key).cloned().ok_or(StatusCode::NOT_FOUND)
    }

    async fn spawn_fake_bucket() -> (String, Recorder) {
        let recorder = Recorder::default();
        let app = Router::new()
            .route("/:bucket/*key", put(put_handler).delete(delete_handler).get(get_handler))
            .with_state(recorder.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}"), recorder)
    }

    fn store(endpoint: &str) -> ObjectStore {
        ObjectStore {
            endpoint_url: endpoint.to_string(),
            bucket: TEST_BUCKET.to_string(),
            region: TEST_REGION.to_string(),
            access_key_id: TEST_ACCESS_KEY.to_string(),
            secret_access_key: TEST_SECRET.to_string(),
            client: reqwest::Client::new(),
        }
    }

    #[test]
    fn from_env_is_none_when_nothing_is_set() {
        for var in ["S3_ENDPOINT_URL", "S3_BUCKET", "S3_REGION", "S3_ACCESS_KEY_ID", "S3_SECRET_ACCESS_KEY"] {
            std::env::remove_var(var);
        }
        assert!(ObjectStore::from_env().unwrap().is_none());
    }

    #[test]
    fn from_env_refuses_a_half_configured_bucket() {
        std::env::set_var("S3_ENDPOINT_URL", "https://example.test");
        std::env::remove_var("S3_BUCKET");
        std::env::remove_var("S3_REGION");
        std::env::remove_var("S3_ACCESS_KEY_ID");
        std::env::remove_var("S3_SECRET_ACCESS_KEY");

        let result = ObjectStore::from_env();

        std::env::remove_var("S3_ENDPOINT_URL");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn put_writes_the_exact_bytes_under_the_right_key() {
        let (endpoint, recorder) = spawn_fake_bucket().await;
        let store = store(&endpoint);

        store.put("documents/client-a/doc-1", b"%PDF-1.4 the content".to_vec(), "application/pdf")
            .await
            .unwrap();

        assert_eq!(
            recorder.objects.lock().unwrap().get("documents/client-a/doc-1").unwrap(),
            b"%PDF-1.4 the content"
        );
        assert!(recorder.rejected.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_wrong_secret_is_rejected_by_the_bucket() {
        let (endpoint, recorder) = spawn_fake_bucket().await;
        let mut store = store(&endpoint);
        store.secret_access_key = "the-wrong-secret".to_string();

        let result = store.put("documents/client-a/doc-2", b"x".to_vec(), "application/pdf").await;

        assert!(result.is_err());
        assert!(!recorder.rejected.lock().unwrap().is_empty());
        assert!(recorder.objects.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_returns_none_for_a_missing_key() {
        let (endpoint, _recorder) = spawn_fake_bucket().await;
        let store = store(&endpoint);

        let result = store.get("documents/client-a/does-not-exist").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn put_then_get_round_trips_the_bytes() {
        let (endpoint, _recorder) = spawn_fake_bucket().await;
        let store = store(&endpoint);

        store.put("documents/client-a/doc-3", b"round trip me".to_vec(), "application/pdf")
            .await
            .unwrap();
        let fetched = store.get("documents/client-a/doc-3").await.unwrap();

        assert_eq!(fetched.as_deref(), Some(b"round trip me".as_slice()));
    }

    #[tokio::test]
    async fn delete_removes_the_object_and_is_idempotent() {
        let (endpoint, recorder) = spawn_fake_bucket().await;
        let store = store(&endpoint);
        store.put("documents/client-a/doc-4", b"x".to_vec(), "application/pdf").await.unwrap();

        store.delete("documents/client-a/doc-4").await.unwrap();
        assert!(recorder.objects.lock().unwrap().get("documents/client-a/doc-4").is_none());

        // A second delete of the same (now-missing) key must not error.
        store.delete("documents/client-a/doc-4").await.unwrap();
    }

    #[tokio::test]
    async fn a_key_with_special_characters_is_encoded_and_signed_consistently() {
        let (endpoint, _recorder) = spawn_fake_bucket().await;
        let store = store(&endpoint);
        let key = "documents/client-a/report with spaces & stuff.pdf";

        store.put(key, b"content".to_vec(), "application/pdf").await.unwrap();
        let fetched = store.get(key).await.unwrap();

        assert_eq!(fetched.as_deref(), Some(b"content".as_slice()));
    }
}
