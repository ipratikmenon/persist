// Serve the built portal with /api proxied to the FastAPI backend, then drive
// it with Playwright: full OTP login, then a screenshot of each tab.
//
// This is the check a build cannot give — that the four tabs render real data
// from the real API, with the real auth flow in front of them.

import { chromium } from '/tmp/shotkit/node_modules/playwright/index.mjs';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const DIST = path.join(ROOT, 'dist');
const OUT = path.join(ROOT, 'screenshots');
const PORT = 5200;
const API = 'http://127.0.0.1:8000';

const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css',
               '.svg': 'image/svg+xml', '.png': 'image/png', '.json': 'application/json' };

// Exactly what Cloudflare will do: /api/* goes to FastAPI with the prefix
// stripped, everything else is the static bundle. The prefix is what keeps
// `/matters` the page distinct from `/matters` the endpoint.
const server = http.createServer((req, res) => {
  if (req.url.startsWith('/api/')) {
    const target = new URL(req.url.slice('/api'.length), API);
    const proxied = http.request(target, { method: req.method, headers: { ...req.headers, host: target.host } }, (upstream) => {
      res.writeHead(upstream.statusCode, upstream.headers);
      upstream.pipe(res);
    });
    proxied.on('error', (e) => { res.writeHead(502); res.end(String(e)); });
    req.pipe(proxied);
    return;
  }
  const urlPath = decodeURIComponent(req.url.split('?')[0]);
  let file = path.join(DIST, urlPath);
  if (!fs.existsSync(file) || fs.statSync(file).isDirectory()) file = path.join(DIST, 'index.html');
  res.writeHead(200, { 'Content-Type': MIME[path.extname(file)] ?? 'application/octet-stream' });
  fs.createReadStream(file).pipe(res);
});

await new Promise(r => server.listen(PORT, r));
fs.mkdirSync(OUT, { recursive: true });

const browser = await chromium.launch({
  executablePath: '/opt/pw-browsers/chromium-1194/chrome-linux/chrome',
  args: ['--force-color-profile=srgb', '--font-render-hinting=none'],
});
const page = await browser.newPage({ viewport: { width: 1100, height: 900 }, deviceScaleFactor: 2 });

// Surface anything the page complains about — a silent console error is how a
// blank screenshot gets mistaken for a working page.
page.on('console', (m) => { if (m.type() === 'error') console.log('  [console]', m.text()); });
page.on('pageerror', (e) => console.log('  [pageerror]', e.message));
page.on('requestfailed', (r) => console.log('  [failed]', r.url(), r.failure()?.errorText));

const shot = async (name) => {
  await page.screenshot({ path: path.join(OUT, `${name}.png`) });
  console.log(`  ✓ ${name}.png`);
};

console.log('Portal:');
await page.goto(`http://localhost:${PORT}/matters`, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('#email, nav a', { timeout: 15000 });
await shot('01-login');

// Step 1 of the OTP flow.
await page.fill('#email', 'anita@petalveda.in');
await page.click('button[type=submit]');
await page.waitForSelector('#code');
await shot('02-login-code');

// The dev backend echoes the code back so this can be automated; production
// never does. Read it from the API rather than the page — the browser is not
// shown it either.
const issued = await fetch(`${API}/auth/request-otp`, {
  method: 'POST', headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ email: 'anita@petalveda.in' }),
}).then(r => r.json());

await page.fill('#code', issued.debugCode);
await page.click('button[type=submit]');
await page.waitForSelector('nav a', { timeout: 10000 });
await page.waitForTimeout(600);
await shot('03-matters');

await page.click('text=PETALVEDA');
await page.waitForTimeout(600);
await shot('04-matter-detail');

await page.click('nav >> text=Documents');
await page.waitForTimeout(600);
await shot('05-documents');

await page.click('nav >> text=Invoices');
await page.waitForTimeout(600);
await shot('06-invoices');

await page.click('text=INV-2026-0042');
await page.waitForTimeout(600);
await shot('07-invoice-detail');

await page.click('text=Raise a query');
await page.waitForTimeout(400);
await shot('08-invoice-query');

await page.click('nav >> text=Account');
await page.waitForTimeout(600);
await shot('09-account');

// A page reload must not look like a logout: the access token is gone from
// memory, so the app mints a new one from the refresh cookie.
await page.reload({ waitUntil: 'domcontentloaded' });
await page.waitForTimeout(800);
const stillIn = await page.locator('nav a').count();
console.log(stillIn > 0 ? '  ✓ session survived a reload' : '  ✗ RELOAD LOGGED THE CLIENT OUT');
await shot('10-after-reload');

await browser.close();
server.close();
console.log(`\nWrote ${fs.readdirSync(OUT).length} screenshots`);
