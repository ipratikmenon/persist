# Session S06 — Auth Module + SF Pro Fonts
**Date:** 2026-04-15
**Session:** S06
**Branch:** main
**Started from:** S05 complete (Phase 1 M1 + M2 + M3 done, cargo test 22/22, pnpm build PASS)

---

## Goal

Complete Phase 1 User Management + RBAC:
- SQLite users table + bcrypt seeding
- Keel login / logout / get_session commands
- Deck auth store, LoginScreen, SessionGate route guard
- Live session display + logout in AppShell

Additional work from user requests:
- File Manager Tree spec added to module-03-documents.md §6 (deferred implementation)
- Replace Google Fonts with Apple SF Pro system font stack everywhere
- Add drafting font catalogue for legal text areas

---

## Actions Taken

### 1. Keel — users migration

Created `src-tauri/src/db/migrations/0005_users.sql`:
- `users` table: id (TEXT PK), name, email UNIQUE, role CHECK (Partner/Associate/Paralegal/Admin), password_hash, is_active (default 1), last_login_at, created_at, updated_at
- `CREATE UNIQUE INDEX idx_users_email ON users(email)`

### 2. Keel — db/queries/users.rs

Created `src-tauri/src/db/queries/users.rs` with:
- `UserRow` struct with `#[derive(sqlx::FromRow)]`
- `get_by_email(pool, email)` — case-insensitive lookup for login
- `get_by_id(pool, id)` — used by get_session
- `count(pool)` — used by seeding logic
- `create(pool, id, name, email, role, hash)` — for admin tooling
- `touch_login(pool, id)` — updates last_login_at on successful auth
- `list_active(pool)` — all is_active=1 users
- 3 unit tests using in-memory SQLite

Updated `src-tauri/src/db/queries/mod.rs` to include `pub mod users`.

### 3. Keel — Cargo.toml

Added to `[dependencies]`:
```toml
bcrypt = "0.15"
```
Added to `[dev-dependencies]`:
```toml
tempfile = "3"   # (was already added in S05 for vault tests)
```

### 4. Keel — AppState + seeding in lib.rs

Added `SessionData` struct:
```rust
pub struct SessionData {
    pub user_id: String,
    pub name:    String,
    pub role:    String,
}
```

Extended `AppState`:
```rust
pub struct AppState {
    pub db:        Arc<Mutex<sqlx::SqlitePool>>,
    pub vault_key: Arc<[u8; 32]>,
    pub vault_dir: std::path::PathBuf,
    pub session:   Arc<Mutex<Option<SessionData>>>,  // new
}
```

Added user seeding block in `.setup()` inside `tauri::async_runtime::block_on`:
- COUNT(*) on users table — if 0, bcrypt-hash "persist2026" at cost 12
- INSERT both attorneys (user-slm: Sree Lakshmi Menon / Partner, user-kt: Kajal Thakur / Associate)
- Wrapped in block_on because .setup() is synchronous — `.await` directly causes E0728

**Bug fixed:** Initial attempt used `.await` directly in the setup closure → E0728 error. Fixed by adding a second `tauri::async_runtime::block_on(async { ... })` block, consistent with the pool initialization pattern on line 62.

### 5. Keel — commands/auth.rs

Replaced stub with full implementation:

`Session` IPC type (camelCase for Tauri serde):
```rust
pub struct Session {
    pub user_id: String,
    pub name:    String,
    pub role:    String,
    pub email:   String,
}
```

Three commands:
- `login(input: LoginInput, state)` — get_by_email → bcrypt::verify → touch_login → set AppState session → return Session
- `logout(state)` — sets session to None
- `get_session(state)` — returns Option<Session>

Key pattern: lock and drop `pool` before locking `session` to avoid deadlock.

### 6. cargo test — 25/25 PASS

```
running 25 tests
test storage::vault::tests::encrypt_decrypt_roundtrip ... ok
test storage::vault::tests::nonce_is_unique_per_call ... ok
test storage::vault::tests::wrong_key_fails ... ok
test storage::vault::tests::delete_removes_file ... ok
test db::queries::documents::tests::... (4 tests) ok
test db::queries::deadlines::tests::... (6 tests) ok
test db::queries::matters::tests::... (7 tests) ok
test db::queries::users::tests::... (3 tests) ok
```

### 7. Deck — src/stores/auth.ts

Created Zustand auth store:
```typescript
interface AuthStore {
  session: Session | null;
  isChecking: boolean;
  setSession: (session: Session | null) => void;
  setChecking: (checking: boolean) => void;
  clearSession: () => void;
}
```
`isChecking` starts true — prevents flash of login screen on startup.

### 8. Deck — LoginScreen.tsx

Full-screen centered login page with:
- P&P monogram circle (slate blue, 52×52, border-radius 14)
- "Persistas & Partners" heading, "Delhi · Intellectual Property" subline
- Quick-select attorney chips: SLM (Sree Lakshmi Menon / Partner) and KT (Kajal Thakur / Associate) — clicking pre-fills email + focuses password field
- Email + password inputs with show/hide toggle (monospace "show"/"hide" button)
- Animated error banner (Framer Motion, opacity + y slide)
- Submit button disabled when email/password empty or loading
- Footer hint: default password `persist2026`
- `handleSubmit` calls `keel.auth.login()` → `setSession()` on success

