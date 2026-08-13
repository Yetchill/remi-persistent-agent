# Remi Architecture

This document is a short map of the current implementation for contributors.

## Start Here

1. `src/agent/runtime.ts` — the only application Agent pipeline.
2. `src/agent/context.ts` and `src/agent/actions.ts` — context contract and validated high-level actions.
3. `src-tauri/src/memory.rs` — selective long-term memory lifecycle and retrieval.
4. `src-tauri/src/database.rs` — SQLite schema and persistence operations.
5. `src/windows/PetWindow.tsx` — desktop-body composition, heartbeat timers, and the Runtime instance.
6. `research/README.md` — deterministic memory evaluation harness.

## System Overview

```text
Pet / Chat / Timers
        │ typed Event
        ▼
AgentRuntime
  Event → Policy → Context → Provider → Parser → Validator → Executor
                    │                                │
                    │                                ├─ high-level action trace
                    │                                ├─ Pet State
                    │                                └─ memory candidate
                    ▼
        SOUL + State + Working Memory + active Long-term Memory
                                     │
                                     ▼
                      SQLite lifecycle + relation + operation trace
```

The Provider is a replaceable reasoning backend. It does not own Remi's identity, state, memory, window, coordinates, or animation. All chat enters `AgentRuntime`; there is no parallel chat API.

## Agent Flow

The source-of-truth pipeline is `AgentRuntime.handleEvent`:

1. Persist a typed event.
2. Apply local event policy. `BODY_HEARTBEAT` never calls an LLM.
3. Build context from SOUL/version, selected Provider/Model, current Pet State and goal, current event, recent conversation, active relevant memories, relationship summary, available actions, and policy versions.
4. Call the selected OpenAI-compatible Provider only when policy requires it.
5. Parse and validate the JSON action envelope against the action whitelist.
6. Execute high-level actions locally and trace success/failure.
7. Persist resulting Pet State, Working Memory, or lifecycle-validated long-term memory.

The LLM may choose `speak`, `remember`, `wander`, `sleep`, `wake`, state/goal changes, or `noop`. Window coordinates and movement interpolation always remain in the local Motion Engine.

## Memory Lifecycle

Working Memory is the bounded recent conversation stored in `messages`. Durable candidates follow one path in `src-tauri/src/memory.rs`:

```text
Candidate
  → worth remembering?
  → Semantic / Episodic / Relationship
  → temporal validity + provenance
  → find active related memory / logical slot
  → ADD | UPDATE | MERGE | SUPERSEDE | IGNORE
  → persist memory, relation, and operation trace
```

Memory status is `active`, `outdated`, `archived`, or `merged`. Relations are `supports`, `contradicts`, `supersedes`, `derived_from`, and `merged_into`. Retrieval expires time-bound memories, searches active memories only, then ranks text relevance, recency, importance, confidence, and source reliability. The current policy is deterministic `evolving-memory-v1`; no vector database is required.

Provenance is explicit: `user_explicit`, `user_correction`, `agent_inferred`, `conversation`, `heartbeat`, `reflection`, or `system`. Heartbeat observations never masquerade as user statements. The Memory Inspector uses the Rust memory API—not SQL from React—to search and inspect provenance/relations, then edit, archive, restore, delete, or pin. Pin boosts importance once but never bypasses relevance retrieval. Consolidation currently performs conservative exact-duplicate merging only.

## Body Heartbeat vs Agent Heartbeat

`BODY_HEARTBEAT` is local and trace-only. Body wandering, legal screen coordinates, and animation are local mechanisms and consume no LLM calls.

`AGENT_HEARTBEAT` is optional and may ask the Provider for one high-level choice: prefer `noop`, or choose `wander`, `sleep`, or a brief `speak`. Proactive speech is rejected locally when Proactive Interaction is off, Do Not Disturb is on, quiet hours are active, the user interacted recently, the agent is busy, cooldown is active, or the hourly limit is reached. Frequency is a policy multiplier, not a random speaking timer. Behavior settings have one source of truth: SQLite `app_settings`.

## Pet Packs and Companion Profiles

Pet appearance remains outside Agent Core. `src-tauri/src/pet_pack.rs` validates and copies a folder-based pack into app data; `src/pet/packs.ts` resolves its PNG frames and the renderer falls back to `idle` for missing states. Only `active_pet_pack_id` changes when switching, so SOUL, memories, conversation, Provider, and state remain untouched.

Companion Profile format v1 is readable JSON managed by `src-tauri/src/profile.rs`. It contains SOUL/version, all long-term memories and relations, relationship summary, behavior/Pet preferences, and allowlisted Provider/model metadata. It never contains API keys, tokens, base URLs, or credential flags. Import previews first, writes a local backup, then replaces portable profile state transactionally; the machine's Provider configuration, Working Memory, traces, and position remain local.

## Source of Truth

| Concern                    | Source of truth                                            |
| -------------------------- | ---------------------------------------------------------- |
| Product overview           | `README.md`                                                 |
| Identity/persona           | runtime app-data `SOUL.md`; seeded from `src/soul/SOUL.md` |
| Agent pipeline             | `src/agent/runtime.ts`                                     |
| Action contract            | `src/agent/actions.ts`, `src/agent/context.ts`             |
| Pet State and settings     | SQLite through Rust commands                               |
| Long-term memory policy    | `src-tauri/src/memory.rs`                                  |
| Persistence/schema         | `src-tauri/src/database.rs`                                |
| Body motion                | `src/pet/motion.ts`, `src-tauri/src/window.rs`             |
| Provider selection         | SQLite catalog plus in-memory API keys                     |
| Pet appearance             | Pet Pack catalog + `active_pet_pack_id` metadata           |
| Portable companion profile | Profile format v1 JSON; Provider secrets stay local        |
| Research scenarios/results | `research/`                                                |

## Directory Map

```text
src/
  agent/       Runtime, event policy, context, action validation, heartbeat gates
  memory/      Tauri memory client and user-controlled Memory Inspector
  pet/         local desktop body, motion, context menu, and Pet Pack renderer
  profile/     portable Companion Profile client and UI
  providers/   Provider catalog and OpenAI-compatible client
  settings/    settings UI and behavior controls
  soul/        SOUL seed/editor
  windows/     pet, chat-bubble, and settings composition roots
src-tauri/src/
  database.rs  SQLite schema/migrations/repositories
  memory.rs    evolving-memory lifecycle/retrieval/consolidation
  pet_pack.rs  Pet Pack validation/import/catalog
  profile.rs   profile export/preview/transactional replace
  provider.rs  OpenAI-compatible transport and LLM trace
  trace.rs     event/action trace commands
  window.rs    macOS window/work-area operations
  bin/         offline research executables
research/
  scenarios/   readable multi-session fixtures
  results/     generated JSON/CSV evidence
```

## Current Boundaries

This MVP intentionally excludes TTS, image generation, Live2D, complex sprites, MCP, multi-Agent frameworks, Windows/Linux support, and large orchestration frameworks. Future features must continue entering through typed events and validated high-level actions instead of bypassing the Runtime.
