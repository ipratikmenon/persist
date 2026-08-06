/**
 * PortalHome — the desktop side of the client portal.
 * Accessed via /portal
 *
 * Two tabs:
 *   Client access — who can log into the portal, per client. Invite and revoke.
 *   Sync          — server URL, on/off, queued changes, last error.
 *
 * Everything here is Partner-only in Keel. The UI gates too, but only so an
 * attorney is not shown a button that will refuse them — Keel is the enforcer.
 */
import { useEffect, useState } from 'react';
import { motion, AnimatePresence, useReducedMotion } from 'motion/react';
import type { Client, PortalUser, PortalUserStatus, SyncStatus } from '@/lib/ipc-types';
import { keel } from '@/lib/tauri';
import { useAuthStore } from '@/stores/auth';
import { can, requiredRole } from '@/lib/permissions';
import { colors, fonts, fontSizes, radius, shadows, spacing } from '@/design-system/tokens';
import { transition } from '@/design-system/motion';

// ---------------------------------------------------------------------------
// Shared bits
// ---------------------------------------------------------------------------

const inputStyle = {
  width: '100%',
  padding: '8px 10px',
  borderRadius: radius.input,
  border: `0.5px solid ${colors.border}`,
  background: colors.bgPrimary,
  color: colors.textPrimary,
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  outline: 'none',
  boxSizing: 'border-box' as const,
};

const labelStyle = {
  display: 'block',
  fontSize: fontSizes.label,
  fontWeight: 500,
  color: colors.textSecondary,
  fontFamily: fonts.ui,
  marginBottom: 5,
};

const sectionLabel = {
  fontSize: fontSizes.label,
  fontWeight: 500,
  color: colors.textTertiary,
  textTransform: 'uppercase' as const,
  letterSpacing: '0.06em',
  marginBottom: spacing[3],
};

const STATUS_STYLE: Record<PortalUserStatus, { bg: string; color: string }> = {
  Invited:   { bg: 'rgba(212,135,42,0.11)',  color: colors.statusWarning },
  Active:    { bg: 'rgba(74,124,89,0.11)',   color: colors.statusClear   },
  Suspended: { bg: 'rgba(154,149,144,0.13)', color: colors.textTertiary  },
  Revoked:   { bg: 'rgba(192,57,43,0.10)',   color: colors.statusUrgent  },
};

function StatusPill({ status }: { status: PortalUserStatus }) {
  const s = STATUS_STYLE[status] ?? STATUS_STYLE.Invited;
  return (
    <span style={{
      display: 'inline-flex', alignItems: 'center', padding: '2px 8px',
      borderRadius: radius.chip, background: s.bg, color: s.color,
      fontSize: fontSizes.label, fontFamily: fonts.ui, fontWeight: 500,
      whiteSpace: 'nowrap',
    }}>
      {status}
    </span>
  );
}

function primaryButtonStyle(disabled: boolean) {
  return {
    padding: `${spacing[2]} ${spacing[5]}`,
    borderRadius: radius.button,
    border: 'none',
    background: colors.accentPrimary,
    color: '#fff',
    fontFamily: fonts.ui,
    fontSize: fontSizes.body,
    fontWeight: 500,
    cursor: disabled ? 'not-allowed' : 'pointer',
    opacity: disabled ? 0.5 : 1,
  };
}

