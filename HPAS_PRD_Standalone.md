# Product Requirements Document
## Hierarchical Parallel Agent System (HPAS)
### A General-Purpose Multi-Agent Orchestration Engine

---

**Version:** 1.0.0  
**Status:** Stable — Platform Agnostic Specification  
**Author:** Internal  
**Date:** April 2026  
**Architectural Reference:** https://github.com/WeaveMindAI/weft + https://weavemind.ai/docs  
**Runtime:** Platform-defined (SQLite/tokio for local; Restate for distributed)

**Changelog:**
- v1.0.0 — Extracted from combined PRD v0.4.0. All platform-specific content removed. Standalone specification finalised. Design Decisions Log carried forward in full.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Problem Statement](#2-problem-statement)
3. [Goals and Non-Goals](#3-goals-and-non-goals)
4. [System Architecture Overview](#4-system-architecture-overview)
5. [Agent Hierarchy Specification](#5-agent-hierarchy-specification)
6. [Component Catalog](#6-component-catalog)
7. [Context Management and Compression](#7-context-management-and-compression)
8. [Token Economics Model](#8-token-economics-model)
9. [Communication Protocols](#9-communication-protocols)
10. [Performance Characteristics](#10-performance-characteristics)
11. [Integration Contract](#11-integration-contract)
12. [Design Decisions Log](#12-design-decisions-log)

---

## 1. Executive Summary

**HPAS (Hierarchical Parallel Agent System)** is a general-purpose multi-agent orchestration engine. It introduces a structured three-level hierarchy of agents — GrandParents (L0), Parents (L1), and Workers (L2) — that operate in parallel, maintain persistent context at every level via delta activation, communicate laterally via a typed message bus, and apply layered semantic compression to achieve token consumption proportional to **information novelty** rather than total work volume.

HPAS is domain-agnostic and platform-agnostic. It can be implemented in any language against any persistence backend. The architecture was directly informed by Weft's design principles (parallelism via typed ports, hierarchy via nested groups, durable execution, null propagation), and extends Weft's model with the components Weft does not provide: persistent LLM session state, semantic compression, vector memory, lateral communication, and entity registry.

HPAS is designed to be embedded as an execution engine inside any product — desktop app, web service, CLI tool, API server — without modification to its core component specifications. Platform-specific binding (how the host product calls HPAS, what persistence backend is used, how the bus is wired) is defined in a separate integration document per product.

**Core claim:** At N=200 parallel tasks, HPAS consumes approximately 127× fewer tokens than single-context approaches and 7.5× fewer than naive parallel approaches, with the efficiency gap widening further on multi-pass workloads.

---

## 2. Problem Statement

### 2.1 Current State of Multi-Agent Systems

Existing multi-agent approaches suffer from three compounding inefficiencies:

**Context re-establishment at every activation.** When an agent wakes up after a pause, it re-reads its full conversation history from scratch. An agent that has processed 50 prior results pays for all 50 results on activation 51, even though 50 of them have not changed. Cost grows O(n²) with work done.

**No lateral communication.** Agents in parallel branches cannot notify each other of changes without routing through a central orchestrator. A change in one branch that affects another forces a full re-run or a complete re-injection of context into the affected agent — typically 10–35× more expensive than a targeted delta message.

**Single-context monolithic execution.** Attempting complex multi-deliverable work in one context window causes token costs to grow quadratically. The model accumulates every prior output as history, and the cost per inference grows linearly with work done.

### 2.2 What Weft Gets Right — and What HPAS Adds

Weft's architecture (documented at weavemind.ai/docs and WeaveMindAI/weft DESIGN.md) solves several of these problems elegantly at the language level. Understanding what it covers and what it doesn't clarifies HPAS's scope.

**Already solved by Weft natively (HPAS inherits these principles):**
- Parallelism — `List[T]` port typing → automatic parallel lane expansion with gather/broadcast
- Hierarchy and encapsulation — nested Groups with typed `self` interfaces and scoped children
- Durable execution — Restate-backed persistence, crash recovery, suspend/resume
- Human-in-the-loop — `HumanQuery` node, durable pause for days, resume on answer
- Type-checked wiring — compiler catches missing connections and type mismatches before runtime
- Null propagation — skipped lanes flow through gracefully without exception machinery

**Not present in Weft — HPAS's novel contribution:**
- Persistent LLM conversation history across activations (`SessionAgent`)
- Token budget management and context ceiling enforcement with two strategies
- Semantic compression at agent boundaries with typed output schemas (`SemanticCompressor`)
- Vector-based O(1) context retrieval regardless of accumulated history (`VectorMemory`)
- Lateral typed communication between sibling agents (`MessageBus`)
- Ordered lateral communication between GrandParents (`OrchestratorBus`)
- Single source of truth for shared entities (`RegistryAgent`)
- Periodic history distillation (`ContextCompressor`)
- Batch API webhook-resumed execution (`SubAgent` batchMode)

Every component in HPAS's novel layer would need to be built in whatever language the host platform uses regardless of whether Weft is used as a runtime — Weft catalog nodes are themselves Rust. HPAS specifies what to build; the host platform determines how.

---

## 3. Goals and Non-Goals

### 3.1 Goals

- Define a complete, platform-agnostic specification for a hierarchical parallel agent system
- Specify all nine components with full input/output contracts, configuration, and behaviour
- Achieve token consumption that scales as O(n log n) with number of tasks, not O(n²)
- Enable delta activation at every hierarchy level — no agent ever re-reads unchanged history
- Enable lateral communication between sibling agents without routing through a central orchestrator
- Define a stable dispatch contract (`Job` / `StructuredResult`) that any host platform implements
- Support three execution modes: Interactive (synchronous), Batch (webhook-resumed), Streaming (progressive)
- Be implementable against any persistence backend via a `WorkflowStore` trait/interface

### 3.2 Non-Goals

- Specifying a particular implementation language or runtime
- Defining platform-specific integration details (covered in per-product integration documents)
- Domain-specific agent logic — all agents are Agent1, Agent2...AgentN
- Multi-tenant cloud isolation (host platform concern)
- UI or visual graph rendering (host platform concern)

---

## 4. System Architecture Overview

### 4.1 Topology

```
┌─────────────────────────────────────────────────────────────────────┐
│                    GRANDPARENT LAYER (L0)                           │
│                                                                     │
│   GrandParent1 ◄──── OrchestratorBus ────► GrandParent2            │
│        │            [strict seq_id order]        │                  │
│        │                    │                    │                  │
│        │           [RegistryAgent]               │                  │
│        │              ↑ ↑ ↑ ↑                   │                  │
└────────┼──────────────┼─┼─┼─┼────────────────────┼──────────────────┘
         │              │ │ │ │                    │
┌────────┼──────────────┼─┼─┼─┼────────────────────┼──────────────────┐
│        ↓    PARENT LAYER (L1)                    ↓                  │
│                                                                     │
│  Parent1 ◄──── MessageBus ────► Parent2 ◄──── MessageBus ────► Parent3
│     │          [seq_id dedup]        │                        │    │
│  [SemComp]                       [SemComp]               [SemComp] │
└─────┼───────────────────────────────────────────────────────────────┘
      │
┌─────┼───────────────────────────────────────────────────────────────┐
│     ↓      AGENT LAYER (L2)                                        │
│                                                                     │
│  Agent1  Agent2  Agent3  Agent4  Agent5  ...  AgentN               │
│  (SubAgent — scoped context, fully parallel, single activation)     │
└─────────────────────────────────────────────────────────────────────┘
```

### 4.2 Data Flow

```
Job Input
    │
    ▼
GrandParent activated (full context, paid once)
    │
    ├── Dispatches domain slice to Parent1  ─┐
    ├── Dispatches domain slice to Parent2   ├─ parallel
    └── Dispatches domain slice to Parent3  ─┘
              │
              ▼
    Parent receives slice, wakes with DELTA only (not full history)
              │
              ├── Dispatches task slice to Agent1  ─┐
              ├── Dispatches task slice to Agent2   ├─ parallel
              └── Dispatches task slice to Agent3  ─┘
                        │
                        ▼
              Agent executes scoped task
                        │
                        ▼
              SemanticCompressor: schema enforcement + token ceiling
                        │
                        ▼
              Parent wakes with delta only (~150 tokens, not raw output)
              Parent writes new entities to RegistryAgent
              Parent publishes delta to MessageBus if sibling impact
                        │
                        ▼
              Parent summarises domain to GrandParent on completion
                        │
                        ▼
              GrandParent integrates (~2,000 tokens delta), decides next wave
```

### 4.3 Core Principles

**Delta activation.** No agent ever re-reads its full history on activation. The persistence layer delivers only the new incoming delta. History is stored durably but not re-injected — the agent's conversation state is maintained by the `SessionAgent` component and appended-to, never replayed.

**Compression at every boundary.** Every agent output passes through `SemanticCompressor` before crossing a hierarchy boundary. Raw reasoning chains never travel upward. Only schema-conforming structured facts cross boundaries.

**Single source of truth.** Shared entities live exclusively in `RegistryAgent`. No agent maintains its own copy. Consistency is enforced at the Registry, not discovered post-hoc.

**Lateral before vertical.** When a change in Parent1's domain affects Parent2's domain, Parent1 publishes to `MessageBus`. Parent2 receives the delta and decides whether to re-dispatch agents. GrandParent is not involved unless the change requires project-level decisions.

**Self-contained deltas.** Every message on any bus carries complete information — old value and new value, not just "something changed." This makes message ordering irrelevant at L1: each event is a complete fact, not a diff that depends on prior state.

---

## 5. Agent Hierarchy Specification

### 5.1 GrandParent Agent (L0)

**Role:** Project-level orchestration. Holds the complete job context. Does not execute tasks directly.

**Activation triggers:**
- Initial job start
- Domain completion signal from any Parent
- Cross-domain conflict escalated via OrchestratorBus
- Human approval received via `ApprovalGate`

**Context contents:**
- Job description and goals
- Domain definitions and assignments
- All Parent completion states
- Cross-domain dependency map
- Running log of domain-level summaries (compressed via SemanticCompressor)

**Output per activation:**
- Next dispatch directive to one or more Parents
- Cross-domain constraint updates published to OrchestratorBus
- Completion signal when all domains done
- Escalation to `ApprovalGate` when confidence is below threshold

**Token budget:** ~20,000 tokens initial activation. ~2,000 tokens per delta activation.

**Parallelism:** Multiple GrandParents operate in parallel for very large jobs. Each owns a subset of domains. GrandParents communicate exclusively via `OrchestratorBus` with strict `seq_id` ordering.

---

### 5.2 Parent Agent (L1)

**Role:** Domain-level coordination. Holds context for one domain. Dispatches agents, integrates compressed outputs, maintains domain state, communicates laterally with sibling Parents.

**Activation triggers:**
- Dispatch from GrandParent
- Agent completion (delta arrival via `SemanticCompressor`)
- `MessageBus` notification from sibling Parent
- `RegistryAgent` conflict notification

**Context contents:**
- Domain scope and constraints
- Current GrandParent directive
- All agent completion states within domain
- Compressed results from completed agents
- Lateral messages from sibling Parents
- Relevant Registry state

**Output per activation:**
- Agent dispatch (task slice + constraints)
- Registry writes (new entities)
- MessageBus publish (if sibling impact)
- Domain summary to GrandParent on completion

**Token budget:** ~15,000 tokens initial. ~1,500 tokens per delta activation.

**Parallelism:** All Parents within a GrandParent's scope run in parallel. No Parent waits for a sibling unless a `blocking` severity event is received on the MessageBus.

---

### 5.3 Agent (L2 — Worker)

**Role:** Executes one scoped task. Receives a precise context slice. Produces structured output. Has no awareness of other agents or the hierarchy above it.

**Designated as:** Agent1, Agent2...AgentN. Generic. Domain-agnostic. The system prompt and context slice provided by the Parent define what the agent actually does.

**Activation triggers:**
- Dispatch from Parent (single activation per task by default)
- Re-dispatch from Parent if `retryOnFlag: true` and agent raised a flag

**Context contents:**
- Task description (from Parent dispatch)
- Relevant constraints and standards
- Relevant Registry entries (queried at dispatch time)
- Output schema definition

**Output per activation:**
- Structured result conforming to declared output schema
- Registry write requests
- Confidence score
- Flags for Parent review

**Token budget:** ~4,000 tokens per activation. Hard ceiling enforced by `SemanticCompressor` on output.

**Parallelism:** All agents dispatched by a Parent run fully in parallel. No agent waits for a sibling.

---

## 6. Component Catalog

Nine components constitute HPAS. All are platform-agnostic specifications.

---

### 6.1 `SessionAgent`

**Purpose:** An LLM inference component with persistent conversation history. Unlike a stateless LLM call, `SessionAgent` maintains a running conversation stored in the persistence layer. On each activation it receives only the new delta — not the full history — appends the delta to its stored conversation, and runs inference. History is never re-injected from scratch.

**Configuration:**
```
model:               String  — LLM model identifier
systemPrompt:        String  — agent role and instructions
scope:               String  — "grandparent" | "parent" | "agent"
maxHistoryTokens:    Number  — compression trigger ceiling
                               GrandParent default: 60,000
                               Parent default: 40,000
contextCeiling:      Number  — hard model context window limit
                               (model max minus 4,000 safety margin)
ceilingStrategy:     String  — "compress_on_approach" | "raise_threshold"
outputSchema:        Dict    — typed output structure enforced on every response
confidenceThreshold: Float   — below this, escalate to parent or human
```

**Context Ceiling Management:**

`compress_on_approach` — When history reaches `maxHistoryTokens`, `ContextCompressor` fires proactively. Target: keep running conversation at 70% of `contextCeiling`. Default for GrandParent and Parent.

`raise_threshold` — Operator sets `maxHistoryTokens` manually for well-bounded jobs. `SessionAgent` still performs a hard pre-activation token count check: if `systemPromptTokens + historyTokens + deltaTokens > contextCeiling`, `ContextCompressor` fires unconditionally regardless of strategy. This is the non-bypassable safety net.

**Inputs:**
```
delta:        Dict   — new information this activation only
constraints:  Dict   — current constraints from parent/grandparent
registryRef:  Ref    — RegistryAgent reference for entity lookup
```

**Outputs:**
```
response:       Dict          — structured output per outputSchema
dispatch:       List[Dict]    — child dispatches (for Parent/GrandParent)
registryWrites: List[Dict]    — entities to register
busPublish:     Dict | null   — lateral message to publish
escalate:       Boolean       — true if confidence below threshold
```

---

### 6.2 `SemanticCompressor`

**Purpose:** Enforces a typed output schema and hard token ceiling at every hierarchy boundary. Strips reasoning chains. Preserves structured facts and flags. Sits between every agent output and the boundary it crosses.

**Configuration:**
```
outputSchema:     Dict    — required fields and types
maxOutputTokens:  Number  — hard ceiling (default: 150 for Agent→Parent)
compressionModel: String  — model for compression (may differ from main model)
preserveFlags:    Boolean — always pass through flags regardless of ceiling
```

**Inputs:**
```
rawOutput: String — full agent response
schema:    Dict   — output schema to enforce
```

**Outputs:**
```
compressed:       Dict          — schema-conforming output
tokenCount:       Number        — actual tokens in compressed output
compressionRatio: Float         — raw/compressed ratio for monitoring
flags:            List[String]  — always preserved, never dropped
```

**Behaviour:** If raw output already conforms to schema and is within ceiling — passthrough. Otherwise run a compression inference call. Never silently drop flags. If output cannot be compressed to schema within ceiling — flag for Parent review rather than truncate.

---

### 6.3 `MessageBus`

**Purpose:** Typed lateral communication channel between sibling agents at L1. Agents publish self-contained delta events. Subscribers receive only events matching their domain filter. Sequence numbers used for deduplication only — strict ordering not enforced at L1.

**Configuration:**
```
busId:       String        — unique identifier
level:       String        — "parent"
subscribers: List[String]  — agent instance IDs subscribed
```

**Publish inputs:**
```
publisherId:     String        — publishing agent ID
seq_id:          Number        — monotonic per publisher (dedup only)
eventType:       String        — "entity_updated" | "constraint_changed" | "conflict" | "completion"
payload:         Dict          — self-contained delta (old + new values)
affectedDomains: List[String]  — subscriber filter
severity:        String        — "info" | "warning" | "blocking"
```

**Per-subscriber outputs:**
```
event:     Dict    — full event payload
seq_id:    Number  — for deduplication
eventType: String
fromAgent: String
```

**Delta self-containment rule:** Every payload carries both old and new values. No event references prior events. A subscriber can correctly interpret any event in any arrival order.

---

### 6.4 `OrchestratorBus`

**Purpose:** Dedicated inter-GrandParent communication channel at L0. Distinct from `MessageBus` in three ways: carries project-level events only, enforces strict global `seq_id` ordering via a serialised counter, and uses a separate schema scoped to L0 concerns.

**Configuration:**
```
busId:       String        — unique identifier, typically "orchestrator_bus"
grandParents: List[String] — all GrandParent instance IDs
```

**Publish inputs:**
```
publisherId: String        — publishing GrandParent ID
seq_id:      Number        — auto-assigned by bus via serialised counter
eventType:   String        — "domain_handoff" | "global_constraint" | "conflict" | "completion"
payload:     Dict          — self-contained project-level delta
affectedGPs: List[String]  — receiving GrandParent filter
```

**Per-subscriber outputs:**
```
event:     Dict    — full event payload
seq_id:    Number  — strictly ordered
eventType: String
fromGP:    String
```

**Ordering guarantee:** Monotonic `seq_id` assigned via a single serialised counter. Receiving GrandParents buffer out-of-order events until preceding `seq_id` is processed.

**Schema comparison:**

| Field | MessageBus (L1) | OrchestratorBus (L0) |
|---|---|---|
| Scope | Domain-level deltas | Project-level events |
| Affected targets | `affectedDomains` | `affectedGPs` |
| Entity refs | Registry entity IDs | Domain IDs |
| `seq_id` | Deduplication only | Strict ordering enforced |
| Max payload | 200 tokens | 500 tokens |

---

### 6.5 `VectorMemory`

**Purpose:** Replaces full history re-injection with semantic retrieval. Agents query at activation time and receive only the top-k most semantically relevant prior results. Context size stays O(1) regardless of accumulated history.

**Configuration:**
```
embeddingModel:      String  — model for generating embeddings
topK:                Number  — results per query (default: 5)
similarityThreshold: Float   — minimum cosine similarity (default: 0.80)
namespace:           String  — scope isolation by agent
```

**Inputs:**
```
query:     String     — current task description or question
store:     Dict|null  — new result to embed and store
namespace: String     — scope filter
```

**Outputs:**
```
relevantContext: String  — top-k results formatted for context injection
matchCount:      Number  — results returned
```

**Storage:** Vector database per deployment. Each entry: `(id, namespace, agent_id, timestamp, embedding_vector, compressed_text)`. Write ordering managed by persistence layer.

---

### 6.6 `RegistryAgent`

**Purpose:** Single source of truth for all shared entities across the agent graph. All agents that create or reference shared entities (IDs, cross-references, counters, named items) do so exclusively through the Registry.

**Configuration:**
```
registryId:     String  — unique identifier
schema:         Dict    — entity types and required fields
conflictPolicy: String  — "reject" | "merge" | "escalate"
```

**Inputs:**
```
write: Dict    — entity to register (type, id, fields, author_agent)
read:  String  — entity ID to look up
query: Dict    — filter query across entity type
```

**Outputs:**
```
result:      Dict    — entity data or write confirmation
conflict:    Dict|null — conflict details if write rejected
entityCount: Number  — total entities of requested type
```

**Implementation note:** Writes serialised by persistence layer to prevent race conditions. Reads are direct queries — not serialised — for low latency.

---

### 6.7 `SubAgent`

**Purpose:** Spawns a scoped worker agent with a specific task, waits for completion, returns the compressed result. Handles dispatch, execution, result collection, and optional batch mode as a single composable unit.

**Configuration:**
```
agentLabel:   String  — display name (Agent1, Agent2, etc.)
model:        String  — LLM model
systemPrompt: String  — scoped role and instructions
outputSchema: Dict    — required output structure
maxTokens:    Number  — context ceiling for this agent
retryOnFlag:  Boolean — auto re-dispatch if agent raises a flag
maxRetries:   Number  — maximum re-dispatch attempts
batchMode:    Boolean — use Batch API (async, webhook-resumed) vs synchronous
```

**Inputs:**
```
taskSlice:   Dict  — scoped context from Parent
registryRef: Ref   — Registry connection
constraints: Dict  — task-specific constraints
```

**Outputs:**
```
result:         Dict          — compressed, schema-conforming output
flags:          List[String]  — issues raised
confidence:     Float         — agent self-reported confidence
registryWrites: List[Dict]    — entities to register
tokensUsed:     Number        — actual tokens consumed
```

**Batch mode:** When `batchMode: true`, dispatch uses the Anthropic Batch API. The calling workflow suspends on a promise keyed by batch request ID. The Batch API completion webhook resolves the promise, resuming the workflow. Zero polling overhead.

---

### 6.8 `ContextCompressor`

**Purpose:** Periodically distils a `SessionAgent`'s accumulated conversation history into a dense factual state document. Triggered when history exceeds `maxHistoryTokens`. Preserves all decisions and facts. Sheds conversational scaffolding.

**Configuration:**
```
triggerThreshold:  Number  — token count that triggers compression
targetTokens:      Number  — target size of compressed history
compressionModel:  String  — model for distillation
preserveDecisions: Boolean — always preserve explicit decisions (default: true)
```

**Inputs:**
```
history:   String — full conversation history
agentRole: String — agent's system prompt for context-aware compression
```

**Outputs:**
```
compressedHistory: String — distilled state document
originalTokens:    Number — tokens before
compressedTokens:  Number — tokens after
ratio:             Float  — compression ratio achieved
```

---

### 6.9 `ApprovalGate`

**Purpose:** Pauses the agent graph at a defined checkpoint pending human review. Supports deadlines, escalation paths, and multi-reviewer flows. Resumes on approval with reviewer comments available to downstream agents.

**Configuration:**
```
reviewers:      List[String] — reviewer identifiers
deadline:       Duration     — time before auto-escalation
escalationPath: String       — recipient if deadline passes without action
requireAll:     Boolean      — all reviewers must approve vs. any one
```

**Inputs:**
```
content:        Dict   — material to review
summary:        String — human-readable summary
checkpointLabel: String — name of this checkpoint
```

**Outputs:**
```
approved:    Boolean
comments:    String
reviewerIds: List[String] — who approved/rejected
reviewedAt:  Timestamp
```

---

## 7. Context Management and Compression

### 7.1 Compression Stack

Four layers operate independently and stack multiplicatively:

**Layer 1 — Architectural scoping**
Each agent receives only the context slice relevant to its task. No agent sees another agent's context. Typical reduction: 60–80% vs monolithic context.

**Layer 2 — Semantic boundary compression**
Every output crossing a hierarchy boundary passes through `SemanticCompressor`. Raw reasoning chains stripped. Only schema-conforming structured facts cross boundaries. Typical reduction: 10:1 to 15:1 on raw agent output size.

**Layer 3 — Semantic retrieval**
`VectorMemory` replaces full history re-injection with top-k retrieval. Agent context size becomes O(1) with respect to accumulated history. Effective context stays constant regardless of how many prior activations have occurred.

**Layer 4 — Periodic distillation**
`ContextCompressor` triggers when any `SessionAgent`'s history exceeds `maxHistoryTokens`. Converts accumulated summaries into a dense state document. Prevents even compressed history from growing unboundedly over very long jobs.

### 7.2 Output Schema Discipline

Every agent boundary has a declared output schema — the compression contract:

| Boundary | Max tokens | Required schema fields |
|---|---|---|
| Agent → Parent | 150 | `id`, `status`, `key_values`, `flags`, `confidence` |
| Parent → GrandParent | 500 | `domain`, `completion_pct`, `decisions`, `cross_domain_impacts`, `blocked_by` |
| MessageBus event | 200 | `event_type`, `source_domain`, `affected_entities`, `delta` |
| OrchestratorBus event | 500 | `event_type`, `affected_gps`, `domain_refs`, `delta` |

Schemas enforced by `SemanticCompressor`. If an agent produces output that cannot be compressed to schema within the ceiling, the compressor flags it for Parent review rather than silently truncating.

### 7.3 VectorMemory Query Strategy

At each Parent activation, before reasoning:

```
query  = current_delta + current_constraints
results = VectorMemory.query(query, topK=5, threshold=0.80, namespace=parent_id)
```

A Parent that has received 200 agent results still only sees 5 at each activation — the 5 most semantically relevant to the current task. O(1) context growth for Parents regardless of job size.

---

## 8. Token Economics Model

### 8.1 Per-Activation Cost Breakdown

| Component | Initial Activation | Delta Activation | Notes |
|---|---|---|---|
| GrandParent | ~20,000 tokens | ~2,000 tokens | Delta = new domain summary only |
| Parent | ~15,000 tokens | ~1,500 tokens | Delta = compressed agent result |
| Agent (SubAgent) | ~4,000 tokens | N/A — single activation | Fixed scoped context |
| ContextCompressor | ~5,000 tokens input | ~500 tokens output | Periodic trigger |
| SemanticCompressor | ~1,200 tokens input | ~150 tokens output | Per boundary crossing |

### 8.2 Scaling Model — N Tasks

For N parallel tasks, K Parents, 1 GrandParent:

```
Total ≈ 20,000 + 17,000K + 6,850N
```

For K=5 Parents, N=100 Agents: ~790,000 tokens.
Compare to single-context O(n²): ~10,000,000 tokens.
**HPAS: ~12.7× more efficient at N=100, ~50× at N=500.**

### 8.3 Batch API Multiplier

All SubAgent L2 calls are non-interactive and batchable. Batch API halves effective cost:

```
SubAgent at batch price: 4,000 × N × 0.5 = 2,000N
```

For N=100, K=5: effective cost drops from ~790,000 to ~590,000 token-equivalents.

### 8.4 Change Propagation Cost

```
Change event published:            ~200 tokens
Affected Parent delta activation:  ~1,500 tokens
Affected Agent re-execution:       ~4,000 tokens (if needed)
Registry update:                   ~100 tokens
─────────────────────────────────────────────
Total per cross-domain change:     ~5,800 tokens
```

Versus full re-run of affected domain: ~50,000–200,000 tokens.
**Change propagation is 10–35× cheaper than re-running.**

### 8.5 Multi-Pass Compounding

At N=200, three passes (generate + review + fix):

| Approach | Total (3 passes) |
|---|---|
| Single context | ~60M tokens |
| Naive parallel | ~3.6M tokens |
| Basic multi-agent | ~1.8M tokens |
| **HPAS** | **~240K tokens** |

Pass 2 and 3 cost ~40K each (vs 160K for Pass 1) because Parents already hold context, only revised agents re-run, and the bus propagates only changed constraints.

---

## 9. Communication Protocols

### 9.1 Vertical — Parent ↔ Child

Direct typed dispatch. No bus involved.

```
Parent → SubAgent dispatch:
  {
    taskId:       String,
    taskType:     String,
    context:      Dict,        // scoped slice
    constraints:  Dict,
    outputSchema: Dict,
    registryRef:  Ref
  }

SubAgent → Parent result:
  {
    taskId:         String,
    status:         "complete" | "flagged" | "failed",
    result:         Dict,        // schema-conforming compressed output
    flags:          List[String],
    confidence:     Float,
    registryWrites: List[Dict]
  }
```

### 9.2 Lateral — MessageBus (L1)

```
MessageBus event:
  {
    eventId:         String,
    seq_id:          Number,       // deduplication only
    eventType:       String,
    sourceDomain:    String,
    affectedDomains: List[String],
    entityRefs:      List[String], // Registry IDs affected
    delta:           Dict,         // self-contained: old + new values
    severity:        "info" | "warning" | "blocking"
  }
```

### 9.3 Lateral — OrchestratorBus (L0)

```
OrchestratorBus event:
  {
    eventId:     String,
    seq_id:      Number,       // strictly ordered
    eventType:   String,
    sourceGP:    String,
    affectedGPs: List[String],
    domainRefs:  List[String],
    delta:       Dict,         // self-contained
    severity:    "info" | "warning" | "blocking"
  }
```

### 9.4 Registry Protocol

```
Write:
  { entityType, entityId, fields, authorAgent, timestamp }

Read:
  { entityId }  →  entity data or null

Query:
  { entityType, filter: Dict }  →  List of matching entities
```

---

## 10. Performance Characteristics

### 10.1 Versus Weft Runtime

HPAS is architecturally informed by Weft but is not dependent on it. When implemented in native Rust with tokio:

| Dimension | Weft + Restate | HPAS native Rust | Advantage |
|---|---|---|---|
| Dispatch latency (local) | ~5–20ms (Restate RPC) | ~0.01ms (tokio) | Rust ~1,000× |
| Parallel task overhead | Medium (Restate per lane) | Minimal (tokio) | Rust |
| Durable execution | Automatic (Restate) | Manual (WorkflowStore) | Weft ergonomics |
| Crash recovery (single machine) | Automatic | SQLite checkpoints | Tie |
| Crash recovery (distributed) | Automatic | Needs Restate or equivalent | Weft |
| Context/token management | Not built in | Native to HPAS | HPAS |
| Bundle size (local app) | Large (Restate + Weft) | Small (tokio + SQLite) | Rust |
| Scale to distributed | Restate handles it | WorkflowStore swap | Tie (with Restate backend) |

### 10.2 Persistence Backend Abstraction

HPAS specifies a `WorkflowStore` interface. Any backend implementing this interface is valid:

```
WorkflowStore interface:
  save(workflow: Workflow) → Result
  load(id: String) → Workflow | null
  resume(id: String, delta: Delta) → Result
  list_pending() → List[Workflow]
```

Local deployment: SQLite. Distributed deployment: Restate. The interface is stable; the backend is swappable without changing any HPAS caller.

---

## 11. Integration Contract

Any product embedding HPAS must implement the following stable dispatch contract. This is the only surface the host product's UI or API layer calls.

### 11.1 Core Types

```
Job {
  id:            String
  job_type:      String
  context:       Dict
  output_schema: Dict
  priority:      "Interactive" | "Batch" | "Streaming"
  max_tokens:    Number | null
}

StructuredResult {
  job_id:      String
  status:      "Complete" | "Flagged" | "Failed"
  data:        Dict           // always conforms to output_schema
  tokens_used: Number
  flags:       List[String]
}
```

### 11.2 Dispatch Modes

**Interactive** — Synchronous. All agents run in parallel via async tasks. Result returned when all agents complete. Target latency: <200ms for typical jobs.

**Batch** — Async. Uses Anthropic Batch API for SubAgent calls. Job ID returned immediately. Result delivered via host platform's event/callback system when webhook received. Zero polling.

**Streaming** — Progressive. Partial results delivered as agents complete. Host platform streams to UI in real time.

### 11.3 Bypass Criteria

Not all host product operations should route through HPAS. Route through HPAS when the operation involves intelligence: API response interpretation, data assembly, semantic compression, cross-referencing, multi-step coordination. Bypass HPAS (direct platform IPC) when the operation is pure data transport with no interpretation: real-time sync, sub-50ms UI state, local file reads.

---

## 12. Design Decisions Log

All decisions made during the design of HPAS v1.0.0.

---

**DD-01 — Hot-reloading via dynamic library loading**

*Decision:* Dynamic library loading via Rust's `libloading` crate for server deployments. New nodes compiled as shared libraries can be loaded without restarting the server.

*Constraint:* macOS hardened runtime blocks unsigned dynamic library loading. For `.app` bundle deployments, new components are compiled in at build time. Hot-reload applies to server deployments only.

---

**DD-02 — VectorMemory write ordering**

*Decision:* Write ordering guaranteed by the persistence layer's virtual object serialisation (Restate) or equivalent single-writer guarantee (SQLite WAL). No additional write queue needed in front of the vector store.

---

**DD-03 — SessionAgent context ceiling management**

*Decision:* Two strategies (`compress_on_approach`, `raise_threshold`) with a hard pre-activation safety net that fires `ContextCompressor` unconditionally if the ceiling would be breached. Default `maxHistoryTokens`: 60,000 (GrandParent), 40,000 (Parent).

---

**DD-04 — MessageBus ordering: deduplication only at L1**

*Decision:* No strict ordering at L1. `seq_id` for deduplication only. Every event payload self-contained (old + new values). Makes ordering irrelevant: each event is a complete fact. Strict ordering at L1 would serialise Parent activations and destroy parallelism.

---

**DD-05 — OrchestratorBus as dedicated L0 node**

*Decision:* `OrchestratorBus` is a distinct component from `MessageBus`. Strict global `seq_id` ordering at L0 via serialised counter. Separate schema, larger payload ceiling (500 vs 200 tokens), separate storage table.

*Rationale:* GrandParent-to-GrandParent events carry fundamentally different information. Out-of-order domain handoffs could cause GrandParents to re-assign work already in progress — strict ordering is required at L0 where it is not at L1.

---

**DD-06 — Batch API delivery via webhook callback**

*Decision:* Webhook callback over cron polling. After batch dispatch, calling workflow suspends on a promise keyed by batch request ID. Batch API completion webhook resolves the promise. Zero polling overhead. Near-zero latency after batch completion.

*Rationale:* Polling wastes resources on "not ready yet" calls. Webhook is event-driven: workflow sleeps with zero consumption, wakes exactly once.

---

**DD-07 — Platform-agnostic specification**

*Decision:* HPAS is specified independently of any host platform, runtime, or language. Platform-specific binding (persistence backend, bus implementation, dispatch interface wiring) is documented in per-product integration documents. This PRD covers only the platform-agnostic specification.

*Rationale:* HPAS should be reusable across products without re-deriving its architecture. Any new product embedding HPAS reads this document, then writes a thin integration document covering only the platform-specific bindings.

---

*End of Document — HPAS Standalone Specification v1.0.0*
