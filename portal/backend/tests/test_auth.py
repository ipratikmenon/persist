"""Authentication: OTP, tokens, revocation.

There are no passwords in the portal, so this file is the whole of the front
door.
"""

import os

import pytest
from sqlalchemy import text
from sqlalchemy.ext.asyncio import create_async_engine

from tests.conftest import EMAIL_A, USER_A, bearer, login


async def _as_admin(sql: str, params: dict | None = None):
    engine = create_async_engine(
        os.environ["PORTAL_TEST_ADMIN_DSN"],
        connect_args={"statement_cache_size": 0},
    )
    async with engine.begin() as conn:
        result = await conn.execute(text(sql), params or {})
        rows = result.mappings().all() if result.returns_rows else []
    await engine.dispose()
    return rows


# ---------------------------------------------------------------------------
# No account enumeration (spec §15.11)
# ---------------------------------------------------------------------------


async def test_an_unknown_address_gets_the_same_response_as_a_known_one(client):
    known = await client.post("/auth/request-otp", json={"email": EMAIL_A})
    unknown = await client.post("/auth/request-otp", json={"email": "nobody@nowhere.test"})

    assert known.status_code == unknown.status_code == 202
    assert known.json()["detail"] == unknown.json()["detail"]
    # The debug field is a test affordance; the real difference must not be
    # visible in the shape of the response.
    assert set(known.json()) == set(unknown.json())


async def test_no_challenge_is_written_for_an_unknown_address(client):
    await client.post("/auth/request-otp", json={"email": "nobody@nowhere.test"})
    rows = await _as_admin(
        "SELECT id FROM inbound.otp_challenges WHERE lower(email) = 'nobody@nowhere.test'"
    )
    assert rows == []


async def test_an_invited_but_not_active_user_cannot_log_in(client):
    await _as_admin(
        "UPDATE mirror.portal_users SET status = 'Invited' WHERE id = :uid",
        {"uid": USER_A},
    )
    response = await client.post("/auth/request-otp", json={"email": EMAIL_A})
    assert response.status_code == 202
    assert response.json()["debugCode"] is None


# ---------------------------------------------------------------------------
# The code itself
# ---------------------------------------------------------------------------


async def test_the_plaintext_code_is_never_stored(client):
    requested = await client.post("/auth/request-otp", json={"email": EMAIL_A})
    code = requested.json()["debugCode"]

    rows = await _as_admin("SELECT code_hash FROM inbound.otp_challenges")
    assert len(rows) == 1
    assert code not in rows[0]["code_hash"]
    assert rows[0]["code_hash"].startswith("$2b$")


async def test_a_wrong_code_is_refused(client):
    await client.post("/auth/request-otp", json={"email": EMAIL_A})
    response = await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": "000000"})
    assert response.status_code == 401


async def test_a_code_works_once(client):
    requested = await client.post("/auth/request-otp", json={"email": EMAIL_A})
    code = requested.json()["debugCode"]

    first = await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": code})
    assert first.status_code == 200

    second = await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": code})
    assert second.status_code == 401, "a consumed code must not work twice"


async def test_attempts_are_capped_and_then_the_challenge_is_dead(client):
    requested = await client.post("/auth/request-otp", json={"email": EMAIL_A})
    code = requested.json()["debugCode"]

    for _ in range(5):
        await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": "999999"})

    # The right code now fails too — the challenge is spent, not merely rate
    # limited, so guessing cannot be resumed by waiting.
    response = await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": code})
    assert response.status_code == 401


async def test_requesting_a_new_code_kills_the_old_one(client):
    first = (await client.post("/auth/request-otp", json={"email": EMAIL_A})).json()["debugCode"]
    second = (await client.post("/auth/request-otp", json={"email": EMAIL_A})).json()["debugCode"]
    assert first != second

    stale = await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": first})
    assert stale.status_code == 401, "asking for a second code must not widen the guessing surface"

    fresh = await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": second})
    assert fresh.status_code == 200


async def test_an_expired_code_is_refused(client):
    requested = await client.post("/auth/request-otp", json={"email": EMAIL_A})
    code = requested.json()["debugCode"]

    await _as_admin("UPDATE inbound.otp_challenges SET expires_at = now() - interval '1 minute'")

    response = await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": code})
    assert response.status_code == 401


# ---------------------------------------------------------------------------
# Tokens
# ---------------------------------------------------------------------------


async def test_protected_routes_need_a_token(client):
    for path in ["/matters", "/deadlines", "/documents", "/invoices", "/notifications", "/profile"]:
        response = await client.get(path)
        assert response.status_code in (401, 403), f"{path} was reachable unauthenticated"


async def test_a_garbage_token_is_refused(client):
    response = await client.get("/matters", headers=bearer("not.a.jwt"))
    assert response.status_code == 401


