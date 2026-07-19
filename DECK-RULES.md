# Deck CLAUDE.md — src/
# Rules for working in the React frontend of Persist Desktop.
# Read this alongside the root CLAUDE.md when working in Deck.

---

## Identity

You are working in **Deck** — the React frontend of Persist Desktop (`src/`).
Deck handles everything the attorney sees and interacts with.
It communicates with Keel exclusively through Tauri's `invoke()` IPC mechanism.

Your scope is `src/` only. Do not touch `src-tauri/` (Keel) from here.

---

## Stack

```
React 19 + TypeScript
Vite (bundler)
Zustand (global state)
React Router v7 (routing)
ProseMirror (rich text — Module 15A)
Tailwind CSS utility classes (layout only — colours from design tokens)
```

---

## The One Rule About AI

Deck NEVER calls the Anthropic API directly. Ever.
The only AI call allowed from Deck is:

```typescript
// src/lib/ai.ts — the only file that knows about AI
import { invoke } from '@tauri-apps/api/core';
import type { AIResponse, AIRequestParams } from './ipc-types';

export async function aiRequest(params: AIRequestParams): Promise<AIResponse> {
    return invoke<AIResponse>('ai_request', {
        taskType: params.taskType,
        context: params.context,
        prompt: params.prompt,
        deepAnalysis: params.deepAnalysis ?? false,
    });
    // Keel decides the model. Deck never knows which model ran.
}
```

## The One Rule About HPAS

For operations requiring intelligence (data assembly, multi-step AI coordination, analysis), use the HPAS dispatch hook instead of `ai.ts` directly. HPAS handles orchestration; `ai_router.rs` handles model selection inside HPAS.

```typescript
// src/hooks/useHpas.ts — use for intelligence operations
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function useHpas() {
    // Synchronous — for interactive operations (<200ms P95)
    const dispatch = async (job: Job): Promise<StructuredResult> =>
        invoke('hpas_dispatch', { job });

    // Async — for batch/background operations (webhook-resumed)
    const dispatchBatch = (job: Job, onComplete: (r: StructuredResult) => void) => {
        invoke('hpas_dispatch', { job });
        listen('hpas:job_complete', (event) => {
            if (event.payload.job_id === job.id) onComplete(event.payload.result);
        });
    };

    return { dispatch, dispatchBatch };
}
```

**Bypass rule:** Use HPAS for intelligence. Use direct Tauri IPC for pure data transport.

| Operation | Use |
|---|---|
| Dashboard data assembly | `useHpas` dispatch |
| Email fetch + summarise | `useHpas` dispatch |
| Background document analysis | `useHpas` dispatchBatch |
| Update cursor position | `invoke()` directly |
| Persist a text edit | `invoke()` directly |
| Open a local file | `invoke()` directly |

HPAS result always conforms to `output_schema` — no null checks, no shape handling needed in components.

---

## Tauri IPC

All Keel commands are called through typed wrappers in `src/lib/tauri.ts`.
Never use raw `invoke()` calls in components — always go through the typed wrapper.

```typescript
// src/lib/tauri.ts — typed wrappers
import { invoke } from '@tauri-apps/api/core';
import type { Matter, CreateMatterInput, Deadline } from './ipc-types';

export const keel = {
    matters: {
        get: (id: string) => invoke<Matter>('get_matter', { id }),
        create: (input: CreateMatterInput) => invoke<Matter>('create_matter', { input }),
        list: (filter: MatterFilter) => invoke<Matter[]>('list_matters', { filter }),
    },
    deadlines: {
        list: (matterId: string) => invoke<Deadline[]>('list_deadlines', { matterId }),
        create: (input: CreateDeadlineInput) => invoke<Deadline>('create_deadline', { input }),
        markComplete: (id: string, notes: string) => invoke<Deadline>('mark_deadline_complete', { id, notes }),
    },
    // ... one namespace per Keel command file
};
```

```typescript
// Usage in a component
import { keel } from '@/lib/tauri';

const matter = await keel.matters.get(matterId);
```

---

## State Management

Use Zustand for ALL global state. No Redux. No Context API for global state.

