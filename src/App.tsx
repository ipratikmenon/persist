import { useEffect } from 'react';
import { Routes, Route, Navigate, useNavigate } from 'react-router';
import { AppShell } from '@/components/shell/AppShell';
import MatterList from '@/pages/Matters/MatterList';
import MatterDetail from '@/pages/Matters/MatterDetail';
import DocketList from '@/pages/Dockets/DocketList';
import IPAssetRecord from '@/pages/Dockets/IPAssetRecord';
import PipelineBoard from '@/pages/Dockets/PipelineBoard';
import DocumentList from '@/pages/Documents/DocumentList';
import BillingHome from '@/pages/Billing/BillingHome';
import LoginScreen from '@/pages/Auth/LoginScreen';
import { useAuthStore } from '@/stores/auth';
import { keel } from '@/lib/tauri';
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
              <Routes>
                <Route index element={<Navigate to="/matters" replace />} />

                {/* Phase 1 M1 — Matter Management */}
                <Route path="/matters"     element={<MatterList />} />
                <Route path="/matters/:id" element={<MatterDetail />} />

                {/* Phase 1 M2 — Docketing */}
                <Route path="/dockets"             element={<DocketList />} />
                <Route path="/dockets/pipeline"    element={<PipelineBoard />} />
                <Route path="/dockets/:matterId"   element={<IPAssetRecord />} />

                {/* Phase 1 M3 — Document Vault */}
                <Route path="/documents" element={<DocumentList />} />

                {/* Phase 2 M4 — Billing */}
                <Route path="/billing" element={<BillingHome />} />

                <Route path="*" element={<Navigate to="/matters" replace />} />
              </Routes>
            </AppShell>
          </SessionGate>
        }
      />
    </Routes>
  );
}