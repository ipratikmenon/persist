"""Test fixtures.

These run against a real PostgreSQL mirror. There is no in-memory substitute:
the thing under test is row-level security, which SQLite does not have and a
mock would simply agree with.

    createdb persist_portal_test
    psql persist_portal_test -f server/migrations/0001_mirror.sql
    psql persist_portal_test -f server/migrations/0002_rls.sql
    psql persist_portal_test -f server/migrations/0003_portal_auth.sql
    psql persist_portal_test -f server/migrations/0004_refresh_tokens.sql

Then point the four PORTAL_TEST_* variables at it. Without them the suite skips
rather than passing vacuously.
"""

import os
import subprocess
import tempfile
from pathlib import Path

import pytest

REQUIRED_ENV = [
    "PORTAL_TEST_READER_DSN",
    "PORTAL_TEST_WRITER_DSN",
    "PORTAL_TEST_AUTH_DSN",
    "PORTAL_TEST_ADMIN_DSN",
]

pytestmark = pytest.mark.skipif(
    not all(os.getenv(v) for v in REQUIRED_ENV),
    reason="set PORTAL_TEST_*_DSN to run the portal suite against PostgreSQL",
)


def _generate_keypair(directory: Path) -> tuple[Path, Path]:
    private = directory / "jwt-private.pem"
    public = directory / "jwt-public.pem"
    subprocess.run(
        ["openssl", "genpkey", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:2048",
         "-out", str(private)],
        check=True, capture_output=True,
    )
    subprocess.run(
        ["openssl", "rsa", "-pubout", "-in", str(private), "-out", str(public)],
        check=True, capture_output=True,
    )
    return private, public


@pytest.fixture(scope="session", autouse=True)
def settings_env():
    if not all(os.getenv(v) for v in REQUIRED_ENV):
        pytest.skip("portal test database not configured")

    tmp = tempfile.TemporaryDirectory()
    private, public = _generate_keypair(Path(tmp.name))

    os.environ.update({
        "PORTAL_READER_DSN": os.environ["PORTAL_TEST_READER_DSN"],
        "PORTAL_WRITER_DSN": os.environ["PORTAL_TEST_WRITER_DSN"],
        "PORTAL_AUTH_DSN": os.environ["PORTAL_TEST_AUTH_DSN"],
        "PORTAL_JWT_PRIVATE_KEY_PATH": str(private),
        "PORTAL_JWT_PUBLIC_KEY_PATH": str(public),
        "PORTAL_OBJECT_BASE_URL": "https://objects.persistas.test",
        "PORTAL_STORAGE_SIGNING_KEY": "test-signing-key-not-a-real-one",
        "PORTAL_EXPOSE_OTP_FOR_TESTS": "true",
        "PORTAL_ALLOWED_ORIGINS": '["https://app.persistas.test"]',
    })

    from app.config import get_settings
    get_settings.cache_clear()

    yield
    tmp.cleanup()


# Two clients, so "the other client's rows" is a real thing and not a null case.
CLIENT_A = "test-client-a"
CLIENT_B = "test-client-b"
USER_A = "test-user-a"
USER_B = "test-user-b"
EMAIL_A = "anita@client-a.test"
EMAIL_B = "ravi@client-b.test"


