// Matters — the client's view of their own work.
//
// Deliberately absent, and not by omission: time entries, hourly rates,
// internal notes, other clients, firm staff lists. The API does not return
// them; this page could not show them if it tried.

import { useEffect, useState } from 'react';
import { Link, useParams } from 'react-router';
import { api, type MatterDetail, type MatterSummary } from '@/lib/api';
import {
  Card, Empty, Loading, Notice, PageTitle, SectionLabel, StatusBadge,
  deadlineTone, formatBytes, formatDate, humanise,
} from '@/components/ui';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';

export function MattersList() {
  const [matters, setMatters] = useState<MatterSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.matters
      .list()
      .then(setMatters)
      .catch((e) => setError(e.message));
  }, []);

  if (error) return <Notice message={error} />;
  if (!matters) return <Loading />;

  return (
    <>
      <PageTitle subtitle="Everything we are handling for you.">Matters</PageTitle>

      {matters.length === 0 ? (
        <Empty>No matters yet. Anything we open for you will appear here.</Empty>
      ) : (
        <div style={{ display: 'grid', gap: spacing[3] }}>
          {matters.map((matter) => (
            <Link
              key={matter.id}
              to={`/matters/${encodeURIComponent(matter.id)}`}
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
                        fontFamily: fonts.ui,
                        fontSize: fontSizes.cardTitle,
                        fontWeight: 500,
                        color: colors.textPrimary,
                      }}
                    >
                      {matter.title}
                    </div>
                    <div
                      style={{
                        marginTop: spacing[1],
                        fontFamily: fonts.mono,
                        fontSize: fontSizes.mono,
                        color: colors.textTertiary,
                      }}
                    >
                      {matter.id}
                    </div>
                  </div>
                  <StatusBadge status={matter.status} />
                </div>

                <div
                  style={{
                    display: 'flex',
                    gap: spacing[6],
                    marginTop: spacing[4],
                    fontFamily: fonts.ui,
                    fontSize: fontSizes.label,
                    color: colors.textSecondary,
                  }}
                >
                  <span>{humanise(matter.matterType)}</span>
                  <span>Opened {formatDate(matter.openedDate)}</span>
                  {matter.nextDeadlineDate && (
                    <span style={{ color: deadlineTone(matter.nextDeadlineDate) }}>
                      Next: {matter.nextDeadlineEvent} — {formatDate(matter.nextDeadlineDate)}
                    </span>
                  )}
                </div>
              </Card>
            </Link>
          ))}
        </div>
      )}
    </>
  );
}

