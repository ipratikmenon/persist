// Capture screenshots of Persist Deck against fixture data.
//
//   npx vite build --config vite.config.screenshots.ts
//   node screenshots/capture.mjs
//
// Serves dist-screenshots, drives it with Playwright, writes PNGs to
// screenshots/out/. Fixtures come from screenshots/mock/core.ts — the real app
// talks to Keel over Tauri IPC, which does not exist in a browser.

import { chromium } from '/tmp/shotkit/node_modules/playwright/index.mjs';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const DIST = path.join(ROOT, 'dist-screenshots');
const OUT  = path.join(ROOT, 'screenshots', 'out');
const PORT = 5199;

const MIME = {
  '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css',
  '.svg': 'image/svg+xml', '.png': 'image/png', '.json': 'application/json',
};

// Static server with SPA fallback — React Router owns the paths.
const server = http.createServer((req, res) => {
  const urlPath = decodeURIComponent(req.url.split('?')[0]);
  let file = path.join(DIST, urlPath);
  if (!fs.existsSync(file) || fs.statSync(file).isDirectory()) {
    file = path.join(DIST, 'index.html');
  }
  res.writeHead(200, { 'Content-Type': MIME[path.extname(file)] ?? 'application/octet-stream' });
  fs.createReadStream(file).pipe(res);
});

await new Promise(r => server.listen(PORT, r));
fs.mkdirSync(OUT, { recursive: true });

const browser = await chromium.launch({
  executablePath: '/opt/pw-browsers/chromium-1194/chrome-linux/chrome',
  args: ['--force-color-profile=srgb', '--font-render-hinting=none'],
});
const page = await browser.newPage({
  viewport: { width: 1440, height: 900 },
  deviceScaleFactor: 2, // retina-ish, so text is legible when zoomed
});

/** Navigate, let entrance animations settle, capture. */
async function shot(name, route, { setup } = {}) {
  await page.goto(`http://localhost:${PORT}${route}`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(700); // Motion entrance + stagger
  if (setup) await setup(page);
  await page.screenshot({ path: path.join(OUT, `${name}.png`) });
  console.log(`  ✓ ${name}.png`);
}

console.log('Capturing:');

await shot('01-matters-list', '/matters');
await shot('02-matter-detail', '/matters/P&P-2026-TM-0042');
await shot('03-docket-list', '/dockets');
await shot('04-pipeline-board', '/dockets/pipeline');
await shot('05-renewal-dashboard', '/dockets/renewals');

// IP asset record — the docket timeline with verification badges + references.
await shot('06-ip-asset-record', '/dockets/P&P-2026-TM-0042');

// Cascade preview drawer: open it, run the preview, capture the chain.
await shot('07-cascade-preview', '/dockets/P&P-2026-TM-0042', {
  setup: async (p) => {
    await p.getByText('Generate chain').first().click();
    await p.waitForTimeout(400);
    await p.getByRole('button', { name: 'Trademark Application' }).click();
    await p.getByRole('button', { name: 'Preview chain' }).click();
    await p.waitForTimeout(500);
  },
});

// Add-asset drawer, to show the form idiom.
await shot('08-ip-asset-drawer', '/dockets/P&P-2026-TM-0042', {
  setup: async (p) => {
    await p.getByRole('button', { name: '+ Add asset' }).click();
    await p.waitForTimeout(450);
  },
});

// Hover a row so the per-document actions are visible — including the
// metadata-stripping "Client copy" export added for B07.
await shot('09-documents', '/documents', {
  setup: async (p) => {
    await p.getByText('Examination-Report-5544121.pdf').first().hover();
    await p.waitForTimeout(300);
  },
});
await shot('10-billing', '/billing');

// Client Portal — the desktop side of M5.
await shot('11-portal-client-access', '/portal');
await shot('12-portal-sync', '/portal', {
  setup: async (p) => {
    await p.getByRole('button', { name: 'Sync' }).click();
    await p.waitForTimeout(400);
  },
});

// The configured firm: sync on, token stored, one rejection outstanding.
await shot('13-portal-sync-live', '/portal?sync=on', {
  setup: async (p) => {
    await p.getByRole('button', { name: 'Sync' }).click();
    await p.waitForTimeout(400);
  },
});

// ---- Drafting: the Smart Form Compiler (M9.8) ----
await shot('14-drafting-picker', '/drafting');

// Empty form: nothing is required of the attorney until they start, and the
// preview pane says what it is waiting for.
await shot('15-smart-form-empty', '/drafting/tm-examination-reply?matter=P%26P-2026-TM-0042');

// Filled: the live preview appears once the required fields are in.
await shot('16-smart-form-filled', '/drafting/tm-examination-reply?matter=P%26P-2026-TM-0042', {
  setup: async (p) => {
    const fill = async (id, value) => {
      const el = p.locator(`#${id}`);
      if (await el.count()) await el.fill(value);
    };
    await fill('ATTORNEY_NAME', 'Sree Lakshmi Menon');
    await fill('REPLY_DATE', '2026-08-11');
    await p.selectOption('#REGISTRY_OFFICE', 'Delhi');
    await fill('TM_NUMBER', '5544121');
    await fill('TM_MARK', 'PETALVEDA');
    await fill('TM_CLASS', '3, 5');
    await fill('APPLICANT_NAME', 'Tata & Sons Pvt Ltd');
    await fill('EXAM_REPORT_DATE', '2026-06-15');
    await fill('SUBMISSIONS', 'The objection under s.11(1) is respectfully denied. The cited mark covers goods in class 30 and is 100% distinct in its trade channels.');
    await fill('GROUNDS_TEXT', 'Prior use since 2019; no likelihood of confusion.');
    // Choosing prior use reveals the first-use date — the conditional field.
    await p.selectOption('#GROUNDS', 'PriorUse');
    await p.waitForTimeout(300);
    await fill('FIRST_USE_DATE', '2019-04-01');
    // Past the debounce, so the preview has rendered.
    await p.waitForTimeout(1800);
  },
});

await browser.close();
server.close();
console.log(`\nWrote ${fs.readdirSync(OUT).length} screenshots to screenshots/out/`);
