"""app/object_store.py against a fake S3-compatible endpoint.

The fake server does not trust the client's math: it independently
recomputes the SigV4 signature from the request it actually received (same
algorithm, written separately here) and rejects anything that does not
match -- the same check a real bucket performs. A test that only checked
"an Authorization header is present" would pass even if the signature were
wrong in a way that happened to still look like a signature.
"""

import hashlib
import hmac
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

from app.config import get_settings
from app.object_store import ObjectStoreError, put_object

_SECRET = "test-secret-access-key"
_ACCESS_KEY = "test-access-key-id"
_REGION = "eu-central-test"
_BUCKET = "persist-test-quarantine"


def _sign(key: bytes, msg: str) -> bytes:
    return hmac.new(key, msg.encode(), hashlib.sha256).digest()


def _expected_signature(method: str, path: str, headers: dict, signed_header_names: list[str]) -> str:
    amz_date = headers["x-amz-date"]
    date_stamp = amz_date[:8]
    payload_hash = headers["x-amz-content-sha256"]

    canonical_headers = "".join(f"{name}:{headers[name]}\n" for name in signed_header_names)
    canonical_request = "\n".join(
        [method, path, "", canonical_headers, ";".join(signed_header_names), payload_hash]
    )
    credential_scope = f"{date_stamp}/{_REGION}/s3/aws4_request"
    string_to_sign = "\n".join(
        [
            "AWS4-HMAC-SHA256",
            amz_date,
            credential_scope,
            hashlib.sha256(canonical_request.encode()).hexdigest(),
        ]
    )
    k = _sign(f"AWS4{_SECRET}".encode(), date_stamp)
    k = _sign(k, _REGION)
    k = _sign(k, "s3")
    k = _sign(k, "aws4_request")
    return hmac.new(k, string_to_sign.encode(), hashlib.sha256).hexdigest()


class _Recorder:
    def __init__(self):
        self.objects: dict[str, bytes] = {}
        self.rejected: list[str] = []


def _make_handler(recorder: _Recorder):
    class Handler(BaseHTTPRequestHandler):
        def do_PUT(self):
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length)

            auth = self.headers.get("Authorization", "")
            if not auth.startswith("AWS4-HMAC-SHA256 "):
                recorder.rejected.append(self.path)
                self.send_response(403)
                self.end_headers()
                return

            credential = auth.split("Credential=")[1].split(",")[0]
            signed_header_names = auth.split("SignedHeaders=")[1].split(",")[0].split(";")
            presented_signature = auth.split("Signature=")[1].strip()

            headers_lower = {k.lower(): v for k, v in self.headers.items()}
            expected = _expected_signature(
                "PUT", self.path, headers_lower, signed_header_names
            )

            access_key_ok = credential.startswith(_ACCESS_KEY + "/")
            payload_ok = headers_lower.get("x-amz-content-sha256") == hashlib.sha256(body).hexdigest()

            if not (access_key_ok and payload_ok and hmac.compare_digest(expected, presented_signature)):
                recorder.rejected.append(self.path)
                self.send_response(403)
                self.end_headers()
                return

            recorder.objects[self.path] = body
            self.send_response(200)
            self.end_headers()

        def log_message(self, *args):
            pass  # keep test output quiet

    return Handler


@pytest.fixture
def fake_s3():
    recorder = _Recorder()
    server = ThreadingHTTPServer(("127.0.0.1", 0), _make_handler(recorder))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    host, port = server.server_address
    yield f"http://{host}:{port}", recorder
    server.shutdown()
    thread.join(timeout=5)


def _configure_s3(monkeypatch, endpoint: str):
    monkeypatch.setenv("PORTAL_S3_ENDPOINT_URL", endpoint)
    monkeypatch.setenv("PORTAL_S3_BUCKET", _BUCKET)
    monkeypatch.setenv("PORTAL_S3_REGION", _REGION)
    monkeypatch.setenv("PORTAL_S3_ACCESS_KEY_ID", _ACCESS_KEY)
    monkeypatch.setenv("PORTAL_S3_SECRET_ACCESS_KEY", _SECRET)
    get_settings.cache_clear()


@pytest.fixture(autouse=True)
def _restore_settings():
    yield
    get_settings.cache_clear()


async def test_unconfigured_store_returns_false_and_writes_nothing(monkeypatch):
    monkeypatch.delenv("PORTAL_S3_ENDPOINT_URL", raising=False)
    get_settings.cache_clear()

    stored = put_object("quarantine/c1/u1", b"bytes", "application/pdf")

    assert stored is False


async def test_a_configured_store_receives_the_exact_bytes_under_the_right_key(monkeypatch, fake_s3):
    endpoint, recorder = fake_s3
    _configure_s3(monkeypatch, endpoint)

    stored = put_object("quarantine/client-a/upload-1", b"%PDF-1.4 the actual content", "application/pdf")

    assert stored is True
    assert recorder.objects[f"/{_BUCKET}/quarantine/client-a/upload-1"] == b"%PDF-1.4 the actual content"
    assert not recorder.rejected


async def test_a_wrong_secret_produces_a_signature_the_server_rejects(monkeypatch, fake_s3):
    """The server's independent recomputation has to actually be able to
    fail, or the two 'real' checks above are not proving anything."""
    endpoint, recorder = fake_s3
    _configure_s3(monkeypatch, endpoint)
    monkeypatch.setenv("PORTAL_S3_SECRET_ACCESS_KEY", "the-wrong-secret-entirely")
    get_settings.cache_clear()

    with pytest.raises(ObjectStoreError):
        put_object("quarantine/client-a/upload-2", b"content", "application/pdf")

    assert recorder.rejected
    assert not recorder.objects


async def test_a_key_with_special_characters_is_uri_encoded_correctly(monkeypatch, fake_s3):
    endpoint, recorder = fake_s3
    _configure_s3(monkeypatch, endpoint)

    # A filename-derived key can carry characters that need percent-encoding
    # in the URL but must still hash and sign against the *raw* key.
    key = "quarantine/client-a/upload with spaces & stuff.pdf"
    stored = put_object(key, b"content", "application/pdf")

    assert stored is True
    assert recorder.objects
