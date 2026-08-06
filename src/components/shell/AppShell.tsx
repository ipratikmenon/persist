import { useNavigate, useLocation } from 'react-router';
import { motion } from 'motion/react';
import { useReducedMotion } from 'motion/react';
import type { ReactNode } from 'react';
import { transition } from '@/design-system/motion';
import { colors, fonts, fontSizes } from '@/design-system/tokens';
import { useAuthStore } from '@/stores/auth';

const NAV_ITEMS = [
  { path: '/matters',          label: 'Matters'   },
  { path: '/dockets',          label: 'Dockets'   },
  { path: '/dockets/renewals', label: 'Renewals'  },
  { path: '/dockets/pipeline', label: 'Pipeline'  },
  { path: '/documents',        label: 'Documents' },
  { path: '/billing',          label: 'Billing'   },
  { path: '/portal',           label: 'Portal'    },
] as const;

// Initials from full name — "Sree Lakshmi Menon" → "SLM"
function initials(name: string): string {
  return name
    .split(' ')
    .filter(Boolean)
    .map(w => w[0].toUpperCase())
    .join('');
}

interface AppShellProps {
  children: ReactNode;
}

export function AppShell({ children }: AppShellProps) {
  const navigate    = useNavigate();
  const location    = useLocation();
  const shouldReduce = useReducedMotion();
  const { session } = useAuthStore();

  const handleLogout = () => {
    const fn = (window as Window & { __persistLogout?: () => void }).__persistLogout;
    if (fn) fn();
  };

  return (
    <div style={{
      display: 'flex',
      height: '100vh',
      background: colors.bgPrimary,
      overflow: 'hidden',
    }}>
      {/* ── Sidebar ─────────────────────────────────────────── */}
      <aside style={{
        width: 220,
        flexShrink: 0,
        background: colors.bgSecondary,
        borderRight: `0.5px solid ${colors.border}`,
        display: 'flex',
        flexDirection: 'column',
        userSelect: 'none',
      }}>
        {/* Firm identity */}
        <div style={{
          padding: '22px 20px 18px',
          borderBottom: `0.5px solid ${colors.border}`,
        }}>
          <div style={{
            fontFamily: fonts.display,
            fontSize: 15,
            fontWeight: 600,
            color: colors.textPrimary,
            lineHeight: 1.3,
            letterSpacing: '-0.01em',
          }}>
            Persistas &amp; Partners
          </div>
          <div style={{
            fontFamily: fonts.mono,
            fontSize: 10,
            color: colors.textTertiary,
            marginTop: 4,
            letterSpacing: '0.06em',
            textTransform: 'uppercase' as const,
          }}>
            Practice Management
          </div>
        </div>

        {/* Navigation */}
        <nav style={{ flex: 1, padding: '10px 0', overflowY: 'auto' }}>
          {NAV_ITEMS.map((item) => {
            // Exact match, or a child route — but never a sibling that merely
            // shares a prefix ('/dockets' must not light up on '/dockets/renewals').
            const isActive = location.pathname === item.path
              || location.pathname.startsWith(`${item.path}/`);
            return (
              <motion.button
                key={item.path}
                onClick={() => navigate(item.path)}
                whileHover={!shouldReduce ? { x: 2 } : undefined}
                transition={transition.fast}
                style={{
                  width: '100%',
                  textAlign: 'left',
                  padding: '9px 20px 9px 17px',
                  background: isActive ? 'rgba(74, 101, 128, 0.09)' : 'transparent',
                  border: 'none',
                  borderLeft: `3px solid ${isActive ? colors.accentPrimary : 'transparent'}`,
                  cursor: 'pointer',
                  color: isActive ? colors.accentPrimary : colors.textSecondary,
                  fontSize: fontSizes.body,
                  fontFamily: fonts.ui,
                  fontWeight: isActive ? 500 : 400,
                }}
              >
                {item.label}
              </motion.button>
            );
          })}
        </nav>

        {/* ── Logged-in user + logout ──────────────────────── */}
        <div style={{
          padding: '12px 20px 14px',
          borderTop: `0.5px solid ${colors.border}`,
        }}>
          {session ? (
            <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
              {/* Avatar circle */}
              <div style={{
                width: 30,
                height: 30,
                borderRadius: '50%',
                background: colors.accentPrimary,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                flexShrink: 0,
              }}>
                <span style={{
                  fontSize: 10,
                  fontFamily: fonts.mono,
                  fontWeight: 500,
                  color: '#fff',
                  letterSpacing: '0.02em',
                }}>
                  {initials(session.name)}
                </span>
              </div>

              {/* Name + role */}
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{
                  fontSize: fontSizes.label,
                  color: colors.textSecondary,
                  fontWeight: 500,
                  fontFamily: fonts.ui,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}>
                  {session.name.split(' ')[0]}
                </div>
                <div style={{
                  fontSize: 10,
                  color: colors.textTertiary,
                  marginTop: 1,
                  fontFamily: fonts.mono,
                  letterSpacing: '0.03em',
                }}>
                  {session.role}
                </div>
              </div>

              {/* Logout button */}
              <button
                onClick={handleLogout}
                title="Sign out"
                style={{
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  color: colors.textTertiary,
                  fontSize: 16,
                  padding: '2px 4px',
                  borderRadius: 4,
                  lineHeight: 1,
                  flexShrink: 0,
                }}
                onMouseEnter={e => (e.currentTarget.style.color = colors.statusUrgent)}
                onMouseLeave={e => (e.currentTarget.style.color = colors.textTertiary)}
              >
                ⎋
              </button>
            </div>
          ) : (
            <div style={{
              fontSize: fontSizes.label,
              color: colors.textTertiary,
              fontFamily: fonts.ui,
            }}>
              Not signed in
            </div>
          )}
        </div>
      </aside>

      {/* ── Main content ────────────────────────────────────── */}
      <main style={{
        flex: 1,
        overflow: 'auto',
        display: 'flex',
        flexDirection: 'column',
      }}>
        {children}
      </main>
    </div>
  );
}
