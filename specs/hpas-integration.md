# specs/hpas-integration.md
# SPEC for Module 24: HPAS Integration in Persist
# This is the Claude Code implementation reference.
# Full platform-agnostic spec: HPAS_PRD_Standalone.md
# Full Persist binding spec: HPAS_Persists_Integration.md

---

## What to Build, in Order

HPAS is built in four phases tied to Persist's feature phases. Never build ahead of need.

---

## Phase 3A — Dispatch Foundation

**Trigger:** Build this alongside the first Phase 3 feature that makes an AI API call
(typically Module 7 AI Drafting or Module 21 Persist Chat).

**Goal:** Stable `hpas_dispatch` → `SubAgent` → Anthropic API path exists. One real
Persist feature works end-to-end through HPAS.

### Files to create

**`src-tauri/src/hpas/mod.rs`**
```rust
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Job {
    pub id:            String,
    pub job_type:      String,
    pub context:       serde_json::Value,
    pub output_schema: serde_json::Value,
    pub priority:      Priority,
    pub max_tokens:    Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StructuredResult {
    pub job_id:      String,
    pub status:      JobStatus,
    pub data:        serde_json::Value,   // always conforms to output_schema
    pub tokens_used: u32,
    pub flags:       Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Priority { Interactive, Batch, Streaming }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum JobStatus { Complete, Flagged, Failed }

pub async fn dispatch(job: Job, state: &crate::AppState) -> Result<StructuredResult> {
    match job.priority {
        Priority::Interactive => dispatch_interactive(job, state).await,
        Priority::Batch       => dispatch_batch(job, state).await,
        Priority::Streaming   => dispatch_streaming(job, state).await,
    }
}
```

**`src-tauri/src/hpas/store.rs`**
```rust
use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait WorkflowStore: Send + Sync {
    async fn save(&self, workflow: &Workflow) -> Result<()>;
    async fn load(&self, id: &str) -> Result<Option<Workflow>>;
    async fn resume(&self, id: &str, delta: &serde_json::Value) -> Result<()>;
    async fn list_pending(&self) -> Result<Vec<Workflow>>;
}

pub struct SqliteStore {
    pub conn: std::sync::Arc<tokio::sync::Mutex<rusqlite::Connection>>,
}

// Phase 5: pub struct RestateStore { ... }
// Same WorkflowStore trait — no callers change.
```

**`src-tauri/src/hpas/agent.rs`**
```rust
// Phase 3A: single SubAgent, synchronous, no IsolatedWorkspace
// Phase 3B: extend with FuturesUnordered parallel dispatch
// Phase 4: add IsolatedWorkspace + batchMode

pub struct SubAgent {
    pub agent_label:       String,
    pub model:             String,
    pub system_prompt:     String,
    pub output_schema:     serde_json::Value,
    pub max_tokens:        u32,
    pub retry_on_flag:     bool,
    pub max_retries:       u32,
    pub batch_mode:        bool,       // Phase 4
    pub isolated_workspace: bool,      // Phase 4
    pub merge_strategy:    String,     // Phase 4: "registry_only"|"file_collect"|"manual"
}

impl SubAgent {
    pub async fn dispatch(&self, task_slice: serde_json::Value) -> Result<SubAgentResult> {
        // Phase 3A: direct Anthropic API call via reqwest
        // Returns schema-conforming compressed result
        let raw_output = call_anthropic_api(&self.model, &self.system_prompt,
                                            &task_slice, self.max_tokens).await?;
        let compressed = compress_output(&raw_output, &self.output_schema).await?;
        Ok(SubAgentResult { result: compressed, flags: vec![], tokens_used: 0, .. })
    }
}
```

