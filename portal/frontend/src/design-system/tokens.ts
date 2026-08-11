// Design tokens — re-exported from Deck.
//
// The portal and the desktop app must not drift apart visually: a client sees
// this portal and the firm's invoices side by side. Rather than keeping a copy
// in step by hand, this re-exports the single source in src/design-system/,
// aliased at build time (see vite.config.ts).
export * from '@deck-tokens';
