# 09 — Implementation Plan

## Build Order

Each phase builds on the previous. Verify before moving on.

### Phase 1: Foundation (Hours 1-2)
> Get the backend skeleton running with a basic echo.

- [ ] Initialize project: `uv init`, FastAPI, pyproject.toml
- [ ] Set up project structure (see 08-api.md for layout)
- [ ] Configure Supabase client (connection, env vars)
- [ ] Install and authenticate `gws` CLI (`npm install -g @googleworkspace/cli && gws auth setup && gws auth login`)
- [ ] Create `/health` endpoint
- [ ] Create basic SSE streaming endpoint (`/chat/stream` that echoes)
- [ ] Verify: `curl` the SSE endpoint, see events streaming

**Key files:**
```
backend/
├── app/main.py
├── app/config.py
├── app/api/health.py
├── app/api/chat.py (echo version)
├── app/core/sse.py
├── app/services/supabase.py
└── pyproject.toml
```

### Phase 2: Database (Hour 2-3)
> Schema deployed, CRUD working.

- [ ] Run SQL from 03-database-schema.md against Supabase
- [ ] Create database models/helpers in `app/models/db.py`
- [ ] Implement session CRUD (`app/api/sessions.py`)
- [ ] Implement task CRUD (`app/api/tasks.py`)
- [ ] Implement message storage
- [ ] Verify: Create session → add message → retrieve it

### Phase 3: LangGraph Core (Hours 3-5)
> The state machine works end-to-end with a mock LLM.

- [ ] Define `FridayState` in `app/agent/state.py`
- [ ] Create graph skeleton in `app/agent/graph.py`
- [ ] Implement `preprocess` node (load context from Supabase)
- [ ] Implement `route_intent` node (LLM call with structured output)
- [ ] Implement `generate_response` node (basic LLM response)
- [ ] Implement `postprocess` node (save to DB)
- [ ] Wire up Supabase checkpointer
- [ ] Verify: Send message → get classified intent → get response → see in DB

### Phase 4: Tools — Google Workspace (gws CLI) (Hours 5-7)
> Agent can access ALL Google Workspace APIs via gws CLI.

- [ ] Implement `gws` tool wrapper (subprocess runner in `app/core/gws_runner.py`)
- [ ] Implement `gws_schema` tool wrapper for API discovery
- [ ] Implement `execute_tools` node in the graph
- [ ] Wire tools to intent routing (action/triage intents trigger tools)
- [ ] Test helper commands: `gws gmail +triage`, `gws calendar +agenda`, `gws workflow +meeting-prep`
- [ ] Test raw API calls: `gws drive files list`, `gws docs documents get`
- [ ] Implement approval detection for write commands (`+send`, `+insert`, etc.)
- [ ] Verify: "What's on my calendar today?" → real calendar data returned

### Phase 5: Tools — Supabase + Task Management (Hour 7-8)
> Agent can manage tasks and learn context.

- [ ] Implement `get_user_tasks` tool
- [ ] Implement `create_task` / `update_task` tools
- [ ] Implement `get_user_context` / `save_user_context` tools
- [ ] Set up Supermemory client and memory_search / memory_store tools
- [ ] Set up Exa client and `exa_search` tool (`langchain-exa`)
- [ ] Verify: "Create a task to review PR" → task appears in DB
- [ ] Verify: "What's the latest on React 19?" → web search results returned

### Phase 6: Prompt Engineering (Hour 8-9)
> Full layered prompts produce good responses.

- [ ] Write all prompt layers in `app/agent/prompts.py`
- [ ] Implement `build_system_prompt()` compositor
- [ ] Implement intent-specific prompt injection
- [ ] Implement progressive context injection for multi-turn
- [ ] Test triage mode: "I'm overwhelmed" → 3 priorities
- [ ] Verify: Responses are concise, empathetic, ADHD-appropriate

### Phase 7: Human-in-the-Loop (Hours 9-10)
> Email sending requires approval and works end-to-end.

- [ ] Implement approval detection for `gws gmail +send` and `gws calendar +insert`
- [ ] Implement `human_approval` node with `interrupt_before`
- [ ] Implement `POST /chat/approve` endpoint
- [ ] Implement approval record storage in Supabase
- [ ] Wire SSE `approval_required` event
- [ ] Verify: "Reply to Sarah's email" → draft shown → approve → email sent

### Phase 8: Heartbeat & Proactive (Hours 10-11)
> Background loop generates proactive nudges.

- [ ] Implement heartbeat loop in `app/services/heartbeat.py`
- [ ] Wire into FastAPI lifespan
- [ ] Implement `run_proactive_check` (calendar + email scan)
- [ ] Implement nudge delivery mechanism
- [ ] Implement `ProactiveNudge` structured output
- [ ] Verify: Set a calendar event 15 min from now → nudge fires

### Phase 9: SSE Streaming Polish (Hour 11-12)
> Full streaming experience with all event types.

- [ ] Wire `astream_events` for token streaming
- [ ] Emit `status` events for each tool call
- [ ] Emit `tool_result` events with summaries
- [ ] Emit proper `done` events with metadata
- [ ] Handle errors gracefully with `error` events
- [ ] Verify: Watch SSE stream — see status → tokens → tool_results → done

### Phase 10: Demo Polish (Hours 12+)
> Everything works for the 3-minute demo.

- [ ] Test full demo flow (see 01-vision.md)
- [ ] Tune response latency (target P50 < 3s)
- [ ] Handle edge cases (empty inbox, no calendar, connection errors)
- [ ] Add graceful error messages
- [ ] Test with real Google account
- [ ] Practice the demo

## Verification Checklist

Before calling it done:

| Check | How |
|-------|-----|
| Health endpoint | `curl localhost:8000/health` → `{"status": "ok"}` |
| SSE streaming | `curl -N localhost:8000/chat/stream` → events flow |
| Intent routing | Send "what's my day like?" → intent=action |
| Email reading | Send "check my email" → `gws gmail +triage` returns real data |
| Calendar reading | Send "what meetings today?" → `gws calendar +agenda` returns real events |
| Triage mode | Send "I'm overwhelmed" → exactly 3 priorities |
| Email draft + approval | Send "reply to X" → `gws gmail +send --dry-run` → approve → sent |
| Proactive nudge | Upcoming event → nudge fires |
| State persistence | Restart server → conversation history preserved |
| Error handling | `gws` not authenticated → graceful error message |

## Environment Variables

```bash
# .env
SUPABASE_URL=https://xxx.supabase.co
SUPABASE_SERVICE_KEY=eyJ...         # Service role key (bypasses RLS)

GEMINI_API_KEY=your-gemini-key       # Google Gemini API

SUPERMEMORY_API_KEY=your-key         # Supermemory semantic memory

EXA_API_KEY=your-exa-key             # Exa neural web search

# Google OAuth is handled by gws CLI (gws auth login)
# No Google API keys needed in env vars — gws manages credentials in ~/.config/gws/

# Optional
LOG_LEVEL=INFO
HEARTBEAT_ENABLED=true
HEARTBEAT_INTERVAL_SECONDS=60
```

## Dependencies (pyproject.toml)

```toml
[project]
name = "friday-backend"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = [
    "fastapi>=0.115",
    "uvicorn[standard]>=0.30",
    "langgraph>=0.2",
    "langchain-google-genai>=2.0",
    "google-genai>=1.0.0",
    "langchain-core>=0.3",
    "supabase>=2.0",
    "pydantic>=2.0",
    "pydantic-settings>=2.0",
    "httpx>=0.27",
    "python-dotenv>=1.0",
    "langchain-exa>=0.2",
    "structlog>=24.0",
]
```
