"""Notifications and profile."""

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import text

from app.auth.deps import CurrentClient, require_client
from app.db import queries
from app.db.session import client_scope
from app.models import NotificationOut, ProfileOut

router = APIRouter(tags=["misc"])


@router.get("/notifications", response_model=list[NotificationOut])
async def list_notifications(
    client: CurrentClient = Depends(require_client),
) -> list[NotificationOut]:
    async with client_scope(client.client_id) as conn:
        rows = await queries.list_notifications(conn, client.client_id)
    return [NotificationOut(**row) for row in rows]


@router.post("/notifications/{notification_id}/read", status_code=status.HTTP_204_NO_CONTENT)
async def mark_read(
    notification_id: str,
    client: CurrentClient = Depends(require_client),
) -> None:
    """Marking a notification read is the one mirror write a client can cause.

    It is not a write the portal performs: `portal_reader` has SELECT only, and
    that separation is worth more than the convenience of a read flag. The row
    is stamped by the desktop on the next sync, from the outbox entry this
    creates.
    """
    raise HTTPException(
        status.HTTP_501_NOT_IMPLEMENTED,
        "Read receipts arrive with the inbound notification queue (M5 Step 3d).",
    )


@router.get("/profile", response_model=ProfileOut)
async def get_profile(client: CurrentClient = Depends(require_client)) -> ProfileOut:
    async with client_scope(client.client_id) as conn:
        row = await queries.get_profile(conn, client.client_id, client.portal_user_id)

    if row is None:
        # The token verified, so this means the portal user vanished between
        # `require_client` and here. Refusing beats inventing a profile.
        raise HTTPException(status.HTTP_404_NOT_FOUND, "No such profile")

    return ProfileOut(**row)


@router.get("/health", include_in_schema=False)
async def health() -> dict[str, str]:
    """Liveness only. It touches no database and requires no auth, so it cannot
    become a way to probe whether the mirror is reachable."""
    return {"status": "ok"}


@router.get("/health/ready", include_in_schema=False)
async def ready() -> dict[str, str]:
    """Readiness: can the reader connection actually be opened?"""
    from app.db.session import reader_engine

    async with reader_engine().connect() as conn:
        await conn.execute(text("SELECT 1"))
    return {"status": "ready"}
