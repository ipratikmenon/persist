"""Cross-client isolation.

This is the test the spec makes a gate (§15.2). Every fixture row belonging to
client B carries a SECRET-B sentinel, so these assertions sweep whole response
bodies rather than checking a field at a time — the leak that matters is the one
through a field nobody thought to check.
"""

from tests.conftest import CLIENT_A, EMAIL_A, EMAIL_B, bearer, login

SENTINELS = [
    "SECRET-B-MATTER",
    "SECRET-B-DEADLINE",
    "SECRET-B-DOC",
    "SECRET-B-NOTIFICATION",
    "M-B-1",
    "INV-B-1",
    "Client B Foods",
]


async def _sweep(client, token: str, paths: list[str]) -> None:
    for path in paths:
        response = await client.get(path, headers=bearer(token))
        assert response.status_code == 200, f"{path}: {response.text}"
        body = response.text
        for sentinel in SENTINELS:
            assert sentinel not in body, f"{sentinel} leaked through {path}: {body}"


async def test_client_a_sees_nothing_of_client_b(client):
    token = await login(client, EMAIL_A)
    await _sweep(client, token, [
        "/matters",
        "/deadlines",
        "/documents",
        "/invoices",
        "/notifications",
        "/profile",
    ])


async def test_client_a_sees_its_own_rows(client):
    """The mirror image. A test that only proves nothing is returned would pass
    just as well against a broken query."""
    token = await login(client, EMAIL_A)

    matters = await client.get("/matters", headers=bearer(token))
    assert [m["id"] for m in matters.json()] == ["M-A-1"]

    invoices = await client.get("/invoices", headers=bearer(token))
    assert [i["id"] for i in invoices.json()] == ["INV-A-1"]

    documents = await client.get("/documents", headers=bearer(token))
    assert [d["id"] for d in documents.json()] == ["DOC-A-1"]


async def test_fetching_another_clients_matter_is_a_404_not_a_403(client):
    """403 would confirm the id is real. 404 tells them nothing."""
    token = await login(client, EMAIL_A)
    response = await client.get("/matters/M-B-1", headers=bearer(token))
    assert response.status_code == 404
    assert "SECRET-B" not in response.text


async def test_fetching_another_clients_invoice_is_a_404(client):
    token = await login(client, EMAIL_A)
    assert (await client.get("/invoices/INV-B-1", headers=bearer(token))).status_code == 404
    assert (await client.get("/invoices/INV-B-1/pdf", headers=bearer(token))).status_code == 404


async def test_downloading_another_clients_document_is_a_404(client):
    token = await login(client, EMAIL_A)
    response = await client.get(
        "/documents/DOC-B-1/download",
        headers=bearer(token),
        follow_redirects=False,
    )
    assert response.status_code == 404


async def test_disputing_another_clients_invoice_is_refused(client):
    token = await login(client, EMAIL_A)
    response = await client.post(
        "/invoices/INV-B-1/dispute",
        headers=bearer(token),
        json={"reason": "This bill is not mine and I would like it explained."},
    )
    assert response.status_code == 404


async def test_rls_holds_with_the_api_filter_removed(client):
    """Belt and braces, tested (spec §9).

    The API filters by client_id *and* RLS enforces it. Here the API filter is
    dropped entirely — an unfiltered `SELECT *` on every mirror table — and the
    reader connection must still return only client A's rows. If this ever
    fails, the second lock was carrying the first.
    """
    from sqlalchemy import text

    from app.db.session import client_scope

    tables = [
        "mirror.matters_public",
        "mirror.deadlines_public",
        "mirror.ip_assets_public",
        "mirror.documents_shared",
        "mirror.invoices_public",
        "mirror.payments_public",
        "mirror.client_notifications",
    ]

    async with client_scope(CLIENT_A) as conn:
        for table in tables:
            rows = await conn.execute(text(f"SELECT client_id FROM {table}"))  # noqa: S608
            owners = {r[0] for r in rows}
            assert owners <= {CLIENT_A}, f"RLS LEAK: {table} returned rows owned by {owners}"


async def test_a_token_for_one_client_cannot_be_used_as_another(client):
    """A forged `cid` must not work even with a valid signature.

    This mints a genuinely signed token whose claims put user A inside client
    B. `require_client` compares the claim against the database and refuses.
    """
    from app.auth.tokens import issue_access_token
    from tests.conftest import CLIENT_B, USER_A

    forged, _ = issue_access_token(USER_A, CLIENT_B)
    response = await client.get("/matters", headers=bearer(forged))
    assert response.status_code == 401


async def test_client_b_sees_its_own_rows_too(client):
    """Guards against the isolation tests passing because B's rows were never
    written in the first place."""
    token = await login(client, EMAIL_B)
    matters = await client.get("/matters", headers=bearer(token))
    assert [m["id"] for m in matters.json()] == ["M-B-1"]
    assert "SECRET-B-MATTER" in matters.text
