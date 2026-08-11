// Invoices — what is owed, what has been paid, and how to query a bill.
//
// No line items. `time_entries` never leaves the desktop, so the HTML view
// shows totals and the GST breakdown; the PDF is the authoritative document.
// Reconstructing a bill from data the client does not have would be a guess
// presented as a record.

import { useEffect, useState } from 'react';
import { Link, useParams } from 'react-router';
import { api, type InvoiceDetail, type InvoiceSummary } from '@/lib/api';
import {
  Button, Card, Empty, Loading, Notice, PageTitle, SectionLabel, StatusBadge,
  formatDate, inr, inputStyle,
} from '@/components/ui';
import { colors, fonts, fontSizes, spacing } from '@/design-system/tokens';

export function InvoicesList() {
  const [invoices, setInvoices] = useState<InvoiceSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.invoices
      .list()
      .then(setInvoices)
      .catch((e) => setError(e.message));
  }, []);

  if (error) return <Notice message={error} />;
  if (!invoices) return <Loading />;

  const outstanding = invoices.reduce(
    (sum, invoice) => sum + Number(invoice.amountDue),
    0,
  );

  return (
    <>
      <PageTitle subtitle="Issued invoices and their payment history.">Invoices</PageTitle>

      {invoices.length === 0 ? (
        <Empty>No invoices yet.</Empty>
      ) : (
        <>
          {outstanding > 0 && (
            <Card style={{ marginBottom: spacing[5] }}>
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'baseline',
                  fontFamily: fonts.ui,
                }}
              >
                <span style={{ fontSize: fontSizes.body, color: colors.textSecondary }}>
                  Total outstanding
                </span>
                <span
                  style={{
                    fontSize: fontSizes.displaySm,
                    fontWeight: 600,
                    color: colors.textPrimary,
                  }}
                >
                  {inr(outstanding.toFixed(2))}
                </span>
              </div>
            </Card>
          )}

          <div style={{ display: 'grid', gap: spacing[3] }}>
            {invoices.map((invoice) => (
              <Link
                key={invoice.id}
                to={`/invoices/${encodeURIComponent(invoice.id)}`}
                style={{ textDecoration: 'none' }}
              >
                <Card>
                  <div
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'flex-start',
                      gap: spacing[4],
                    }}
                  >
                    <div>
                      <div
                        style={{
                          fontFamily: fonts.mono,
                          fontSize: fontSizes.cardTitle,
                          color: colors.textPrimary,
                        }}
                      >
                        {invoice.id}
                      </div>
                      <div
                        style={{
                          marginTop: spacing[2],
                          fontFamily: fonts.ui,
                          fontSize: fontSizes.label,
                          color: colors.textSecondary,
                        }}
                      >
                        Issued {formatDate(invoice.invoiceDate)}
                        {invoice.dueDate && ` · due ${formatDate(invoice.dueDate)}`}
                      </div>
                    </div>

                    <div style={{ textAlign: 'right' }}>
                      <div
                        style={{
                          fontFamily: fonts.ui,
                          fontSize: fontSizes.cardTitle,
                          fontWeight: 600,
                          color: colors.textPrimary,
                        }}
                      >
                        {inr(invoice.amountDue)}
                      </div>
                      <div style={{ marginTop: spacing[2] }}>
                        <StatusBadge status={invoice.status} />
                      </div>
                    </div>
                  </div>
                </Card>
              </Link>
            ))}
          </div>
        </>
      )}
    </>
  );
}

