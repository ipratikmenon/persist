"""What the API returns, and what it must never return.

The Pydantic models are the second lock on the projection (`projection.rs` is
the first). These tests assert on whole response bodies rather than on named
fields, because the field that leaks is the one nobody thought to assert on.
"""

import os

from sqlalchemy import text
from sqlalchemy.ext.asyncio import create_async_engine

from tests.conftest import CLIENT_A, EMAIL_A, bearer, login


async def _as_admin(sql: str, params: dict | None = None) -> None:
    engine = create_async_engine(
        os.environ["PORTAL_TEST_ADMIN_DSN"],
        connect_args={"statement_cache_size": 0},
    )
    async with engine.begin() as conn:
        await conn.execute(text(sql), params or {})
    await engine.dispose()


# ---------------------------------------------------------------------------
# Object keys and hashes must not reach a browser
# ---------------------------------------------------------------------------


async def test_document_listings_carry_no_object_key_or_hash(client):
    token = await login(client, EMAIL_A)
    body = (await client.get("/documents", headers=bearer(token))).text

    assert "vault/a/doc-a-1" not in body, "the object key reached the client"
    assert "object_key" not in body
    assert "sha256" not in body


async def test_invoice_listings_carry_no_pdf_object_key(client):
    token = await login(client, EMAIL_A)

    listing = (await client.get("/invoices", headers=bearer(token))).text
    detail = (await client.get("/invoices/INV-A-1", headers=bearer(token))).text

    for body in (listing, detail):
        assert "vault/a/inv-a-1.pdf" not in body
        assert "pdf_object_key" not in body


async def test_a_download_redirects_to_a_signed_url_that_expires(client):
    from urllib.parse import parse_qs, urlparse

    from app.storage import verify

    token = await login(client, EMAIL_A)
    response = await client.get(
        "/documents/DOC-A-1/download",
        headers=bearer(token),
        follow_redirects=False,
    )

    assert response.status_code == 302
    location = response.headers["location"]
    assert response.headers["cache-control"] == "no-store"

    query = parse_qs(urlparse(location).query)
    assert verify("vault/a/doc-a-1", int(query["expires"][0]), query["signature"][0])


async def test_a_tampered_signed_url_does_not_verify(client):
    """The signature covers the key and the expiry together, so neither can be
    changed on its own."""
    from urllib.parse import parse_qs, urlparse

    from app.storage import verify

    token = await login(client, EMAIL_A)
    response = await client.get(
        "/documents/DOC-A-1/download",
        headers=bearer(token),
        follow_redirects=False,
    )
    query = parse_qs(urlparse(response.headers["location"]).query)
    expires, signature = int(query["expires"][0]), query["signature"][0]

    # Swap the object key, keep the signature.
    assert not verify("vault/b/doc-b-1", expires, signature)
    # Extend the expiry, keep the signature.
    assert not verify("vault/a/doc-a-1", expires + 86_400, signature)


async def test_an_expired_signature_is_refused(client):
    from app.storage import _sign, verify
    from app.config import get_settings

    past = 1_000_000_000  # comfortably in the past
    signature = _sign(get_settings().storage_signing_key.encode(), "vault/a/doc-a-1", past)
    assert not verify("vault/a/doc-a-1", past, signature)


# ---------------------------------------------------------------------------
# Money
# ---------------------------------------------------------------------------


async def test_money_survives_as_exact_decimals(client):
    """The mirror is NUMERIC(14,2). A total shown to a client must not drift."""
    token = await login(client, EMAIL_A)
    invoice = (await client.get("/invoices/INV-A-1", headers=bearer(token))).json()

    assert invoice["subtotal"] == "70000.01"
    assert invoice["total_with_tax"] == "82600.01"
    assert invoice["amount_paid"] == "10000.00"
    assert invoice["amount_due"] == "72600.01"


async def test_invoice_detail_carries_no_line_items(client):
    """`time_entries` never syncs, so there is nothing to reconstruct a bill
    from — and the API must not invent one (spec §14)."""
    token = await login(client, EMAIL_A)
    invoice = (await client.get("/invoices/INV-A-1", headers=bearer(token))).json()

    assert "line_items" not in invoice
    assert "hours" not in str(invoice)
    assert "rate" not in str(invoice)


async def test_payments_appear_on_the_invoice(client):
    token = await login(client, EMAIL_A)
    invoice = (await client.get("/invoices/INV-A-1", headers=bearer(token))).json()

    assert [p["id"] for p in invoice["payments"]] == ["PAY-A-1"]
    assert invoice["payments"][0]["amount"] == "10000.00"


# ---------------------------------------------------------------------------
# Matter detail
# ---------------------------------------------------------------------------