```
src/stores/
  matters.ts    ← active matter, matter list, filters
  ui.ts         ← sidebar state, active tab, theme, current module
  chat.ts       ← Persist Chat session, history
  sync.ts       ← sync status, last synced timestamp
  mail.ts       ← active mail tab, thread state
```

```typescript
// Pattern for a Zustand store
import { create } from 'zustand';

interface MattersStore {
    activeMatter: Matter | null;
    setActiveMatter: (matter: Matter | null) => void;
}

export const useMattersStore = create<MattersStore>((set) => ({
    activeMatter: null,
    setActiveMatter: (matter) => set({ activeMatter: matter }),
}));
```

Never use `useState` for data that needs to persist across component unmounts
or be shared between components in different subtrees. That belongs in a Zustand store.

---

## Design System

**Do not hardcode any colour, font, or spacing value in a component.**
Import from the design system tokens:

```typescript
// src/design-system/tokens.ts
export const colors = {
    bgPrimary: '#F9F7F4',
    bgSecondary: '#F0ECE5',
    bgTertiary: '#E8EDE6',
    textPrimary: '#2C2C2A',
    textSecondary: '#6B6862',
    textTertiary: '#9A9590',
    accentPrimary: '#4A6580',
    accentSecondary: '#B5604A',
    statusUrgent: '#C0392B',
    statusWarning: '#D4872A',
    statusClear: '#4A7C59',
    border: '#E2DDD8',
} as const;
```

Use Tailwind utility classes for layout and spacing only.
Use CSS variables for colour (applied via `tokens.ts` at the `:root` level).

**Typography classes** (defined in `design-system/typography.ts`):
```
.text-display    → Playfair Display 28px/600
.text-section    → Playfair Display 20px/500
.text-card-title → DM Sans 15px/500
.text-body       → DM Sans 14px/400
.text-label      → DM Sans 12px/400
.text-legal      → Georgia 13px/400
.text-mono       → JetBrains Mono 12px/400
```

---

## Component Structure

```
src/
├── pages/
│   ├── Matters/
│   │   ├── MatterList.tsx        ← list view
│   │   ├── MatterDetail.tsx      ← single matter
│   │   └── index.ts              ← barrel export
│   ├── Dockets/
│   │   ├── DocketList.tsx
│   │   ├── IPAssetRecord.tsx
│   │   ├── PipelineBoard.tsx
│   │   └── index.ts
│   └── ... (one folder per module)
│
├── components/
│   ├── ui/                       ← primitive UI components (Button, Card, Badge, etc.)
│   ├── matters/                  ← matter-specific components
│   ├── deadlines/                ← deadline cards, urgency indicators
│   ├── mail/                     ← mail module components
│   ├── editor/                   ← ProseMirror Persist Editor (Module 15A)
│   ├── pdf/                      ← PDF viewer and markup tools
│   ├── chat/                     ← Persist Chat overlay
│   └── design-system/            ← design tokens, typography, shared styles
│
└── lib/
    ├── ai.ts                     ← THE ONLY AI ENTRY POINT
    ├── tauri.ts                  ← Typed invoke() wrappers
    ├── ipc-types.ts              ← TypeScript types matching Rust structs
    └── shortcuts.ts              ← Global keyboard shortcuts
```

**Component rules:**
- One component per file
- Named exports for all components (no default exports in components/)
- Props interfaces defined in the same file as the component
- Co-locate component-specific styles with the component (CSS modules or Tailwind)

---

## The Persist Editor (ProseMirror — Module 15A)

All rich text inputs across the platform use the ProseMirror-based Persist Editor.
Do NOT use `<textarea>`, `contenteditable`, or any other editor library.

The editor is in `src/components/editor/`:
```
editor/
  PersistEditor.tsx         ← main editor component
  plugins/
    markdown.ts             ← live Markdown rendering
    smart-tags.ts           ← @ and # tag system
    expansions.ts           ← /shorthand expansion system
  schema.ts                 ← ProseMirror document schema
  serialise.ts              ← ProseMirror doc → Markdown / HTML / LaTeX
```

