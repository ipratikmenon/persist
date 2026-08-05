-- Migration 0002: Row-level security and role separation
-- Persist sync server — see specs/module-05-portal.md §8
--
-- Root CLAUDE.md: "Row-level security enforced at PostgreSQL level — not just
-- the ORM." This file is that enforcement. The portal API also filters by
-- client_id, but that is the second lock, not the first.
--
-- THE MODEL
--
--   portal_reader  SELECT only on mirror.*   — the portal's read connection.
--                  Structurally incapable of mutating the mirror.
--   portal_writer  INSERT only on the two client-authored inbound tables.
--                  Cannot read another client's rows, cannot touch mirror.*.
--   sync_writer    Full DML on both schemas — used only by the sync server over
--                  mTLS. Never exposed to the internet.
--
-- Neither portal role owns any table, so FORCE ROW LEVEL SECURITY applies to
-- them. (A table owner bypasses RLS unless FORCE is set; we set it anyway so an
-- ownership change cannot silently disable isolation.)
--
-- CLIENT CONTEXT
--
-- The API sets `app.current_client_id` per request with SET LOCAL, inside the
-- transaction, from the validated JWT. SET LOCAL is essential: a pooled
-- connection would otherwise carry one client's context into the next request.
-- current_setting(..., true) returns NULL when unset, and every policy compares
-- with `=`, so an unset context matches NOTHING rather than everything.

-- ---------------------------------------------------------------------------
-- Roles
-- ---------------------------------------------------------------------------

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'portal_reader') THEN
        CREATE ROLE portal_reader NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'portal_writer') THEN
        CREATE ROLE portal_writer NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'sync_writer') THEN
        CREATE ROLE sync_writer NOLOGIN;
    END IF;
END
$$;

GRANT USAGE ON SCHEMA mirror  TO portal_reader, portal_writer, sync_writer;
GRANT USAGE ON SCHEMA inbound TO portal_writer, sync_writer;

-- The portal reads the mirror and can do nothing else to it.
GRANT SELECT ON ALL TABLES IN SCHEMA mirror TO portal_reader;

-- The portal writes only client-authored items, and reads back only its own.
GRANT SELECT, INSERT ON inbound.client_uploads   TO portal_writer;
GRANT SELECT, INSERT ON inbound.invoice_disputes TO portal_writer;
-- Deliberately NOT granted on inbound.otp_challenges: the auth path uses a
-- separate privileged connection. A leaked portal credential must not be able
-- to read OTP hashes.

-- The sync server applies desktop pushes and acks inbound items.
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA mirror  TO sync_writer;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA inbound TO sync_writer;

-- ---------------------------------------------------------------------------
-- Enable RLS everywhere client data lives
-- ---------------------------------------------------------------------------

ALTER TABLE mirror.clients               ENABLE ROW LEVEL SECURITY;
ALTER TABLE mirror.portal_users          ENABLE ROW LEVEL SECURITY;
ALTER TABLE mirror.matters_public        ENABLE ROW LEVEL SECURITY;
ALTER TABLE mirror.deadlines_public      ENABLE ROW LEVEL SECURITY;
ALTER TABLE mirror.ip_assets_public      ENABLE ROW LEVEL SECURITY;
ALTER TABLE mirror.documents_shared      ENABLE ROW LEVEL SECURITY;
ALTER TABLE mirror.invoices_public       ENABLE ROW LEVEL SECURITY;
ALTER TABLE mirror.payments_public       ENABLE ROW LEVEL SECURITY;
ALTER TABLE mirror.client_notifications  ENABLE ROW LEVEL SECURITY;
ALTER TABLE inbound.client_uploads       ENABLE ROW LEVEL SECURITY;
ALTER TABLE inbound.invoice_disputes     ENABLE ROW LEVEL SECURITY;

ALTER TABLE mirror.clients               FORCE ROW LEVEL SECURITY;
ALTER TABLE mirror.portal_users          FORCE ROW LEVEL SECURITY;
ALTER TABLE mirror.matters_public        FORCE ROW LEVEL SECURITY;
ALTER TABLE mirror.deadlines_public      FORCE ROW LEVEL SECURITY;
ALTER TABLE mirror.ip_assets_public      FORCE ROW LEVEL SECURITY;
ALTER TABLE mirror.documents_shared      FORCE ROW LEVEL SECURITY;
ALTER TABLE mirror.invoices_public       FORCE ROW LEVEL SECURITY;
ALTER TABLE mirror.payments_public       FORCE ROW LEVEL SECURITY;
ALTER TABLE mirror.client_notifications  FORCE ROW LEVEL SECURITY;
ALTER TABLE inbound.client_uploads       FORCE ROW LEVEL SECURITY;
ALTER TABLE inbound.invoice_disputes     FORCE ROW LEVEL SECURITY;

-- ---------------------------------------------------------------------------
-- Policies — mirror.* : a client sees only its own rows
-- ---------------------------------------------------------------------------

