// Typography classes and style objects — use in className or style props.
// All values reference token values — never hardcode.

import { fonts, fontSizes, colors } from './tokens';
import type { CSSProperties } from 'react';

export const textStyles = {
  display: {
    fontFamily: fonts.display,
    fontSize: fontSizes.displayLg,
    fontWeight: 600,
    color: colors.textPrimary,
    lineHeight: 1.25,
  } satisfies CSSProperties,

  section: {
    fontFamily: fonts.display,
    fontSize: fontSizes.displaySm,
    fontWeight: 500,
    color: colors.textPrimary,
    lineHeight: 1.3,
  } satisfies CSSProperties,

  cardTitle: {
    fontFamily: fonts.ui,
    fontSize: fontSizes.cardTitle,
    fontWeight: 500,
    color: colors.textPrimary,
    lineHeight: 1.4,
  } satisfies CSSProperties,

  body: {
    fontFamily: fonts.ui,
    fontSize: fontSizes.body,
    fontWeight: 400,
    color: colors.textPrimary,
    lineHeight: 1.55,
  } satisfies CSSProperties,

  label: {
    fontFamily: fonts.ui,
    fontSize: fontSizes.label,
    fontWeight: 400,
    color: colors.textSecondary,
    lineHeight: 1.4,
  } satisfies CSSProperties,

  legal: {
    fontFamily: fonts.legal,
    fontSize: fontSizes.legal,
    fontWeight: 400,
    color: colors.textPrimary,
    lineHeight: 1.7,
  } satisfies CSSProperties,

  mono: {
    fontFamily: fonts.mono,
    fontSize: fontSizes.mono,
    fontWeight: 400,
    color: colors.textPrimary,
    lineHeight: 1.5,
  } satisfies CSSProperties,

  muted: {
    fontFamily: fonts.ui,
    fontSize: fontSizes.body,
    fontWeight: 400,
    color: colors.textTertiary,
    lineHeight: 1.55,
  } satisfies CSSProperties,
} as const;
