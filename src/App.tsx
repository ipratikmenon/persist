import { useEffect, useRef } from 'react';
import { Routes, Route, Navigate, useNavigate } from 'react-router';
import { AppShell } from '@/components/shell/AppShell';
import MatterList from '@/pages/Matters/MatterList';
import MatterDetail from '@/pages/Matters/MatterDetail';
import DocketList from '@/pages/Dockets/DocketList';
import IPAssetRecord from '@/pages/Dockets/IPAssetRecord';
import PipelineBoard from '@/pages/Dockets/PipelineBoard';
import RenewalDashboard from '@/pages/Dockets/RenewalDashboard';
import DocumentList from '@/pages/Documents/DocumentList';
import BillingHome from '@/pages/Billing/BillingHome';
import PortalHome from '@/pages/Portal/PortalHome';
import LoginScreen from '@/pages/Auth/LoginScreen';
import { useAuthStore } from '@/stores/auth';
import { keel } from '@/lib/tauri';
import { msUntil } from '@/lib/dates';
import { colors, fonts, fontSizes } from '@/design-system/tokens';

// ---------------------------------------------------------------------------
// Session bootstrap — check Keel for an existing session on every app start
// ---------------------------------------------------------------------------

function SessionGate({ children }: { children: React.ReactNode }) {
  const { session, isChecking, setSession } = useAuthStore();

  useEffect(() => {
    keel.auth.getSession()
      .then(s => setSession(s ?? null))
      .catch(() => setSession(null));
  }, []);

  if (isChecking) {
    return (
      <div style={{
        position: 'fixed', inset: 0,
        background: colors.bgPrimary,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        color: colors.textTertiary,
        fontFamily: fonts.ui,
        fontSize: fontSizes.body,
      }}>
        Loading…
      </div>
    );
  }

  if (!session) {
    return <LoginScreen />;
  }

  return <>{children}</>;
}

// ---------------------------------------------------------------------------
// Session keep-alive — extends an 8-hour session while the attorney is working,
// and drops to the login screen the moment it lapses (B01).
//
// Keel is authoritative: it only extends sessions that are still valid, so this
// can never revive a dead session. Deck's job is just to ask at sensible times.
// ---------------------------------------------------------------------------

/** How often we evaluate the session. */
const SESSION_CHECK_INTERVAL_MS = 5 * 60 * 1000;   // 5 minutes
/** Refresh once the session has less than this left — well inside the 8h window. */
const SESSION_REFRESH_THRESHOLD_MS = 2 * 60 * 60 * 1000;  // 2 hours

function SessionKeepAlive() {
  const activeSinceLastCheck = useRef(false);

  useEffect(() => {
    const markActive = () => { activeSinceLastCheck.current = true; };
    window.addEventListener('pointerdown', markActive);
    window.addEventListener('keydown', markActive);

    const check = async () => {
      const { session, setSession, clearSession } = useAuthStore.getState();
      if (!session) return;

      const remaining = msUntil(session.expiresAt);

      // Already lapsed — confirm with Keel, which clears the stored token.
      if (remaining === null || remaining <= 0) {
        const current = await keel.auth.getSession().catch(() => null);
        if (current) setSession(current); else clearSession();
        return;
      }

      // Only extend for an attorney who is actually working. An idle app is
      // left to expire on schedule rather than renewing itself forever.
      if (activeSinceLastCheck.current && remaining < SESSION_REFRESH_THRESHOLD_MS) {
        activeSinceLastCheck.current = false;
        const refreshed = await keel.auth.refreshSession().catch(() => null);
        if (refreshed) setSession(refreshed); else clearSession();
      }
    };

    const timer = window.setInterval(check, SESSION_CHECK_INTERVAL_MS);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener('pointerdown', markActive);
      window.removeEventListener('keydown', markActive);
    };
  }, []);

  return null;
}

// ---------------------------------------------------------------------------
// Logout handler — available to any child via the auth store
// ---------------------------------------------------------------------------

function LogoutHandler() {
  const navigate = useNavigate();
  const { clearSession } = useAuthStore();

  // Expose a global logout function on the window for AppShell to call.
  // This keeps the router context available without prop-drilling.
  useEffect(() => {
    (window as Window & { __persistLogout?: () => void }).__persistLogout = async () => {
      await keel.auth.logout();
      clearSession();
      navigate('/login', { replace: true });
    };
    return () => {
      delete (window as Window & { __persistLogout?: () => void }).__persistLogout;
    };
  }, [clearSession, navigate]);

  return null;
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

export default function App() {
  return (
    <Routes>
      {/* Login — no shell, no guard */}
      <Route path="/login" element={<LoginScreen />} />

      {/* All protected routes — wrapped in SessionGate */}
      <Route
        path="/*"
        element={
          <SessionGate>
            <AppShell>
              <LogoutHandler />
              <SessionKeepAlive />
              <Routes>
                <Route index element={<Navigate to="/matters" replace />} />

                {/* Phase 1 M1 — Matter Management */}
                <Route path="/matters"     element={<MatterList />} />
                <Route path="/matters/:id" element={<MatterDetail />} />

                {/* Phase 1 M2 — Docketing */}
                <Route path="/dockets"             element={<DocketList />} />
                <Route path="/dockets/pipeline"    element={<PipelineBoard />} />
                <Route path="/dockets/renewals"    element={<RenewalDashboard />} />
                <Route path="/dockets/:matterId"   element={<IPAssetRecord />} />

                {/* Phase 1 M3 — Document Vault */}
                <Route path="/documents" element={<DocumentList />} />

                {/* Phase 2 M4 — Billing */}
                <Route path="/billing" element={<BillingHome />} />

                {/* Phase 2 M5 — Client Portal */}
                <Route path="/portal" element={<PortalHome />} />

                <Route path="*" element={<Navigate to="/matters" replace />} />
              </Routes>
            </AppShell>
          </SessionGate>
        }
      />
    </Routes>
  );
}