Usage:
```tsx
import { PersistEditor } from '@/components/editor/PersistEditor';

<PersistEditor
    matterId={currentMatter.id}
    surface="email-compose"    // or 'document', 'notes', 'chat', 'meeting'
    onChange={(content) => setBody(content)}
    placeholder="Write your reply..."
/>
```

---

## AI Visual Indicators

Three AI signals used across Deck (Module 22.11):

```tsx
// 1. Sparkle chip — inline AI-generated content
<span className="ai-sparkle">✦</span>

// 2. Draft banner — on AI-drafted documents
<div className="draft-banner">DRAFT — AI GENERATED</div>

// 3. Deep analysis indicator — on Opus-level responses
<span className="deep-analysis-chip">⚙</span>
```

These are defined in `src/design-system/ai-indicators.tsx` — always use these components,
never create ad-hoc AI indicators.

---

## Keyboard Shortcuts

Global shortcuts are registered in `src/lib/shortcuts.ts` using Tauri's global shortcut plugin.
Module-specific shortcuts (e.g. mail module shortcuts) are registered when the module mounts
and unregistered when it unmounts.

```typescript
// src/lib/shortcuts.ts
export const SHORTCUTS = {
    PERSIST_CHAT: 'CmdOrCtrl+/',    // Open Hummingbird chat overlay
    COMMAND_PALETTE: 'CmdOrCtrl+K', // Open command palette
    MAIL_DONE: 'E',                 // Mark email done (mail module)
    MAIL_SNOOZE: 'S',               // Snooze email
    MAIL_PIN: 'P',                  // Pin email
    MAIL_REPLY: 'R',                // Reply
} as const;
```

---

## Animation — Motion v12

All animations use Motion v12 (`motion/react`). No other animation library. No raw CSS keyframes for anything Motion can handle.

```typescript
import { motion, AnimatePresence, useSpring, LayoutGroup } from 'motion/react';
```

**Always use animation tokens — never hardcode:**
```typescript
import { transition, ease, duration } from '@/design-system/motion';

// ✓ Correct
<motion.div whileHover={{ y: -2 }} transition={transition.fast}>

// ✗ Wrong — hardcoded values
<motion.div whileHover={{ y: -2 }} transition={{ duration: 0.15 }}>
```

**Respect reduced motion — always:**
```typescript
import { useReducedMotion } from 'motion/react';
const shouldReduce = useReducedMotion();
// Conditionally simplify all animations when true
```

**Core animation rules:**
- Cards: `whileHover={{ y: -2, boxShadow }}` + `whileTap={{ scale: 0.99 }}`
- Lists: staggered entrance with `variants` + `staggerChildren: 0.05`
- Exit: use `AnimatePresence` + `mode="popLayout"` for lists
- Panels: right panel slides from right, bottom panel from bottom, modals scale from centre
- Layout changes: use `layout` prop — never animate height/width manually
- Number changes: use `AnimatedNumber` component from `components/ui/`
- Status changes: wrap with `AnimatePresence mode="wait"` and key on the value
- Completion states: sequence with `animate()` controls

**What NOT to do:**
- Do NOT import from `framer-motion` — the package is renamed to `motion`
- Do NOT use CSS `transition` or `@keyframes` for anything Motion handles
- Do NOT hardcode animation values — use `transition.*` and `ease.*` tokens
- Do NOT animate without reduced motion check on new components
- Do NOT add GSAP, anime.js, or any other animation library
- Do NOT use `fetch()` directly for anything — all data comes from Keel via invoke()
- Do NOT use React Context for global state — Zustand only
- Do NOT hardcode colours, fonts, or spacing in components
- Do NOT create new editor components — use PersistEditor everywhere
- Do NOT call `invoke()` directly in components — use `keel.*` wrappers from `lib/tauri.ts`
- Do NOT import Anthropic SDK — AI calls go through `lib/ai.ts` which calls Keel

---

## Test Patterns

```bash
pnpm test          # Vitest unit tests
pnpm build         # Type-check + Vite build — must pass before commit
```

Write component tests with Vitest + React Testing Library.
Test files co-located: `MatterList.test.tsx` next to `MatterList.tsx`.
Test the behaviour, not the implementation — test what the user sees.
