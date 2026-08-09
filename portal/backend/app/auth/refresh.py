"""Refresh tokens: rotation, and reuse detection.

The token is a 256-bit random value. Only its SHA-256 is stored, so a dump of
`inbound.refresh_tokens` is not a set of working credentials.

Rotation is unconditional — every refresh mints a new token and retires the old
one. Presenting a retired token means either a replay or a theft the legitimate
client has already rotated past, and nothing can tell those apart, so the whole
family is revoked. The client logs in again; a thief gets nothing.
"""

import hashlib
import secrets
import uuid
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta

from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncConnection

from app.config import get_settings

COOKIE_NAME = "persist_refresh"


@dataclass(frozen=True)
class RefreshResult:
    token: str
    portal_user_id: str
    client_id: str
    expires_at: datetime


def _hash(token: str) -> str:
    return hashlib.sha256(token.encode()).hexdigest()


async def issue(
    conn: AsyncConnection,
    portal_user_id: str,
    client_id: str,
    family_id: str | None = None,
) -> RefreshResult:
    settings = get_settings()
    token = secrets.token_urlsafe(32)
    expires_at = datetime.now(UTC) + timedelta(days=settings.refresh_token_days)

    await conn.execute(
        text("""
            INSERT INTO inbound.refresh_tokens
                (id, portal_user_id, client_id, token_hash, family_id, expires_at)
            VALUES (:id, :uid, :cid, :hash, :family, :expires)
        """),
        {
            "id": str(uuid.uuid4()),
            "uid": portal_user_id,
            "cid": client_id,
            "hash": _hash(token),
            "family": family_id or str(uuid.uuid4()),
            "expires": expires_at,
        },
    )
    return RefreshResult(
        token=token,
        portal_user_id=portal_user_id,
        client_id=client_id,
        expires_at=expires_at,
    )


async def _revoke_family(conn: AsyncConnection, family_id: str) -> None:
    await conn.execute(
        text("""
            UPDATE inbound.refresh_tokens
            SET revoked_at = now()
            WHERE family_id = :family AND revoked_at IS NULL
        """),
        {"family": family_id},
    )


async def rotate(conn: AsyncConnection, presented: str) -> RefreshResult | None:
    """Exchange a refresh token for a new one. None on any failure."""
    row = await conn.execute(
        text("""
            SELECT id, portal_user_id, client_id, family_id,
                   used_at, revoked_at, expires_at
            FROM inbound.refresh_tokens
            WHERE token_hash = :hash
        """),
        {"hash": _hash(presented)},
    )
    record = row.mappings().first()
    if record is None:
        return None

    # Reuse. Whether replay or theft, the family is finished.
    if record["used_at"] is not None:
        await _revoke_family(conn, record["family_id"])
        return None

    if record["revoked_at"] is not None:
        return None
    if record["expires_at"] <= datetime.now(UTC):
        return None

    # Status is re-checked here, not taken from the stored row: a user revoked
    # from the desktop must not be able to refresh their way onward.
    status_row = await conn.execute(
        text("SELECT status FROM mirror.portal_users WHERE id = :uid"),
        {"uid": record["portal_user_id"]},
    )
    if status_row.scalar_one_or_none() != "Active":
        await _revoke_family(conn, record["family_id"])
        return None

    issued = await issue(
        conn,
        record["portal_user_id"],
        record["client_id"],
        family_id=record["family_id"],
    )

    await conn.execute(
        text("""
            UPDATE inbound.refresh_tokens
            SET used_at = now(), replaced_by = :new_hash
            WHERE id = :id
        """),
        {"id": record["id"], "new_hash": _hash(issued.token)},
    )

    return issued


async def revoke(conn: AsyncConnection, presented: str) -> bool:
    """Logout. Revokes the whole family, so every device from that login stops."""
    row = await conn.execute(
        text("SELECT family_id FROM inbound.refresh_tokens WHERE token_hash = :hash"),
        {"hash": _hash(presented)},
    )
    family_id = row.scalar_one_or_none()
    if family_id is None:
        return False

    await _revoke_family(conn, family_id)
    return True
