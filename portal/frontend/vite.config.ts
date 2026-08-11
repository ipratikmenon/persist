import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'node:path';

// The portal is a browser app. No Tauri, no invoke() — data comes from the
// FastAPI backend over fetch().
//
// The design tokens are aliased to Deck's file rather than copied. A client
// sees the portal and the firm's invoices side by side; two drifting copies of
// the palette would show. `fs.allow` is what lets Vite read outside the root.
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
      '@deck-tokens': path.resolve(__dirname, '../../src/design-system/tokens.ts'),
    },
  },
  server: {
    fs: { allow: [path.resolve(__dirname), path.resolve(__dirname, '../../src')] },
    proxy: {
      // Dev only. In production Cloudflare routes /api to the FastAPI host.
      '/api': { target: 'http://127.0.0.1:8000', changeOrigin: true,
                rewrite: (p) => p.replace(/^\/api/, '') },
    },
  },
});