**`src-tauri/src/hpas/compressor.rs`**
```rust
// Pure function — no state, no DB writes in Phase 3A
// Phase 3C: add StructuralMemory auto-writes

pub async fn compress(
    raw_output: &str,
    output_schema: &serde_json::Value,
    max_tokens: u32,
) -> Result<CompressedOutput> {
    // If raw_output already conforms to schema and within ceiling: passthrough
    // Otherwise: call compression model to distil to schema
    // Never silently drop flags — always preserve
}
```

**`src-tauri/src/commands.rs` additions**
```rust
#[tauri::command]
pub async fn hpas_dispatch(
    job: hpas::Job,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let result = hpas::dispatch(job, &state).await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(result).unwrap())
}

// Phase 4: batch webhook
#[tauri::command]
pub async fn hpas_batch_webhook(
    payload: hpas::BatchWebhookPayload,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    hpas::resolve_batch_promise(payload.batch_id, payload.results, &state).await
        .map_err(|e| e.to_string())
}
```

**`src/hooks/useHpas.ts`**
```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface Job {
    id?: string;
    job_type: string;
    context: Record<string, unknown>;
    output_schema: Record<string, unknown>;
    priority: 'Interactive' | 'Batch' | 'Streaming';
    max_tokens?: number;
}

export interface StructuredResult {
    job_id: string;
    status: 'Complete' | 'Flagged' | 'Failed';
    data: Record<string, unknown>;   // always conforms to output_schema
    tokens_used: number;
    flags: string[];
}

export function useHpas() {
    const dispatch = async (job: Job): Promise<StructuredResult> => {
        const jobWithId = { ...job, id: job.id ?? crypto.randomUUID() };
        return invoke<StructuredResult>('hpas_dispatch', { job: jobWithId });
    };

    const dispatchBatch = (
        job: Job,
        onComplete: (result: StructuredResult) => void
    ): void => {
        const jobWithId = { ...job, id: job.id ?? crypto.randomUUID(), priority: 'Batch' as const };
        invoke('hpas_dispatch', { job: jobWithId });
        listen<{ job_id: string; result: StructuredResult }>('hpas:job_complete', (event) => {
            if (event.payload.job_id === jobWithId.id) {
                onComplete(event.payload.result);
            }
        });
    };

    return { dispatch, dispatchBatch };
}
```

**Register command in `lib.rs`:**
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::hpas_dispatch,
    commands::hpas_batch_webhook,  // Phase 4
])
```

### Phase 3A validation test

```rust
#[tokio::test]
async fn test_hpas_dispatch_returns_schema_conforming_result() {
    let job = Job {
        id: "test-001".into(),
        job_type: "summarise_text".into(),
        context: json!({ "text": "The quick brown fox." }),
        output_schema: json!({ "summary": "string", "word_count": "number" }),
        priority: Priority::Interactive,
        max_tokens: Some(500),
    };
    let result = dispatch(job, &mock_state()).await.unwrap();
    assert_eq!(result.status, JobStatus::Complete);
    assert!(result.data.get("summary").is_some());
    assert!(result.data.get("word_count").is_some());
}
```

---

## Phase 3B — Parallel Dispatch

**`src-tauri/src/hpas/session.rs`**
```rust
// SessionAgent — persists conversation history in SQLite
// Receives only the delta per activation, not full history
// Fires ContextCompressor when history > maxHistoryTokens

pub struct SessionAgent {
    pub id:                  String,
    pub model:               String,
    pub system_prompt:       String,
    pub scope:               AgentScope,      // GrandParent | Parent | Agent
    pub max_history_tokens:  u32,             // 60k for GP, 40k for Parent
    pub context_ceiling:     u32,
    pub ceiling_strategy:    CeilingStrategy, // CompressOnApproach | RaiseThreshold
    pub output_schema:       serde_json::Value,
    pub confidence_threshold: f32,
}

