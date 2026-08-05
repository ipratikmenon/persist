-- RLS acceptance gate — specs/module-05-portal.md §8
--
-- "test_rls_blocks_cross_client_read ... is the module's acceptance gate; it
--  must exist before any portal endpoint is written."
--
-- Seeds two clients, assumes the identity of the portal roles, and asserts that
-- one client can never see the other's rows — querying every mirror table
-- UNFILTERED, exactly as a buggy or malicious API would.
--
-- Run:  server/scripts/test-rls.sh
-- Any failed assertion raises an exception, so psql exits non-zero.

\set ON_ERROR_STOP on

BEGIN;

-- ---------------------------------------------------------------------------
-- Seed: two clients with a full complement of rows each
-- ---------------------------------------------------------------------------

INSERT INTO mirror.clients (id, name) VALUES
    ('client-a', 'Alpha Innovations Pvt Ltd'),
    ('client-b', 'Beta Traders LLP');

INSERT INTO mirror.portal_users (id, client_id, full_name, email, status) VALUES
    ('pu-a', 'client-a', 'Anita Rao',  'anita@alpha.example',  'Active'),
    ('pu-b', 'client-b', 'Bharat Shah', 'bharat@beta.example', 'Active');

INSERT INTO mirror.matters_public
    (id, client_id, title, matter_type, status, opened_date, client_notes) VALUES
    ('M-A', 'client-a', 'ALPHAMARK trademark', 'Trademark', 'Active', '2026-01-10', 'Alpha only'),
    ('M-B', 'client-b', 'BETAMARK trademark',  'Trademark', 'Active', '2026-02-11', 'Beta only');

INSERT INTO mirror.deadlines_public (id, client_id, matter_id, docketing_event, due_date, status) VALUES
    ('D-A', 'client-a', 'M-A', 'Examination response', '2026-09-01', 'Pending'),
    ('D-B', 'client-b', 'M-B', 'Examination response', '2026-09-02', 'Pending');

INSERT INTO mirror.ip_assets_public (id, client_id, matter_id, asset_type, title, status) VALUES
    ('IP-A', 'client-a', 'M-A', 'Trademark', 'ALPHAMARK', 'Pending'),
    ('IP-B', 'client-b', 'M-B', 'Trademark', 'BETAMARK',  'Pending');

INSERT INTO mirror.documents_shared
    (id, client_id, matter_id, filename, category, mime_type, file_size_bytes, object_key, sha256) VALUES
    ('DOC-A', 'client-a', 'M-A', 'alpha-cert.pdf', 'Certificate', 'application/pdf', 1000, 'k/alpha', 'sha-a'),
    ('DOC-B', 'client-b', 'M-B', 'beta-cert.pdf',  'Certificate', 'application/pdf', 1000, 'k/beta',  'sha-b');

INSERT INTO mirror.invoices_public (id, client_id, status, invoice_date, total_with_tax) VALUES
    ('INV-A', 'client-a', 'Sent', '2026-03-01', 11800.00),
    ('INV-B', 'client-b', 'Sent', '2026-03-02', 23600.00);

INSERT INTO mirror.payments_public (id, client_id, invoice_id, amount, payment_date, method) VALUES
    ('PAY-A', 'client-a', 'INV-A', 11800.00, '2026-03-15', 'NEFT'),
    ('PAY-B', 'client-b', 'INV-B', 23600.00, '2026-03-16', 'NEFT');

INSERT INTO mirror.client_notifications (id, client_id, kind, title) VALUES
    ('N-A', 'client-a', 'InvoiceIssued', 'Invoice INV-A issued'),
    ('N-B', 'client-b', 'InvoiceIssued', 'Invoice INV-B issued');

INSERT INTO inbound.client_uploads
    (id, client_id, portal_user_id, filename, mime_type, file_size_bytes, object_key, sha256) VALUES
    ('UP-A', 'client-a', 'pu-a', 'poa-alpha.pdf', 'application/pdf', 500, 'q/alpha', 'sha-ua'),
    ('UP-B', 'client-b', 'pu-b', 'poa-beta.pdf',  'application/pdf', 500, 'q/beta',  'sha-ub');

