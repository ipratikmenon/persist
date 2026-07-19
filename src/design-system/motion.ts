// Motion design tokens — use these in all Motion v12 animations.
// Never hardcode duration or easing values in components.
// Import: import { transition, ease, duration } from '@/design-system/motion';

export const duration = {
  fast:   0.15,   // 150ms — hover states, micro-interactions
  normal: 0.30,   // 300ms — panel slides, list entrance
  slow:   0.50,   // 500ms — page transitions, modal entrance
} as const;

export const ease = {
  outExpo:  [0.16, 1, 0.3, 1] as [number, number, number, number],
  outQuart: [0.25, 1, 0.5, 1] as [number, number, number, number],
  inOut:    [0.4, 0, 0.2, 1]  as [number, number, number, number],
  spring:   { type: 'spring', stiffness: 400, damping: 30 } as const,
} as const;

export const transition = {
  fast:   { duration: duration.fast,   ease: ease.outExpo },
  normal: { duration: duration.normal, ease: ease.outExpo },
  slow:   { duration: duration.slow,   ease: ease.outExpo },
  spring: ease.spring,
} as const;

// Stagger children — use with variants on list containers
export const stagger = {
  fast:   { staggerChildren: 0.03 },
  normal: { staggerChildren: 0.05 },
  slow:   { staggerChildren: 0.08 },
} as const;

// Standard card hover
export const cardHover = {
  whileHover: { y: -2, boxShadow: '0 4px 12px rgba(0,0,0,0.10)' },
  whileTap:   { scale: 0.99 },
  transition: transition.fast,
} as const;

// Panel slide variants
export const panelVariants = {
  right: {
    hidden:  { x: '100%', opacity: 0 },
    visible: { x: 0,      opacity: 1, transition: transition.normal },
    exit:    { x: '100%', opacity: 0, transition: transition.fast  },
  },
  bottom: {
    hidden:  { y: '100%', opacity: 0 },
    visible: { y: 0,      opacity: 1, transition: transition.normal },
    exit:    { y: '100%', opacity: 0, transition: transition.fast  },
  },
} as const;

// Modal scale variant
export const modalVariants = {
  hidden:  { scale: 0.96, opacity: 0 },
  visible: { scale: 1,    opacity: 1, transition: transition.normal },
  exit:    { scale: 0.96, opacity: 0, transition: transition.fast  },
} as const;