-- mirror.clients keys on id rather than client_id.
CREATE POLICY client_isolation ON mirror.clients
    FOR SELECT TO portal_reader
    USING (id = current_setting('app.current_client_id', true));

CREATE POLICY client_isolation ON mirror.portal_users
    FOR SELECT TO portal_reader
    USING (client_id = current_setting('app.current_client_id', true));

CREATE POLICY client_isolation ON mirror.matters_public
    FOR SELECT TO portal_reader
    USING (client_id = current_setting('app.current_client_id', true));

CREATE POLICY client_isolation ON mirror.deadlines_public
    FOR SELECT TO portal_reader
    USING (client_id = current_setting('app.current_client_id', true));

CREATE POLICY client_isolation ON mirror.ip_assets_public
    FOR SELECT TO portal_reader
    USING (client_id = current_setting('app.current_client_id', true));

CREATE POLICY client_isolation ON mirror.documents_shared
    FOR SELECT TO portal_reader
    USING (client_id = current_setting('app.current_client_id', true));

CREATE POLICY client_isolation ON mirror.invoices_public
    FOR SELECT TO portal_reader
    USING (client_id = current_setting('app.current_client_id', true));

CREATE POLICY client_isolation ON mirror.payments_public
    FOR SELECT TO portal_reader
    USING (client_id = current_setting('app.current_client_id', true));

CREATE POLICY client_isolation ON mirror.client_notifications
    FOR SELECT TO portal_reader
    USING (client_id = current_setting('app.current_client_id', true));

-- ---------------------------------------------------------------------------
-- Policies — inbound.* : read own, and insert only as oneself
-- ---------------------------------------------------------------------------
-- WITH CHECK is what stops a client from filing an upload or a dispute under
-- another client's id. Without it, isolation would be read-only.

CREATE POLICY client_isolation_read ON inbound.client_uploads
    FOR SELECT TO portal_writer
    USING (client_id = current_setting('app.current_client_id', true));

CREATE POLICY client_isolation_write ON inbound.client_uploads
    FOR INSERT TO portal_writer
    WITH CHECK (client_id = current_setting('app.current_client_id', true));

CREATE POLICY client_isolation_read ON inbound.invoice_disputes
    FOR SELECT TO portal_writer
    USING (client_id = current_setting('app.current_client_id', true));

CREATE POLICY client_isolation_write ON inbound.invoice_disputes
    FOR INSERT TO portal_writer
    WITH CHECK (client_id = current_setting('app.current_client_id', true));

-- ---------------------------------------------------------------------------
-- Policies — sync_writer bypasses client scoping
-- ---------------------------------------------------------------------------
-- The sync server holds no client context; it applies desktop pushes across all
-- clients. It reaches PostgreSQL only from the desktop over mTLS.

CREATE POLICY sync_full_access ON mirror.clients              FOR ALL TO sync_writer USING (true) WITH CHECK (true);
CREATE POLICY sync_full_access ON mirror.portal_users         FOR ALL TO sync_writer USING (true) WITH CHECK (true);
CREATE POLICY sync_full_access ON mirror.matters_public       FOR ALL TO sync_writer USING (true) WITH CHECK (true);
CREATE POLICY sync_full_access ON mirror.deadlines_public     FOR ALL TO sync_writer USING (true) WITH CHECK (true);
CREATE POLICY sync_full_access ON mirror.ip_assets_public     FOR ALL TO sync_writer USING (true) WITH CHECK (true);
CREATE POLICY sync_full_access ON mirror.documents_shared     FOR ALL TO sync_writer USING (true) WITH CHECK (true);
CREATE POLICY sync_full_access ON mirror.invoices_public      FOR ALL TO sync_writer USING (true) WITH CHECK (true);
CREATE POLICY sync_full_access ON mirror.payments_public      FOR ALL TO sync_writer USING (true) WITH CHECK (true);
CREATE POLICY sync_full_access ON mirror.client_notifications FOR ALL TO sync_writer USING (true) WITH CHECK (true);
CREATE POLICY sync_full_access ON inbound.client_uploads      FOR ALL TO sync_writer USING (true) WITH CHECK (true);
CREATE POLICY sync_full_access ON inbound.invoice_disputes    FOR ALL TO sync_writer USING (true) WITH CHECK (true);

-- ---------------------------------------------------------------------------
-- Future tables inherit the grants, not the policies
-- ---------------------------------------------------------------------------
-- A new mirror table gets SELECT for portal_reader automatically, but has NO
-- policy until someone writes one — and with RLS enabled and no policy, it
-- returns zero rows. That is the safe direction: a forgotten policy hides data
-- rather than exposing it. Enabling RLS on new tables is still required and is
-- asserted by tests/rls_test.sql.

ALTER DEFAULT PRIVILEGES IN SCHEMA mirror  GRANT SELECT ON TABLES TO portal_reader;
ALTER DEFAULT PRIVILEGES IN SCHEMA mirror  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO sync_writer;
ALTER DEFAULT PRIVILEGES IN SCHEMA inbound GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO sync_writer;