export function MatterDetailPage() {
  const { id = '' } = useParams();
  const [matter, setMatter] = useState<MatterDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setMatter(null);
    setError(null);
    api.matters
      .get(id)
      .then(setMatter)
      .catch((e) => setError(e.message));
  }, [id]);

  if (error) return <Notice message={error} />;
  if (!matter) return <Loading />;

  return (
    <>
      <Link
        to="/matters"
        style={{
          fontFamily: fonts.ui,
          fontSize: fontSizes.label,
          color: colors.accentPrimary,
          textDecoration: 'none',
        }}
      >
        ← All matters
      </Link>

      <div style={{ marginTop: spacing[3] }}>
        <PageTitle>{matter.title}</PageTitle>
      </div>

      <Card style={{ marginBottom: spacing[6] }}>
        <dl
          style={{
            display: 'grid',
            gridTemplateColumns: 'max-content 1fr',
            gap: `${spacing[2]} ${spacing[5]}`,
            margin: 0,
            fontFamily: fonts.ui,
            fontSize: fontSizes.body,
          }}
        >
          <Term>Reference</Term>
          <Detail mono>{matter.id}</Detail>
          <Term>Status</Term>
          <Detail><StatusBadge status={matter.status} /></Detail>
          <Term>Type</Term>
          <Detail>{humanise(matter.matterType)}</Detail>
          <Term>Acting for you</Term>
          <Detail>{matter.responsibleAttorney ?? '—'}</Detail>
          {matter.forum && (
            <>
              <Term>Forum</Term>
              <Detail>{matter.forum}</Detail>
            </>
          )}
          <Term>Opened</Term>
          <Detail>{formatDate(matter.openedDate)}</Detail>
        </dl>

        {matter.clientNotes && (
          <p
            style={{
              margin: `${spacing[5]} 0 0`,
              paddingTop: spacing[4],
              borderTop: `0.5px solid ${colors.border}`,
              fontFamily: fonts.ui,
              fontSize: fontSizes.body,
              color: colors.textPrimary,
              lineHeight: 1.6,
            }}
          >
            {matter.clientNotes}
          </p>
        )}
      </Card>

      <section style={{ marginBottom: spacing[6] }}>
        <SectionLabel>Upcoming dates</SectionLabel>
        {matter.deadlines.length === 0 ? (
          <Empty>Nothing scheduled.</Empty>
        ) : (
          <Card style={{ padding: 0, overflow: 'hidden' }}>
            {matter.deadlines.map((deadline, index) => (
              <div
                key={deadline.id}
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
                <span style={{ color: colors.textPrimary }}>{deadline.docketingEvent}</span>
                <span style={{ color: deadlineTone(deadline.dueDate), whiteSpace: 'nowrap' }}>
                  {formatDate(deadline.dueDate)}
                </span>
              </div>
            ))}
          </Card>
        )}
      </section>

      {matter.ipAssets.length > 0 && (
        <section style={{ marginBottom: spacing[6] }}>
          <SectionLabel>Registrations</SectionLabel>
          <div style={{ display: 'grid', gap: spacing[3] }}>
            {matter.ipAssets.map((asset) => (
              <Card key={asset.id}>
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    gap: spacing[4],
                  }}
                >
                  <div
                    style={{
                      fontFamily: fonts.ui,
                      fontSize: fontSizes.cardTitle,
                      fontWeight: 500,
                      color: colors.textPrimary,
                    }}
                  >
                    {asset.title}
                  </div>
                  <StatusBadge status={asset.status} />
                </div>
                <div
                  style={{
                    display: 'flex',
                    gap: spacing[5],
                    marginTop: spacing[3],
                    fontFamily: fonts.ui,
                    fontSize: fontSizes.label,
                    color: colors.textSecondary,
                  }}
                >
                  <span>{humanise(asset.assetType)}</span>
                  {asset.applicationNumber && (
                    <span style={{ fontFamily: fonts.mono }}>
                      App. {asset.applicationNumber}
                    </span>
                  )}
                  {asset.classes.length > 0 && <span>Class {asset.classes.join(', ')}</span>}
                  {asset.nextRenewalDate && (
                    <span>Renewal {formatDate(asset.nextRenewalDate)}</span>
                  )}
                </div>
              </Card>
            ))}
          </div>
        </section>
      )}

      <section>
        <SectionLabel>Documents</SectionLabel>
        {matter.documents.length === 0 ? (
          <Empty>Nothing shared on this matter yet.</Empty>
        ) : (
          <Card style={{ padding: 0, overflow: 'hidden' }}>
            {matter.documents.map((doc, index) => (
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
                <span style={{ color: colors.textTertiary, fontSize: fontSizes.label }}>
                  {formatBytes(doc.fileSizeBytes)}
                </span>
              </a>
            ))}
          </Card>
        )}
      </section>
    </>
  );
}

function Term({ children }: { children: React.ReactNode }) {
  return <dt style={{ color: colors.textSecondary }}>{children}</dt>;
}

function Detail({ children, mono }: { children: React.ReactNode; mono?: boolean }) {
  return (
    <dd
      style={{
        margin: 0,
        color: colors.textPrimary,
        fontFamily: mono ? fonts.mono : undefined,
        fontSize: mono ? fontSizes.mono : undefined,
      }}
    >
      {children}
    </dd>
  );
}

export const cardLinkRadius = radius.card;
