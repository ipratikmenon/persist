"""A minimal S3-compatible client (AWS Signature Version 4, path-style).

Hetzner Object Storage speaks the S3 API, and SigV4 is what every
S3-compatible provider expects for a non-presigned request, so this
hand-rolls the one operation the portal needs -- PUT -- rather than pull in
a full SDK for one verb. `storage.py` already hand-rolls the *download*
side (a custom HMAC over the object key, verified at an edge); this is the
matching upload side, signed the way the bucket itself actually requires.

Path-style addressing (`<endpoint>/<bucket>/<key>`) is used throughout: it
needs no bucket-specific DNS and every S3-compatible provider supports it.

WHAT IS NOT HERE

A live Hetzner bucket. `s3_endpoint_url` (and its siblings) are unset until
one is provisioned; until then `put_object` returns False without writing
anything, mirroring storage.py's own "the signing is real, the bucket isn't"
split. See app/scanning.py for the same split applied to ClamAV.
"""

import hashlib
import hmac
import http.client
import logging
from datetime import UTC, datetime
from urllib.parse import quote, urlsplit

from app.config import get_settings

log = logging.getLogger(__name__)

_SERVICE = "s3"
_ALGORITHM = "AWS4-HMAC-SHA256"


class ObjectStoreError(Exception):
    """A configured store refused the request or could not be reached.

    Never raised for an unconfigured store -- that is `put_object` returning
    False, a distinct and expected state in a deployment with no bucket yet.
    """


def _hmac(key: bytes, msg: str) -> bytes:
    return hmac.new(key, msg.encode(), hashlib.sha256).digest()


def _signing_key(secret_key: str, date_stamp: str, region: str) -> bytes:
    k_date = _hmac(f"AWS4{secret_key}".encode(), date_stamp)
    k_region = _hmac(k_date, region)
    k_service = _hmac(k_region, _SERVICE)
    return _hmac(k_service, "aws4_request")


def _canonical_path(bucket: str, key: str) -> str:
    # Each segment is URI-encoded independently; '/' between them is not.
    return "/" + "/".join(quote(part, safe="") for part in f"{bucket}/{key}".split("/"))


def _build_request(
    *, method: str, key: str, body: bytes, content_type: str | None
) -> tuple[str, dict[str, str]]:
    """Returns (url, headers) with a valid SigV4 Authorization header."""
    settings = get_settings()
    now = datetime.now(UTC)
    amz_date = now.strftime("%Y%m%dT%H%M%SZ")
    date_stamp = now.strftime("%Y%m%d")

    host = urlsplit(settings.s3_endpoint_url).netloc
    path = _canonical_path(settings.s3_bucket, key)
    payload_hash = hashlib.sha256(body).hexdigest()

    to_sign = {
        "host": host,
        "x-amz-content-sha256": payload_hash,
        "x-amz-date": amz_date,
    }
    if content_type:
        to_sign["content-type"] = content_type

    signed_header_names = ";".join(sorted(to_sign))
    canonical_headers = "".join(f"{k}:{v}\n" for k, v in sorted(to_sign.items()))
    canonical_request = "\n".join(
        [method, path, "", canonical_headers, signed_header_names, payload_hash]
    )

    credential_scope = f"{date_stamp}/{settings.s3_region}/{_SERVICE}/aws4_request"
    string_to_sign = "\n".join(
        [
            _ALGORITHM,
            amz_date,
            credential_scope,
            hashlib.sha256(canonical_request.encode()).hexdigest(),
        ]
    )

    signing_key = _signing_key(settings.s3_secret_access_key, date_stamp, settings.s3_region)
    signature = hmac.new(signing_key, string_to_sign.encode(), hashlib.sha256).hexdigest()

    authorization = (
        f"{_ALGORITHM} Credential={settings.s3_access_key_id}/{credential_scope}, "
        f"SignedHeaders={signed_header_names}, Signature={signature}"
    )

    url = f"{settings.s3_endpoint_url.rstrip('/')}{path}"
    headers = {k.title(): v for k, v in to_sign.items()}
    headers["Authorization"] = authorization
    return url, headers


def _send(method: str, url: str, headers: dict[str, str], body: bytes, timeout: float) -> int:
    parts = urlsplit(url)
    conn_cls = http.client.HTTPSConnection if parts.scheme == "https" else http.client.HTTPConnection
    conn = conn_cls(parts.hostname, parts.port, timeout=timeout)
    try:
        conn.request(method, parts.path or "/", body=body, headers=headers)
        response = conn.getresponse()
        response.read()  # drain, so the connection can close cleanly
        return response.status
    finally:
        conn.close()


def put_object(key: str, data: bytes, content_type: str) -> bool:
    """True once stored. False if no store is configured -- a deployment
    with no bucket yet, same as ClamAV's unconfigured case. Raises
    ObjectStoreError if a configured store rejects the write or cannot be
    reached, since silently dropping bytes there would leave a database row
    pointing at an object that was never written."""
    settings = get_settings()
    if settings.s3_endpoint_url is None:
        log.info("object storage not configured; %s was not written", key)
        return False

    url, headers = _build_request(method="PUT", key=key, body=data, content_type=content_type)
    try:
        status = _send("PUT", url, headers, data, timeout=30.0)
    except OSError as e:
        raise ObjectStoreError(f"could not reach object storage: {e}") from e

    if status not in (200, 201, 204):
        raise ObjectStoreError(f"object storage refused the write (HTTP {status})")
    return True
