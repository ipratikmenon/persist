"""Login. No passwords anywhere.

`request-otp` always returns 202. That is not politeness — it is the whole of
the anti-enumeration defence (spec §15.11), so it must hold for an unknown
address, an inactive user and a revoked one alike.

Every failure past that point returns the same 401 with the same message. Wrong
code, expired code, attempts exhausted, user revoked: telling them apart would
be a free oracle for anyone probing.
"""

import logging

from fastapi import APIRouter, Cookie, HTTPException, Request, Response, status
from slowapi.util import get_remote_address

from app.auth import otp, refresh
from app.auth.tokens import issue_access_token
from app.config import get_settings
from app.db.session import auth_scope
from app.models import RequestOtpIn, RequestOtpOut, TokenOut, VerifyOtpIn
from app.rate_limit import limiter

log = logging.getLogger(__name__)
router = APIRouter(prefix="/auth", tags=["auth"])

_REFUSED = HTTPException(
    status_code=status.HTTP_401_UNAUTHORIZED,
    detail="That code is not valid.",
    headers={"WWW-Authenticate": "Bearer"},
)


def _set_refresh_cookie(response: Response, token: str, max_age_days: int) -> None:
    response.set_cookie(
        refresh.COOKIE_NAME,
        token,
        max_age=max_age_days * 24 * 3600,
        httponly=True,   # unreachable from JavaScript, so XSS cannot lift it
        secure=True,     # HTTPS only
        samesite="lax",  # not sent on cross-site POSTs
        path="/auth",    # only the routes that need it ever see it
    )


def _otp_email_key(request: Request) -> str:
    """Keys the per-address limit.

    The body has not been parsed when slowapi evaluates this, so the address is
    read from the raw body cached by Starlette. Falling back to the IP means a
    malformed request is still limited rather than unlimited.
    """
    email = getattr(request.state, "otp_email", None)
    return f"otp:{email}" if email else get_remote_address(request)


@router.post("/request-otp", status_code=status.HTTP_202_ACCEPTED)
@limiter.limit("3/hour", key_func=_otp_email_key)
@limiter.limit("10/hour")
async def request_otp(
    request: Request,
    response: Response,  # slowapi writes the rate-limit headers here
    body: RequestOtpIn,
) -> RequestOtpOut:
    settings = get_settings()
    request.state.otp_email = body.email.strip().lower()  # keys the per-address limit

    async with auth_scope() as conn:
        code = await otp.request_otp(conn, body.email)

    if code is None:
        log.info("otp requested for an address with no active portal user")
        return RequestOtpOut()

    # TODO(M5 Step 3d): hand to the email sender. Until that exists nothing is
    # delivered — and the code is never returned to the browser or logged.
    log.info("otp issued (delivery not yet wired)")

    return RequestOtpOut(debug_code=code if settings.expose_otp_for_tests else None)


@router.post("/verify-otp")
@limiter.limit("10/hour")
async def verify_otp(request: Request, response: Response, body: VerifyOtpIn) -> TokenOut:
    settings = get_settings()

    # The refusal is raised *after* the transaction closes, never inside it.
    # Raising inside would roll the transaction back — and the attempt counter
    # with it, which would leave the code brute-forceable however low the cap.
    async with auth_scope() as conn:
        identity = await otp.verify_otp(conn, body.email, body.code)
        issued = (
            await refresh.issue(conn, identity.portal_user_id, identity.client_id)
            if identity is not None
            else None
        )

    if identity is None or issued is None:
        raise _REFUSED

    token, expires_in = issue_access_token(identity.portal_user_id, identity.client_id)
    _set_refresh_cookie(response, issued.token, settings.refresh_token_days)

    return TokenOut(access_token=token, expires_in=expires_in)


@router.post("/refresh")
@limiter.limit("60/hour")
async def refresh_token(
    request: Request,
    response: Response,
    persist_refresh: str | None = Cookie(default=None),
) -> TokenOut:
    settings = get_settings()
    if not persist_refresh:
        raise _REFUSED

    # Same reason as verify-otp: a reuse detected here revokes the family, and
    # that revocation must survive the 401 rather than roll back with it.
    async with auth_scope() as conn:
        rotated = await refresh.rotate(conn, persist_refresh)

    if rotated is None:
        # Also the path a detected reuse takes. The family is already revoked;
        # the client sees only that they must log in again.
        response.delete_cookie(refresh.COOKIE_NAME, path="/auth")
        raise _REFUSED

    token, expires_in = issue_access_token(rotated.portal_user_id, rotated.client_id)
    _set_refresh_cookie(response, rotated.token, settings.refresh_token_days)

    return TokenOut(access_token=token, expires_in=expires_in)


@router.post("/logout", status_code=status.HTTP_204_NO_CONTENT)
async def logout(
    response: Response,
    persist_refresh: str | None = Cookie(default=None),
) -> None:
    if persist_refresh:
        async with auth_scope() as conn:
            await refresh.revoke(conn, persist_refresh)

    # Cleared whether or not the token was known, so a stale cookie cannot
    # survive a logout.
    response.delete_cookie(refresh.COOKIE_NAME, path="/auth")