@pytest.fixture(autouse=True)
async def seeded_mirror(settings_env):
    """A clean two-client mirror per test, written as the owner so RLS does not
    interfere with setup. Torn down afterwards, so a leak in one test cannot
    look like a pass in the next."""
    from sqlalchemy import text
    from sqlalchemy.ext.asyncio import create_async_engine

    admin = create_async_engine(
        os.environ["PORTAL_TEST_ADMIN_DSN"],
        connect_args={"statement_cache_size": 0},
    )

    async with admin.begin() as conn:
        await conn.execute(text("TRUNCATE mirror.clients CASCADE"))
        await conn.execute(text("TRUNCATE inbound.otp_challenges"))
        await conn.execute(text("TRUNCATE inbound.refresh_tokens"))

        await conn.execute(text("""
            INSERT INTO mirror.clients (id, name) VALUES
                (:a, 'Client A Botanicals'), (:b, 'Client B Foods')
        """), {"a": CLIENT_A, "b": CLIENT_B})

        await conn.execute(text("""
            INSERT INTO mirror.portal_users (id, client_id, full_name, email, status) VALUES
                (:ua, :a, 'Anita Rao', :ea, 'Active'),
                (:ub, :b, 'Ravi Menon', :eb, 'Active')
        """), {"ua": USER_A, "a": CLIENT_A, "ea": EMAIL_A,
               "ub": USER_B, "b": CLIENT_B, "eb": EMAIL_B})

        await conn.execute(text("""
            INSERT INTO mirror.matters_public
                (id, client_id, title, matter_type, status, opened_date,
                 jurisdiction, client_notes, responsible_attorney) VALUES
                ('M-A-1', :a, 'PETALVEDA', 'Trademark', 'Active', DATE '2026-01-10',
                 'India', 'Filed and awaiting examination', 'Sree Lakshmi Menon'),
                ('M-B-1', :b, 'SECRET-B-MATTER', 'Patent', 'Active', DATE '2026-02-11',
                 'India', 'B only', 'Kajal Thakur')
        """), {"a": CLIENT_A, "b": CLIENT_B})

        await conn.execute(text("""
            INSERT INTO mirror.deadlines_public
                (id, client_id, matter_id, docketing_event, due_date, status) VALUES
                ('D-A-1', :a, 'M-A-1', 'Reply to Examination Report', DATE '2026-11-30', 'Pending'),
                ('D-B-1', :b, 'M-B-1', 'SECRET-B-DEADLINE', DATE '2026-12-01', 'Pending')
        """), {"a": CLIENT_A, "b": CLIENT_B})

        await conn.execute(text("""
            INSERT INTO mirror.documents_shared
                (id, client_id, matter_id, filename, category, mime_type,
                 file_size_bytes, object_key, sha256) VALUES
                ('DOC-A-1', :a, 'M-A-1', 'Examination-Report.pdf', 'Official',
                 'application/pdf', 12345, 'vault/a/doc-a-1', 'aaa'),
                ('DOC-B-1', :b, 'M-B-1', 'SECRET-B-DOC.pdf', 'Official',
                 'application/pdf', 999, 'vault/b/doc-b-1', 'bbb')
        """), {"a": CLIENT_A, "b": CLIENT_B})

        await conn.execute(text("""
            INSERT INTO mirror.invoices_public
                (id, client_id, status, invoice_date, subtotal, cgst_amount,
                 sgst_amount, total_with_tax, amount_paid, pdf_object_key) VALUES
                ('INV-A-1', :a, 'Sent', DATE '2026-08-01', 70000.01, 6300.00,
                 6300.00, 82600.01, 10000.00, 'vault/a/inv-a-1.pdf'),
                ('INV-B-1', :b, 'Sent', DATE '2026-08-02', 5000.00, 450.00,
                 450.00, 5900.00, 0.00, 'vault/b/inv-b-1.pdf')
        """), {"a": CLIENT_A, "b": CLIENT_B})

        await conn.execute(text("""
            INSERT INTO mirror.payments_public
                (id, client_id, invoice_id, amount, payment_date, method) VALUES
                ('PAY-A-1', :a, 'INV-A-1', 10000.00, DATE '2026-08-05', 'NEFT')
        """), {"a": CLIENT_A})

        await conn.execute(text("""
            INSERT INTO mirror.client_notifications (id, client_id, kind, title) VALUES
                ('N-A-1', :a, 'DocumentShared', 'A document is ready'),
                ('N-B-1', :b, 'DocumentShared', 'SECRET-B-NOTIFICATION')
        """), {"a": CLIENT_A, "b": CLIENT_B})

    await admin.dispose()

    # The limits are real and tested separately. Left in place across tests they
    # would make every suite order-dependent, since all requests share one IP.
    from app.rate_limit import limiter
    limiter.reset()

    yield

    from app.db.session import dispose_engines
    await dispose_engines()


@pytest.fixture
async def client():
    """An httpx client bound to the ASGI app — no socket, no uvicorn."""
    from httpx import ASGITransport, AsyncClient

    from app.main import app

    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="https://portal.test",
    ) as c:
        yield c


async def login(client, email: str) -> str:
    """Full OTP round trip. Returns an access token."""
    requested = await client.post("/auth/request-otp", json={"email": email})
    assert requested.status_code == 202
    code = requested.json()["debug_code"]
    assert code, "the test settings must expose the code"

    verified = await client.post("/auth/verify-otp", json={"email": email, "code": code})
    assert verified.status_code == 200, verified.text
    return verified.json()["access_token"]


def bearer(token: str) -> dict[str, str]:
    return {"Authorization": f"Bearer {token}"}
