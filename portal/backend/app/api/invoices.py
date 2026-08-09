"""Invoices, payments, and raising a query on a bill.

No line items. `time_entries` never leaves the desktop, so the HTML view shows
totals and the GST breakdown; the PDF is the authoritative document (spec §14).
"""

import uuid

from fastapi import APIRouter, Depends, HTTPException, status
from fastapi.responses import RedirectResponse

from app.auth.deps import CurrentClient, require_client
from app.db import queries
from app.db.session import client_scope, client_write_scope
from app.models import DisputeIn, DisputeOut, InvoiceDetail, InvoiceSummary
from app.storage import signed_url

router = APIRouter(tags=["invoices"])


@router.get("/invoices", response_model=list[InvoiceSummary])
async def list_invoices(client: CurrentClient = Depends(require_client)) -> list[InvoiceSummary]:
    async with client_scope(client.client_id) as conn:
        rows = await queries.list_invoices(conn, client.client_id)
    return [InvoiceSummary(**row) for row in rows]


@router.get("/invoices/{invoice_id}", response_model=InvoiceDetail)
async def get_invoice(
    invoice_id: str,
    client: CurrentClient = Depends(require_client),
) -> InvoiceDetail:
    async with client_scope(client.client_id) as conn:
        invoice = await queries.get_invoice(conn, client.client_id, invoice_id)
        if invoice is None:
            raise HTTPException(status.HTTP_404_NOT_FOUND, "No such invoice")
        payments = await queries.list_payments(conn, client.client_id, invoice_id)

    return InvoiceDetail(**invoice, payments=payments)


@router.get("/invoices/{invoice_id}/pdf")
async def invoice_pdf(
    invoice_id: str,
    client: CurrentClient = Depends(require_client),
) -> RedirectResponse:
    async with client_scope(client.client_id) as conn:
        object_key = await queries.get_invoice_pdf_key(conn, client.client_id, invoice_id)

    if object_key is None:
        # Either no such invoice, or the firm has not generated the PDF yet.
        # Both are "nothing to download here" from the client's side.
        raise HTTPException(status.HTTP_404_NOT_FOUND, "No PDF is available for that invoice")

    url, _ = signed_url(object_key)
    response = RedirectResponse(url, status_code=status.HTTP_302_FOUND)
    response.headers["Cache-Control"] = "no-store"
    return response


@router.post(
    "/invoices/{invoice_id}/dispute",
    response_model=DisputeOut,
    status_code=status.HTTP_202_ACCEPTED,
)
async def raise_dispute(
    invoice_id: str,
    body: DisputeIn,
    client: CurrentClient = Depends(require_client),
) -> DisputeOut:
    # The invoice must be one of theirs. RLS makes another client's invoice
    # invisible, so this reads as "no such invoice".
    async with client_scope(client.client_id) as conn:
        if await queries.get_invoice(conn, client.client_id, invoice_id) is None:
            raise HTTPException(status.HTTP_404_NOT_FOUND, "No such invoice")

    async with client_write_scope(client.client_id) as conn:
        row = await queries.insert_dispute(
            conn,
            dispute_id=str(uuid.uuid4()),
            client_id=client.client_id,
            portal_user_id=client.portal_user_id,
            invoice_id=invoice_id,
            reason=body.reason.strip(),
        )

    # 202, not 200: the firm has received the query, not resolved it.
    return DisputeOut(**row)
