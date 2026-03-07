# 08 — API & Streaming

## FastAPI Application Structure

```
backend/
├── app/
│   ├── main.py              # FastAPI app, CORS, lifespan
│   ├── config.py             # Settings (env vars)
│   ├── dependencies.py       # Auth, DB session deps
│   ├── api/
│   │   ├── chat.py           # POST /chat/stream, POST /chat/approve
│   │   ├── sessions.py       # GET /sessions, GET /sessions/{id}
│   │   ├── tasks.py          # CRUD /tasks
│   │   └── health.py         # GET /health
│   ├── agent/
│   │   ├── graph.py          # LangGraph StateGraph definition
│   │   ├── nodes.py          # Node implementations
│   │   ├── state.py          # FridayState definition
│   │   ├── prompts.py        # All prompt layers
│   │   └── tools/
│   │       ├── google.py     # MCP Google Workspace tools
│   │       ├── supabase.py   # Custom Supabase tools
│   │       └── registry.py   # Tool-intent mapping
│   ├── models/
│   │   ├── schemas.py        # Pydantic request/response models
│   │   ├── events.py         # SSE event models
│   │   └── db.py             # Database models
│   ├── services/
│   │   ├── supabase.py       # Supabase client
│   │   ├── heartbeat.py      # Background heartbeat loop
│   │   └── checkpointer.py   # LangGraph Supabase checkpointer
│   └── core/
│       ├── auth.py           # Supabase Auth JWT validation
│       └── sse.py            # SSE streaming utilities
├── pyproject.toml
└── .env
```

## Endpoints

### `POST /chat/stream` — Main chat endpoint (SSE)

```python
from fastapi import APIRouter, Depends
from fastapi.responses import StreamingResponse
from app.models.schemas import ChatRequest
from app.agent.graph import friday_graph
from app.core.sse import sse_stream

router = APIRouter(prefix="/chat", tags=["chat"])


@router.post("/stream")
async def chat_stream(
    request: ChatRequest,
    user=Depends(get_current_user),
):
    """Stream a chat response via Server-Sent Events.

    SSE event types:
    - status: Progress updates ("Reading emails...", "Checking calendar...")
    - token: Streamed response text chunks
    - tool_result: Tool execution summaries
    - approval_required: Action needs user approval
    - done: Response complete with metadata
    - error: Something went wrong
    """
    session_id = request.session_id or await create_session(user.id)

    async def event_generator():
        async for event in run_graph_with_streaming(
            graph=friday_graph,
            session_id=session_id,
            user_id=user.id,
            message=request.message,
        ):
            yield format_sse_event(event)

    return StreamingResponse(
        event_generator(),
        media_type="text/event-stream",
        headers={
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
            "X-Accel-Buffering": "no",  # Disable nginx buffering
        },
    )
```

### `POST /chat/approve` — Handle approval responses

```python
@router.post("/approve")
async def chat_approve(
    request: ApproveRequest,
    user=Depends(get_current_user),
):
    """Resume the graph after user approves/rejects an action.

    Returns a new SSE stream with the result of the approved action.
    """
    # Update approval record in DB
    await update_approval(request.approval_id, request.response)

    # Resume the graph from the interrupt point
    async def event_generator():
        async for event in resume_graph_with_streaming(
            graph=friday_graph,
            session_id=request.session_id,
            approval_result=request.response.model_dump(),
        ):
            yield format_sse_event(event)

    return StreamingResponse(
        event_generator(),
        media_type="text/event-stream",
    )
```

### `GET /sessions` — List user sessions

```python
sessions_router = APIRouter(prefix="/sessions", tags=["sessions"])


@sessions_router.get("/")
async def list_sessions(
    user=Depends(get_current_user),
    limit: int = 20,
    offset: int = 0,
) -> list[SessionResponse]:
    """List user's chat sessions, most recent first."""
    return await get_user_sessions(user.id, limit, offset)


@sessions_router.get("/{session_id}")
async def get_session(
    session_id: str,
    user=Depends(get_current_user),
) -> SessionResponse:
    """Get a specific session with message history."""
    return await get_session_with_messages(session_id, user.id)
```

### `CRUD /tasks` — Task management

```python
tasks_router = APIRouter(prefix="/tasks", tags=["tasks"])


@tasks_router.get("/")
async def list_tasks(
    user=Depends(get_current_user),
    status: str | None = None,
    priority: str | None = None,
) -> list[TaskResponse]:
    """List user's tasks with optional filters."""


@tasks_router.post("/")
async def create_task_endpoint(
    task: CreateTaskRequest,
    user=Depends(get_current_user),
) -> TaskResponse:
    """Manually create a task."""


@tasks_router.patch("/{task_id}")
async def update_task_endpoint(
    task_id: str,
    update: UpdateTaskRequest,
    user=Depends(get_current_user),
) -> TaskResponse:
    """Update a task's status, priority, or details."""
```

### `GET /health` — Health check

```python
@router.get("/health")
async def health():
    return {"status": "ok", "service": "friday-backend"}
```

## SSE Streaming Utility