// SQLite schema for session history
// CREATE TABLE hpas_sessions (
//   id TEXT PRIMARY KEY,
//   agent_id TEXT NOT NULL,
//   conversation TEXT NOT NULL,   -- JSON array of message objects
//   total_tokens INTEGER DEFAULT 0,
//   created_at DATETIME,
//   updated_at DATETIME
// );
```

**`src-tauri/src/hpas/compressor_ctx.rs`**
```rust
// ContextCompressor — distils accumulated SessionAgent history into dense state
// Triggered by SessionAgent pre-activation token check
// Config: triggerThreshold, targetTokens, preserveDecisions=true
```

**Parallel dispatch in `agent.rs` (Phase 3B extension):**
```rust
use futures::stream::{FuturesUnordered, StreamExt};

pub async fn dispatch_parallel(
    agents: Vec<(SubAgent, serde_json::Value)>,
) -> Result<Vec<SubAgentResult>> {
    let mut futures = FuturesUnordered::new();
    for (agent, task_slice) in agents {
        futures.push(async move { agent.dispatch(task_slice).await });
    }
    let mut results = vec![];
    while let Some(result) = futures.next().await {
        results.push(result?);
    }
    Ok(results)
    // Wall time ≤ slowest agent, not sum of all agents
}
```

---

## Phase 3C — Communication + Zero-Cost Memory

**SQLite tables (add to a new migration `hpas_memory.sql`):**
```sql
-- MistakeMemory
CREATE TABLE IF NOT EXISTS hpas_mistakes (
    id              TEXT PRIMARY KEY,
    agent_id        TEXT NOT NULL,
    task_type       TEXT NOT NULL,
    namespace       TEXT NOT NULL,
    flag            TEXT NOT NULL,
    description     TEXT NOT NULL,
    context_snapshot TEXT,
    occurred_count  INTEGER DEFAULT 1,
    confidence_tag  TEXT DEFAULT 'ONCE',
    created_at      DATETIME DEFAULT (datetime('now')),
    last_seen_at    DATETIME DEFAULT (datetime('now')),
    job_id          TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_mistakes_lookup ON hpas_mistakes(task_type, namespace, confidence_tag);

-- StructuralMemory nodes
CREATE TABLE IF NOT EXISTS hpas_struct_nodes (
    id         TEXT PRIMARY KEY,
    label      TEXT NOT NULL,
    node_type  TEXT NOT NULL,
    properties TEXT,
    confidence REAL DEFAULT 1.0,
    degree     INTEGER DEFAULT 0,
    created_by TEXT,
    graph_id   TEXT NOT NULL,
    created_at DATETIME DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_nodes_degree ON hpas_struct_nodes(degree DESC);

-- StructuralMemory edges
CREATE TABLE IF NOT EXISTS hpas_struct_edges (
    id         TEXT PRIMARY KEY,
    from_id    TEXT NOT NULL REFERENCES hpas_struct_nodes(id),
    to_id      TEXT NOT NULL REFERENCES hpas_struct_nodes(id),
    relation   TEXT NOT NULL,
    weight     REAL DEFAULT 1.0,
    confidence REAL DEFAULT 1.0,
    created_by TEXT,
    graph_id   TEXT NOT NULL,
    created_at DATETIME DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_edges_from ON hpas_struct_edges(from_id);

-- MessageBus sequence counters
CREATE TABLE IF NOT EXISTS hpas_bus_counters (
    bus_id  TEXT PRIMARY KEY,
    seq_id  INTEGER NOT NULL DEFAULT 0
);

-- VectorMemory (requires sqlite-vss extension loaded)
-- CREATE VIRTUAL TABLE IF NOT EXISTS hpas_vectors USING vss0(embedding(1536));
-- Note: actual creation deferred until sqlite-vss confirmed loaded

-- RegistryAgent
CREATE TABLE IF NOT EXISTS hpas_registry (
    id                TEXT PRIMARY KEY,
    entity_type       TEXT NOT NULL,
    entity_id         TEXT NOT NULL,
    fields            TEXT NOT NULL,   -- JSON
    author_agent      TEXT NOT NULL,
    confidence        REAL DEFAULT 1.0,
    confidence_source TEXT NOT NULL,   -- 'extracted'|'inferred'|'ambiguous'
    created_at        DATETIME DEFAULT (datetime('now')),
    updated_at        DATETIME DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_registry_entity ON hpas_registry(entity_type, entity_id);

-- ApprovalGate pending state (Phase 4)
CREATE TABLE IF NOT EXISTS hpas_approvals (
    id               TEXT PRIMARY KEY,
    job_id           TEXT NOT NULL,
    checkpoint_label TEXT NOT NULL,
    content          TEXT NOT NULL,   -- JSON
    summary          TEXT,
    status           TEXT DEFAULT 'Pending',
    reviewer_ids     TEXT,            -- JSON array
    deadline         DATETIME,
    approved         INTEGER,
    comments         TEXT,
    reviewed_at      DATETIME,
    created_at       DATETIME DEFAULT (datetime('now'))
);

-- PipelineSpec phases (Phase 4)
CREATE TABLE IF NOT EXISTS hpas_pipeline_phases (
    id          TEXT PRIMARY KEY,
    approval_id TEXT NOT NULL REFERENCES hpas_approvals(id),
    phase_name  TEXT NOT NULL,
    status      TEXT DEFAULT 'Pending',
    artifact    TEXT,               -- JSON: phase output artifact
    comments    TEXT,
    completed_at DATETIME,
    created_at  DATETIME DEFAULT (datetime('now'))
);
```

**`src-tauri/src/hpas/mistake_memory.rs`**
```rust
// Core operations:
pub async fn write_mistake(conn: &Connection, mistake: &Mistake) -> Result<()> {
    // Upsert: if same (task_type, flag, namespace) exists → increment occurred_count
    // Promote to CONFIRMED if occurred_count >= confirmThreshold (default: 2)
}

pub async fn query_mistakes(
    conn: &Connection,
    task_type: &str,
    namespace: &str,
    max: usize,
) -> Result<Vec<Mistake>> {
    // ORDER BY confidence_tag DESC, last_seen_at DESC LIMIT max
    // CONFIRMED sorts before ONCE
}
```

**`src-tauri/src/hpas/structural_memory.rs`**
```rust
pub async fn god_nodes(conn: &Connection, graph_id: &str, top_k: usize) -> Result<Vec<Node>> {
    // SELECT * FROM hpas_struct_nodes WHERE graph_id = ? ORDER BY degree DESC LIMIT top_k
}

pub async fn bfs(conn: &Connection, start_id: &str, depth: usize) -> Result<Vec<Node>> { ... }

pub async fn write_node(conn: &Connection, node: &Node) -> Result<()> {
    // INSERT OR REPLACE — updates degree on edge writes
}

pub async fn write_edge(conn: &Connection, edge: &Edge) -> Result<()> {
    // INSERT OR REPLACE — increments degree on both from_id and to_id nodes
}
```

**Updated activation sequence in `session.rs`:**
```
1. MistakeMemory.query(task_type, namespace)    → prepend CONFIRMED first, then ONCE
2. StructuralMemory.god_nodes(graph_id, top_k=3) → prepend most-connected entities
3. VectorMemory.query(current_delta, top_k=5)   → semantic retrieval
4. Assemble context slice from Parent dispatch
5. Inject: [mistakes] + [god_nodes] + [vector_context] + [task_slice]
6. Run inference
7. On flag raised → MistakeMemory.write(flag)
   SemanticCompressor → auto-writes to StructuralMemory
```

**Updated `compressor.rs` for StructuralMemory auto-writes:**
```rust
// After compressing agent output:
let relationships = extract_structural_relationships(&compressed_output);
for rel in relationships {
    structural_memory::write_node(&conn, &rel.node).await?;
    structural_memory::write_edge(&conn, &rel.edge).await?;
}
// Agents never call StructuralMemory directly — compressor handles it
```

---

## Phase 4 — ValidationGate + IsolatedWorkspace + ApprovalGate

**`src-tauri/src/hpas/validation_gate.rs`**
```rust
pub enum ValidatorType { Command, Http, Function }

pub struct ValidationGate {
    pub validator_type:  ValidatorType,
    pub validator:       String,
    pub pass_condition:  String,
    pub on_fail:         FailureAction,
    pub max_retries:     u32,
    pub retry_delay:     std::time::Duration,
    pub include_output:  bool,
    pub failure_mistake: bool,    // default: true
}

impl ValidationGate {
    pub async fn validate(&self, agent_output: &serde_json::Value, ...) -> Result<ValidationResult> {
        for attempt in 0..=self.max_retries {
            let validator_output = match self.validator_type {
                ValidatorType::Command  => run_command(&self.validator, agent_output).await?,
                ValidatorType::Http     => call_http(&self.validator, agent_output).await?,
                ValidatorType::Function => call_function(&self.validator, agent_output).await?,
            };
            if eval_condition(&self.pass_condition, &validator_output) {
                return Ok(ValidationResult { passed: true, attempt_count: attempt + 1, .. });
            }
            if self.failure_mistake {
                mistake_memory::write_mistake(&conn, &Mistake { flag: "validation_failed", .. }).await?;
            }
            // Re-dispatch agent with diagnostic appended
            if attempt < self.max_retries {
                let enriched = if self.include_output {
                    append_diagnostic(task_slice, &validator_output)
                } else { task_slice.clone() };
                current_output = sub_agent.dispatch(enriched).await?.result;
            }
        }
        Ok(ValidationResult { passed: false, escalated: true, .. })
    }
}
```

**IsolatedWorkspace in `agent.rs` (Phase 4 extension):**
```rust
if self.isolated_workspace {
    let workspace = tempfile::tempdir()?;
    // Agent receives workspace path in task_slice
    let enriched = task_slice_with_workspace(&task_slice, workspace.path());
    let raw_output = run_inference(&enriched).await?;
    let files = collect_workspace_files(workspace.path())?;
    // workspace auto-cleaned when tempdir drops
    Ok(SubAgentResult { result: compress(raw_output), workspace_files: files, .. })
}
```

---

## Bypass Criteria (enforce in every PR review)

Route through HPAS when the operation involves intelligence:
- API response interpretation
- Multi-step data assembly
- Semantic compression or cross-referencing
- Multi-agent coordination

Bypass HPAS (direct `invoke()`) when the operation is pure data transport:
- Real-time UI state updates
- Cursor position / editor state
- Local file reads
- Simple CRUD operations (create matter, upload document, mark deadline complete)

---

## Tests Required Before Each Phase Ships

### Phase 3A
- `test_hpas_dispatch_returns_schema_conforming_result` — result.data matches output_schema
- `test_hpas_dispatch_no_null_checks_needed` — result.data has no undefined fields

### Phase 3B
- `test_parallel_dispatch_wall_time` — 3 agents run in parallel; total ≤ slowest + 10ms
- `test_session_agent_delta_only` — SessionAgent history never replayed in full

### Phase 3C
- `test_mistake_capture` — agent raises flag → MistakeMemory entry created → next activation receives it
- `test_mistake_confirmed_promotion` — same mistake raised twice → confidence_tag = CONFIRMED
- `test_structural_memory_god_nodes` — agent output → compressor → nodes written → god_nodes returns correct top entity
- `test_registry_confidence_fields` — entity written with confidence=0.4 → read back with same value

### Phase 4
- `test_validation_gate_retry` — agent output fails → diagnostic appended → retry → passes
- `test_isolated_workspace_no_collision` — two parallel agents write same filename → no collision
- `test_pipeline_spec_rejection` — phase 2 rejected → agent re-dispatched with reviewer comments
