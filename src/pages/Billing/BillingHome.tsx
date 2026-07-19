/**
 * BillingHome — tabbed container for the Billing section.
 *
 * Tab 1: Invoices (InvoiceList)
 * Tab 2: Time Entries (TimeTracker)
 * Tab 3: Settings (FirmSettings — Partner-only)
 */
import { useState } from 'react';
import { motion } from 'motion/react';
import { useReducedMotion } from 'motion/react';
import { colors, fonts, fontSizes, spacing } from '@/design-system/tokens';
import { transition } from '@/design-system/motion';
import { useAuthStore } from '@/stores/auth';
import InvoiceList from './InvoiceList';
import TimeTracker from './TimeTracker';
import FirmSettingsPanel from './FirmSettingsPanel';

const TABS = [
  { key: 'invoices',     label: 'Invoices' },
  { key: 'time-entries', label: 'Time Entries' },
  { key: 'settings',     label: 'Settings' },
] as const;

type TabKey = typeof TABS[number]['key'];

export default function BillingHome() {
  const [activeTab, setActiveTab] = useState<TabKey>('invoices');
  const shouldReduce = useReducedMotion();
  const { session } = useAuthStore();
  const isPartner = session?.role === 'Partner' || session?.role === 'Admin';

  return (
    <div style={{ padding: `${spacing[6]} ${spacing[8]}`, height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div style={{ marginBottom: spacing[6] }}>
        <h1 style={{
          fontFamily: fonts.display,
          fontSize: 28,
          fontWeight: 600,
          color: colors.textPrimary,
          margin: 0,
          letterSpacing: '-0.02em',
        }}>
          Billing
        </h1>
        <p style={{
          fontFamily: fonts.ui,
          fontSize: fontSizes.label,
          color: colors.textTertiary,
          margin: `${spacing[1]} 0 0`,
          letterSpacing: '0.06em',
          textTransform: 'uppercase' as const,
        }}>
          Time tracking, invoices &amp; payments
        </p>
      </div>

      {/* Tab bar */}
      <div style={{
        display: 'flex',
        gap: spacing[1],
        borderBottom: `0.5px solid ${colors.border}`,
        marginBottom: spacing[5],
      }}>
        {TABS.map(tab => {
          // Hide settings tab for non-partner roles
          if (tab.key === 'settings' && !isPartner) return null;

          const isActive = activeTab === tab.key;
          return (
            <motion.button
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              whileHover={!shouldReduce ? { y: -1 } : undefined}
              transition={transition.fast}
              style={{
                padding: `${spacing[3]} ${spacing[4]}`,
                background: 'transparent',
                border: 'none',
                borderBottom: `2px solid ${isActive ? colors.accentPrimary : 'transparent'}`,
                cursor: 'pointer',
                fontFamily: fonts.ui,
                fontSize: fontSizes.body,
                fontWeight: isActive ? 500 : 400,
                color: isActive ? colors.accentPrimary : colors.textSecondary,
                marginBottom: -0.5,
                transition: `color ${transition.fast.duration}s`,
              }}
            >
              {tab.label}
            </motion.button>
          );
        })}
      </div>

      {/* Tab content */}
      <div style={{ flex: 1, overflow: 'auto' }}>
        {activeTab === 'invoices'     && <InvoiceList />}
        {activeTab === 'time-entries' && <TimeTracker />}
        {activeTab === 'settings'     && isPartner && <FirmSettingsPanel />}
      </div>
    </div>
  );
}
