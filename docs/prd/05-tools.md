# 05 — Agent Tools

## Tool Architecture

Tools are organized in three categories:
1. **Composio Google Workspace** — Pre-built LangGraph-compatible tools via Composio
2. **Supermemory Tools** — Semantic memory search and storage
3. **Custom Supabase Tools** — Internal tools for operational state management

> **Why Composio?** Gemini + Google Workspace + Composio share the Google ecosystem. Composio provides pre-built `@tool` wrappers that work with LangGraph out of the box — no manual Google API code needed. Composio handles per-user OAuth via its entity system.

All tools follow a consistent interface:

```python
from langchain_core.tools import tool
from pydantic import BaseModel, Field


class ToolResult(BaseModel):
    """Standard tool result wrapper."""
    success: bool
    data: dict | list | str | None = None
    error: str | None = None
    duration_ms: int | None = None
```

## Composio Google Workspace Tools

These are pre-built tools from Composio's Google Workspace integration. Composio handles per-user Google OAuth and token management via its entity system. User identity is managed by Supabase Auth.

### gmail_read_inbox
```python
@tool
async def gmail_read_inbox(
    max_results: int = Field(default=10, description="Max emails to return (1-50)"),
    query: str = Field(default="", description="Gmail search query (e.g. 'is:unread from:boss')"),
    include_body: bool = Field(default=False, description="Include full email body text"),
) -> list[dict]:
    """Read emails from the user's Gmail inbox.

    Returns a list of emails with: id, subject, from, date, snippet, labels.
    Use `query` for Gmail search syntax (is:unread, from:, subject:, etc).
    Set include_body=True only when you need the full text (costs more tokens).

    IMPORTANT: Default to is:unread unless the user asks for something specific.
    """
```

### gmail_send
```python
@tool
async def gmail_send(
    to: str = Field(description="Recipient email address"),
    subject: str = Field(description="Email subject line"),
    body: str = Field(description="Email body (plain text or HTML)"),
    reply_to_id: str | None = Field(default=None, description="Message ID to reply to (for threading)"),
    cc: str | None = Field(default=None, description="CC recipients, comma-separated"),
) -> dict:
    """Send an email via Gmail on behalf of the user.

    REQUIRES HUMAN APPROVAL. This tool will pause execution and ask the user
    to review the draft before sending. Never send without approval.

    When replying, always set reply_to_id to maintain the email thread.
    """
```

### calendar_list_events
```python
@tool
async def calendar_list_events(
    time_min: str = Field(description="Start time (ISO 8601, e.g. '2026-03-07T00:00:00Z')"),
    time_max: str = Field(description="End time (ISO 8601)"),
    max_results: int = Field(default=10, description="Max events to return"),
) -> list[dict]:
    """List events from the user's Google Calendar.

    Returns: id, summary, start, end, location, attendees, description.
    For 'today's events', use midnight-to-midnight in user's timezone.
    For 'upcoming', use now to +24h.
    """
```

### calendar_create_event
```python
@tool
async def calendar_create_event(
    summary: str = Field(description="Event title"),
    start: str = Field(description="Start time (ISO 8601)"),
    end: str = Field(description="End time (ISO 8601)"),
    description: str | None = Field(default=None, description="Event description"),
    attendees: list[str] | None = Field(default=None, description="Attendee email addresses"),
    location: str | None = Field(default=None, description="Event location"),
) -> dict:
    """Create a new Google Calendar event.

    REQUIRES HUMAN APPROVAL. Shows the user the event details before creating.
    Always confirm timezone with the user if ambiguous.
    """
```

### docs_read
```python
@tool
async def docs_read(
    doc_id: str = Field(description="Google Doc ID (from URL or search)"),
    extract_summary: bool = Field(default=True, description="Return a summary instead of full text"),
) -> dict:
    """Read content from a Google Doc.

    Returns the document title and content (full text or summary).
    Use extract_summary=True to save tokens when you just need the gist.
    """
```

### drive_search
```python
@tool
async def drive_search(
    query: str = Field(description="Search query for Google Drive"),
    file_type: str | None = Field(default=None, description="Filter by type: document, spreadsheet, presentation, pdf"),
    max_results: int = Field(default=5, description="Max files to return"),
) -> list[dict]:
    """Search the user's Google Drive for files.

    Returns: id, name, mimeType, modifiedTime, webViewLink.
    Use this to find documents the user references (e.g. 'my API migration notes').
    """
```

## Custom Supabase Tools

These manage FRIDAY's internal state.

### get_user_tasks
```python
@tool
async def get_user_tasks(
    status: str | None = Field(default=None, description="Filter by status: pending, in_progress, done, dismissed"),
    priority: str | None = Field(default=None, description="Filter by priority: critical, high, medium, low"),
    limit: int = Field(default=10, description="Max tasks to return"),
) -> list[dict]:
    """Get the user's current tasks from FRIDAY's task list.

    Tasks come from multiple sources: extracted from emails, calendar events,
    or manually created by the user. Always check tasks before giving advice
    about what the user should do next.
    """
```