### 9. Deck — App.tsx rewrite

Three components:
- `SessionGate`: calls `keel.auth.getSession()` on mount → `setSession(s ?? null)`. Shows "Loading…" spinner while `isChecking`. Shows `<LoginScreen />` when no session.
- `LogoutHandler`: registers `window.__persistLogout` async function (calls `keel.auth.logout()` + `clearSession()` + `navigate('/login')`)
- `/login` route placed outside SessionGate — accessible when unauthenticated

### 10. Deck — AppShell.tsx update

Bottom section now shows live session data:
- `initials(name)` helper: "Sree Lakshmi Menon" → "SLM"
- Avatar circle with initials (white on accentPrimary)
- First name (`session.name.split(' ')[0]`) + role in tertiary mono
- Logout button (⎋) → `window.__persistLogout()`; hover color → statusUrgent

### 11. SF Pro system fonts

Removed Google Fonts import from `src/design-system/global.css`.

Updated `src/design-system/tokens.ts`:
```typescript
fonts = {
  display: "-apple-system, 'SF Pro Display', BlinkMacSystemFont, system-ui, sans-serif",
  ui:      "-apple-system, 'SF Pro Text', BlinkMacSystemFont, system-ui, sans-serif",
  mono:    "'SF Mono', ui-monospace, Menlo, Monaco, monospace",
  legal:   "Georgia, 'Times New Roman', Times, serif",  // unchanged
}
```

Added drafting font catalogue for legal text areas:
```typescript
export interface DraftingFont { label: string; value: string; stack: string; }
export const draftingFonts: DraftingFont[] = [
  // georgia, times, palatino, garamond, baskerville, arial, calibri, courier
]
export function draftingFontStack(value: string): string { ... }
```

### 12. specs/module-03-documents.md — §6 File Manager Tree

Added §6 File Manager Tree spec:
- Folders per category (7 total: Correspondence, Filing, Certificate, SearchReport, Invoice, Contract, Other)
- Folder name bold when count > 0
- `(N)` count badge per folder
- All categories always visible (even empty)
- Component plan: DocumentTree.tsx, DocumentTreeFolder.tsx, DocumentTreeFile.tsx
- No new Keel queries needed — derives counts from already-loaded DocumentMeta[]
- Deferred to Phase 1 M3.1 (after ≥1 matter has real documents)

### 13. pnpm build — PASS

```
✓ 462 modules transformed.
dist/assets/index-DVhpRHhS.js   439.46 kB │ gzip: 132.57 kB
✓ built in 585ms
```

---

## Tests

| Suite | Before | After |
|---|---|---|
| cargo test | 22/22 | 25/25 |
| pnpm build | PASS (431.91kb) | PASS (439.46kb) |

---

## Files Changed

### Keel (Rust — src-tauri/)
- `src/db/migrations/0005_users.sql` — CREATED
- `src/db/queries/users.rs` — CREATED
- `src/db/queries/mod.rs` — added pub mod users
- `src/commands/auth.rs` — REPLACED (stub → full login/logout/get_session)
- `src/lib.rs` — added SessionData, session to AppState, user seeding block_on
- `Cargo.toml` — added bcrypt = "0.15"

### Deck (React — src/)
- `stores/auth.ts` — CREATED
- `pages/Auth/LoginScreen.tsx` — CREATED
- `App.tsx` — REWRITTEN (SessionGate + LogoutHandler + /login outside guard)
- `components/shell/AppShell.tsx` — UPDATED (live session, initials, logout ⎋)
- `design-system/tokens.ts` — UPDATED (SF Pro fonts + drafting catalogue)
- `design-system/global.css` — UPDATED (Google Fonts removed)
- `lib/ipc-types.ts` — UPDATED (email added to Session, UploadDocumentInput extended)

### Specs
- `specs/module-03-documents.md` — §6 File Manager Tree added

---

## Decisions Made

| Decision | Reasoning |
|---|---|
| In-memory session (AppState) vs DB session | Restart always requires re-auth — no persistent login tokens. Simpler, safer for local desktop. |
| window.__persistLogout pattern | AppShell needs logout but is outside router context for useNavigate. LogoutHandler inside router registers the fn; AppShell calls it. Avoids prop drilling. |
| block_on for user seeding | Tauri .setup() closure is sync. Two separate block_on calls (pool init + seeding) are idiomatic. |
| SF Pro as default system font | Zero network requests, perfect macOS rendering, no Flash of Invisible Text (FOIT). Apple SF Pro available on all target machines (macOS + Windows via WebKit). |
| Drafting fonts deferred until drafting module | 8-choice catalogue in tokens.ts ready; UI toggle wired when ProseMirror editor lands in Phase 4. |

---

## Bugs Found / Fixed

| Bug | Root Cause | Fix |
|---|---|---|
| E0728: await in non-async setup | .setup() closure is synchronous — `.await` inside it is not allowed | Wrapped seeding in `tauri::async_runtime::block_on(async { ... })` |

---

## Next Session

1. **Validation pass** — run `pnpm tauri dev`, test the full login → matter → docket → document flow end-to-end
2. **Phase 1 M3.1** — implement File Manager Tree (DocumentTree.tsx) once a matter has real documents
3. **SCHEMA.md update** — add users table documentation
4. **Phase 2 planning** — billing module spec + Hetzner sync server scaffold
