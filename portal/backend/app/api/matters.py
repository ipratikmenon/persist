"""Matters, deadlines and IP assets — the client's view of their own work."""

from fastapi import APIRouter, Depends, HTTPException, Query, status

from app.auth.deps import CurrentClient, require_client
from app.db import queries
from app.db.session import client_scope
from app.models import DeadlineOut, MatterDetail, MatterSummary

router = APIRouter(tags=["matters"])


@router.get("/matters", response_model=list[MatterSummary])
async def list_matters(client: CurrentClient = Depends(require_client)) -> list[MatterSummary]:
    async with client_scope(client.client_id) as conn:
        rows = await queries.list_matters(conn, client.client_id)
    return [MatterSummary(**row) for row in rows]


@router.get("/matters/{matter_id}", response_model=MatterDetail)
async def get_matter(
    matter_id: str,
    client: CurrentClient = Depends(require_client),
) -> MatterDetail:
    async with client_scope(client.client_id) as conn:
        matter = await queries.get_matter(conn, client.client_id, matter_id)
        if matter is None:
            # 404, not 403. Another client's matter and a matter that does not
            # exist must be indistinguishable, or the response confirms which
            # ids are real.
            raise HTTPException(status.HTTP_404_NOT_FOUND, "No such matter")

        deadlines = await queries.list_deadlines(conn, client.client_id, matter_id)
        ip_assets = await queries.list_ip_assets(conn, client.client_id, matter_id)
        documents = await queries.list_documents(conn, client.client_id, matter_id)

    return MatterDetail(
        **matter,
        deadlines=deadlines,
        ip_assets=[_ip_asset(a) for a in ip_assets],
        documents=documents,
    )


def _ip_asset(row: dict) -> dict:
    # `classes` arrives as JSONB. The mirror stores a list of integers; anything
    # else is treated as absent rather than passed through to a client.
    classes = row.get("classes")
    row = dict(row)
    row["classes"] = classes if isinstance(classes, list) else []
    row["next_renewal_date"] = row.pop("expiry_date", None)
    return row


@router.get("/deadlines", response_model=list[DeadlineOut])
async def list_deadlines(
    matter_id: str | None = Query(default=None),
    client: CurrentClient = Depends(require_client),
) -> list[DeadlineOut]:
    async with client_scope(client.client_id) as conn:
        rows = await queries.list_deadlines(conn, client.client_id, matter_id)
    return [DeadlineOut(**row) for row in rows]
