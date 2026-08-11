// Profile — who you are to us, and how to sign out.
//
// Everything is read-only. A change of name, address or contact details goes
// through the firm, because the firm's record is the one that appears on a
// filing: letting a client edit it here would create two versions of the truth
// and only one of them would reach the Registry.

import { useEffect, useState } from 'react';
import { api, type Notification, type Profile as ProfileData } from '@/lib/api';
import {
  Button, Card, Empty, Loading, Notice, PageTitle, SectionLabel, formatDate,
} from '@/components/ui';
import { colors, fonts, fontSizes, spacing } from '@/design-system/tokens';

export function Profile({ onSignedOut }: { onSignedOut: () => void }) {
  const [profile, setProfile] = useState<ProfileData | null>(null);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([api.profile.get(), api.notifications.list()])
      .then(([p, n]) => {
        setProfile(p);
        setNotifications(n);
      })
      .catch((e) => setError(e.message));
  }, []);

  if (error) return <Notice message={error} />;
  if (!profile) return <Loading />;

  return (
    <>
      <PageTitle subtitle="Your details, as we hold them.">Your account</PageTitle>

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
          <dt style={{ color: colors.textSecondary }}>Name</dt>
          <dd style={{ margin: 0, color: colors.textPrimary }}>{profile.fullName}</dd>
          <dt style={{ color: colors.textSecondary }}>Email</dt>
          <dd style={{ margin: 0, color: colors.textPrimary }}>{profile.email}</dd>
          {profile.phone && (
            <>
              <dt style={{ color: colors.textSecondary }}>Phone</dt>
              <dd style={{ margin: 0, color: colors.textPrimary }}>{profile.phone}</dd>
            </>
          )}
          <dt style={{ color: colors.textSecondary }}>Client</dt>
          <dd style={{ margin: 0, color: colors.textPrimary }}>{profile.clientName}</dd>
        </dl>

        <p
          style={{
            margin: `${spacing[5]} 0 0`,
            paddingTop: spacing[4],
            borderTop: `0.5px solid ${colors.border}`,
            fontFamily: fonts.ui,
            fontSize: 11,
            color: colors.textTertiary,
            lineHeight: 1.55,
          }}
        >
          To change any of these, write to us. We keep one record of your details so
          that what appears on a filing is always the version we hold.
        </p>
      </Card>

      <section style={{ marginBottom: spacing[8] }}>
        <SectionLabel>Recent updates</SectionLabel>
        {notifications.length === 0 ? (
          <Empty>Nothing yet.</Empty>
        ) : (
          <Card style={{ padding: 0, overflow: 'hidden' }}>
            {notifications.slice(0, 10).map((note, index) => (
              <div
                key={note.id}
                style={{
                  padding: `${spacing[3]} ${spacing[5]}`,
                  borderTop: index === 0 ? 'none' : `0.5px solid ${colors.border}`,
                  borderLeft: note.isRead
                    ? '2px solid transparent'
                    : `2px solid ${colors.accentSecondary}`,
                  fontFamily: fonts.ui,
                }}
              >
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    gap: spacing[4],
                    fontSize: fontSizes.body,
                    color: colors.textPrimary,
                  }}
                >
                  <span>{note.title}</span>
                  <span
                    style={{
                      color: colors.textTertiary,
                      fontSize: fontSizes.label,
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {formatDate(note.createdAt)}
                  </span>
                </div>
                {note.body && (
                  <p
                    style={{
                      margin: `${spacing[1]} 0 0`,
                      fontSize: fontSizes.label,
                      color: colors.textSecondary,
                    }}
                  >
                    {note.body}
                  </p>
                )}
              </div>
            ))}
          </Card>
        )}
      </section>

      <Button
        variant="quiet"
        onClick={async () => {
          await api.auth.logout();
          onSignedOut();
        }}
      >
        Sign out
      </Button>
    </>
  );
}
