"""The dependency every protected route hangs off.

`require_client` is the single place a request acquires a client id. Nothing
downstream may derive one any other way — not from a path parameter, not from a
body field, not from a header. That is what makes the RLS scope trustworthy: if
there were a second source, one of them would eventually be wrong.
"""

from dataclasses import dataclass

from fastapi import Depends, HTTPException, Request, status
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from sqlalchemy import text

from app.auth.tokens import decode_access_token
from app.db.session import auth_scope

_bearer = HTTPBearer(auto_error=False)


@dataclass(frozen=True)
class CurrentClient:
    portal_user_id: str
    client_id: str


_UNAUTHORISED = HTTPException(
    status_code=status.HTTP_401_UNAUTHORIZED,
    detail="Not authenticated",
    headers={"WWW-Authenticate": "Bearer"},
)


async def require_client(
    request: Request,
    credentials: HTTPAuthorizationCredentials | None = Depends(_bearer),
) -> CurrentClient:
    if credentials is None or credentials.scheme.lower() != "bearer":
        raise _UNAUTHORISED

    claims = decode_access_token(credentials.credentials)
    if claims is None:
        raise _UNAUTHORISED

    # Revocation must take effect at the next request, not at token expiry
    # (spec §15.10). This costs one indexed lookup per request, which is the
    # right price for an attorney being able to cut off access immediately.
    async with auth_scope() as conn:
        row = await conn.execute(
            text("""
                SELECT status, client_id
                FROM mirror.portal_users
                WHERE id = :uid
            """),
            {"uid": claims.portal_user_id},
        )
        record = row.mappings().first()

    if record is None or record["status"] != "Active":
        raise _UNAUTHORISED

    # A token whose `cid` disagrees with the database is refused rather than
    # reconciled. It should be impossible; if it happens, something is wrong
    # enough that serving the request is the wrong response.
    if record["client_id"] != claims.client_id:
        raise _UNAUTHORISED

    request.state.client_id = claims.client_id
    return CurrentClient(
        portal_user_id=claims.portal_user_id,
        client_id=claims.client_id,
    )
