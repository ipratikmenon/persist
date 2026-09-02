// Capture screenshots of Persist Deck against fixture data.
//
//   npx vite build --config vite.config.screenshots.ts
//   node screenshots/capture.mjs
//
// Serves dist-screenshots, drives it with Playwright, writes PNGs to
// screenshots/out/. Fixtures come from screenshots/mock/core.ts — the real app
// talks to Keel over Tauri IPC, which does not exist in a browser.

// Playwright is not a dependency of the app — it is only ever used here, and
// pulling it into package.json would put a browser driver in the desktop
// build's lockfile. It lives in a scratch install instead, which means this
// path can go missing when a machine is rebuilt:
//
//   mkdir -p /tmp/shotkit && cd /tmp/shotkit \
//     && PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm i playwright
//
// The browsers themselves are already on the image at PLAYWRIGHT_BROWSERS_PATH.
const { chromium } = await import('/tmp/shotkit/node_modules/playwright/index.mjs').catch(() => {
  console.error(
    'Playwright is not installed. Run:\n' +
      '  mkdir -p /tmp/shotkit && cd /tmp/shotkit && ' +
      'PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm i playwright',
  );
  process.exit(1);
});
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
  // The versioned directory changes when the image's Playwright does; the
  // unversioned one is the image's stable alias.
  executablePath: fs.existsSync('/opt/pw-browsers/chromium-1194/chrome-linux/chrome')
    ? '/opt/pw-browsers/chromium-1194/chrome-linux/chrome'
    : '/opt/pw-browsers/chromium/chrome-linux/chrome',
  args: ['--force-color-profile=srgb', '--font-render-hinting=none'],
});
const page = await browser.newPage({
  viewport: { width: 1440, height: 900 },
  deviceScaleFactor: 2, // retina-ish, so text is legible when zoomed
});

/** Capture one element rather than the window — for a panel whose controls are
 *  the point and whose surroundings are not. */
