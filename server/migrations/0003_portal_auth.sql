-- 0003 — the auth role.
--
-- 0002 gave the portal two roles, and neither can log a client in:
--
--   portal_reader  SELECT on mirror.*, every row filtered by
--                  app.current_client_id — which is exactly what login has not
--                  established yet.
--   portal_writer  INSERT on the two client-authored inbound tables. No access
--                  to inbound.otp_challenges at all.
--
-- 0001 anticipated this: "Never client-readable — the portal role cannot SELECT
-- here, only the auth path may, via a dedicated function or elevated role."
-- This is that role.
--
-- portal_auth is deliberately narrow. It can resolve an email to a portal
-- identity and manage OTP challenges. It cannot read a single matter, deadline,
-- document or invoice — so a compromised auth connection yields the client
-- roster and nothing about the firm's work. Every request past login runs on
-- portal_reader with RLS engaged.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'portal_auth') THEN
        CREATE ROLE portal_auth NOLOGIN;
    END IF;
END
$$;

GRANT USAGE ON SCHEMA mirror  TO portal_auth;
GRANT USAGE ON SCHEMA inbound TO portal_auth;

-- The challenge lifecycle: issue, count an attempt, consume.
GRANT SELECT, INSERT, UPDATE, DELETE ON inbound.otp_challenges TO portal_auth;

-- Enough of portal_users to answer "is this email a live portal identity, and
-- which client is it?". Nothing else in mirror.* is granted.
GRANT SELECT ON mirror.portal_users TO portal_auth;
-- The desktop stamps last_login_at, but the portal is what observes the login.
GRANT UPDATE (last_login_at) ON mirror.portal_users TO portal_auth;

-- portal_users carries FORCE ROW LEVEL SECURITY from 0002, so a grant alone
-- gives portal_auth nothing. Login cannot be scoped by client_id — establishing
-- the client_id IS the point of logging in — so this policy is unscoped by
-- necessity, and the narrowness of the grants above is what contains it.
CREATE POLICY portal_auth_lookup ON mirror.portal_users
    FOR SELECT TO portal_auth
    USING (true);

CREATE POLICY portal_auth_touch_login ON mirror.portal_users
    FOR UPDATE TO portal_auth
    USING (true)
    WITH CHECK (true);

-- otp_challenges has no RLS: it holds no client-scoped rows, portal_auth is the
-- only role granted anything on it, and a challenge is keyed by an email that
-- has not yet been proven to belong to anyone.