INSERT INTO inbound.invoice_disputes (id, client_id, portal_user_id, invoice_id, reason) VALUES
    ('DIS-A', 'client-a', 'pu-a', 'INV-A', 'Alpha query'),
    ('DIS-B', 'client-b', 'pu-b', 'INV-B', 'Beta query');

-- ---------------------------------------------------------------------------
-- Test 1 — a client reading the mirror sees ONLY its own rows
-- ---------------------------------------------------------------------------

SET LOCAL ROLE portal_reader;
SET LOCAL app.current_client_id = 'client-a';

DO $$
DECLARE
    tbl      TEXT;
    foreign_rows INTEGER;
    own_rows     INTEGER;
BEGIN
    FOREACH tbl IN ARRAY ARRAY[
        'mirror.portal_users',
        'mirror.matters_public',
        'mirror.deadlines_public',
        'mirror.ip_assets_public',
        'mirror.documents_shared',
        'mirror.invoices_public',
        'mirror.payments_public',
        'mirror.client_notifications'
    ]
    LOOP
        -- Deliberately UNFILTERED: this is what a buggy API would issue.
        EXECUTE format('SELECT count(*) FROM %s WHERE client_id <> %L', tbl, 'client-a')
            INTO foreign_rows;
        IF foreign_rows <> 0 THEN
            RAISE EXCEPTION 'RLS LEAK: % returned % row(s) belonging to another client', tbl, foreign_rows;
        END IF;

        EXECUTE format('SELECT count(*) FROM %s', tbl) INTO own_rows;
        IF own_rows <> 1 THEN
            RAISE EXCEPTION 'RLS TOO STRICT: % returned % own row(s), expected 1', tbl, own_rows;
        END IF;
    END LOOP;

    -- mirror.clients keys on id, not client_id.
    SELECT count(*) INTO foreign_rows FROM mirror.clients WHERE id <> 'client-a';
    IF foreign_rows <> 0 THEN
        RAISE EXCEPTION 'RLS LEAK: mirror.clients exposed another client';
    END IF;

    RAISE NOTICE 'PASS: cross-client read blocked on all mirror tables';
END
$$;

-- ---------------------------------------------------------------------------
-- Test 2 — switching client context switches the visible rows
-- ---------------------------------------------------------------------------

SET LOCAL app.current_client_id = 'client-b';

DO $$
DECLARE
    t TEXT;
BEGIN
    SELECT title INTO t FROM mirror.matters_public;
    IF t IS DISTINCT FROM 'BETAMARK trademark' THEN
        RAISE EXCEPTION 'context switch failed: expected Beta''s matter, got %', COALESCE(t, '<none>');
    END IF;
    RAISE NOTICE 'PASS: client context switch isolates correctly';
END
$$;

-- ---------------------------------------------------------------------------
-- Test 3 — NO client context means NO rows (never "all rows")
-- ---------------------------------------------------------------------------
-- current_setting(..., true) is NULL when unset, and `client_id = NULL` is NULL,
-- so the policy matches nothing. A request that forgets to set context must
-- fail closed.

RESET app.current_client_id;

DO $$
DECLARE
    n INTEGER;
BEGIN
    SELECT count(*) INTO n FROM mirror.matters_public;
    IF n <> 0 THEN
        RAISE EXCEPTION 'FAIL-OPEN: unset client context exposed % row(s)', n;
    END IF;
    RAISE NOTICE 'PASS: unset context returns zero rows';
END
$$;

-- ---------------------------------------------------------------------------
-- Test 4 — the portal cannot mutate the mirror
-- ---------------------------------------------------------------------------

SET LOCAL app.current_client_id = 'client-a';

DO $$
BEGIN
    BEGIN
        EXECUTE $q$UPDATE mirror.invoices_public SET amount_paid = 999999 WHERE id = 'INV-A'$q$;
        RAISE EXCEPTION 'PRIVILEGE ESCALATION: portal_reader updated the mirror';
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'PASS: portal_reader cannot UPDATE the mirror';
    END;

    BEGIN
        EXECUTE $q$DELETE FROM mirror.documents_shared WHERE id = 'DOC-A'$q$;
        RAISE EXCEPTION 'PRIVILEGE ESCALATION: portal_reader deleted from the mirror';
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'PASS: portal_reader cannot DELETE from the mirror';
    END;

    BEGIN
        EXECUTE $q$INSERT INTO mirror.matters_public
                   (id, client_id, title, matter_type, status, opened_date)
                   VALUES ('M-X','client-a','Injected','Trademark','Active','2026-01-01')$q$;
        RAISE EXCEPTION 'PRIVILEGE ESCALATION: portal_reader inserted into the mirror';
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'PASS: portal_reader cannot INSERT into the mirror';
    END;
