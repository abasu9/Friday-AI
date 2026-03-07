# 04 — LangGraph State Machine

## State Definition

```python
from typing import Annotated, Literal, Optional
from langgraph.graph import MessagesState
from pydantic import BaseModel, Field
import operator


class FridayState(MessagesState):
    """Extended state for FRIDAY agent.

    Inherits `messages` from MessagesState (with add-only reducer).
    All custom fields use explicit reducers or are overwritten each run.
    """
    # --- Session context (loaded in preprocess) ---
    session_id: str
    user_id: str
    user_context: dict = Field(default_factory=dict)       # From user_context table
    active_tasks: list[dict] = Field(default_factory=list) # Current user tasks
    semantic_context: list[dict] = Field(default_factory=list)

    # --- Intent routing ---
    intent: Optional[Literal["chat", "action", "triage", "proactive"]] = None
    confidence: float = 0.0

    # --- Tool execution ---
    tool_calls_made: Annotated[list[dict], operator.add] = Field(default_factory=list)
    pending_approval: Optional[dict] = None  # Set when action needs human approval
    approval_result: Optional[dict] = None   # Set when user responds to approval

    # --- Response ---
    response_draft: Optional[str] = None
    structured_output: Optional[dict] = None

    # --- Control flow ---
    should_stream: bool = True
    error: Optional[str] = None
```

## Graph Definition

```python
from langgraph.graph import StateGraph, START, END

graph = StateGraph(FridayState)

# --- Add nodes ---
graph.add_node("preprocess", preprocess_node)
graph.add_node("route_intent", route_intent_node)
graph.add_node("execute_tools", execute_tools_node)
graph.add_node("generate_response", generate_response_node)
graph.add_node("human_approval", human_approval_node)
graph.add_node("postprocess", postprocess_node)

# --- Add edges ---
graph.add_edge(START, "preprocess")
graph.add_edge("preprocess", "route_intent")

# Conditional: route based on intent
graph.add_conditional_edges(
    "route_intent",
    route_after_intent,
    {
        "chat": "generate_response",       # Direct LLM response
        "action": "execute_tools",         # Needs tool calls
        "triage": "execute_tools",         # Triage fetches data then responds
        "proactive": "execute_tools",      # Proactive checks tools
    }
)

graph.add_edge("execute_tools", "generate_response")

# Conditional: check if approval needed
graph.add_conditional_edges(
    "generate_response",
    check_approval_needed,
    {
        "needs_approval": "human_approval",
        "no_approval": "postprocess",
    }
)

graph.add_edge("human_approval", "postprocess")
graph.add_edge("postprocess", END)

# --- Compile ---
friday_graph = graph.compile(
    checkpointer=supabase_checkpointer,    # Persist state to Supabase
    interrupt_before=["human_approval"],    # Pause for user approval
)
```

## Visual Graph

```
                    START
                      │
                      ▼
                ┌────────────┐
                │ preprocess │  Load context: Supabase + Supermemory
                └─────┬──────┘
                      │
                      ▼
               ┌──────────────┐
               │ route_intent │  Classify: chat | action | triage | proactive
               └──────┬───────┘
                      │
            ┌─────────┼──────────┐
            │         │          │
     intent=chat  intent=action  intent=triage/proactive
            │         │          │
            │         ▼          │
            │  ┌──────────────┐  │
            │  │execute_tools │◄─┘  Call gws CLI / Supabase tools
            │  └──────┬───────┘
            │         │
            ▼         ▼
        ┌───────────────────┐
        │ generate_response │  LLM generates final response
        └────────┬──────────┘
                 │
        ┌────────┴────────┐
        │                 │
  needs_approval    no_approval
        │                 │
        ▼                 │
  ┌──────────────┐        │
  │human_approval│        │  (interrupt_before — waits for user)
  └──────┬───────┘        │
         │                │
         ▼                ▼
     ┌──────────────┐
     │ postprocess  │  Save state, emit done event
     └──────┬───────┘
            │
            ▼
           END
```

## Node Implementations

### preprocess
```python
async def preprocess_node(state: FridayState) -> dict:
    # Load context from Supabase (operational) + Supermemory (semantic)
    """Load all context needed for this turn."""
    user_context = await load_user_context(state["user_id"])
    active_tasks = await load_active_tasks(state["user_id"])
    semantic_context = await supermemory_search(state["user_id"], state["messages"][-1].content)
    recent_messages = state["messages"][-20:]  # Last 20 messages for context window

    return {
        "user_context": user_context,
        "active_tasks": active_tasks,
        "semantic_context": semantic_context,
        "messages": recent_messages,  # Trim for token budget
    }
```

### route_intent
```python
async def route_intent_node(state: FridayState) -> dict:
    """Classify the user's intent using a fast LLM call."""
    last_message = state["messages"][-1]

    classification = await llm.with_structured_output(IntentClassification).ainvoke([
        SystemMessage(content=INTENT_ROUTER_PROMPT),
        last_message,
    ])

    return {
        "intent": classification.intent,
        "confidence": classification.confidence,
    }
```

### execute_tools
```python
async def execute_tools_node(state: FridayState) -> dict:
    """Execute tools based on intent and LLM tool selection."""
    tools = get_tools_for_intent(state["intent"])

    response = await llm.bind_tools(tools).ainvoke(
        build_tool_messages(state),
    )

    tool_results = []
    for tool_call in response.tool_calls:
        result = await execute_tool(tool_call, state)
        tool_results.append(result)

    return {
        "tool_calls_made": tool_results,
        "messages": [response] + [ToolMessage(...) for r in tool_results],
    }
```

### generate_response
```python
async def generate_response_node(state: FridayState) -> dict:
    """Generate the final response incorporating tool results."""
    system_prompt = build_system_prompt(state)

    response = await llm.ainvoke([
        SystemMessage(content=system_prompt),
        *state["messages"],
    ])

    # Check if response contains an action requiring approval
    pending = extract_pending_approval(response)

    return {
        "response_draft": response.content,
        "pending_approval": pending,
        "messages": [response],
    }
```

### human_approval
```python
async def human_approval_node(state: FridayState) -> dict:
    """Wait for user approval of a pending action.

    This node is interrupted before execution (interrupt_before).
    When resumed, approval_result will be populated.
    """
    approval = state["approval_result"]

    if approval and approval["status"] == "approved":
        # Execute the approved action
        result = await execute_approved_action(state["pending_approval"], approval)
        return {
            "tool_calls_made": [result],
            "pending_approval": None,
            "approval_result": None,
        }
    else:
        return {
            "pending_approval": None,
            "approval_result": None,
            "messages": [AIMessage(content="Got it, I won't do that.")],
        }
```

### postprocess
```python
async def postprocess_node(state: FridayState) -> dict:
    """Save state and clean up."""
    await save_message_to_db(state)
    await update_session_metadata(state)

    # Learn from interaction (update user_context)
    if state["tool_calls_made"]:
        await update_user_context(state)

    # Save learned patterns to Supermemory
    if should_save_memory(state):
        await supermemory_store(state)

    return {}
```

## Routing Functions

```python
def route_after_intent(state: FridayState) -> str:
    """Route based on classified intent."""
    return state["intent"] or "chat"


def check_approval_needed(state: FridayState) -> str:
    """Check if the response requires human approval."""
    if state["pending_approval"]:
        return "needs_approval"
    return "no_approval"
```
