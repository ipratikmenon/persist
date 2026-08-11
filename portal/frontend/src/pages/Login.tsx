// Sign in. Two steps, no password anywhere.
//
// The backend answers `request-otp` identically whether or not the address is
// known, so this screen must too — it always advances to the code step. Telling
// someone "no such account" would hand an attacker a client list.

import { useEffect, useRef, useState } from 'react';
import { api, ApiError } from '@/lib/api';
import { Button, Notice, inputStyle } from '@/components/ui';
import { colors, fonts, fontSizes, radius, shadows, spacing } from '@/design-system/tokens';

type Step = 'email' | 'code';

export function Login({ onSignedIn }: { onSignedIn: () => void }) {
  const [step, setStep] = useState<Step>('email');
  const [email, setEmail] = useState('');
  const [code, setCode] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const codeInput = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (step === 'code') codeInput.current?.focus();
  }, [step]);

  const sendCode = async () => {
    if (!email.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await api.auth.requestOtp(email.trim());
      // Advances whether or not the address is known — see the note above.
      setStep('code');
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Could not send a code. Please try again.');
    } finally {
      setBusy(false);
    }
  };

  const verify = async () => {
    if (!code.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await api.auth.verifyOtp(email.trim(), code.trim());
      onSignedIn();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'That code is not valid.');
      setCode('');
      codeInput.current?.focus();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      style={{
        minHeight: '100vh',
        display: 'grid',
        placeItems: 'center',
        background: colors.bgPrimary,
        padding: spacing[5],
      }}
    >
      <main
        style={{
          width: '100%',
          maxWidth: 380,
          background: colors.bgSecondary,
          border: `0.5px solid ${colors.border}`,
          borderRadius: radius.card,
          boxShadow: shadows.card,
          padding: spacing[8],
        }}
      >
        <h1
          style={{
            margin: 0,
            fontFamily: fonts.display,
            fontSize: fontSizes.displaySm,
            fontWeight: 600,
            color: colors.accentPrimary,
          }}
        >
          Persistas &amp; Partners
        </h1>
        <p
          style={{
            margin: `${spacing[1]} 0 ${spacing[6]}`,
            fontFamily: fonts.ui,
            fontSize: fontSizes.label,
            letterSpacing: '0.08em',
            textTransform: 'uppercase',
            color: colors.textSecondary,
          }}
        >
          Client Portal
        </p>

        {step === 'email' ? (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              sendCode();
            }}
          >
            <label
              htmlFor="email"
              style={{
                display: 'block',
                marginBottom: spacing[2],
                fontFamily: fonts.ui,
                fontSize: fontSizes.body,
                color: colors.textPrimary,
              }}
            >
              Your email address
            </label>
            <input
              id="email"
              type="email"
              autoComplete="email"
              autoFocus
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="you@company.com"
              style={inputStyle}
            />
            <p
              style={{
                margin: `${spacing[2]} 0 ${spacing[5]}`,
                fontFamily: fonts.ui,
                fontSize: 11,
                color: colors.textTertiary,
              }}
            >
              We will send a six-digit code. There is no password to remember.
            </p>

            {error && <div style={{ marginBottom: spacing[4] }}><Notice message={error} /></div>}

            <Button type="submit" disabled={busy || !email.trim()}>
              {busy ? 'Sending…' : 'Send code'}
            </Button>
          </form>
        ) : (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              verify();
            }}
          >
            <label
              htmlFor="code"
              style={{
                display: 'block',
                marginBottom: spacing[2],
                fontFamily: fonts.ui,
                fontSize: fontSizes.body,
                color: colors.textPrimary,
              }}
            >
              Six-digit code
            </label>
            <input
              id="code"
              ref={codeInput}
              inputMode="numeric"
              autoComplete="one-time-code"
              maxLength={6}
              value={code}
              onChange={(e) => setCode(e.target.value.replace(/\D/g, ''))}
              placeholder="123456"
              style={{
                ...inputStyle,
                fontFamily: fonts.mono,
                fontSize: '20px',
                letterSpacing: '0.3em',
                textAlign: 'center',
              }}
            />
            <p
              style={{
                margin: `${spacing[2]} 0 ${spacing[5]}`,
                fontFamily: fonts.ui,
                fontSize: 11,
                color: colors.textTertiary,
              }}
            >
              Sent to {email}. The code expires in ten minutes.
            </p>

            {error && <div style={{ marginBottom: spacing[4] }}><Notice message={error} /></div>}

            <div style={{ display: 'flex', gap: spacing[3], alignItems: 'center' }}>
              <Button type="submit" disabled={busy || code.length < 4}>
                {busy ? 'Checking…' : 'Sign in'}
              </Button>
              <Button
                variant="quiet"
                disabled={busy}
                onClick={() => {
                  setStep('email');
                  setCode('');
                  setError(null);
                }}
              >
                Use another address
              </Button>
            </div>
          </form>
        )}
      </main>
    </div>
  );
}