export function InvoiceDetailPage() {
  const { id = '' } = useParams();
  const [invoice, setInvoice] = useState<InvoiceDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [querying, setQuerying] = useState(false);
  const [reason, setReason] = useState('');
  const [raised, setRaised] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.invoices
      .get(id)
      .then(setInvoice)
      .catch((e) => setError(e.message));
  }, [id]);

  if (error) return <Notice message={error} />;
  if (!invoice) return <Loading />;

  const submitQuery = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.invoices.dispute(id, reason.trim());
      setRaised(true);
      setQuerying(false);
      setReason('');
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Link
        to="/invoices"
        style={{
          fontFamily: fonts.ui,
          fontSize: fontSizes.label,
          color: colors.accentPrimary,
          textDecoration: 'none',
        }}
      >
        ← All invoices
      </Link>

      <div style={{ marginTop: spacing[3] }}>
        <PageTitle subtitle={`Issued ${formatDate(invoice.invoiceDate)}`}>
          {invoice.id}
        </PageTitle>
      </div>

      <Card style={{ marginBottom: spacing[5] }}>
        <Row label="Subtotal (before GST)" value={inr(invoice.subtotal)} />
        {Number(invoice.cgstAmount) > 0 && (
          <>
            <Row label="CGST" value={inr(invoice.cgstAmount)} muted />
            <Row label="SGST" value={inr(invoice.sgstAmount)} muted />
          </>
        )}
        {Number(invoice.igstAmount) > 0 && (
          <Row label="IGST" value={inr(invoice.igstAmount)} muted />
        )}
        <div
          style={{
            borderTop: `0.5px solid ${colors.border}`,
            marginTop: spacing[3],
            paddingTop: spacing[3],
          }}
        >
          <Row label="Total" value={inr(invoice.totalWithTax)} strong />
          <Row label="Paid" value={inr(invoice.amountPaid)} muted />
          <Row label="Amount due" value={inr(invoice.amountDue)} strong />
        </div>
      </Card>

      {invoice.payments.length > 0 && (
        <section style={{ marginBottom: spacing[5] }}>
          <SectionLabel>Payments received</SectionLabel>
          <Card style={{ padding: 0, overflow: 'hidden' }}>
            {invoice.payments.map((payment, index) => (
              <div
                key={payment.id}
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  padding: `${spacing[3]} ${spacing[5]}`,
                  borderTop: index === 0 ? 'none' : `0.5px solid ${colors.border}`,
                  fontFamily: fonts.ui,
                  fontSize: fontSizes.body,
                }}
              >
                <span style={{ color: colors.textSecondary }}>
                  {formatDate(payment.paymentDate)} · {payment.method}
                </span>
                <span style={{ color: colors.textPrimary }}>{inr(payment.amount)}</span>
              </div>
            ))}
          </Card>
        </section>
      )}

      {invoice.notes && (
        <Card style={{ marginBottom: spacing[5] }}>
          <SectionLabel>Notes</SectionLabel>
          <p
            style={{
              margin: 0,
              fontFamily: fonts.ui,
              fontSize: fontSizes.body,
              color: colors.textPrimary,
              lineHeight: 1.6,
            }}
          >
            {invoice.notes}
          </p>
        </Card>
      )}

      <div style={{ display: 'flex', gap: spacing[3], flexWrap: 'wrap' }}>
        <a href={api.invoices.pdfUrl(invoice.id)} style={{ textDecoration: 'none' }}>
          <Button variant="secondary">Download PDF</Button>
        </a>
        {!raised && !querying && (
          <Button variant="quiet" onClick={() => setQuerying(true)}>
            Raise a query
          </Button>
        )}
      </div>

      {raised && (
        <div style={{ marginTop: spacing[4] }}>
          <Notice
            tone="ok"
            message="Your query has reached us. Someone will come back to you shortly."
          />
        </div>
      )}

      {querying && (
        <Card style={{ marginTop: spacing[4] }}>
          <SectionLabel>What would you like us to look at?</SectionLabel>
          <textarea
            autoFocus
            rows={4}
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            placeholder="Tell us which part of the invoice you would like explained."
            style={{ ...inputStyle, resize: 'vertical', lineHeight: 1.5 }}
          />
          <p
            style={{
              margin: `${spacing[2]} 0 ${spacing[4]}`,
              fontFamily: fonts.ui,
              fontSize: 11,
              color: colors.textTertiary,
            }}
          >
            At least a sentence, so we know what to check.
          </p>
          <div style={{ display: 'flex', gap: spacing[3] }}>
            <Button onClick={submitQuery} disabled={busy || reason.trim().length < 10}>
              {busy ? 'Sending…' : 'Send query'}
            </Button>
            <Button variant="quiet" onClick={() => setQuerying(false)} disabled={busy}>
              Cancel
            </Button>
          </div>
        </Card>
      )}
    </>
  );
}

function Row({
  label,
  value,
  muted,
  strong,
}: {
  label: string;
  value: string;
  muted?: boolean;
  strong?: boolean;
}) {
  return (
    <div
      style={{
        display: 'flex',
        justifyContent: 'space-between',
        padding: `${spacing[1]} 0`,
        fontFamily: fonts.ui,
        fontSize: strong ? fontSizes.cardTitle : fontSizes.body,
        fontWeight: strong ? 600 : 400,
        color: muted ? colors.textSecondary : colors.textPrimary,
      }}
    >
      <span>{label}</span>
      <span>{value}</span>
    </div>
  );
}