```python
import json
from typing import AsyncGenerator
from app.models.events import SSEEvent


def format_sse_event(event: SSEEvent) -> str:
    """Format a Pydantic model as an SSE event string."""
    data = json.dumps(event.model_dump())
    return f"event: {event.type}\ndata: {data}\n\n"


async def run_graph_with_streaming(
    graph, session_id: str, user_id: str, message: str
) -> AsyncGenerator[SSEEvent, None]:
    """Run the LangGraph and yield SSE events."""

    # Emit status
    yield StatusEvent(message="Processing your message...")

    config = {
        "configurable": {
            "thread_id": session_id,
            "user_id": user_id,
        }
    }

    input_state = {
        "messages": [HumanMessage(content=message)],
        "session_id": session_id,
        "user_id": user_id,
    }

    async for event in graph.astream_events(input_state, config, version="v2"):
        kind = event["event"]

        if kind == "on_chat_model_stream":
            chunk = event["data"]["chunk"]
            if chunk.content:
                yield TokenEvent(content=chunk.content)

        elif kind == "on_tool_start":
            tool_name = event["name"]
            yield StatusEvent(message=f"Using {tool_name}...")

        elif kind == "on_tool_end":
            yield ToolResultEvent(
                tool_name=event["name"],
                success=True,
                summary=summarize_tool_output(event["data"]),
                duration_ms=event.get("duration_ms", 0),
            )

        elif kind == "on_chain_end" and event["name"] == "human_approval":
            state = event["data"]["output"]
            if state.get("pending_approval"):
                yield ApprovalRequiredEvent(
                    approval_id=state["pending_approval"]["id"],
                    request=ApprovalRequest(**state["pending_approval"]),
                )

    # Final done event
    yield DoneEvent(
        session_id=session_id,
        message_id="...",
        intent="...",
        tool_calls_count=0,
        total_duration_ms=0,
    )
```

## Heartbeat Loop

```python
import asyncio
from contextlib import asynccontextmanager


@asynccontextmanager
async def lifespan(app):
    """Start/stop the heartbeat loop with the app."""
    task = asyncio.create_task(heartbeat_loop())
    yield
    task.cancel()


async def heartbeat_loop():
    """Background loop that checks for proactive nudge opportunities."""
    while True:
        try:
            # Get all users with active heartbeat configs
            users = await get_active_heartbeat_users()

            for user in users:
                config = user["heartbeat_state"]["config"]
                interval = config.get("check_interval_seconds", 60)

                # Check if it's time to poll
                last_check = user["heartbeat_state"]["last_calendar_check"]
                if should_check(last_check, interval):
                    nudge = await run_proactive_check(user)
                    if nudge and nudge.should_nudge:
                        await deliver_nudge(user["id"], nudge)
                        await update_heartbeat_state(user["id"])

        except Exception as e:
            logger.error(f"Heartbeat error: {e}")

        await asyncio.sleep(10)  # Poll every 10s, per-user interval is separate


async def run_proactive_check(user: dict) -> ProactiveNudge | None:
    """Run the proactive check for a single user."""
    result = await friday_graph.ainvoke(
        {
            "messages": [SystemMessage(content="Run proactive check")],
            "session_id": f"heartbeat-{user['id']}",
            "user_id": user["id"],
            "intent": "proactive",
        },
        config={"configurable": {"thread_id": f"heartbeat-{user['id']}"}},
    )
    return result.get("structured_output")
```

## CORS Configuration

```python
from fastapi.middleware.cors import CORSMiddleware

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],         # Desktop app — restrict in production
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
```

## Authentication (Supabase Auth)

Authentication is handled via **Supabase Auth with Google as the OAuth provider**.

### Client-Side (Desktop App)
```typescript
// Frontend initiates Google sign-in via Supabase JS SDK
const { data, error } = await supabase.auth.signInWithOAuth({
  provider: 'google',
  options: {
    scopes: 'https://www.googleapis.com/auth/gmail.modify https://www.googleapis.com/auth/calendar https://www.googleapis.com/auth/documents.readonly https://www.googleapis.com/auth/drive.readonly',
    redirectTo: 'http://localhost:3118/auth/callback',
  },
});
```

### Server-Side (FastAPI)
```python
from supabase import create_client

async def get_current_user(authorization: str = Header(...)) -> dict:
    """Validate Supabase Auth JWT and return user profile."""
    token = authorization.replace("Bearer ", "")
    supabase = create_client(SUPABASE_URL, SUPABASE_ANON_KEY)

    # Verify JWT and get user
    user_response = supabase.auth.get_user(token)
    user = user_response.user

    # Get Google provider token for Composio/Google API calls
    session = supabase.auth.get_session()
    google_token = session.provider_token if session else None

    return {
        "id": user.id,
        "email": user.email,
        "google_token": google_token,
    }
```

### `GET /auth/session` — Current user session

```python
@router.get("/auth/session")
async def get_auth_session(user=Depends(get_current_user)):
    """Return current user session and Google token availability."""
    return {
        "user_id": user["id"],
        "email": user["email"],
        "google_connected": user["google_token"] is not None,
    }
```

> **Note**: Google OAuth client ID/secret are configured in the Supabase Dashboard, not in backend env vars. The backend only needs `SUPABASE_URL` and `SUPABASE_ANON_KEY` to validate JWTs.
