// Screenshot harness — stands in for the Tauri dialog and fs plugins.
// Neither is exercised by the screenshots; these exist so the bundle links.
export async function save(): Promise<string | null> { return null; }
export async function open(): Promise<string | null> { return null; }
export async function writeFile(): Promise<void> {}
export async function readFile(): Promise<Uint8Array> { return new Uint8Array(); }
