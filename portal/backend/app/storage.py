"""Signed, time-limited URLs for stored objects.

Spec §15.5: no direct object URLs, ever. A document reaches a browser as a URL
that stops working in five minutes, so a link pasted into an email or left in a
browser history is not an access grant.

The signature is HMAC-SHA256 over the object key and the expiry together.
Signing them together is what matters — signing the key alone would let anyone
extend the expiry, and signing the expiry alone would let anyone swap the key.

WHAT IS NOT HERE

The bucket itself. Hetzner Object Storage is not provisioned yet, so
`object_base_url` points at whatever serves the objects and the edge is expected
to verify the signature. `verify()` is the reference implementation of that
check, and the tests exercise both directions.
"""

import base64
import hashlib
import hmac
from datetime import UTC, datetime, timedelta
from urllib.parse import urlencode

from app.config import get_settings


def _sign(key: bytes, object_key: str, expires_at: int) -> str:
    mac = hmac.new(key, f"{object_key}\n{expires_at}".encode(), hashlib.sha256)
    return base64.urlsafe_b64encode(mac.digest()).decode().rstrip("=")


def signed_url(object_key: str) -> tuple[str, datetime]:
    """Returns (url, expiry). Never returns the bare object key."""
    settings = get_settings()
    expires = datetime.now(UTC) + timedelta(seconds=settings.signed_url_ttl_seconds)
    expires_at = int(expires.timestamp())

    signature = _sign(settings.storage_signing_key.encode(), object_key, expires_at)
    query = urlencode({"expires": expires_at, "signature": signature})
    base = settings.object_base_url.rstrip("/")

    return f"{base}/{object_key}?{query}", expires


def verify(object_key: str, expires_at: int, signature: str) -> bool:
    """The check an object-storage edge performs. Expiry first, then a
    constant-time signature compare."""
    settings = get_settings()
    if expires_at < int(datetime.now(UTC).timestamp()):
        return False
    expected = _sign(settings.storage_signing_key.encode(), object_key, expires_at)
    return hmac.compare_digest(expected, signature)