function ErrorNote({ message }: { message: string }) {
  return (
    <div style={{
      padding: spacing[3], borderRadius: radius.input,
      background: 'rgba(192,57,43,0.08)', color: colors.statusUrgent,
      fontFamily: fonts.ui, fontSize: fontSizes.label, lineHeight: 1.5,
    }}>
      {message}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Client access
// ---------------------------------------------------------------------------

function ClientAccessTab({ allowed }: { allowed: boolean }) {
  const shouldReduce = useReducedMotion();

  const [clients, setClients]   = useState<Client[]>([]);
  const [clientId, setClientId] = useState('');
  const [users, setUsers]       = useState<PortalUser[]>([]);
  const [loading, setLoading]   = useState(true);
  const [error, setError]       = useState<string | null>(null);

  const [name, setName]   = useState('');
  const [email, setEmail] = useState('');
  const [phone, setPhone] = useState('');
  const [inviting, setInviting] = useState(false);

  useEffect(() => {
    keel.clients.list()
      .then(cs => {
        setClients(cs);
        if (cs.length > 0) setClientId(cs[0].id);
      })
      .catch(() => setClients([]))
      .finally(() => setLoading(false));
  }, []);

  const loadUsers = async (id: string) => {
    if (!id) return;
    try {
      setUsers(await keel.portalUsers.list(id));
    } catch {
      setUsers([]);
    }
  };

  useEffect(() => { loadUsers(clientId); }, [clientId]);

  const invite = async () => {
    setInviting(true);
    setError(null);
    try {
      await keel.portalUsers.invite({
        clientId,
        fullName: name.trim(),
        email: email.trim(),
        phone: phone.trim() || undefined,
      });
      setName(''); setEmail(''); setPhone('');
      await loadUsers(clientId);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setInviting(false);
    }
  };

  const revoke = async (user: PortalUser) => {
    if (!window.confirm(
      `Revoke portal access for ${user.fullName}?\n\n` +
      `They lose access at their next request, not when their session expires.`
    )) return;

    setError(null);
    try {
      await keel.portalUsers.revoke(user.id);
      await loadUsers(clientId);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  if (loading) {
    return <div style={{ color: colors.textTertiary, fontFamily: fonts.ui }}>Loading…</div>;
  }

  const canInvite = allowed && clientId && name.trim() && email.trim();

  return (
    <div style={{ maxWidth: 720 }}>
      <div style={{ marginBottom: spacing[6] }}>
        <label style={labelStyle}>Client</label>
        <select
          value={clientId}
          onChange={e => setClientId(e.target.value)}
          style={{ ...inputStyle, cursor: 'pointer', maxWidth: 380 }}
        >
          {clients.map(c => <option key={c.id} value={c.id}>{c.name}</option>)}
        </select>
      </div>

      <section style={{ marginBottom: spacing[8] }}>
        <div style={sectionLabel}>Portal users ({users.length})</div>

        {users.length === 0 ? (
          <div style={{
            padding: spacing[6], borderRadius: radius.card,
            background: colors.bgSecondary, border: `0.5px solid ${colors.border}`,
            color: colors.textSecondary, fontFamily: fonts.ui,
            fontSize: fontSizes.body, textAlign: 'center' as const,
          }}>
            Nobody at this client has portal access yet.
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: spacing[2] }}>
            <AnimatePresence mode="popLayout">
              {users.map(u => (
                <motion.div
                  key={u.id}
                  layout
                  initial={{ opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0 }}
                  transition={transition.fast}
                  style={{
                    display: 'flex', alignItems: 'center', gap: spacing[4],
                    padding: spacing[4], borderRadius: radius.card,
                    background: colors.bgSecondary,
                    border: `0.5px solid ${colors.border}`,
                    boxShadow: shadows.card,
                  }}
                >
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{
                      fontFamily: fonts.ui, fontSize: fontSizes.cardTitle,
                      fontWeight: 500, color: colors.textPrimary, marginBottom: 2,
                    }}>
                      {u.fullName}
                    </div>
                    <div style={{
                      fontFamily: fonts.ui, fontSize: fontSizes.label,
                      color: colors.textSecondary,
                    }}>
                      {u.email}{u.phone ? ` · ${u.phone}` : ''}
                    </div>
                  </div>

                  <StatusPill status={u.status} />

                  {allowed && u.status !== 'Revoked' && (
                    <motion.button
                      onClick={() => revoke(u)}
                      whileHover={!shouldReduce ? { opacity: 0.8 } : undefined}
                      style={{
                        padding: '4px 12px', borderRadius: radius.button,
                        border: `0.5px solid ${colors.statusUrgent}`,
                        background: 'transparent', color: colors.statusUrgent,
                        fontFamily: fonts.ui, fontSize: fontSizes.label,
                        cursor: 'pointer', flexShrink: 0,
                      }}
                    >
                      Revoke
                    </motion.button>
                  )}
                </motion.div>
              ))}
            </AnimatePresence>
          </div>
        )}
      </section>

      <section>
        <div style={sectionLabel}>Invite someone</div>

        {!allowed ? (
          <p style={{
            margin: 0, fontFamily: fonts.ui, fontSize: fontSizes.body,
            color: colors.textTertiary,
          }}>
            {requiredRole('ManagePortalUsers')} access is required to invite or
            revoke portal users.
          </p>
        ) : (
          <div style={{
            padding: spacing[5], borderRadius: radius.card,
            background: colors.bgSecondary, border: `0.5px solid ${colors.border}`,
            display: 'flex', flexDirection: 'column', gap: spacing[4],
          }}>
            <div style={{ display: 'flex', gap: spacing[3] }}>
              <div style={{ flex: 1 }}>
                <label style={labelStyle}>Full name</label>
                <input value={name} onChange={e => setName(e.target.value)}
                  placeholder="Anita Rao" style={inputStyle} />
              </div>
              <div style={{ flex: 1 }}>
                <label style={labelStyle}>Email</label>
                <input value={email} onChange={e => setEmail(e.target.value)}
                  placeholder="anita@client.example" style={inputStyle} />
              </div>
            </div>

            <div style={{ maxWidth: 260 }}>
              <label style={labelStyle}>Phone (optional)</label>
              <input value={phone} onChange={e => setPhone(e.target.value)}
                placeholder="+91 98100 11223" style={inputStyle} />
            </div>

            <p style={{
              margin: 0, fontSize: 11, color: colors.textTertiary, fontFamily: fonts.ui,
            }}>
              One email address maps to exactly one client. They sign in with a
              one-time code — the portal stores no passwords.
            </p>

            {error && <ErrorNote message={error} />}

            <motion.button
              onClick={invite}
              disabled={!canInvite || inviting}
              whileHover={!shouldReduce && canInvite ? { opacity: 0.88 } : undefined}
              style={{ ...primaryButtonStyle(!canInvite || inviting), alignSelf: 'flex-start' }}
            >
              {inviting ? 'Inviting…' : 'Send invitation'}
            </motion.button>
          </div>
        )}
      </section>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

function SyncTab({ allowed }: { allowed: boolean }) {
  const shouldReduce = useReducedMotion();

  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [url, setUrl]       = useState('');
  const [busy, setBusy]     = useState(false);
  const [error, setError]   = useState<string | null>(null);
  const [note, setNote]     = useState<string | null>(null);

  const load = async () => {
    try {
      const s = await keel.sync.status();
      setStatus(s);
      setUrl(s.serverUrl ?? '');
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  useEffect(() => { load(); }, []);

  const saveUrl = async () => {
    setBusy(true); setError(null); setNote(null);
    try {
      setStatus(await keel.sync.setServer(url.trim()));
      setNote('Server address saved.');
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const toggle = async () => {
    if (!status) return;
    setBusy(true); setError(null); setNote(null);
    try {
      setStatus(await keel.sync.setEnabled(!status.isEnabled));
    } catch (e: unknown) {
      // Keel refuses to enable without a server URL rather than doing nothing.
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const syncNow = async () => {
    setBusy(true); setError(null); setNote(null);
    try {
      setStatus(await keel.sync.trigger());
      setNote('Sync complete.');
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
      load();
    }
  };

  if (!status) {
    return <div style={{ color: colors.textTertiary, fontFamily: fonts.ui }}>Loading…</div>;
  }

  return (
    <div style={{ maxWidth: 640 }}>
      {/* State summary */}
      <div style={{
        padding: spacing[5], borderRadius: radius.card,
        background: status.isEnabled ? 'rgba(74,124,89,0.06)' : colors.bgSecondary,
        border: `0.5px solid ${status.isEnabled ? colors.statusClear : colors.border}`,
        boxShadow: shadows.card, marginBottom: spacing[6],
      }}>
        <div style={{
          display: 'flex', alignItems: 'center', justifyContent: 'space-between',
          marginBottom: spacing[3],
        }}>
          <div style={{
            fontFamily: fonts.display, fontSize: fontSizes.displaySm, fontWeight: 500,
            color: colors.textPrimary,
          }}>
            Sync is {status.isEnabled ? 'on' : 'off'}
          </div>

          {allowed && (
            <motion.button
              onClick={toggle}
              disabled={busy}
              whileHover={!shouldReduce && !busy ? { opacity: 0.85 } : undefined}
              style={{
                padding: '5px 14px', borderRadius: radius.button,
                border: `0.5px solid ${status.isEnabled ? colors.statusUrgent : colors.accentPrimary}`,
                background: 'transparent',
                color: status.isEnabled ? colors.statusUrgent : colors.accentPrimary,
                fontFamily: fonts.ui, fontSize: fontSizes.label, fontWeight: 500,
                cursor: busy ? 'not-allowed' : 'pointer',
              }}
            >
              {status.isEnabled ? 'Turn off' : 'Turn on'}
            </motion.button>
          )}
        </div>

        <p style={{
          margin: 0, fontFamily: fonts.ui, fontSize: fontSizes.body,
          color: colors.textSecondary, lineHeight: 1.55,
        }}>
          {status.isEnabled
            ? 'Matter status, client-visible deadlines, shared documents and issued invoices are published to the client portal. Internal notes, time entries and drafts never leave this machine.'
            : 'Nothing leaves this machine. Persist works exactly as it does now; the client portal simply has no data.'}
        </p>

        <div style={{
          display: 'flex', gap: spacing[5], marginTop: spacing[4],
          fontFamily: fonts.ui, fontSize: fontSizes.label, color: colors.textSecondary,
        }}>
          <span>
            <strong style={{ color: colors.textPrimary }}>{status.pendingChanges}</strong>
            {' '}change{status.pendingChanges === 1 ? '' : 's'} queued
          </span>
          <span>
            Last sync: {status.lastSyncedAt ?? 'never'}
          </span>
        </div>
      </div>

      {/* Server address */}
      <section style={{ marginBottom: spacing[6] }}>
        <div style={sectionLabel}>Sync server</div>
        <div style={{ display: 'flex', gap: spacing[3], alignItems: 'flex-end' }}>
          <div style={{ flex: 1 }}>
            <input
              value={url}
              onChange={e => setUrl(e.target.value)}
              placeholder="https://sync.persistas.example"
              disabled={!allowed}
              style={{ ...inputStyle, fontFamily: fonts.mono, fontSize: fontSizes.mono }}
            />
          </div>
          {allowed && (
            <button
              onClick={saveUrl}
              disabled={busy}
              style={{
                padding: `${spacing[2]} ${spacing[4]}`, borderRadius: radius.button,
                border: `0.5px solid ${colors.border}`, background: 'transparent',
                color: colors.textSecondary, fontFamily: fonts.ui,
                fontSize: fontSizes.body, cursor: busy ? 'not-allowed' : 'pointer',
              }}
            >
              Save
            </button>
          )}
        </div>
        <p style={{
          margin: `${spacing[2]} 0 0`, fontSize: 11,
          color: colors.textTertiary, fontFamily: fonts.ui,
        }}>
          Sync cannot be turned on until an address is set.
        </p>
      </section>

      {/* Actions + messages */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: spacing[3] }}>
        {status.lastError && (
          <ErrorNote message={`Last sync error: ${status.lastError}`} />
        )}
        {error && <ErrorNote message={error} />}
        {note && (
          <div style={{
            padding: spacing[3], borderRadius: radius.input,
            background: 'rgba(74,124,89,0.09)', color: colors.statusClear,
            fontFamily: fonts.ui, fontSize: fontSizes.label,
          }}>
            {note}
          </div>
        )}

        {status.isEnabled && (
          <motion.button
            onClick={syncNow}
            disabled={busy}
            whileHover={!shouldReduce && !busy ? { opacity: 0.88 } : undefined}
            style={{ ...primaryButtonStyle(busy), alignSelf: 'flex-start' }}
          >
            {busy ? 'Syncing…' : 'Sync now'}
          </motion.button>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const TABS = ['Client access', 'Sync'] as const;
type Tab = (typeof TABS)[number];

export default function PortalHome() {
  const [tab, setTab] = useState<Tab>('Client access');
  const role = useAuthStore(s => s.session?.role);
  const allowed = can(role, 'ManagePortalUsers');

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      <header style={{
        padding: `${spacing[5]} ${spacing[8]} 0`,
        borderBottom: `0.5px solid ${colors.border}`,
        flexShrink: 0,
      }}>
        <h1 style={{
          fontFamily: fonts.display, fontSize: fontSizes.displayLg, fontWeight: 600,
          color: colors.textPrimary, margin: '0 0 4px', lineHeight: 1.25,
        }}>
          Client Portal
        </h1>
        <p style={{
          margin: `0 0 ${spacing[4]}`, fontFamily: fonts.ui,
          fontSize: fontSizes.label, color: colors.textSecondary,
        }}>
          What clients can see, and who can see it.
        </p>

        <div style={{ display: 'flex', gap: spacing[5] }}>
          {TABS.map(t => (
            <button
              key={t}
              onClick={() => setTab(t)}
              style={{
                padding: `${spacing[3]} 0`, border: 'none', background: 'transparent',
                borderBottom: tab === t
                  ? `2px solid ${colors.accentPrimary}`
                  : '2px solid transparent',
                color: tab === t ? colors.accentPrimary : colors.textSecondary,
                fontFamily: fonts.ui, fontSize: fontSizes.body,
                fontWeight: tab === t ? 500 : 400, cursor: 'pointer',
              }}
            >
              {t}
            </button>
          ))}
        </div>
      </header>

      <div style={{ flex: 1, overflow: 'auto', padding: `${spacing[6]} ${spacing[8]}` }}>
        {tab === 'Client access' ? <ClientAccessTab allowed={allowed} /> : <SyncTab allowed={allowed} />}
      </div>
    </div>
  );
}