### create_task
```python
@tool
async def create_task(
    title: str = Field(description="Short task title"),
    description: str | None = Field(default=None, description="Detailed description"),
    priority: str = Field(default="medium", description="Priority: critical, high, medium, low"),
    due_at: str | None = Field(default=None, description="Due date (ISO 8601)"),
    source: str = Field(default="agent", description="Source: email, calendar, manual, agent"),
    source_ref: dict | None = Field(default=None, description="Reference to source item"),
) -> dict:
    """Create a new task in the user's FRIDAY task list.

    Use this when you identify an actionable item from emails, calendar events,
    or conversation. Set appropriate priority based on urgency and user context.
    """
```

### update_task
```python
@tool
async def update_task(
    task_id: str = Field(description="Task UUID"),
    status: str | None = Field(default=None, description="New status: pending, in_progress, done, dismissed"),
    priority: str | None = Field(default=None, description="New priority"),
    title: str | None = Field(default=None, description="Updated title"),
) -> dict:
    """Update an existing task's status, priority, or title.

    Use this when the user completes, dismisses, or reprioritizes a task.
    """
```

### get_user_context
```python
@tool
async def get_user_context(
    context_key: str | None = Field(default=None, description="Specific context key to retrieve"),
) -> dict | list[dict]:
    """Retrieve learned context about the user.

    Context includes patterns, preferences, and relationships FRIDAY has
    learned over time. Use this to personalize responses.

    Examples of context_keys:
    - 'email_patterns': Which emails the user prioritizes
    - 'meeting_prep_style': How the user likes to prepare for meetings
    - 'communication_style': Formal vs casual preferences
    """
```

### save_user_context
```python
@tool
async def save_user_context(
    context_key: str = Field(description="Context category key"),
    context_value: dict = Field(description="The learned context data"),
    confidence: float = Field(default=0.5, description="Confidence level 0.0-1.0"),
    source: str = Field(description="What interaction this was learned from"),
) -> dict:
    """Save a new piece of learned context about the user.

    Use this sparingly — only when you observe a clear, repeated pattern.
    Do NOT save one-off preferences. Wait for at least 2-3 confirming signals.
    """
```

## Supermemory Tools

These connect to Supermemory's RAG API for semantic memory operations.

### memory_search
```python
@tool
async def memory_search(
    query: str = Field(description="Natural language search query"),
    memory_types: list[str] | None = Field(default=None, description="Filter by type: pattern, commitment, entity, summary"),
    limit: int = Field(default=5, description="Max memories to return"),
) -> list[dict]:
    """Search the user's semantic memory for relevant context.

    Returns memories ranked by relevance: patterns, commitments, meeting summaries,
    entity relationships, and learned behaviors.

    Use this BEFORE giving advice — check what FRIDAY has learned about the user.
    Prefer this over get_user_context for nuanced, cross-session context.
    """
```

### memory_store
```python
@tool
async def memory_store(
    content: str = Field(description="The memory content to store"),
    memory_type: str = Field(description="Type: pattern, commitment, entity, summary, preference"),
    metadata: dict | None = Field(default=None, description="Additional context (source session, confidence, etc.)"),
) -> dict:
    """Store a new memory in the user's semantic memory.

    Use this to save:
    - Patterns: "User always ignores marketing emails"
    - Commitments: "User promised to send report by Friday"
    - Entities: "Sarah = user's manager, works on API team"
    - Summaries: "Meeting about Q3 planning discussed budget cuts"

    Be selective — only store clear, repeated patterns or explicit commitments.
    """
```

## Tool-to-Intent Mapping

```python
TOOL_INTENT_MAP = {
    "chat": [
        get_user_tasks,
        get_user_context,
        memory_search,
    ],
    "action": [
        gmail_read_inbox,
        gmail_send,
        calendar_list_events,
        calendar_create_event,
        docs_read,
        drive_search,
        get_user_tasks,
        create_task,
        update_task,
        get_user_context,
        save_user_context,
        memory_search,
        memory_store,
    ],
    "triage": [
        gmail_read_inbox,
        calendar_list_events,
        get_user_tasks,
        get_user_context,
        memory_search,
    ],
    "proactive": [
        gmail_read_inbox,
        calendar_list_events,
        get_user_tasks,
        get_user_context,
        create_task,
        memory_search,
        memory_store,
    ],
}


def get_tools_for_intent(intent: str) -> list:
    """Return the tool set available for a given intent."""
    return TOOL_INTENT_MAP.get(intent, TOOL_INTENT_MAP["chat"])
```

## Tools Requiring Approval

```python
APPROVAL_REQUIRED_TOOLS = {
    "gmail_send",
    "calendar_create_event",
}
```

Any tool call to these functions triggers the `human_approval` node in the StateGraph.