async function elementShot(name, route, selector, { setup } = {}) {
  await page.goto(`http://localhost:${PORT}${route}`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(700);
  if (setup) await setup(page);
  const target = page.locator(selector).first();
  await target.scrollIntoViewIfNeeded();
  await page.waitForTimeout(250);
  await target.screenshot({ path: path.join(OUT, `${name}.png`) });
  console.log(`  ✓ ${name}.png`);
}

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

// ---- Page setup + annexures (S24) ----
//
// The legal notice, filled in, with the page-setup panel open. This is where an
// attorney chooses paper, typeface, size, spacing, numbering and where the
// letterhead goes — the formatting they would otherwise be doing by hand.
const fillNotice = async (p) => {
  const fill = async (id, value) => {
    const el = p.locator(`#${id}`);
    if (await el.count()) await el.fill(value);
  };
  await fill('RECIPIENT_NAME', 'Mr. Mohammed Danish');
  await p.selectOption('#MODE_OF_SERVICE', { index: 1 });
  await fill('SUBJECT', 'DEMAND FOR REFUND OF ₹1,04,000/- WITH INTEREST');
  await p.selectOption('#SALUTATION', { index: 1 });
  await fill('CLIENT_NAME', 'Mr. Nikhil Prabhakar');
  await fill('CLIENT_DESCRIPTION', 'son of P. Prabhakaran');
  await fill('CLIENT_ADDRESS', 'A-004, Mangal Apartment, Mayur Vihar Phase-III, New Delhi-110096');
  await addSections(p, SECTIONS.slice(0, 2));
  await p.waitForTimeout(1200);
};

// The numbered sections of the notice. There is no right number of them, which
// is the whole reason the field is a list.
const SECTIONS = [
  ['Background',
   'That my client and you entered into an arrangement on 11 February 2026 for the ' +
   'purchase of a motor vehicle, and that a sum of ₹1,04,000/- was paid by my client ' +
   'to you in part performance thereof.'],
  ['Breach',
   'That you have failed and neglected to deliver the said vehicle or to refund the ' +
   'said sum, despite repeated requests made by my client both orally and in writing.'],
  ['Demand',
   'That you are hereby called upon to refund the said sum together with interest ' +
   'thereon within fifteen days of the receipt of this notice.'],
];

const addSections = async (p, sections) => {
  for (const [heading, body] of sections) {
    await p.getByRole('button', { name: '+ Add a section' }).click();
    await p.waitForTimeout(120);
    await p.locator('input[id^="SECTIONS_BLOCK-"][id$="-HEADING"]').last().fill(heading);
    await p.locator('textarea[id^="SECTIONS_BLOCK-"][id$="-BODY"]').last().fill(body);
  }
};

const NOTICE = '/drafting/legal-notice?matter=P%26P-2026-TM-0042';

const PANEL = 'section:has(> button[aria-expanded])';

// The whole window, so the panel is seen where it sits — under the form, beside
// the live preview.
await shot('17-page-setup', NOTICE, {
  setup: async (p) => {
    await fillNotice(p);
    await p.getByRole('button', { name: /Page setup/ }).click();
    await p.waitForTimeout(400);
    await p.locator(PANEL).first().scrollIntoViewIfNeeded();
    await p.waitForTimeout(300);
  },
});

// The panel alone, at rest: the firm's house format.
await elementShot('18-page-setup-default', NOTICE, PANEL, {
  setup: async (p) => {
    await fillNotice(p);
    await p.getByRole('button', { name: /Page setup/ }).click();
    await p.waitForTimeout(400);
  },
});

// And with everything changed — Legal paper, Times at 14pt, 1.5 spacing,
// letterhead on page one only, bare page numbers starting at 7.
await elementShot('19-page-setup-changed', NOTICE, PANEL, {
  setup: async (p) => {
    await fillNotice(p);
    await p.getByRole('button', { name: /Page setup/ }).click();
    await p.waitForTimeout(300);
    await p.getByLabel('Paper').selectOption('legal');
    await p.getByLabel('Typeface').selectOption('times');
    await p.getByLabel('Size').selectOption('14');
    await p.getByLabel('Line spacing').selectOption('oneAndHalf');
    await p.getByLabel('Page numbers').selectOption('page');
    await p.getByLabel('First page numbered').fill('7');
    await p.getByLabel('Letterhead on').selectOption('firstPageOnly');
    await p.getByLabel('Bold throughout').check();
    await p.waitForTimeout(400);
  },
});

// Letterhead on named pages — the branch that reveals the page list.
await elementShot('20-letterhead-named-pages', NOTICE, PANEL, {
  setup: async (p) => {
    await fillNotice(p);
    await p.getByRole('button', { name: /Page setup/ }).click();
    await p.waitForTimeout(300);
    await p.getByLabel('Letterhead on').selectOption('pages');
    await p.waitForTimeout(200);
    await p.getByLabel('Which pages').fill('1, 5, 9');
    await p.waitForTimeout(400);
  },
});

// Annexures: two files attached, named, and marked A and B by Keel.
await elementShot('21-annexures', NOTICE, 'section:has-text("Attach annexures"):not(:has(section))', {
  setup: async (p) => {
    await fillNotice(p);
    await p.getByLabel('Attach annexures').check();
    await p.waitForTimeout(200);
    for (const title of ['Receipt one', 'WhatsApp payment confirmation']) {
      await p.getByRole('button', { name: '+ Add annexure' }).click();
      await p.waitForTimeout(150);
      await p.getByPlaceholder(/Name this annexure/).last().fill(title);
      await p.getByRole('button', { name: 'Choose file' }).last().click();
      await p.waitForTimeout(250);
    }
    // Past the debounce, so Keel has come back with the marks.
    await p.waitForTimeout(1500);
  },
});

// Repeating groups: three numbered sections, added one at a time, numbered by
// their order rather than by anything the attorney typed.
await shot('22-repeating-sections', NOTICE, {
  setup: async (p) => {
    await fillNotice(p);
    await addSections(p, SECTIONS.slice(2));
    await p.locator('label[for="SECTIONS_BLOCK"]').scrollIntoViewIfNeeded();
    // Past the debounce, so the preview beside it carries the sections.
    await p.waitForTimeout(1800);
  },
});

// The schedule of payments: three tranches entered, and a total the attorney
// never typed.
await shot('23-payment-schedule', NOTICE, {
  setup: async (p) => {
    await fillNotice(p);
    // A section, so the preview beside the schedule is a real notice rather
    // than the "fill the required fields" placeholder.
    await addSections(p, SECTIONS.slice(0, 1));
    const add = p.getByRole('button', { name: 'Add a payment' });
    const tranches = [
      ['2026-02-11', '7000', 'UPI', 'UTR 418223901'],
      ['2026-03-04', '12000', 'Bank transfer', 'NEFT 55120'],
    ];
    for (let i = 0; i < tranches.length; i += 1) {
      await add.click();
      await p.waitForTimeout(150);
      const [date, amount, mode, reference] = tranches[i];
      await p.locator(`#PAYMENTS_BLOCK-${i}-DATE`).fill(date);
      await p.locator(`#PAYMENTS_BLOCK-${i}-AMOUNT`).fill(amount);
      await p.locator(`#PAYMENTS_BLOCK-${i}-MODE`).selectOption(mode);
      await p.locator(`#PAYMENTS_BLOCK-${i}-REFERENCE`).fill(reference);
    }
    await p.locator('label[for="PAYMENTS_BLOCK"]').scrollIntoViewIfNeeded();
    // Past the debounce, so the preview beside it carries the schedule.
    await p.waitForTimeout(1800);
  },
});

await browser.close();
server.close();
console.log(`\nWrote ${fs.readdirSync(OUT).length} screenshots to screenshots/out/`);
