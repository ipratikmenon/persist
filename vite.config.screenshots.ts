// Screenshot build — the normal app, with Tauri IPC swapped for fixtures.
//
// Used only by screenshots/capture.mjs. `pnpm build` is untouched, so nothing
// here can reach a shipped binary.
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'node:path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: [
      { find: '@tauri-apps/api/core',      replacement: path.resolve(__dirname, 'screenshots/mock/core.ts') },
      { find: '@tauri-apps/plugin-dialog', replacement: path.resolve(__dirname, 'screenshots/mock/plugins.ts') },
      { find: '@tauri-apps/plugin-fs',     replacement: path.resolve(__dirname, 'screenshots/mock/plugins.ts') },
      { find: '@', replacement: path.resolve(__dirname, 'src') },
    ],
  },
  build: { outDir: 'dist-screenshots', emptyOutDir: true },
});
