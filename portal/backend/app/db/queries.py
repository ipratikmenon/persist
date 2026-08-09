"""Every SQL statement the portal runs.

Confined to one module on purpose, mirroring the desktop's rule that raw SQL
lives only in `db/queries/`. A query written inline in a route is a query nobody
reviews for scoping.

BELT AND BRACES (spec §9)

Row-level security is the guarantee: `client_scope` sets
`app.current_client_id`, and `portal_reader` cannot see a row belonging to
anyone else no matter what SQL runs. Every statement here *also* filters by
`client_id`. The redundancy is deliberate — and it is tested, by disabling one
lock and asserting the other still holds.
"""

from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncConnection

# ---------------------------------------------------------------------------
# Matters
# ---------------------------------------------------------------------------

_MATTER_SUMMARY_COLS = """
    id, title, matter_type, status, opened_date,
    next_deadline_date, next_deadline_event
"""


async def list_matters(conn: AsyncConnection, client_id: str) -> list[dict]:
    rows = await conn.execute(
        text(f"""
            SELECT {_MATTER_SUMMARY_COLS}
            FROM mirror.matters_public
            WHERE client_id = :cid
            ORDER BY opened_date DESC, id
        """),
        {"cid": client_id},
    )
    return [dict(r) for r in rows.mappings()]


async def get_matter(conn: AsyncConnection, client_id: str, matter_id: str) -> dict | None:
    row = await conn.execute(
        text(f"""
            SELECT {_MATTER_SUMMARY_COLS},
                   forum, jurisdiction, client_notes, responsible_attorney
            FROM mirror.matters_public
            WHERE client_id = :cid AND id = :mid
        """),
        {"cid": client_id, "mid": matter_id},
    )
    record = row.mappings().first()
    return dict(record) if record else None


# ---------------------------------------------------------------------------
# Deadlines
# ---------------------------------------------------------------------------


async def list_deadlines(
    conn: AsyncConnection, client_id: str, matter_id: str | None = None
) -> list[dict]:
    rows = await conn.execute(
        text("""
            SELECT id, matter_id, docketing_event, due_date, status
            FROM mirror.deadlines_public
            WHERE client_id = :cid
              AND (CAST(:mid AS text) IS NULL OR matter_id = :mid)
            ORDER BY due_date, id
        """),
        {"cid": client_id, "mid": matter_id},
    )
    return [dict(r) for r in rows.mappings()]


# ---------------------------------------------------------------------------
# IP assets
# ---------------------------------------------------------------------------


async def list_ip_assets(
    conn: AsyncConnection, client_id: str, matter_id: str | None = None
) -> list[dict]:
    rows = await conn.execute(
        text("""
            SELECT id, matter_id, asset_type, title, application_number,
                   registration_number, status, classes, expiry_date
            FROM mirror.ip_assets_public
            WHERE client_id = :cid
              AND (CAST(:mid AS text) IS NULL OR matter_id = :mid)
            ORDER BY title, id
        """),
        {"cid": client_id, "mid": matter_id},
    )
    return [dict(r) for r in rows.mappings()]


# ---------------------------------------------------------------------------
# Documents
#
# `object_key` and `sha256` are never selected into a response path. The key is
# fetched only by `get_document_object_key`, which exists to be handed straight
# to the signer.
# ---------------------------------------------------------------------------


async def list_documents(
    conn: AsyncConnection, client_id: str, matter_id: str | None = None
) -> list[dict]:
    rows = await conn.execute(
        text("""
            SELECT id, matter_id, filename, category, file_size_bytes, shared_at
            FROM mirror.documents_shared
            WHERE client_id = :cid
              AND (CAST(:mid AS text) IS NULL OR matter_id = :mid)
            ORDER BY shared_at DESC, id
        """),
        {"cid": client_id, "mid": matter_id},
    )
    return [dict(r) for r in rows.mappings()]


async def get_document_object_key(
    conn: AsyncConnection, client_id: str, document_id: str
) -> str | None:
    row = await conn.execute(
        text("""
            SELECT object_key
            FROM mirror.documents_shared
            WHERE client_id = :cid AND id = :did
        """),
        {"cid": client_id, "did": document_id},
    )
    return row.scalar_one_or_none()


# ---------------------------------------------------------------------------
# Invoices
#
# The mirror's CHECK already forbids a draft here — a draft must never reach the
# table at all. The API adds no status filter, because filtering would quietly
# paper over a projection bug instead of surfacing it.
# ---------------------------------------------------------------------------


async def list_invoices(conn: AsyncConnection, client_id: str) -> list[dict]:
    rows = await conn.execute(
        text("""
            SELECT id, status, invoice_date, due_date,
                   total_with_tax, amount_paid,
                   (total_with_tax - amount_paid) AS amount_due
            FROM mirror.invoices_public
            WHERE client_id = :cid
            ORDER BY invoice_date DESC, id
        """),
        {"cid": client_id},
    )
    return [dict(r) for r in rows.mappings()]


