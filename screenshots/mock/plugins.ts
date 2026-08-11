// Screenshot harness — stands in for the Tauri dialog and fs plugins.
export async function save(): Promise<string | null> { return null; }

// The annexure picker calls this. Returning a path rather than null lets the
// screenshots show a row with a file attached, which is the state worth seeing.
let picked = 0;
const FILES = [
  '/Users/slm/Documents/Receipt-14-July-2026.pdf',
  '/Users/slm/Documents/WhatsApp-payment-confirmation.png',
  '/Users/slm/Documents/Bank-statement-July-2026.pdf',
];
export async function open(): Promise<string | null> {
  return FILES[picked++ % FILES.length];
}

export async function writeFile(): Promise<void> {}
export async function readFile(): Promise<Uint8Array> { return new Uint8Array(); }
