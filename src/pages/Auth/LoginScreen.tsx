/**
 * LoginScreen — full-screen authentication gate.
 *
 * Shown whenever there is no active session (app start, after logout).
 * Two firm attorneys are pre-seeded with default password "persist2026".
 * This is a local desktop app — no network auth, no OAuth.
 */
import { useState, useRef, useEffect } from 'react';
import { motion } from 'motion/react';
import { keel } from '@/lib/tauri';
import { useAuthStore } from '@/stores/auth';
import { colors, fonts, fontSizes, radius, spacing } from '@/design-system/tokens';
import { transition } from '@/design-system/motion';

// ---------------------------------------------------------------------------
// Quick-select attorney chips (firm has exactly 2 users for Phase 1)
// ---------------------------------------------------------------------------

const ATTORNEYS = [
  { name: 'Sree Lakshmi Menon', email: 'slm@persist.in', initials: 'SLM', role: 'Partner' },
  { name: 'Kajal Thakur',       email: 'kt@persist.in',  initials: 'KT',  role: 'Associate' },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function LoginScreen() {
  const { setSession } = useAuthStore();

  const [email, setEmail]         = useState('');
  const [password, setPassword]   = useState('');
  const [showPwd, setShowPwd]     = useState(false);
  const [loading, setLoading]     = useState(false);
  const [error, setError]         = useState<string | null>(null);

  const emailRef    = useRef<HTMLInputElement>(null);
  const passwordRef = useRef<HTMLInputElement>(null);

  // Auto-focus email on mount
  useEffect(() => { emailRef.current?.focus(); }, []);

  const handleAttorneySelect = (attorneyEmail: string) => {
    setEmail(attorneyEmail);
    setError(null);
    setTimeout(() => passwordRef.current?.focus(), 50);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!email.trim() || !password) return;
    setLoading(true);
    setError(null);
    try {
      const session = await keel.auth.login({ email: email.trim(), password });
      setSession(session);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setPassword('');
      passwordRef.current?.focus();
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{
      position: 'fixed', inset: 0,
      background: colors.bgPrimary,
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      padding: spacing[8],
      // macOS-style subtle noise texture feel via layered backgrounds
      backgroundImage: 'radial-gradient(ellipse at 50% 0%, rgba(74,101,128,0.06) 0%, transparent 70%)',
    }}>

      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
        style={{
          width: '100%',
          maxWidth: 380,
          display: 'flex',
          flexDirection: 'column',
          gap: spacing[8],
        }}
      >
        {/* ── Firm identity ─────────────────────────────── */}
        <div style={{ textAlign: 'center' }}>
          {/* Monogram lockup */}
          <div style={{
            width: 52,
            height: 52,
            borderRadius: 14,
            background: colors.accentPrimary,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            margin: '0 auto 16px',
            boxShadow: '0 4px 16px rgba(74,101,128,0.30)',
          }}>
            <span style={{
              fontFamily: fonts.display,
              fontSize: '18px',
              fontWeight: 600,
              color: '#fff',
              letterSpacing: '-0.02em',
            }}>
              P&P
            </span>
          </div>

          <h1 style={{
            fontFamily: fonts.display,
            fontSize: '22px',
            fontWeight: 600,
            color: colors.textPrimary,
            margin: '0 0 4px',
            letterSpacing: '-0.02em',
          }}>
            Persistas &amp; Partners
          </h1>
          <p style={{
            fontSize: fontSizes.label,
            color: colors.textTertiary,
            margin: 0,
            fontFamily: fonts.ui,
            letterSpacing: '0.06em',
            textTransform: 'uppercase' as const,
          }}>
            Delhi · Intellectual Property
          </p>
        </div>

        {/* ── Quick-select chips ────────────────────────── */}
        <div>
          <p style={{
            fontSize: fontSizes.label,
            color: colors.textTertiary,
            fontFamily: fonts.ui,
            margin: `0 0 ${spacing[2]}`,
            fontWeight: 500,
            textTransform: 'uppercase' as const,
            letterSpacing: '0.06em',
          }}>
            Sign in as
          </p>
          <div style={{ display: 'flex', gap: spacing[3] }}>
            {ATTORNEYS.map(a => {
              const selected = email === a.email;
              return (
                <button
                  key={a.email}
                  onClick={() => handleAttorneySelect(a.email)}
                  style={{
                    flex: 1,
                    padding: `${spacing[3]} ${spacing[3]}`,
                    borderRadius: radius.card,
                    border: selected
                      ? `1.5px solid ${colors.accentPrimary}`
                      : `0.5px solid ${colors.border}`,
                    background: selected
                      ? 'rgba(74,101,128,0.08)'
                      : colors.bgSecondary,
                    cursor: 'pointer',
                    textAlign: 'left' as const,
                    transition: `all ${transition.fast.duration}s`,
                  }}
                >
                  {/* Initials circle */}
                  <div style={{
                    width: 32,
                    height: 32,
                    borderRadius: '50%',
                    background: selected ? colors.accentPrimary : colors.border,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    marginBottom: spacing[2],
                    transition: `background ${transition.fast.duration}s`,
                  }}>
                    <span style={{
                      fontSize: '11px',
                      fontFamily: fonts.mono,
                      fontWeight: 500,
                      color: selected ? '#fff' : colors.textSecondary,
                      letterSpacing: '0.02em',
                    }}>
                      {a.initials}
                    </span>
                  </div>
                  <div style={{
                    fontSize: '13px',
                    fontWeight: 500,
                    color: selected ? colors.accentPrimary : colors.textPrimary,
                    fontFamily: fonts.ui,
                    marginBottom: 2,
                    lineHeight: 1.2,
                  }}>
                    {a.name.split(' ')[0]}
                  </div>
                  <div style={{
                    fontSize: fontSizes.label,
                    color: colors.textTertiary,
                    fontFamily: fonts.ui,
                  }}>
                    {a.role}
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        {/* ── Login form ────────────────────────────────── */}
        <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: spacing[3] }}>
          {/* Email */}
          <div>
            <label style={labelStyle}>Email</label>
            <input
              ref={emailRef}
              type="email"
              value={email}
              onChange={e => { setEmail(e.target.value); setError(null); }}
              placeholder="you@persist.in"
              autoComplete="username"
              style={inputStyle(!!error)}
            />
          </div>

          {/* Password */}
          <div>
            <label style={labelStyle}>Password</label>
            <div style={{ position: 'relative' }}>
              <input
                ref={passwordRef}
                type={showPwd ? 'text' : 'password'}
                value={password}
                onChange={e => { setPassword(e.target.value); setError(null); }}
                placeholder="••••••••"
                autoComplete="current-password"
                style={{ ...inputStyle(!!error), paddingRight: 40 }}
              />
              <button
                type="button"
                onClick={() => setShowPwd(v => !v)}
                style={{
                  position: 'absolute',
                  right: 12,
                  top: '50%',
                  transform: 'translateY(-50%)',
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  color: colors.textTertiary,
                  fontSize: fontSizes.label,
                  padding: 0,
                  fontFamily: fonts.mono,
                }}
              >
                {showPwd ? 'hide' : 'show'}
              </button>
            </div>
          </div>

          {/* Error */}
          {error && (
            <motion.div
              initial={{ opacity: 0, y: -4 }}
              animate={{ opacity: 1, y: 0 }}
              style={{
                padding: `${spacing[3]} ${spacing[4]}`,
                background: 'rgba(192,57,43,0.06)',
                border: `0.5px solid rgba(192,57,43,0.25)`,
                borderRadius: radius.card,
                fontSize: fontSizes.label,
                color: colors.statusUrgent,
                fontFamily: fonts.ui,
              }}
            >
              {error}
            </motion.div>
          )}

          {/* Submit */}
          <button
            type="submit"
            disabled={loading || !email.trim() || !password}
            style={{
              marginTop: spacing[1],
              padding: `${spacing[3]} 0`,
              borderRadius: radius.button,
              border: 'none',
              background: (loading || !email.trim() || !password)
                ? colors.border
                : colors.accentPrimary,
              color: (loading || !email.trim() || !password)
                ? colors.textTertiary
                : '#fff',
              fontFamily: fonts.ui,
              fontSize: fontSizes.body,
              fontWeight: 500,
              cursor: (loading || !email.trim() || !password) ? 'default' : 'pointer',
              transition: `background ${transition.fast.duration}s`,
              letterSpacing: '0.01em',
            }}
          >
            {loading ? 'Signing in…' : 'Sign in'}
          </button>
        </form>

        {/* ── Footer hint ───────────────────────────────── */}
        <p style={{
          textAlign: 'center',
          fontSize: fontSizes.label,
          color: colors.textTertiary,
          fontFamily: fonts.ui,
          margin: 0,
          lineHeight: 1.5,
        }}>
          Default password: <span style={{ fontFamily: fonts.mono, color: colors.textSecondary }}>persist2026</span>
        </p>
      </motion.div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shared input styles
// ---------------------------------------------------------------------------

const labelStyle: React.CSSProperties = {
  display: 'block',
  fontSize: '11px',
  fontWeight: 500,
  color: colors.textTertiary,
  fontFamily: fonts.ui,
  textTransform: 'uppercase',
  letterSpacing: '0.06em',
  marginBottom: spacing[1],
};

const inputStyle = (hasError: boolean): React.CSSProperties => ({
  width: '100%',
  padding: `${spacing[3]} ${spacing[3]}`,
  borderRadius: radius.input,
  border: `0.5px solid ${hasError ? 'rgba(192,57,43,0.5)' : colors.border}`,
  background: colors.bgSecondary,
  color: colors.textPrimary,
  fontFamily: fonts.ui,
  fontSize: fontSizes.body,
  outline: 'none',
  boxSizing: 'border-box' as const,
});
