// Documents — download what the firm shared, send what the firm asked for.
//
// Downloads go through the API, which redirects to a URL that stops working in
// five minutes. The portal never handles the object itself, so a link copied
// out of the page is not a lasting grant of access.
//
// Uploads land in a quarantine queue. Nothing a client sends reaches the firm's
// vault until an attorney has reviewed it, and the page says so rather than
// implying the file is filed.

import { useEffect, useRef, useState } from 'react';
import { api, type SharedDocument, type Upload } from '@/lib/api';
import {
  Button, Card, Empty, Loading, Notice, PageTitle, SectionLabel, StatusBadge,
  formatBytes, formatDate,
} from '@/components/ui';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';

export function Documents() {
  const [documents, setDocuments] = useState<SharedDocument[] | null>(null);
  const [uploads, setUploads] = useState<Upload[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const [sent, setSent] = useState<string | null>(null);
  const filePicker = useRef<HTMLInputElement>(null);

  const load = async () => {
    try {
      const [docs, ups] = await Promise.all([api.documents.list(), api.documents.uploads()]);
      setDocuments(docs);
      setUploads(ups);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  useEffect(() => {
    load();
  }, []);

  const send = async (file: File) => {
    setSending(true);
    setError(null);
    setSent(null);
    try {
      await api.documents.upload(file);
      setSent(`${file.name} sent. We will confirm once it has been checked.`);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSending(false);
      if (filePicker.current) filePicker.current.value = '';
    }
  };

  if (!documents && !error) return <Loading />;

  return (
    <>
      <PageTitle subtitle="Everything we have shared with you, and anything you send us.">
        Documents
      </PageTitle>

      <section style={{ marginBottom: spacing[8] }}>
        <SectionLabel>Shared with you</SectionLabel>
        {!documents || documents.length === 0 ? (
          <Empty>Nothing shared yet.</Empty>
        ) : (
          <Card style={{ padding: 0, overflow: 'hidden' }}>
            {documents.map((doc, index) => (
              <a
                key={doc.id}
                href={api.documents.downloadUrl(doc.id)}
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  gap: spacing[4],
                  padding: `${spacing[3]} ${spacing[5]}`,
                  borderTop: index === 0 ? 'none' : `0.5px solid ${colors.border}`,
                  fontFamily: fonts.ui,
                  fontSize: fontSizes.body,
                  color: colors.textPrimary,
                  textDecoration: 'none',
                }}
              >
                <span>{doc.filename}</span>
                <span
                  style={{
                    display: 'flex',
                    gap: spacing[4],
                    color: colors.textTertiary,
                    fontSize: fontSizes.label,
                    whiteSpace: 'nowrap',
                  }}
                >
                  {doc.category && <span>{doc.category}</span>}
                  <span>{formatBytes(doc.fileSizeBytes)}</span>
                  <span>{formatDate(doc.sharedAt)}</span>
                </span>
              </a>
            ))}
          </Card>
        )}
      </section>

      <section>
        <SectionLabel>Send us a document</SectionLabel>

        <Card>
          <p
            style={{
              margin: `0 0 ${spacing[4]}`,
              fontFamily: fonts.ui,
              fontSize: fontSizes.body,
              color: colors.textSecondary,
              lineHeight: 1.55,
            }}
          >
            PDF, image or Word document, up to 25&nbsp;MB. Files are checked before
            they reach your file, so there is a short delay before we confirm.
          </p>

          <input
            ref={filePicker}
            type="file"
            accept=".pdf,.jpg,.jpeg,.png,.doc,.docx"
            style={{ display: 'none' }}
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) send(file);
            }}
          />

          <Button
            onClick={() => filePicker.current?.click()}
            disabled={sending}
            variant="secondary"
          >
            {sending ? 'Sending…' : 'Choose a file'}
          </Button>

          {sent && <div style={{ marginTop: spacing[4] }}><Notice message={sent} tone="ok" /></div>}
          {error && <div style={{ marginTop: spacing[4] }}><Notice message={error} /></div>}
        </Card>

        {uploads.length > 0 && (
          <div style={{ marginTop: spacing[5] }}>
            <SectionLabel>Sent by you</SectionLabel>
            <Card style={{ padding: 0, overflow: 'hidden' }}>
              {uploads.map((upload, index) => (
                <div
                  key={upload.id}
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    gap: spacing[4],
                    padding: `${spacing[3]} ${spacing[5]}`,
                    borderTop: index === 0 ? 'none' : `0.5px solid ${colors.border}`,
                    fontFamily: fonts.ui,
                    fontSize: fontSizes.body,
                  }}
                >
                  <span style={{ color: colors.textPrimary }}>{upload.filename}</span>
                  <span
                    style={{
                      display: 'flex',
                      gap: spacing[4],
                      alignItems: 'center',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    <span style={{ color: colors.textTertiary, fontSize: fontSizes.label }}>
                      {formatDate(upload.uploadedAt)}
                    </span>
                    <StatusBadge status={upload.status} />
                  </span>
                </div>
              ))}
            </Card>
            <p
              style={{
                margin: `${spacing[2]} 0 0`,
                fontFamily: fonts.ui,
                fontSize: 11,
                color: colors.textTertiary,
              }}
            >
              &ldquo;Pending&rdquo; means we have it and are checking it. We will let you know
              once it is on your file.
            </p>
          </div>
        )}
      </section>
    </>
  );
}

export const documentsRadius = radius.card;
