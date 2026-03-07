# 05 — Agent Tools

## Tool Architecture

Tools are organized in four categories:
1. **gws CLI** — Google Workspace CLI with dynamic API discovery and 100+ agent skills
2. **Exa** — Neural web search for real-time information beyond Google Workspace
3. **Supermemory Tools** — Semantic memory search and storage
4. **Custom Supabase Tools** — Internal tools for operational state management

> **Why `gws` CLI?** The [Google Workspace CLI](https://github.com/googleworkspace/cli) dynamically discovers ALL Google Workspace APIs at runtime via Google's Discovery Service. It ships with 100+ agent skills (helpers, workflows, recipes, personas), outputs structured JSON, and supports `--dry-run` for safe previewing. One subprocess wrapper gives the LangGraph agent access to Gmail, Calendar, Docs, Drive, Sheets, Chat, Meet, Tasks, Keep, Forms, Slides, Admin, and more — with zero custom API code.

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

## gws CLI Tools

These tools wrap the Google Workspace CLI (`gws`) binary. The user authenticates once via `gws auth login`, and the agent calls any Google Workspace API through subprocess.

### gws (primary tool)
```python
@tool
async def gws(
    command: str = Field(description="The gws command to execute (e.g. 'gmail +triage', 'calendar +agenda', 'drive files list --params {\"pageSize\": 5}')"),
    dry_run: bool = Field(default=False, description="Preview the API call without executing it"),
) -> str:
    """Execute any Google Workspace CLI command.

    The gws CLI provides dynamic access to ALL Google Workspace APIs.
    It includes 100+ agent skills with helper commands (+send, +triage, +agenda, etc.)
    and supports raw API calls to any Google Workspace service.

    Helper commands (most common):
    - 'gmail +triage' — unread inbox summary (sender, subject, date)
    - 'gmail +send --to EMAIL --subject SUBJ --body TEXT' — send email
    - 'calendar +agenda' — upcoming events across all calendars
    - 'calendar +insert' — create a new event
    - 'workflow +meeting-prep' — prep for next meeting (agenda, attendees, docs)
    - 'workflow +standup-report' — today's meetings + open tasks
    - 'workflow +email-to-task' — convert email to Google Task
    - 'workflow +weekly-digest' — weekly summary
    - 'docs +write' — append text to a Google Doc
    - 'drive +upload' — upload a file
    - 'sheets +read' — read spreadsheet values
    - 'sheets +append' — append a row

    Raw API calls (for anything not covered by helpers):
    - 'gmail users messages list --params {"userId": "me", "q": "is:unread"}'
    - 'calendar events list --params {"calendarId": "primary", "timeMin": "..."}'
    - 'drive files list --params {"q": "name contains ...", "pageSize": 10}'
    - 'docs documents get --params {"documentId": "DOC_ID"}'

    Use 'schema <service>.<method>' to inspect any API method's parameters.

    IMPORTANT: Use --dry-run for any write/delete operations to preview first.
    IMPORTANT: For email sending, ALWAYS show the draft to the user before executing.
    """
```

### gws_schema (discovery tool)
```python
@tool
async def gws_schema(
    method: str = Field(description="API method to inspect (e.g. 'gmail.users.messages.list', 'calendar.events.insert')"),
) -> str:
    """Inspect any Google Workspace API method's parameters and body schema.

    Use this BEFORE calling unfamiliar API methods to understand:
    - Required vs optional parameters
    - Parameter types and formats
    - Request body structure

    Examples:
    - 'gmail.users.messages.list' — see query params for listing emails
    - 'calendar.events.insert' — see event creation body structure
    - 'drive.files.create' — see file upload parameters
    """
```

### Available gws Skills (built-in)

The gws CLI ships with these pre-built capabilities:

**Core Services** (18):
Gmail, Calendar, Drive, Docs, Sheets, Slides, Chat, Meet, Tasks, Keep, Forms, Classroom, People, Admin Reports, Events, Model Armor, Workflow, Shared

**Helper Commands** (16+):
| Command | Description |
|---------|-------------|
| `gmail +send` | Send an email |
| `gmail +triage` | Unread inbox summary |
| `gmail +watch` | Watch for new emails (NDJSON stream) |
| `calendar +agenda` | Upcoming events |
| `calendar +insert` | Create event |
| `docs +write` | Append text to a doc |
| `drive +upload` | Upload a file |
| `sheets +read` | Read spreadsheet values |
| `sheets +append` | Append row |
| `chat +send` | Send Chat message |
| `workflow +meeting-prep` | Prep for next meeting |
| `workflow +standup-report` | Today's standup summary |
| `workflow +email-to-task` | Email → Google Task |
| `workflow +weekly-digest` | Weekly summary |
| `workflow +file-announce` | Announce Drive file in Chat |

**Workflow Recipes** (50+):
Multi-step task sequences like `recipe-block-focus-time`, `recipe-find-free-time`, `recipe-save-email-attachments`, `recipe-create-vacation-responder`, etc.

**Personas** (10):
Role-based skill bundles including `persona-exec-assistant`, `persona-project-manager`, `persona-team-lead`, `persona-researcher`.

## Exa Web Search

Exa is a neural search engine designed for AI agents. The `langchain-exa` integration provides LangGraph-compatible tools out of the box.

### exa_search
```python
from langchain_exa import ExaSearchRetriever

@tool
async def exa_search(
    query: str = Field(description="Natural language search query"),
    num_results: int = Field(default=5, description="Number of results to return"),
    search_type: str = Field(default="auto", description="Search type: 'auto', 'neural', or 'keyword'"),
    use_autoprompt: bool = Field(default=True, description="Let Exa optimize the query for better results"),
) -> list[dict]:
    """Search the web for real-time information using Exa's neural search.

    Use this when the user's question requires information BEYOND their
    Google Workspace — news, research, documentation, public information,
    company lookups, industry trends, etc.

    Examples:
    - "What's the latest on the React 19 release?" → web search
    - "Find me info about Acme Corp before my meeting" → company research
    - "What are best practices for OKR planning?" → knowledge search

    Do NOT use this for:
    - User's emails, calendar, or docs → use gws tools instead
    - User's tasks or preferences → use Supabase/Supermemory tools instead
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
        exa_search,
    ],
    "action": [
        gws,
        gws_schema,
        exa_search,
        get_user_tasks,
        create_task,
        update_task,
        get_user_context,
        save_user_context,
        memory_search,
        memory_store,
    ],
    "triage": [
        gws,
        get_user_tasks,
        get_user_context,
        memory_search,
    ],
    "proactive": [
        gws,
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
# These gws commands require human approval before execution
APPROVAL_REQUIRED_PATTERNS = [
    "gmail +send",
    "gmail users messages send",
    "calendar +insert",
    "calendar events insert",
    "calendar events create",
    "chat +send",
]
```

Any gws command matching these patterns triggers the `human_approval` node in the StateGraph. The agent uses `--dry-run` to preview the action first, then waits for user approval before executing.
