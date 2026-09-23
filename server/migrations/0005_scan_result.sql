-- 0005 — let the portal record its own scan result.
--
-- Module 05 Step 3d wires ClamAV into `POST /documents/upload`: the portal
-- scans the bytes it just quarantined and needs to write the verdict back
-- onto the row it inserted moments earlier, in the same request.
--
-- 0002_rls.sql granted `portal_writer` only SELECT and INSERT on
-- `inbound.client_uploads` — deliberately, so a compromised portal
-- credential could file uploads but never rewrite one already filed. A
-- blanket UPDATE grant would reopen that: this grants UPDATE on exactly the
-- one column the scan step needs to change, `scan_status`, and adds the
-- matching RLS policy so it is still scoped to the caller's own client.
--
-- `status`, `filename`, `object_key`, `sha256` and the rest stay
-- update-only by `sync_writer` — the desktop is still the only actor that
-- ever moves an upload to Ingested or Rejected.

GRANT UPDATE (scan_status) ON inbound.client_uploads TO portal_writer;

CREATE POLICY client_isolation_scan_update ON inbound.client_uploads
    FOR UPDATE TO portal_writer
    USING (client_id = current_setting('app.current_client_id', true))
    WITH CHECK (client_id = current_setting('app.current_client_id', true));
