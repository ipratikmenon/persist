# Integration Document
## HPAS in Persists
### Platform Binding Specification

---

**Version:** 1.0.0  
**Status:** Active  
**Author:** Internal  
**Date:** April 2026  
**HPAS Spec Version:** 1.0.0 (see HPAS_PRD_Standalone.md)  
**Host Product:** Persists — local-first macOS desktop app (Tauri + React + Rust)

**Changelog:**
- v1.0.0 — Extracted from combined PRD v0.4.0. Contains only Persists-specific binding. All platform-agnostic content lives in HPAS_PRD_Standalone.md.

---

## Purpose of This Document

This document covers only what is specific to running HPAS inside Persists. It does not repeat HPAS's architecture, component specifications, token economics, or design decisions — those are fully defined in `HPAS_PRD_Standalone.md`.

Read this document for: module layout, Rust implementation choices, Tauri integration, macOS constraints, the `WorkflowStore` backend used, the implementation sequence relative to Persists phases, and success metrics.

---

## Table of Contents

1. [Platform Overview](#1-platform-overview)
2. [Module Layout](#2-module-layout)
3. [Dispatch Contract Implementation](#3-dispatch-contract-implementation)
4. [Component Bindings](#4-component-bindings)
5. [macOS Constraints](#5-macos-constraints)
6. [Implementation Plan](#6-implementation-plan)
7. [Technical Dependencies](#7-technical-dependencies)
8. [Success Metrics](#8-success-metrics)

---

## 1. Platform Overview

**Persists** is a local-first macOS desktop application built with:
- **Tauri** — Rust backend + WebView frontend bridge
- **React + TypeScript** — UI layer
- **Rust** — all backend logic including HPAS

HPAS is implemented as a native Rust module (`src-tauri/src/hpas/`) inside Persists. It is the execution engine for all meaningful operations in Persists — data fetching, API calls, integrations, and multi-agent coordination. The React UI never calls external APIs directly. Everything goes through HPAS.

**Distribution:** Single self-contained `.app` bundle. No user-installed dependencies. HPAS compiles into Persists' existing Tauri binary.

**Scaling path:** Local first (SQLite + tokio). Cloud later (Restate replaces SQLite via `WorkflowStore` trait swap — no callers change).

---

## 2. Module Layout

```
persists/
├── src/                              — React frontend
│   ├── hooks/
│   │   └── useHpas.ts               — single hook, all HPAS calls from React
│   └── components/
│       └── AgentGraph.tsx           — real-time agent topology (Phase 4, optional)
└── src-tauri/
    ├── src/
    │   ├── main.rs
    │   ├── commands.rs              — Tauri commands, including hpas_dispatch
    │   └── hpas/
    │       ├── mod.rs               — dispatch router, Job/StructuredResult types
    │       ├── agent.rs             — SubAgent (tokio::spawn)
    │       ├── compressor.rs        — SemanticCompressor (pure Rust function)
    │       ├── store.rs             — WorkflowStore trait + SqliteStore impl
    │       ├── session.rs           — SessionAgent with SQLite history
    │       ├── memory.rs            — VectorMemory with sqlite-vss
    │       ├── registry.rs          — RegistryAgent (SQLite table)
    │       ├── bus.rs               — MessageBus + OrchestratorBus via Tauri events
    │       └── compressor_ctx.rs    — ContextCompressor (periodic distillation)
    └── Cargo.toml
```

One folder. Inside Persists. No separate repo. When cloud scale is needed, `hpas/` is extracted into its own crate — trivial because the interface was stable from day one.

---

## 3. Dispatch Contract Implementation

### 3.1 Rust Types

```rust
// hpas/mod.rs

#[derive(Serialize, Deserialize, Clone)]
pub struct Job {
    pub id:            String,
    pub job_type:      String,
    pub context:       serde_json::Value,
    pub output_schema: serde_json::Value,
    pub priority:      Priority,
    pub max_tokens:    Option<u32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StructuredResult {
    pub job_id:      String,
    pub status:      JobStatus,
    pub data:        serde_json::Value,  // always conforms to output_schema
    pub tokens_used: u32,
    pub flags:       Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Priority { Interactive, Batch, Streaming }

#[derive(Serialize, Deserialize, Clone)]
pub enum JobStatus { Complete, Flagged, Failed }

pub async fn dispatch(job: Job) -> Result<StructuredResult> {
    match job.priority {
        Priority::Interactive => dispatch_interactive(job).await,
        Priority::Batch       => dispatch_batch(job).await,
        Priority::Streaming   => dispatch_streaming(job).await,
    }
}
```

### 3.2 Tauri Command Bridge

```rust
// commands.rs

#[tauri::command]
pub async fn hpas_dispatch(
    job: JobPayload,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let result = hpas::dispatch(job.into()).await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(result).unwrap())
}

// Batch completions pushed to React via Tauri events — never polled
// React: app.listen("hpas:job_complete", handler)
app.emit("hpas:job_complete", JobResult { job_id, result })?;

// Batch API webhook received by Tauri's local HTTP server
#[tauri::command]
pub async fn hpas_batch_webhook(payload: BatchWebhookPayload) -> Result<(), String> {
    hpas::resolve_batch_promise(payload.batch_id, payload.results).await
        .map_err(|e| e.to_string())
}
```

### 3.3 React Hook

```typescript
// hooks/useHpas.ts

export function useHpas() {
    const dispatch = async (job: Job): Promise<StructuredResult> => {
        return await invoke("hpas_dispatch", { job })
    }

    const dispatchBatch = (job: Job, onComplete: (r: StructuredResult) => void) => {
        invoke("hpas_dispatch", { job })
        listen("hpas:job_complete", (event) => {
            if (event.payload.job_id === job.id) onComplete(event.payload.result)
        })
    }

    return { dispatch, dispatchBatch }
}

// Usage — React components never call external APIs directly
const { dispatch } = useHpas()
const dashboard = await dispatch({
    job_type: "load_dashboard",
    context: { userId },
    output_schema: DashboardSchema,
    priority: "Interactive"
})
// dashboard always conforms to DashboardSchema — no null checks, no shape handling
```

---

## 4. Component Bindings

How each HPAS component maps to Persists-specific infrastructure:

| HPAS Component | Persists Implementation |
|---|---|
| `SessionAgent` | Rust struct; history persisted to SQLite via `WorkflowStore` |
| `SemanticCompressor` | Pure Rust function; no state; called inline at every boundary |
| `MessageBus` | Tauri event system (`app.emit` / `app.listen`); `seq_id` in SQLite counter table |
| `OrchestratorBus` | Separate Tauri event channel; monotonic counter via SQLite serialised write |
| `VectorMemory` | sqlite-vss extension; bundled in app Resources folder |
| `RegistryAgent` | SQLite table per registry; Rust serialised writes |
| `SubAgent` | `tokio::spawn` per agent; `batchMode` uses Anthropic Batch API + Tauri local HTTP webhook |
| `ContextCompressor` | Rust function triggered by `SessionAgent` pre-activation token check |
| `ApprovalGate` | Tauri notification system + SQLite pending state; deadline via Tauri timer |

### 4.1 WorkflowStore Trait (Persistence Abstraction)

```rust
// hpas/store.rs

pub trait WorkflowStore: Send + Sync {
    async fn save(&self, workflow: &Workflow) -> Result<()>;
    async fn load(&self, id: &str) -> Result<Option<Workflow>>;
    async fn resume(&self, id: &str, delta: Delta) -> Result<()>;
    async fn list_pending(&self) -> Result<Vec<Workflow>>;
}

// Phase 3A–4: SqliteStore implements WorkflowStore
pub struct SqliteStore { conn: Arc<Mutex<Connection>> }

// Phase 5+: RestateStore implements WorkflowStore (no callers change)
// pub struct RestateStore { client: RestateClient }
```

### 4.2 MessageBus via Tauri Events

Tauri's event system is the `MessageBus` and `OrchestratorBus` implementation for local Persists:

```rust
// hpas/bus.rs

pub async fn publish(app: &AppHandle, event: BusEvent) -> Result<()> {
    // Assign seq_id from SQLite counter
    let seq_id = increment_counter(&app, &event.bus_id).await?;
    let payload = BusEventPayload { seq_id, ..event.into() };
    app.emit(&format!("hpas:bus:{}", event.bus_id), payload)?;
    Ok(())
}

// Subscribers registered at startup
app.listen("hpas:bus:parent_bus_a", move |event| {
    let payload: BusEventPayload = serde_json::from_str(&event.payload()).unwrap();
    // Dedup by seq_id, process delta
});
```

In-process Tauri events have ~0.01ms latency — far below the 500ms target from HPAS spec Section 8.4.

### 4.3 Batch API Webhook

Tauri's local HTTP server receives the Anthropic Batch API webhook directly. No tunnel needed for local development — Anthropic's webhook delivery to `localhost` works when the batch job completes on the same machine that initiated it.

```rust
// Tauri plugin or axum route registered at app startup
async fn batch_webhook_handler(
    payload: BatchWebhookPayload,
    State(hpas): State<HpasEngine>,
) -> StatusCode {
    hpas.resolve_batch_promise(payload.batch_id, payload.results).await;
    StatusCode::OK
}
```

### 4.4 Bypass Criteria

Not all Persists operations route through HPAS:

| Operation | Path |
|---|---|
| Load dashboard data | HPAS Interactive |
| Fetch + summarise emails | HPAS Interactive |
| Background document analysis | HPAS Batch |
| Stream agent results to UI | HPAS Streaming |
| Update cursor position | Direct Tauri IPC |
| Persist a text edit | Direct Tauri IPC |
| Open a local file | Direct Tauri IPC |
| Real-time collaborative sync | Direct Tauri IPC |

Rule: **intelligence (interpretation, assembly, compression) → HPAS. Pure data transport → direct Tauri IPC.**

---

## 5. macOS Constraints

### 5.1 .app Bundle Self-Containment

- SQLite statically linked via `rusqlite` with `bundled` feature flag
- sqlite-vss bundled as a loadable extension in `Contents/Resources/`
- No dynamic library loading inside `.app` (macOS hardened runtime restriction — DD-01 applies to server deployments only)
- All HPAS components compiled in at build time
- Zero user-installed dependencies

### 5.2 Notarisation Requirements

- Outbound network entitlement declared for all APIs HPAS calls (Anthropic, external integrations)
- No JIT execution (tokio is standard async, not JIT)
- App data in `~/Library/Application Support/com.yourname.persists/` via Tauri path API
- sqlite-vss extension must be signed as part of the bundle

### 5.3 App Store vs Direct Distribution

Direct distribution (recommended for Persists initially): notarisation without App Store sandbox. No per-domain network restrictions.

App Store (future): stricter sandbox requires explicitly declared entitlements per API domain. Manageable at submission time.

### 5.4 Cold Launch Impact

HPAS must not increase Persists' cold launch time by more than 200ms. SQLite connection is opened lazily on first dispatch, not at startup. Tokio runtime is shared with Tauri's existing async runtime.

---

## 6. Implementation Plan

HPAS components are introduced exactly when a Persists feature first needs them. Nothing is built speculatively.

---

### Phase 3A — Dispatch Foundation

*Introduce alongside the first Phase 3 Persists feature that requires an API call.*

**Goal:** Stable dispatch interface exists. React calls HPAS. One real feature works end-to-end.

Tasks:
- Define `Job`, `StructuredResult`, `Priority`, `JobStatus` in `hpas/mod.rs`
- Implement `WorkflowStore` trait and `SqliteStore` in `store.rs`
- Implement `hpas_dispatch` Tauri command in `commands.rs`
- Write `useHpas.ts` React hook — mock returns initially, real dispatch wired in same PR
- Implement `SubAgent` in `agent.rs` — single tokio task, Anthropic API call, typed result
- Implement `SemanticCompressor` in `compressor.rs` — pure function, no state
- Wire first real Persists feature through HPAS
- Verify React receives correctly typed, schema-conforming data with no null checks needed

**Claude Code usage:** High. `SubAgent` and `SemanticCompressor` follow clear patterns. `cargo build` is the validation oracle.

---

### Phase 3B — Parallel Dispatch

*Introduce when first multi-fetch feature is needed.*

**Goal:** Multiple agents run in parallel. Parent assembles results. Mirrors Weft's `List[T]` lane expansion.

Tasks:
- Extend `agent.rs` with `FuturesUnordered` parallel dispatch
- Implement `SessionAgent` in `session.rs` — SQLite-persisted conversation history
- Implement `ContextCompressor` in `compressor_ctx.rs`
- Write 3-agent parallel test
- Verify Parent delta activation cost (~1,500 tokens)
- Verify parallel wall-time ≤ slowest agent (not sum of agents)

**Claude Code usage:** High for parallelism. Medium for SQLite session history.

---

### Phase 3C — Communication Layer

*Introduce when cross-domain coordination is needed.*

**Goal:** Sibling agents notify each other. Registry ensures entity consistency.

Tasks:
- Implement `RegistryAgent` in `registry.rs` — SQLite table, serialised writes
- Implement `MessageBus` + `OrchestratorBus` in `bus.rs` — Tauri events + SQLite counters
- Implement `VectorMemory` in `memory.rs` — sqlite-vss
- Write 2-Parent lateral communication test
- Verify MessageBus latency (<500ms — should be ~0.01ms via Tauri events)
- Verify RegistryAgent write ordering under concurrent agents

**Claude Code usage:** High for Registry and VectorMemory. Bus Tauri event wiring needs human review for ordering correctness.

---

### Phase 4 — Full Hierarchy + ApprovalGate

*Introduce when GrandParent-level coordination is needed.*

**Goal:** Full 3-level hierarchy. Human approval integrated. End-to-end job runs.

Tasks:
- Implement `ApprovalGate` — Tauri notification + SQLite pending state + deadline timer
- Wire full GrandParent → Parent → Agent graph for a real Persists job
- Implement Batch API dispatch in `SubAgent` (`batchMode: true`) + Tauri webhook handler
- Run N=20 parallel agents, measure actual vs modelled token costs
- Build optional `AgentGraph.tsx` real-time React visualisation

**Claude Code usage:** Medium. All primitives exist. Integration testing is the main work.

---

### Phase 5 — Cloud Scale Preparation

*Introduce when Persists goes multi-user / distributed.*

**Goal:** SQLite replaced by Restate. No interface changes.

Tasks:
- Add `restate-sdk-rust` Cargo dependency
- Implement `RestateStore` behind the existing `WorkflowStore` trait
- Feature-flag: `HPAS_BACKEND=restate` at compile time
- Verify all existing tests pass with Restate backend
- Document cloud deployment requirements (Restate server, Postgres for Registry and VectorMemory at scale)
- Remove SQLite dependency from cloud build profile; keep for local `.app`

**Claude Code usage:** Medium. Interface is stable. Backend swap is mechanical.

---

## 7. Technical Dependencies

### 7.1 Rust Crate Additions

```toml
# Cargo.toml additions
tokio        = { version = "1", features = ["full"] }
rusqlite     = { version = "0.31", features = ["bundled"] }
serde        = { version = "1", features = ["derive"] }
serde_json   = "1"
tiktoken-rs  = "0.5"     # token counting for context ceiling
reqwest      = { version = "0.12", features = ["json"] }
uuid         = { version = "1", features = ["v4"] }
```

sqlite-vss bundled separately as a loadable extension — not a Cargo dependency.

### 7.2 API Requirements

- **Anthropic API key** — for SessionAgent, SubAgent, SemanticCompressor, ContextCompressor
- **Anthropic Batch API access** — for Phase 4 batchMode SubAgent calls
- Keys stored in Tauri's secure keychain. Never in SQLite. Never in plaintext.

### 7.3 No Additional Infrastructure

The Persists `.app` bundle requires zero additional user-facing infrastructure. No Docker, no Postgres, no Node.js, no Restate server. Everything runs inside the single binary.

---

## 8. Success Metrics

### 8.1 Token Efficiency (vs HPAS standalone spec targets)

| Metric | Target | Measurement |
|---|---|---|
| Token cost vs single-context at N=50 | ≥ 8× reduction | Anthropic usage logs |
| Token cost vs single-context at N=200 | ≥ 20× reduction | Anthropic usage logs |
| Parent delta activation cost | ≤ 2,000 tokens | Per-activation log |
| Agent execution cost | ≤ 4,500 tokens average | Per-activation log |
| SemanticCompressor ratio | ≥ 8:1 average | Compressor output log |
| Change propagation vs full re-run | ≥ 10× cheaper | Targeted vs full-run comparison |

### 8.2 Persists-Specific Performance

| Metric | Target |
|---|---|
| Cold launch time increase from HPAS | ≤ 200ms |
| Interactive dispatch latency (P95) | ≤ 200ms |
| MessageBus event delivery (Tauri events) | ≤ 5ms (in-process) |
| RegistryAgent read latency | ≤ 10ms (SQLite) |
| ContextCompressor trigger-to-completion | ≤ 30 seconds |
| .app bundle size increase from HPAS | ≤ 5MB |

### 8.3 Correctness

| Metric | Target |
|---|---|
| Registry consistency under concurrent writes | 100% |
| Schema conformance at all boundaries | 100% |
| Agent flags preserved through compression | 100% |
| Batch webhook delivery reliability | ≥ 99.5% |
| SessionAgent history continuity across restarts | 100% |
| WorkflowStore crash recovery (resume from last checkpoint) | 100% |

---

*End of Document — HPAS-in-Persists Integration Spec v1.0.0*
