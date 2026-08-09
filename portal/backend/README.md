# Persist Portal API

The client-facing half of Module 5. Serves each client their own rows, and only
their own.

## What protects a client's privilege here

Four independent locks, in order of how much they are trusted:

1. **PostgreSQL row-level security.** The request runs as `portal_reader`, which
   is not the table owner, on tables carrying `FORCE ROW LEVEL SECURITY`. Each
   transaction opens with `SET LOCAL app.current_client_id`. This is the
   guarantee — it holds even if every line of Python above it is wrong.
2. **Role separation.** Three connections, three roles. `portal_reader` has
   `SELECT` only on `mirror.*`; `portal_writer` has `INSERT` on exactly two
   inbound tables; `portal_auth` can resolve an email and manage OTP challenges
   and cannot read a single matter. The portal is structurally incapable of
   mutating the mirror.
3. **A `client_id` filter in every query.** The second lock. `test_rls_holds_
   with_the_api_filter_removed` drops it deliberately and asserts the first lock
   still holds.
4. **Pydantic response models**, `extra="forbid"`. A column added to the mirror
   cannot reach a client without someone adding it to a model on purpose — there
   is a test that adds one and proves it does not come out.

`SET LOCAL` rather than `SET` matters: the setting dies with the transaction, so
a pooled connection cannot carry one client's context into the next request.

## Running it

```bash
uv venv && uv pip install -e . --group dev

# Keys — RS256, per spec §10.
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out jwt-private.pem
openssl rsa -pubout -in jwt-private.pem -out jwt-public.pem

export PORTAL_READER_DSN="postgresql+asyncpg://portal_reader:...@host/persist_mirror"
export PORTAL_WRITER_DSN="postgresql+asyncpg://portal_writer:...@host/persist_mirror"
export PORTAL_AUTH_DSN="postgresql+asyncpg://portal_auth:...@host/persist_mirror"
export PORTAL_JWT_PRIVATE_KEY_PATH=jwt-private.pem
export PORTAL_JWT_PUBLIC_KEY_PATH=jwt-public.pem
export PORTAL_OBJECT_BASE_URL="https://objects.example"
export PORTAL_STORAGE_SIGNING_KEY="..."
export PORTAL_ALLOWED_ORIGINS='["https://app.persistas.com"]'

uvicorn app.main:app
```

Nothing has a development default. A portal that silently starts against the
wrong database, or with a guessable signing key, is worse than one that refuses
to start.

## Tests

They need a real PostgreSQL mirror. There is no in-memory substitute: the thing
under test is row-level security, which SQLite does not have and a mock would
simply agree with. Without the four `PORTAL_TEST_*` variables the suite skips
rather than passing vacuously.

```bash
createdb persist_portal_test
for m in 0001_mirror 0002_rls 0003_portal_auth 0004_refresh_tokens; do
  psql persist_portal_test -f ../../server/migrations/$m.sql
done

source .test-env    # the four PORTAL_TEST_*_DSN variables
pytest
```

**Verify the isolation tests can fail.** A green suite means nothing until you
have watched it go red:

```sql
ALTER TABLE mirror.matters_public DISABLE ROW LEVEL SECURITY;
```

`test_rls_holds_with_the_api_filter_removed` must fail. Re-enable it afterwards.

## Not built yet

- **Object storage.** `signed_url` produces a real HMAC signature over the key
  and expiry together, and `verify` is the reference check an edge performs —
  but no bucket is provisioned, so uploaded bytes are not yet written anywhere.
  The upload row is created regardless, so nothing a client sends is silently
  dropped; it sits `Pending`.
- **Virus scanning.** Uploads are recorded `scan_status = 'Pending'` and the
  desktop refuses to ingest anything else.
- **Email/SMS delivery.** `request-otp` issues and stores a challenge; nothing
  sends it yet. The code is never returned to the browser outside tests.
- **Read receipts** on notifications — `portal_reader` cannot write to `mirror.*`
  by design, so this needs an inbound queue rather than a direct update.
- **Rate-limit storage** is in-memory, so limits are per worker. Correct for one
  uvicorn process, wrong for several; Redis is the fix.
