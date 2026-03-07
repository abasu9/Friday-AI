# 02 — System Architecture

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Desktop App (Frontend)                │
│              (owned by teammate — not our scope)         │
└──────────────────────┬──────────────────────────────────┘
                       │ HTTP/SSE (Supabase Auth — Google OAuth)
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
│  │  │ Composio    │  │ Custom Supabase Tools     │  │   │
│  │  │ Google Tools│  │ (tasks, context, prefs)   │  │   │
│  │  │ gmail,cal,  │  │                           │  │   │
│  │  │ docs,drive  │  │                           │  │   │
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
│  │ user_prefs │ │ approvals    │ │ heartbeat_state   │  │
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
      3. execute_tools: call MCP/Supabase tools as needed
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
  → If approved: execute action
  → If rejected: acknowledge and continue
```

## Key Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Agent framework | Custom LangGraph StateGraph | Control over routing, approval, heartbeat; matches InfoSavvy patterns |
| NOT create_react_agent | — | Too opaque; can't inject preprocessing, can't pause for approval |
| Database | Supabase Postgres | Unified state + checkpointing; real-time subscriptions for heartbeat |
| Streaming | SSE (not WebSocket) | Simpler, sufficient for our use case, matches InfoSavvy pattern |
| Google Tools | Composio | Pre-built LangGraph-compatible Google Workspace tools; handles per-user OAuth |
| Memory/RAG | Supermemory | Persistent semantic memory for patterns, entities, commitments |
| Auth | Supabase Auth (Google provider) | Handles Google OAuth, JWT, RLS integration; single sign-in with all scopes |
| LLM | Gemini (Google DeepMind) | Native Google ecosystem integration; structured JSON output; gemini-3.0-flash for speed, gemini-3.0-pro for complex reasoning |
| Validation | Pydantic v2 | Type safety, serialization, schema generation for structured output |

### Existing Prior Art

The codebase already includes a working Gemini integration:
- `backend/app/gemini_processor.py` — `FridayExtractor` class using `gemini-2.0-flash` via `google-genai` SDK
- `backend/app/main.py` — `/friday-extract` endpoint for structured meeting extraction
- `docs/google-calendar-heartbeat-plan.md` — Detailed plan for calendar sync and heartbeat architecture

**Dual SDK approach:**
- `langchain-google-genai` — for LangGraph agent (tool calling, structured output, streaming)
- `google-genai` — for direct Gemini API calls (meeting extraction, fast processing)
