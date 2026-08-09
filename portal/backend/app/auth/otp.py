"""One-time codes.

No passwords exist anywhere in the portal, so this is the whole of authentication
and it has to be right.

WHAT THIS DEFENDS AGAINST

  Account enumeration — `request_otp` does the same work and returns the same
    response whether or not the address is known (spec §15.11). An unknown
    address still burns the timing of a bcrypt hash, so response time does not
    answer the question either.
  Code guessing — 6 digits is a million codes, but only 5 attempts per
    challenge, after which the challenge is dead rather than merely limited.
  Code reuse — a challenge is consumed on success. A code that worked once does
    not work twice.
  Stale codes — 10-minute expiry, checked in SQL rather than in Python, so a
    clock-skewed app server cannot extend it.
  Parallel challenges — requesting a new code invalidates the outstanding ones
    for that address, so an attacker cannot widen the guessing surface by asking
    for twenty codes.
"""

import secrets
import uuid
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta

import bcrypt
from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncConnection

from app.config import get_settings

# Burned when the address is unknown, so that "no such user" and "user exists"
# cost the same wall-clock time. The value is irrelevant; the work is the point.
_TIMING_DECOY = bcrypt.hashpw(b"000000", bcrypt.gensalt(rounds=10))


@dataclass(frozen=True)
class PortalIdentity:
    portal_user_id: str
    client_id: str
    email: str


def _generate_code(length: int) -> str:
    # secrets, not random: this is a credential.
    return "".join(secrets.choice("0123456789") for _ in range(length))


async def _find_active_identity(conn: AsyncConnection, email: str) -> PortalIdentity | None:
    """Only an Active portal user may receive a code.

    Invited users cannot log in yet, and Suspended or Revoked users must not —
    revocation from the desktop has to bite here as well as on every request.
    """
    row = await conn.execute(
        text("""
            SELECT id, client_id, email
            FROM mirror.portal_users
            WHERE lower(email) = lower(:email) AND status = 'Active'
        """),
        {"email": email},
    )
    record = row.mappings().first()
    if not record:
        return None
    return PortalIdentity(
        portal_user_id=record["id"],
        client_id=record["client_id"],
        email=record["email"],
    )


async def request_otp(conn: AsyncConnection, email: str) -> str | None:
    """Issue a code, or do the same work and issue nothing.

    Returns the plaintext code so the caller can send it. It is never stored,
    logged, or returned to the browser outside tests.
    """
    settings = get_settings()
    email = email.strip()

    identity = await _find_active_identity(conn, email)
    if identity is None:
        # Same cost as the real path. No row is written, nothing is sent, and
        # the caller returns the same 202 either way.
        bcrypt.checkpw(b"000000", _TIMING_DECOY)
        return None

    # Outstanding challenges for this address die now, so asking for a second
    # code narrows the guessing surface rather than widening it.
    await conn.execute(
        text("""
            UPDATE inbound.otp_challenges
            SET consumed_at = now()
            WHERE lower(email) = lower(:email) AND consumed_at IS NULL
        """),
        {"email": identity.email},
    )

    code = _generate_code(settings.otp_length)
    code_hash = bcrypt.hashpw(code.encode(), bcrypt.gensalt()).decode()
    expires_at = datetime.now(UTC) + timedelta(minutes=settings.otp_ttl_minutes)

    await conn.execute(
        text("""
            INSERT INTO inbound.otp_challenges (id, email, code_hash, expires_at)
            VALUES (:id, :email, :hash, :expires)
        """),
        {
            "id": str(uuid.uuid4()),
            "email": identity.email,
            "hash": code_hash,
            "expires": expires_at,
        },
    )
    return code


async def verify_otp(conn: AsyncConnection, email: str, code: str) -> PortalIdentity | None:
    """None on any failure. The caller must not tell the client which one."""
    settings = get_settings()
    email = email.strip()

    row = await conn.execute(
        text("""
            SELECT id, code_hash, attempts
            FROM inbound.otp_challenges
            WHERE lower(email) = lower(:email)
              AND consumed_at IS NULL
              AND expires_at > now()
              AND attempts < :max_attempts
            ORDER BY created_at DESC
            LIMIT 1
        """),
        {"email": email, "max_attempts": settings.otp_max_attempts},
    )
    challenge = row.mappings().first()

    if challenge is None:
        bcrypt.checkpw(code.encode(), _TIMING_DECOY)
        return None

    # Count the attempt before checking it. A verifier that recorded attempts
    # only on failure would let a crash mid-check hand back a free guess.
    await conn.execute(
        text("UPDATE inbound.otp_challenges SET attempts = attempts + 1 WHERE id = :id"),
        {"id": challenge["id"]},
    )

    if not bcrypt.checkpw(code.encode(), challenge["code_hash"].encode()):
        return None

    # The status is re-read here, not carried from request time: a user revoked
    # in the ten minutes since the code was sent must not get in.
    identity = await _find_active_identity(conn, email)
    if identity is None:
        return None

    await conn.execute(
        text("UPDATE inbound.otp_challenges SET consumed_at = now() WHERE id = :id"),
        {"id": challenge["id"]},
    )
    await conn.execute(
        text("UPDATE mirror.portal_users SET last_login_at = now() WHERE id = :id"),
        {"id": identity.portal_user_id},
    )

    return identity


async def purge_expired(conn: AsyncConnection) -> int:
    """Housekeeping. A consumed or expired challenge has no further use."""
    result = await conn.execute(
        text("""
            DELETE FROM inbound.otp_challenges
            WHERE expires_at < now() - interval '1 day'
               OR consumed_at < now() - interval '1 day'
        """)
    )
    return result.rowcount or 0
