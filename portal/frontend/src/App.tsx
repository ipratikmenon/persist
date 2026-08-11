// The portal shell.
//
// Four tabs, per PORTAL-RULES.md. Nothing renders until the session question is
// settled: on a cold load the access token is gone (it lives in memory only),
// so the app asks the backend to mint one from the refresh cookie first. Showing
// the login screen while that is in flight would flash a sign-in page at
// somebody who is already signed in.

import { useCallback, useEffect, useState } from 'react';
import { BrowserRouter, Navigate, NavLink, Route, Routes } from 'react-router';
import { onSessionEnded, restoreSession } from '@/lib/api';
import { Login } from '@/pages/Login';
import { MatterDetailPage, MattersList } from '@/pages/Matters';
import { Documents } from '@/pages/Documents';
import { InvoiceDetailPage, InvoicesList } from '@/pages/Invoices';
import { Profile } from '@/pages/Profile';
import { colors, fonts, fontSizes, spacing } from '@/design-system/tokens';

type SessionState = 'checking' | 'in' | 'out';

const TABS = [
  { to: '/matters', label: 'Matters' },
  { to: '/documents', label: 'Documents' },
  { to: '/invoices', label: 'Invoices' },
  { to: '/account', label: 'Account' },
];

export function App() {
  const [session, setSession] = useState<SessionState>('checking');

  useEffect(() => {
    restoreSession().then((ok) => setSession(ok ? 'in' : 'out'));
  }, []);

  // The api layer signals when a refresh fails mid-session, so an expired
  // session returns to the login screen rather than showing empty pages.
  useEffect(() => onSessionEnded(() => setSession('out')), []);

  const signedIn = useCallback(() => setSession('in'), []);
  const signedOut = useCallback(() => setSession('out'), []);

  if (session === 'checking') {
    return <div style={{ minHeight: '100vh', background: colors.bgPrimary }} />;
  }

  if (session === 'out') {
    return <Login onSignedIn={signedIn} />;
  }

  return (
    <BrowserRouter>
      <div style={{ minHeight: '100vh', background: colors.bgPrimary }}>
        <header
          style={{
            borderBottom: `0.5px solid ${colors.border}`,
            background: colors.bgSecondary,
          }}
        >
          <div
            style={{
              maxWidth: 880,
              margin: '0 auto',
              padding: `${spacing[5]} ${spacing[5]} 0`,
            }}
          >
            <div
              style={{
                fontFamily: fonts.display,
                fontSize: fontSizes.cardTitle,
                fontWeight: 600,
                color: colors.accentPrimary,
              }}
            >
              Persistas &amp; Partners
            </div>

            <nav style={{ display: 'flex', gap: spacing[5], marginTop: spacing[4] }}>
              {TABS.map((tab) => (
                <NavLink
                  key={tab.to}
                  to={tab.to}
                  style={({ isActive }) => ({
                    padding: `${spacing[2]} 0`,
                    fontFamily: fonts.ui,
                    fontSize: fontSizes.body,
                    fontWeight: isActive ? 500 : 400,
                    color: isActive ? colors.accentPrimary : colors.textSecondary,
                    borderBottom: `2px solid ${isActive ? colors.accentPrimary : 'transparent'}`,
                    textDecoration: 'none',
                  })}
                >
                  {tab.label}
                </NavLink>
              ))}
            </nav>
          </div>
        </header>

        <main style={{ maxWidth: 880, margin: '0 auto', padding: spacing[8] }}>
          <Routes>
            <Route path="/" element={<Navigate to="/matters" replace />} />
            <Route path="/matters" element={<MattersList />} />
            <Route path="/matters/:id" element={<MatterDetailPage />} />
            <Route path="/documents" element={<Documents />} />
            <Route path="/invoices" element={<InvoicesList />} />
            <Route path="/invoices/:id" element={<InvoiceDetailPage />} />
            <Route path="/account" element={<Profile onSignedOut={signedOut} />} />
            {/* An unknown path is not an error worth a page of its own. */}
            <Route path="*" element={<Navigate to="/matters" replace />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}
