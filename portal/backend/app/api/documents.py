"""Documents: download what the firm shared, upload what the firm asked for.

Downloads never expose an object key — the route redirects to a URL that stops
working in five minutes (spec §15.5).

Uploads are untrusted until scanned, hashed and ingested by the desktop
(spec §15.12). Nothing here writes to `mirror.*`; the row lands in
`inbound.client_uploads` and waits for an attorney.
"""

import asyncio
import hashlib
import logging
import uuid

from fastapi import (
    APIRouter,
    Depends,
    File,
    Form,
    HTTPException,
    Query,
    UploadFile,
    status,
)
from fastapi.responses import RedirectResponse

from app.auth.deps import CurrentClient, require_client
from app.config import get_settings
from app.db import queries
from app.db.session import client_scope, client_write_scope
from app.models import DocumentOut, UploadOut
from app.object_store import ObjectStoreError, put_object
from app.scanning import scan_bytes
from app.storage import signed_url

log = logging.getLogger(__name__)

router = APIRouter(tags=["documents"])


@router.get("/documents", response_model=list[DocumentOut])
async def list_documents(
    matter_id: str | None = Query(default=None),
    client: CurrentClient = Depends(require_client),
) -> list[DocumentOut]:
    async with client_scope(client.client_id) as conn:
        rows = await queries.list_documents(conn, client.client_id, matter_id)
    return [DocumentOut(**row) for row in rows]


@router.get("/documents/{document_id}/download")
async def download_document(
    document_id: str,
    client: CurrentClient = Depends(require_client),
) -> RedirectResponse:
    async with client_scope(client.client_id) as conn:
        object_key = await queries.get_document_object_key(conn, client.client_id, document_id)

    if object_key is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "No such document")

    url, _ = signed_url(object_key)
    response = RedirectResponse(url, status_code=status.HTTP_302_FOUND)
    # The redirect carries a live credential in its Location header. It must not
    # sit in a shared cache or a browser's back-forward store.
    response.headers["Cache-Control"] = "no-store"
    return response


@router.post(
    "/documents/upload",
    response_model=UploadOut,
    status_code=status.HTTP_202_ACCEPTED,
)
async def upload_document(
    file: UploadFile = File(...),
    matter_id: str | None = Form(default=None),
    client: CurrentClient = Depends(require_client),
) -> UploadOut:
    settings = get_settings()

    # Read once, in full, so the size and the hash describe the same bytes.
    # A streamed size check could be defeated by a lying Content-Length.
    contents = await file.read()
    if len(contents) == 0:
        raise HTTPException(status.HTTP_400_BAD_REQUEST, "That file is empty")
    if len(contents) > settings.max_upload_bytes:
        raise HTTPException(
            status.HTTP_413_CONTENT_TOO_LARGE,
            f"Files must be under {settings.max_upload_bytes // (1024 * 1024)} MB",
        )
    if file.content_type not in settings.allowed_upload_mime_types:
        raise HTTPException(
            status.HTTP_415_UNSUPPORTED_MEDIA_TYPE,
            "That file type cannot be accepted. Send a PDF, an image, or a Word document.",
        )

    # If the client attributed the upload to a matter, it must be one of theirs.
    # RLS makes a foreign matter invisible, so this reads as "no such matter".
    if matter_id:
        async with client_scope(client.client_id) as conn:
            if await queries.get_matter(conn, client.client_id, matter_id) is None:
                raise HTTPException(status.HTTP_404_NOT_FOUND, "No such matter")

    upload_id = str(uuid.uuid4())
    sha256 = hashlib.sha256(contents).hexdigest()

    # Quarantine. The desktop verifies this hash before anything reaches the
    # vault, so a swapped object between here and ingest does not go unnoticed.
    object_key = f"quarantine/{client.client_id}/{upload_id}"

    # Written before the row exists: a row that names an object never
    # actually stored is worse than refusing the upload outright, since the
    # desktop would later find nothing at that key when it tries to pull it.
    # An unconfigured store (dev/test, or a firm that hasn't provisioned one
    # yet) is not a failure — see object_store.py — and simply skips this.
    try:
        await asyncio.to_thread(put_object, object_key, contents, file.content_type)
    except ObjectStoreError as e:
        log.error("quarantine object storage failed: %s", e)
        raise HTTPException(
            status.HTTP_503_SERVICE_UNAVAILABLE,
            "Could not store that file right now. Try again shortly.",
        ) from e

    async with client_write_scope(client.client_id) as conn:
        row = await queries.insert_upload(
            conn,
            upload_id=upload_id,
            client_id=client.client_id,
            portal_user_id=client.portal_user_id,
            matter_id=matter_id,
            filename=_safe_filename(file.filename),
            mime_type=file.content_type,
            file_size_bytes=len(contents),
            object_key=object_key,
            sha256=sha256,
        )

        # An unconfigured scanner (same dev/test case as above) returns None
        # and leaves scan_status at its inserted default, Pending — never
        # waved through as Clean, per spec §15.12.
        scan_result = await scan_bytes(contents)
        if scan_result is not None:
            await queries.update_upload_scan_status(
                conn, client_id=client.client_id, upload_id=upload_id, scan_status=scan_result
            )
            row["scan_status"] = scan_result

    return UploadOut(**row)


@router.get("/uploads", response_model=list[UploadOut])
async def list_uploads(client: CurrentClient = Depends(require_client)) -> list[UploadOut]:
    """What the client has sent and where it has got to."""
    async with client_write_scope(client.client_id) as conn:
        rows = await queries.list_uploads(conn, client.client_id)
    return [UploadOut(**row) for row in rows]


def _safe_filename(raw: str | None) -> str:
    """A filename from a browser is untrusted input, not a path.

    Directory separators and leading dots are stripped so the name cannot
    traverse or hide, and it is truncated so it cannot be used to bloat a row.
    The name is display-only in any case: storage is keyed by UUID.
    """
    name = (raw or "upload").replace("\\", "/").rsplit("/", 1)[-1]
    name = name.lstrip(".").strip() or "upload"
    return name[:255]
