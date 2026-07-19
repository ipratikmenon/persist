-- Phase 2 Module 4: Time Tracking & Billing
-- Tables: firm_settings, time_entries, invoices, invoice_line_items, payments

-- ---------------------------------------------------------------------------
-- firm_settings — single-row config. Always row id=1. Use UPSERT to update.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS firm_settings (
    id                  INTEGER PRIMARY KEY DEFAULT 1,
    firm_name           TEXT    NOT NULL DEFAULT 'Persistas & Partners',
    firm_gstin          TEXT,                              -- e.g. 07AABCP1234Z1
    firm_address        TEXT,                              -- multi-line, used on invoice
    firm_pan            TEXT,
    bank_name           TEXT,
    bank_account        TEXT,
    bank_ifsc           TEXT,
    default_hourly_rate REAL    NOT NULL DEFAULT 5000.0,   -- INR per hour
    partner_rate        REAL    NOT NULL DEFAULT 8000.0,
    associate_rate      REAL    NOT NULL DEFAULT 4000.0,
    paralegal_rate      REAL    NOT NULL DEFAULT 2000.0,
    gst_rate            REAL    NOT NULL DEFAULT 0.18,     -- 18% legal services
    updated_at          DATETIME NOT NULL DEFAULT (datetime('now'))
);

-- Seed default row so get_firm_settings never returns empty
INSERT OR IGNORE INTO firm_settings (id) VALUES (1);

-- ---------------------------------------------------------------------------
-- time_entries — billable and non-billable time per matter per attorney
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS time_entries (
    id             TEXT    PRIMARY KEY,
    matter_id      TEXT    NOT NULL REFERENCES matters(id),
    user_id        TEXT    NOT NULL REFERENCES users(id),
    date           DATE    NOT NULL,
    hours          REAL    NOT NULL CHECK (hours >= 0.25),   -- min 15 min
    description    TEXT    NOT NULL,
    activity_code  TEXT    NOT NULL DEFAULT 'L300'
        CHECK (activity_code IN ('L100','L200','L300','L400','L500','L600','L700','L800','L900')),
    rate_per_hour  REAL    NOT NULL,                         -- INR — snapshot at entry time
    is_billable    INTEGER NOT NULL DEFAULT 1,
    is_invoiced    INTEGER NOT NULL DEFAULT 0,               -- 1 = locked to invoice
    invoice_id     TEXT    REFERENCES invoices(id),          -- set when invoiced
    created_at     DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at     DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_time_entries_matter  ON time_entries(matter_id);
CREATE INDEX IF NOT EXISTS idx_time_entries_user    ON time_entries(user_id);
CREATE INDEX IF NOT EXISTS idx_time_entries_date    ON time_entries(date DESC);
CREATE INDEX IF NOT EXISTS idx_time_entries_invoice ON time_entries(invoice_id);

-- ---------------------------------------------------------------------------
-- invoices — GST tax invoices for clients
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS invoices (
    id              TEXT    PRIMARY KEY,              -- INV-YYYY-NNNN
    client_id       TEXT    NOT NULL REFERENCES clients(id),
    matter_ids      TEXT    NOT NULL DEFAULT '[]',    -- JSON array
    status          TEXT    NOT NULL DEFAULT 'Draft'
        CHECK (status IN ('Draft','Sent','Paid','PartiallyPaid','Cancelled')),
    invoice_date    DATE    NOT NULL,
    due_date        DATE,
    subtotal        REAL    NOT NULL DEFAULT 0.0,
    cgst_amount     REAL    NOT NULL DEFAULT 0.0,     -- 9% intra-state
    sgst_amount     REAL    NOT NULL DEFAULT 0.0,     -- 9% intra-state
    igst_amount     REAL    NOT NULL DEFAULT 0.0,     -- 18% inter-state
    total_with_tax  REAL    NOT NULL DEFAULT 0.0,
    amount_paid     REAL    NOT NULL DEFAULT 0.0,
    notes           TEXT,
    gst_type        TEXT    NOT NULL DEFAULT 'Intra'
        CHECK (gst_type IN ('Intra','Inter')),
    pdf_doc_id      TEXT    REFERENCES documents(id),  -- vault-stored PDF
    created_by      TEXT    NOT NULL REFERENCES users(id),
    created_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_invoices_client ON invoices(client_id);
CREATE INDEX IF NOT EXISTS idx_invoices_status ON invoices(status);
CREATE INDEX IF NOT EXISTS idx_invoices_date   ON invoices(invoice_date DESC);

-- ---------------------------------------------------------------------------
-- invoice_line_items — individual lines on an invoice
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS invoice_line_items (
    id              TEXT    PRIMARY KEY,
    invoice_id      TEXT    NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    time_entry_id   TEXT    REFERENCES time_entries(id),    -- NULL for fixed-fee
    description     TEXT    NOT NULL,
    activity_code   TEXT,
    hours           REAL,                                   -- NULL for fixed-fee
    rate            REAL    NOT NULL,                       -- INR per hour or fixed fee
    amount          REAL    NOT NULL,                       -- hours * rate or fixed fee
    sort_order      INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_line_items_invoice ON invoice_line_items(invoice_id);

-- ---------------------------------------------------------------------------
-- payments — partial or full payment records against invoices
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS payments (
    id            TEXT    PRIMARY KEY,
    invoice_id    TEXT    NOT NULL REFERENCES invoices(id),
    amount        REAL    NOT NULL CHECK (amount > 0),
    payment_date  DATE    NOT NULL,
    method        TEXT    NOT NULL DEFAULT 'BankTransfer'
        CHECK (method IN ('BankTransfer','Cheque','Cash','UPI','NEFT','RTGS')),
    reference     TEXT,                                     -- UTR / cheque no / UPI ref
    notes         TEXT,
    recorded_by   TEXT    NOT NULL REFERENCES users(id),
    created_at    DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_payments_invoice ON payments(invoice_id);
