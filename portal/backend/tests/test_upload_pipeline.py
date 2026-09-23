"""The upload route end to end, with a scanner and a bucket both configured.

test_responses.py already covers the route's shape when neither is
configured (scan_status stays Pending, per spec §15.12's safe default).
This file is the other half: prove `/documents/upload` actually calls
through to app.object_store and app.scanning, not just that those modules
work in isolation.
"""

import threading

import pytest

from app.config import get_settings
from tests.conftest import EMAIL_A, bearer, login
from tests.test_object_store import _BUCKET as S3_BUCKET
from tests.test_object_store import _make_handler as make_s3_handler
from tests.test_object_store import _Recorder as S3Recorder
from tests.test_scanning import EICAR, _FakeClamd


@pytest.fixture
def _uploads_configured(monkeypatch):
    from http.server import ThreadingHTTPServer

    recorder = S3Recorder()
    s3 = ThreadingHTTPServer(("127.0.0.1", 0), make_s3_handler(recorder))
    s3_thread = threading.Thread(target=s3.serve_forever, daemon=True)
    s3_thread.start()
    s3_host, s3_port = s3.server_address

    monkeypatch.setenv("PORTAL_S3_ENDPOINT_URL", f"http://{s3_host}:{s3_port}")
    monkeypatch.setenv("PORTAL_S3_BUCKET", S3_BUCKET)
    monkeypatch.setenv("PORTAL_S3_REGION", "eu-central-test")
    monkeypatch.setenv("PORTAL_S3_ACCESS_KEY_ID", "test-access-key-id")
    monkeypatch.setenv("PORTAL_S3_SECRET_ACCESS_KEY", "test-secret-access-key")
    get_settings.cache_clear()

    yield recorder

    s3.shutdown()
    s3_thread.join(timeout=5)
    for key in (
        "PORTAL_S3_ENDPOINT_URL", "PORTAL_S3_BUCKET", "PORTAL_S3_REGION",
        "PORTAL_S3_ACCESS_KEY_ID", "PORTAL_S3_SECRET_ACCESS_KEY",
        "PORTAL_CLAMD_HOST", "PORTAL_CLAMD_PORT",
    ):
        monkeypatch.delenv(key, raising=False)
    get_settings.cache_clear()


def _point_at_clamd(monkeypatch, port: int):
    monkeypatch.setenv("PORTAL_CLAMD_HOST", "127.0.0.1")
    monkeypatch.setenv("PORTAL_CLAMD_PORT", str(port))
    get_settings.cache_clear()


async def test_a_clean_upload_is_stored_and_marked_clean(client, monkeypatch, _uploads_configured):
    clamd = _FakeClamd(b"stream: OK\0")
    _point_at_clamd(monkeypatch, clamd.port)

    token = await login(client, EMAIL_A)
    response = await client.post(
        "/documents/upload",
        headers=bearer(token),
        files={"file": ("poa.pdf", b"%PDF-1.4 a real-looking document", "application/pdf")},
    )
    clamd.join()

    assert response.status_code == 202
    body = response.json()
    assert body["scanStatus"] == "Clean"

    stored_bytes = [v for k, v in _uploads_configured.objects.items() if body["id"] in k]
    assert stored_bytes == [b"%PDF-1.4 a real-looking document"]


async def test_an_infected_upload_is_still_accepted_but_marked_infected(
    client, monkeypatch, _uploads_configured
):
    """Spec §15.12: untrusted until scanned clean. Infected is a terminal
    verdict recorded on the row, not a silent 500 -- the desktop's pull
    already filters to scan_status = 'Clean', so an Infected row simply
    never reaches the queue the desktop acts on."""
    clamd = _FakeClamd(b"stream: Eicar-Test-Signature FOUND\0")
    _point_at_clamd(monkeypatch, clamd.port)

    token = await login(client, EMAIL_A)
    response = await client.post(
        "/documents/upload",
        headers=bearer(token),
        files={"file": ("eicar.pdf", EICAR, "application/pdf")},
    )
    clamd.join()

    assert response.status_code == 202
    assert response.json()["scanStatus"] == "Infected"


async def test_a_store_that_refuses_the_write_fails_the_upload_outright(client, monkeypatch):
    """No scanner needed for this one: the object store rejects the PUT, and
    the route must not insert a row that names an object nothing wrote."""
    monkeypatch.setenv("PORTAL_S3_ENDPOINT_URL", "http://127.0.0.1:1")  # nothing listens here
    monkeypatch.setenv("PORTAL_S3_BUCKET", "whatever")
    monkeypatch.setenv("PORTAL_S3_REGION", "eu-central-test")
    monkeypatch.setenv("PORTAL_S3_ACCESS_KEY_ID", "x")
    monkeypatch.setenv("PORTAL_S3_SECRET_ACCESS_KEY", "y")
    get_settings.cache_clear()

    token = await login(client, EMAIL_A)
    response = await client.post(
        "/documents/upload",
        headers=bearer(token),
        files={"file": ("poa.pdf", b"%PDF-1.4 x", "application/pdf")},
    )

    assert response.status_code == 503

    from tests.test_auth import _as_admin

    rows = await _as_admin("SELECT id FROM inbound.client_uploads WHERE filename = 'poa.pdf'")
    assert rows == []