async def get_invoice(conn: AsyncConnection, client_id: str, invoice_id: str) -> dict | None:
    row = await conn.execute(
        text("""
            SELECT id, status, invoice_date, due_date,
                   subtotal, cgst_amount, sgst_amount, igst_amount,
                   total_with_tax, amount_paid,
                   (total_with_tax - amount_paid) AS amount_due,
                   notes
            FROM mirror.invoices_public
            WHERE client_id = :cid AND id = :iid
        """),
        {"cid": client_id, "iid": invoice_id},
    )
    record = row.mappings().first()
    return dict(record) if record else None


async def get_invoice_pdf_key(
    conn: AsyncConnection, client_id: str, invoice_id: str
) -> str | None:
    row = await conn.execute(
        text("""
            SELECT pdf_object_key
            FROM mirror.invoices_public
            WHERE client_id = :cid AND id = :iid
        """),
        {"cid": client_id, "iid": invoice_id},
    )
    return row.scalar_one_or_none()


async def list_payments(conn: AsyncConnection, client_id: str, invoice_id: str) -> list[dict]:
    rows = await conn.execute(
        text("""
            SELECT id, amount, payment_date, method
            FROM mirror.payments_public
            WHERE client_id = :cid AND invoice_id = :iid
            ORDER BY payment_date, id
        """),
        {"cid": client_id, "iid": invoice_id},
    )
    return [dict(r) for r in rows.mappings()]


# ---------------------------------------------------------------------------
# Notifications
# ---------------------------------------------------------------------------


async def list_notifications(conn: AsyncConnection, client_id: str) -> list[dict]:
    rows = await conn.execute(
        text("""
            SELECT id, kind, title, body, matter_id,
                   (read_at IS NOT NULL) AS is_read, created_at
            FROM mirror.client_notifications
            WHERE client_id = :cid
            ORDER BY created_at DESC, id
        """),
        {"cid": client_id},
    )
    return [dict(r) for r in rows.mappings()]


# ---------------------------------------------------------------------------
# Inbound — the only two tables a client may write to
# ---------------------------------------------------------------------------


async def insert_dispute(
    conn: AsyncConnection,
    *,
    dispute_id: str,
    client_id: str,
    portal_user_id: str,
    invoice_id: str,
    reason: str,
) -> dict:
    row = await conn.execute(
        text("""
            INSERT INTO inbound.invoice_disputes
                (id, client_id, portal_user_id, invoice_id, reason)
            VALUES (:id, :cid, :uid, :iid, :reason)
            RETURNING id, invoice_id, status, raised_at
        """),
        {
            "id": dispute_id,
            "cid": client_id,
            "uid": portal_user_id,
            "iid": invoice_id,
            "reason": reason,
        },
    )
    return dict(row.mappings().one())


async def insert_upload(
    conn: AsyncConnection,
    *,
    upload_id: str,
    client_id: str,
    portal_user_id: str,
    matter_id: str | None,
    filename: str,
    mime_type: str,
    file_size_bytes: int,
    object_key: str,
    sha256: str,
) -> dict:
    row = await conn.execute(
        text("""
            INSERT INTO inbound.client_uploads
                (id, client_id, portal_user_id, matter_id, filename, mime_type,
                 file_size_bytes, object_key, sha256)
            VALUES (:id, :cid, :uid, :mid, :filename, :mime, :size, :key, :sha)
            RETURNING id, filename, file_size_bytes, scan_status, status, uploaded_at
        """),
        {
            "id": upload_id,
            "cid": client_id,
            "uid": portal_user_id,
            "mid": matter_id,
            "filename": filename,
            "mime": mime_type,
            "size": file_size_bytes,
            "key": object_key,
            "sha": sha256,
        },
    )
    return dict(row.mappings().one())


async def list_uploads(conn: AsyncConnection, client_id: str) -> list[dict]:
    rows = await conn.execute(
        text("""
            SELECT id, filename, file_size_bytes, scan_status, status, uploaded_at
            FROM inbound.client_uploads
            WHERE client_id = :cid
            ORDER BY uploaded_at DESC, id
        """),
        {"cid": client_id},
    )
    return [dict(r) for r in rows.mappings()]


# ---------------------------------------------------------------------------
# Profile
# ---------------------------------------------------------------------------


async def get_profile(conn: AsyncConnection, client_id: str, portal_user_id: str) -> dict | None:
    row = await conn.execute(
        text("""
            SELECT u.id, u.full_name, u.email, u.phone, c.name AS client_name
            FROM mirror.portal_users u
            JOIN mirror.clients c ON c.id = u.client_id
            WHERE u.client_id = :cid AND u.id = :uid
        """),
        {"cid": client_id, "uid": portal_user_id},
    )
    record = row.mappings().first()
    return dict(record) if record else None
