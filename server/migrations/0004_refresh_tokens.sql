-- 0004 — refresh tokens.
--
-- Access tokens last 15 minutes so that revoking a portal user from the desktop
-- bites quickly. Without a refresh mechanism that would mean asking a client to
-- read an emailed code every quarter of an hour, so the short access token is
-- paired with a long-lived rotating refresh token.
--
-- ROTATION AND REUSE DETECTION
--
-- Each refresh mints a new token and marks the old one used, chained by
-- `replaced_by`. A refresh token presented twice means either a replay or a
-- stolen token that the legitimate client has since rotated past — and since
-- nothing can tell those apart, the whole chain is revoked and the client logs
-- in again. Losing a session is a small cost; leaving a thief with a live one
-- is not.
--
-- Only the SHA-256 of the token is stored. A dump of this table is not a set of
-- working credentials, and unlike the OTP codes these are high-entropy random
-- values, so a plain hash is sufficient and bcrypt's cost would be wasted on
-- every refresh.

CREATE TABLE inbound.refresh_tokens (
    id             TEXT PRIMARY KEY,
    portal_user_id TEXT NOT NULL REFERENCES mirror.portal_users(id) ON DELETE CASCADE,
    client_id      TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,

    token_hash     TEXT NOT NULL UNIQUE,
    -- Every token minted from one login shares a family. Reuse kills the family,
    -- not merely the token presented.
    family_id      TEXT NOT NULL,

    expires_at     TIMESTAMPTZ NOT NULL,
    used_at        TIMESTAMPTZ,
    revoked_at     TIMESTAMPTZ,
    replaced_by    TEXT,

    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_refresh_tokens_hash   ON inbound.refresh_tokens(token_hash);
CREATE INDEX idx_refresh_tokens_family ON inbound.refresh_tokens(family_id);
CREATE INDEX idx_refresh_tokens_expiry ON inbound.refresh_tokens(expires_at);

-- The auth role owns this table's lifecycle. portal_reader and portal_writer
-- get nothing: a session credential is not client data, and the connection that
-- serves matters has no business reading one.
GRANT SELECT, INSERT, UPDATE, DELETE ON inbound.refresh_tokens TO portal_auth;

-- No RLS. portal_auth is the only role with any grant here, and rows are keyed
-- by a hash the holder must already possess.