async def test_matter_detail_assembles_its_own_children(client):
    token = await login(client, EMAIL_A)
    matter = (await client.get("/matters/M-A-1", headers=bearer(token))).json()

    assert matter["title"] == "PETALVEDA"
    assert matter["responsible_attorney"] == "Sree Lakshmi Menon"
    assert [d["id"] for d in matter["deadlines"]] == ["D-A-1"]
    assert [d["id"] for d in matter["documents"]] == ["DOC-A-1"]


async def test_a_column_added_to_the_mirror_does_not_reach_the_client(client):
    """The response model forbids extras, so a new mirror column has to be added
    to a model deliberately before a client can ever see it.

    This is the guard for the failure mode the whole projection design exists to
    prevent: someone adds a column, it flows through a `SELECT *`, and nobody
    notices until it is in a client's browser.
    """
    await _as_admin("ALTER TABLE mirror.matters_public ADD COLUMN internal_leak TEXT")
    await _as_admin(
        "UPDATE mirror.matters_public SET internal_leak = 'STRATEGY_LEAK' WHERE client_id = :cid",
        {"cid": CLIENT_A},
    )
    try:
        token = await login(client, EMAIL_A)
        listing = (await client.get("/matters", headers=bearer(token))).text
        detail = (await client.get("/matters/M-A-1", headers=bearer(token))).text

        assert "STRATEGY_LEAK" not in listing
        assert "STRATEGY_LEAK" not in detail
    finally:
        await _as_admin("ALTER TABLE mirror.matters_public DROP COLUMN internal_leak")


# ---------------------------------------------------------------------------
# Uploads
# ---------------------------------------------------------------------------


async def test_an_upload_is_queued_not_ingested(client):
    token = await login(client, EMAIL_A)
    response = await client.post(
        "/documents/upload",
        headers=bearer(token),
        files={"file": ("poa.pdf", b"%PDF-1.4 fake", "application/pdf")},
        data={"matter_id": "M-A-1"},
    )

    assert response.status_code == 202
    body = response.json()
    # Untrusted until the desktop says otherwise (spec §15.12).
    assert body["scan_status"] == "Pending"
    assert body["status"] == "Pending"


async def test_an_upload_attributed_to_another_clients_matter_is_refused(client):
    token = await login(client, EMAIL_A)
    response = await client.post(
        "/documents/upload",
        headers=bearer(token),
        files={"file": ("poa.pdf", b"%PDF-1.4 fake", "application/pdf")},
        data={"matter_id": "M-B-1"},
    )
    assert response.status_code == 404


async def test_an_unsupported_file_type_is_refused(client):
    token = await login(client, EMAIL_A)
    response = await client.post(
        "/documents/upload",
        headers=bearer(token),
        files={"file": ("script.sh", b"#!/bin/sh\nrm -rf /", "application/x-sh")},
    )
    assert response.status_code == 415


async def test_an_oversized_upload_is_refused(client):
    from app.config import get_settings

    token = await login(client, EMAIL_A)
    too_big = b"x" * (get_settings().max_upload_bytes + 1)
    response = await client.post(
        "/documents/upload",
        headers=bearer(token),
        files={"file": ("big.pdf", too_big, "application/pdf")},
    )
    assert response.status_code == 413


async def test_an_empty_upload_is_refused(client):
    token = await login(client, EMAIL_A)
    response = await client.post(
        "/documents/upload",
        headers=bearer(token),
        files={"file": ("empty.pdf", b"", "application/pdf")},
    )
    assert response.status_code == 400


async def test_a_traversing_filename_is_flattened(client):
    """A filename from a browser is untrusted input, not a path."""
    token = await login(client, EMAIL_A)
    response = await client.post(
        "/documents/upload",
        headers=bearer(token),
        files={"file": ("../../../etc/passwd", b"%PDF-1.4 x", "application/pdf")},
    )
    assert response.status_code == 202
    assert response.json()["filename"] == "passwd"


# ---------------------------------------------------------------------------
# Headers
# ---------------------------------------------------------------------------


async def test_security_headers_are_on_every_response(client):
    token = await login(client, EMAIL_A)
    response = await client.get("/matters", headers=bearer(token))

    assert response.headers["strict-transport-security"].startswith("max-age=")
    assert response.headers["x-content-type-options"] == "nosniff"
    assert response.headers["x-frame-options"] == "DENY"
    assert response.headers["cache-control"] == "no-store"


async def test_the_openapi_schema_is_not_served(client):
    """The portal is not a public API. A schema would only describe attack
    surface to someone with no business calling it."""
    for path in ["/openapi.json", "/docs", "/redoc"]:
        assert (await client.get(path)).status_code == 404