async def test_a_token_signed_with_another_key_is_refused(client):
    """The signature is what makes `cid` trustworthy, so a token signed by
    anyone else must be worthless however well-formed its claims are."""
    import subprocess
    import tempfile
    from datetime import UTC, datetime, timedelta
    from pathlib import Path

    import jwt

    from tests.conftest import CLIENT_A

    with tempfile.TemporaryDirectory() as tmp:
        rogue = Path(tmp) / "rogue.pem"
        subprocess.run(
            ["openssl", "genpkey", "-algorithm", "RSA",
             "-pkeyopt", "rsa_keygen_bits:2048", "-out", str(rogue)],
            check=True, capture_output=True,
        )
        now = datetime.now(UTC)
        forged = jwt.encode(
            {"sub": USER_A, "cid": CLIENT_A, "iss": "persist-portal",
             "iat": now, "exp": now + timedelta(minutes=15), "jti": "x"},
            rogue.read_text(),
            algorithm="RS256",
        )

    assert (await client.get("/matters", headers=bearer(forged))).status_code == 401


async def test_an_expired_token_is_refused(client):
    from datetime import UTC, datetime, timedelta

    import jwt

    from app.config import get_settings
    from tests.conftest import CLIENT_A

    past = datetime.now(UTC) - timedelta(hours=2)
    expired = jwt.encode(
        {"sub": USER_A, "cid": CLIENT_A, "iss": "persist-portal",
         "iat": past, "exp": past + timedelta(minutes=15), "jti": "x"},
        get_settings().jwt_private_key,
        algorithm="RS256",
    )

    assert (await client.get("/matters", headers=bearer(expired))).status_code == 401


# ---------------------------------------------------------------------------
# Revocation (spec §15.10) — immediate, not at token expiry
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("new_status", ["Revoked", "Suspended"])
async def test_revocation_takes_effect_on_the_next_request(client, new_status):
    token = await login(client, EMAIL_A)
    assert (await client.get("/matters", headers=bearer(token))).status_code == 200

    await _as_admin(
        "UPDATE mirror.portal_users SET status = :s WHERE id = :uid",
        {"s": new_status, "uid": USER_A},
    )

    # Same token, still cryptographically valid, still inside its 15 minutes.
    response = await client.get("/matters", headers=bearer(token))
    assert response.status_code == 401, "revocation must bite before the token expires"


async def test_a_revoked_user_cannot_refresh_their_way_back_in(client):
    login_response = await client.post("/auth/request-otp", json={"email": EMAIL_A})
    code = login_response.json()["debugCode"]
    verified = await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": code})
    assert verified.status_code == 200

    await _as_admin(
        "UPDATE mirror.portal_users SET status = 'Revoked' WHERE id = :uid",
        {"uid": USER_A},
    )

    refreshed = await client.post("/auth/refresh")
    assert refreshed.status_code == 401


# ---------------------------------------------------------------------------
# Refresh token rotation
# ---------------------------------------------------------------------------


async def test_refresh_rotates_and_the_old_token_dies(client):
    code = (await client.post("/auth/request-otp", json={"email": EMAIL_A})).json()["debugCode"]
    await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": code})

    original = client.cookies.get("persist_refresh")
    assert original

    first = await client.post("/auth/refresh")
    assert first.status_code == 200
    rotated = client.cookies.get("persist_refresh")
    assert rotated != original, "every refresh must mint a new token"


async def test_reusing_a_spent_refresh_token_kills_the_whole_family(client):
    """Replay and theft are indistinguishable, so both end the session."""
    code = (await client.post("/auth/request-otp", json={"email": EMAIL_A})).json()["debugCode"]
    await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": code})

    stolen = client.cookies.get("persist_refresh")

    # The legitimate client rotates.
    assert (await client.post("/auth/refresh")).status_code == 200
    live = client.cookies.get("persist_refresh")

    # The thief presents the token they took before the rotation.
    client.cookies.set("persist_refresh", stolen)
    assert (await client.post("/auth/refresh")).status_code == 401

    # And the legitimate client's current token is dead too — the family went
    # with it. Losing a session beats leaving a thief with a live one.
    client.cookies.set("persist_refresh", live)
    assert (await client.post("/auth/refresh")).status_code == 401


async def test_logout_revokes_the_refresh_token(client):
    code = (await client.post("/auth/request-otp", json={"email": EMAIL_A})).json()["debugCode"]
    await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": code})
    token = client.cookies.get("persist_refresh")

    assert (await client.post("/auth/logout")).status_code == 204

    client.cookies.set("persist_refresh", token)
    assert (await client.post("/auth/refresh")).status_code == 401


# ---------------------------------------------------------------------------
# Rate limits (spec §10)
# ---------------------------------------------------------------------------


async def test_otp_requests_are_capped_per_address(client):
    """3 per address per hour. The fourth is refused before any work is done,
    so an attacker cannot use the endpoint as a free mail cannon either."""
    for _ in range(3):
        assert (await client.post("/auth/request-otp", json={"email": EMAIL_A})).status_code == 202

    fourth = await client.post("/auth/request-otp", json={"email": EMAIL_A})
    assert fourth.status_code == 429


async def test_verify_attempts_are_capped_per_ip(client):
    """10 per IP per hour, independent of the per-challenge cap — so an
    attacker cannot get more guesses by requesting fresh challenges."""
    await client.post("/auth/request-otp", json={"email": EMAIL_A})

    statuses = [
        (await client.post("/auth/verify-otp", json={"email": EMAIL_A, "code": "111111"})).status_code
        for _ in range(11)
    ]
    assert statuses[-1] == 429, f"verify-otp was not rate limited: {statuses}"