END
$$;

-- ---------------------------------------------------------------------------
-- Test 5 — OTP hashes are unreachable from the portal roles
-- ---------------------------------------------------------------------------

DO $$
BEGIN
    BEGIN
        EXECUTE $q$SELECT count(*) FROM inbound.otp_challenges$q$;
        RAISE EXCEPTION 'LEAK: portal_reader can read OTP challenge hashes';
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'PASS: OTP challenges unreachable from portal_reader';
    END;
END
$$;

-- ---------------------------------------------------------------------------
-- Test 6 — a client cannot file inbound items as another client
-- ---------------------------------------------------------------------------

RESET ROLE;
SET LOCAL ROLE portal_writer;
SET LOCAL app.current_client_id = 'client-a';

DO $$
DECLARE
    n INTEGER;
BEGIN
    -- Reads are scoped.
    SELECT count(*) INTO n FROM inbound.client_uploads WHERE client_id <> 'client-a';
    IF n <> 0 THEN
        RAISE EXCEPTION 'RLS LEAK: inbound.client_uploads exposed % foreign row(s)', n;
    END IF;

    SELECT count(*) INTO n FROM inbound.invoice_disputes WHERE client_id <> 'client-a';
    IF n <> 0 THEN
        RAISE EXCEPTION 'RLS LEAK: inbound.invoice_disputes exposed % foreign row(s)', n;
    END IF;

    -- Writing as oneself is allowed.
    BEGIN
        EXECUTE $q$INSERT INTO inbound.invoice_disputes
                   (id, client_id, portal_user_id, invoice_id, reason)
                   VALUES ('DIS-OWN','client-a','pu-a','INV-A','legitimate')$q$;
        RAISE NOTICE 'PASS: client can file its own dispute';
    EXCEPTION WHEN OTHERS THEN
        RAISE EXCEPTION 'client blocked from filing its own dispute: %', SQLERRM;
    END;

    -- Writing as someone else must be refused by the WITH CHECK clause.
    BEGIN
        EXECUTE $q$INSERT INTO inbound.invoice_disputes
                   (id, client_id, portal_user_id, invoice_id, reason)
                   VALUES ('DIS-FORGED','client-b','pu-b','INV-B','forged')$q$;
        RAISE EXCEPTION 'SPOOFING: client-a filed a dispute as client-b';
    EXCEPTION WHEN insufficient_privilege OR check_violation THEN
        RAISE NOTICE 'PASS: cannot file inbound items as another client';
    END;
END
$$;

-- ---------------------------------------------------------------------------
-- Test 7 — every client-data table actually has RLS enabled and forced
-- ---------------------------------------------------------------------------
-- Guards the failure mode where a future migration adds a mirror table and
-- forgets to protect it.

RESET ROLE;

DO $$
DECLARE
    r RECORD;
    unprotected TEXT := '';
BEGIN
    FOR r IN
        SELECT c.relname, n.nspname, c.relrowsecurity, c.relforcerowsecurity
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname IN ('mirror', 'inbound')
          AND c.relkind = 'r'
          -- otp_challenges is protected by having no grant at all, not by RLS.
          AND NOT (n.nspname = 'inbound' AND c.relname = 'otp_challenges')
    LOOP
        IF NOT r.relrowsecurity OR NOT r.relforcerowsecurity THEN
            unprotected := unprotected || format('%s.%s ', r.nspname, r.relname);
        END IF;
    END LOOP;

    IF unprotected <> '' THEN
        RAISE EXCEPTION 'TABLES WITHOUT FORCED RLS: %', unprotected;
    END IF;
    RAISE NOTICE 'PASS: every mirror/inbound table has RLS enabled and forced';
END
$$;

-- Nothing is kept: this is a test, not a fixture.
ROLLBACK;
