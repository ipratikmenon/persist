"""JWT issue and verify.

RS256, per spec §10. `cid` is the claim that drives row-level security, so it is
the one claim that must never be taken on trust from anywhere but a verified
signature — never from a header, a query parameter or a request body.

Access tokens are short (15 minutes) because revocation from the desktop must
bite quickly. That is a floor, not the mechanism: `require_client` re-checks the
portal user's status on every request, so a revoked user is refused at the next
call rather than at expiry (spec §15.10).
"""

import uuid
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta

import jwt

from app.config import get_settings

ALGORITHM = "RS256"
ISSUER = "persist-portal"


@dataclass(frozen=True)
class Claims:
    portal_user_id: str
    client_id: str
    jti: str


def issue_access_token(portal_user_id: str, client_id: str) -> tuple[str, int]:
    """Returns (token, seconds until expiry)."""
    settings = get_settings()
    ttl = timedelta(minutes=settings.access_token_minutes)
    now = datetime.now(UTC)

    token = jwt.encode(
        {
            "sub": portal_user_id,
            "cid": client_id,
            "iss": ISSUER,
            "iat": now,
            "exp": now + ttl,
            "jti": str(uuid.uuid4()),
        },
        settings.jwt_private_key,
        algorithm=ALGORITHM,
    )
    return token, int(ttl.total_seconds())


def decode_access_token(token: str) -> Claims | None:
    """None on anything wrong — expired, wrong issuer, bad signature, missing
    claims. The caller turns that into a 401; distinguishing the cases for a
    client would only help someone probing."""
    settings = get_settings()
    try:
        payload = jwt.decode(
            token,
            settings.jwt_public_key,
            algorithms=[ALGORITHM],
            issuer=ISSUER,
            options={"require": ["sub", "cid", "exp", "iat", "jti"]},
        )
    except jwt.PyJWTError:
        return None

    sub, cid, jti = payload.get("sub"), payload.get("cid"), payload.get("jti")
    if not isinstance(sub, str) or not isinstance(cid, str) or not isinstance(jti, str):
        return None
    if not sub or not cid:
        return None

    return Claims(portal_user_id=sub, client_id=cid, jti=jti)
