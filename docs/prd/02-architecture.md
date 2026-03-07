# 02 — System Architecture

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Desktop App (Frontend)                │
│              (owned by teammate — not our scope)         │
└──────────────────────┬──────────────────────────────────┘
                       │ HTTP/SSE
                       ▼
┌─────────────────────────────────────────────────────────┐
│                  FastAPI Backend (our scope)             │
│                                                         │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │ REST API │  │ SSE Streaming│  │ Heartbeat Loop   │  │
│  │ Endpoints│  │ /chat/stream │  │ (background)     │  │
│  └────┬─────┘  └──────┬───────┘  └────────┬─────────┘  │
│       │               │                    │            │
│       ▼               ▼                    ▼            │
│  ┌──────────────────────────────────────────────────┐   │
│  │           LangGraph StateGraph                    │   │
│  │                                                   │   │
│  │  preprocess → route_intent → execute_tools        │   │
│  │       │            │              │               │   │
│  │       │            │              ▼               │   │
│  │       │            │      generate_response       │   │
│  │       │            │              │               │   │
│  │       │            ▼              ▼               │   │
│  │       │    human_approval → postprocess           │   │
│  │       │                                           │   │
│  └──────────────────────────────────────────────────┘   │
│                         │                               │
│                         ▼                               │
│  ┌──────────────────────────────────────────────────┐   │
│  │                 Agent Tools                       │   │
│  │                                                   │   │
│  │  ┌─────────────┐  ┌───────────────────────────┐  │   │
│  │  │ gws CLI     │  │ Custom Supabase Tools     │  │   │
│  │  │ (subprocess)│  │ (tasks, context, prefs)   │  │   │
│  │  │ gmail,cal,  │  │                           │  │   │
│  │  │ docs,drive, │  │                           │  │   │
│  │  │ sheets,chat │  │                           │  │   │
│  │  │ meet,tasks..│  │                           │  │   │
│  │  └─────────────┘  └───────────────────────────┘  │   │
│  └──────────────────────────────────────────────────┘   │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│                   Supabase Postgres                      │
│                                                          │
│  ┌────────────┐ ┌──────────────┐ ┌───────────────────┐  │
│  │ sessions   │ │ messages     │ │ user_context      │  │
│  │ tasks      │ │ tool_calls   │ │ checkpoints       │  │
│  │ profiles   │ │ approvals    │ │ heartbeat_state   │  │
│  └────────────┘ └──────────────┘ └───────────────────┘  │
└─────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────┐
│                    Supermemory (RAG)                     │
│                                                          │
│  Semantic memory: patterns, entities, commitments,       │
│  session summaries, learned behaviors                    │
└─────────────────────────────────────────────────────────┘
```

## Data Flow

### 1. Chat Request Flow
```
Desktop App → POST /chat/stream (SSE)
  → FastAPI receives message + session_id
  → LangGraph StateGraph invoked:
      1. preprocess: load user context, recent messages, active tasks
      2. route_intent: classify intent (chat, action, triage, proactive)
      3. execute_tools: call gws CLI / Supabase tools as needed
      4. generate_response: LLM generates response with tool results
      5. human_approval: if action requires approval, pause and wait
      6. postprocess: save state, emit final SSE event
  → SSE events streamed back: status → tokens → tool_results → done
```

### 2. Heartbeat Flow (Background)
```
Every N seconds (configurable, default 60s):
  → Check calendar for upcoming events (15 min window)
  → Check email for high-priority unread
  → Check overdue tasks
  → If anything found → generate proactive nudge
  → Push via SSE/webhook to desktop app
```

### 3. Human-in-the-Loop Flow
```
Agent wants to send email:
  → StateGraph enters "human_approval" node
  → SSE event: { type: "approval_required", action: "send_email", draft: {...} }
  → Desktop app shows [Approve] [Edit] [Reject]
  → User response → POST /chat/approve
  → StateGraph resumes from approval node
  → If approved: execute action (gws gmail +send ...)
  → If rejected: acknowledge and continue
```

## Key Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Agent framework | Custom LangGraph StateGraph | Control over routing, approval, heartbeat; matches InfoSavvy patterns |
| NOT create_react_agent | — | Too opaque; can't inject preprocessing, can't pause for approval |
| Database | Supabase Postgres | Unified state + checkpointing; real-time subscriptions for heartbeat |
| Streaming | SSE (not WebSocket) | Simpler, sufficient for our use case, matches InfoSavvy pattern |
| Google Tools | `gws` CLI (Google Workspace CLI) | 100+ agent skills, dynamic API discovery via Google Discovery Service, structured JSON output, `--dry-run` for safe previewing; covers ALL Google Workspace APIs automatically |
| Web Search | Exa | Neural search for real-time web info (company research, docs, news); `langchain-exa` integration |
| Memory/RAG | Supermemory | Persistent semantic memory for patterns, entities, commitments |
| Auth | Google OAuth via `gws auth` | Direct Google OAuth — user runs `gws auth login` once; no third-party auth layer needed |
| LLM | Gemini (Google DeepMind) | Native Google ecosystem integration; structured JSON output; gemini-3.0-flash for speed, gemini-3.0-pro for complex reasoning |
| Validation | Pydantic v2 | Type safety, serialization, schema generation for structured output |

### Why `gws` CLI over Composio / Direct SDK

The [Google Workspace CLI](https://github.com/googleworkspace/cli) (`gws`) gives us:

1. **Dynamic API discovery** — reads Google's Discovery Service at runtime, so when Google adds a new API endpoint, the agent can use it immediately with zero code changes
2. **100+ pre-built agent skills** — including `+meeting-prep`, `+standup-report`, `+email-to-task`, `+triage` — these are FRIDAY's core features, already implemented
3. **Structured JSON output** — every command returns JSON, perfect for LLM consumption
4. **`--dry-run` flag** — preview mutations before executing, ideal for human-in-the-loop
5. **50+ workflow recipes** — multi-step task sequences for common operations
6. **10 role-based personas** — including `persona-exec-assistant` which aligns with FRIDAY's purpose
7. **Single tool integration** — one `gws` subprocess wrapper gives the agent access to ALL of Google Workspace

### Existing Prior Art

The codebase already includes a working Gemini integration:
- `backend/app/gemini_processor.py` — `FridayExtractor` class using `gemini-2.0-flash` via `google-genai` SDK
- `backend/app/main.py` — `/friday-extract` endpoint for structured meeting extraction
- `docs/google-calendar-heartbeat-plan.md` — Detailed plan for calendar sync and heartbeat architecture

**Dual SDK approach:**
- `langchain-google-genai` — for LangGraph agent (tool calling, structured output, streaming)
- `google-genai` — for direct Gemini API calls (meeting extraction, fast processing)
