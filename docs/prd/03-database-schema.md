# 03 — Database Schema (Supabase Postgres)

## Overview

Single Supabase project handles:
- Session & message storage
- LangGraph checkpointing
- User context & preferences
- Task tracking
- Heartbeat state
- Human-in-the-loop approvals
- Supermemory integration

## Types & Enums

```sql
-- Intent categories for routing
CREATE TYPE intent_type AS ENUM (
  'chat',           -- General conversation
  'action',         -- Requires tool use (email, calendar, etc.)
  'triage',         -- Overwhelm mode — prioritize and simplify
  'proactive'       -- System-initiated nudge
);

-- Message roles
CREATE TYPE message_role AS ENUM ('user', 'assistant', 'system', 'tool');

-- Approval states
CREATE TYPE approval_status AS ENUM ('pending', 'approved', 'rejected', 'expired');

-- Task priority levels
CREATE TYPE task_priority AS ENUM ('critical', 'high', 'medium', 'low');

-- Task status
CREATE TYPE task_status AS ENUM ('pending', 'in_progress', 'done', 'dismissed');

-- Tool call status
CREATE TYPE tool_call_status AS ENUM ('running', 'completed', 'error');
```

## Tables

### `profiles`
```sql
CREATE TABLE profiles (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  email TEXT,
  display_name TEXT,
  preferences JSONB DEFAULT '{}',    -- UI prefs, notification settings
  timezone TEXT DEFAULT 'UTC',
  created_at TIMESTAMPTZ DEFAULT now(),
  updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_profiles_email ON profiles(email);
```

> **Note**: Google OAuth is handled by `gws auth` directly (not Supabase Auth). The `profiles` table stores user preferences for FRIDAY's agent, not auth state.

### `sessions`
```sql
CREATE TABLE sessions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID REFERENCES profiles(id) ON DELETE CASCADE,
  title TEXT,                         -- Auto-generated from first message
  is_active BOOLEAN DEFAULT true,
  metadata JSONB DEFAULT '{}',        -- Session-level context
  created_at TIMESTAMPTZ DEFAULT now(),
  updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_sessions_user_active ON sessions(user_id, is_active);
CREATE INDEX idx_sessions_updated ON sessions(updated_at DESC);
```

### `messages`
```sql
CREATE TABLE messages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  role message_role NOT NULL,
  content TEXT NOT NULL,
  intent intent_type,                 -- Classified intent (null for non-user msgs)
  token_count INTEGER,                -- For context window management
  metadata JSONB DEFAULT '{}',        -- Extra data (model, latency, etc.)
  created_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_messages_session ON messages(session_id, created_at);
CREATE INDEX idx_messages_intent ON messages(intent) WHERE intent IS NOT NULL;
```

### `tool_calls`
```sql
CREATE TABLE tool_calls (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  tool_name TEXT NOT NULL,            -- e.g. 'gmail_read', 'calendar_list'
  input JSONB NOT NULL,               -- Tool input parameters
  output JSONB,                       -- Tool response
  status tool_call_status DEFAULT 'running',
  error TEXT,                         -- Error message if failed
  duration_ms INTEGER,                -- Execution time
  created_at TIMESTAMPTZ DEFAULT now(),
  completed_at TIMESTAMPTZ
);

CREATE INDEX idx_tool_calls_message ON tool_calls(message_id);
CREATE INDEX idx_tool_calls_session ON tool_calls(session_id, created_at);
```

### `approvals`
```sql
CREATE TABLE approvals (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  tool_call_id UUID REFERENCES tool_calls(id),
  action_type TEXT NOT NULL,          -- 'send_email', 'create_event', etc.
  action_payload JSONB NOT NULL,      -- Full action details for user review
  status approval_status DEFAULT 'pending',
  user_response JSONB,                -- Edit payload if user modified
  expires_at TIMESTAMPTZ,             -- Auto-expire after N minutes
  created_at TIMESTAMPTZ DEFAULT now(),
  resolved_at TIMESTAMPTZ
);

CREATE INDEX idx_approvals_session_pending ON approvals(session_id, status)
  WHERE status = 'pending';
```

### `tasks`
```sql
CREATE TABLE tasks (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID REFERENCES profiles(id) ON DELETE CASCADE,
  session_id UUID REFERENCES sessions(id),   -- Which session created it
  title TEXT NOT NULL,
  description TEXT,
  priority task_priority DEFAULT 'medium',
  status task_status DEFAULT 'pending',
  due_at TIMESTAMPTZ,
  source TEXT,                        -- 'email', 'calendar', 'manual', 'agent'
  source_ref JSONB,                   -- Reference to source (email_id, event_id, etc.)
  metadata JSONB DEFAULT '{}',
  created_at TIMESTAMPTZ DEFAULT now(),
  updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_tasks_user_status ON tasks(user_id, status);
CREATE INDEX idx_tasks_user_priority ON tasks(user_id, priority, due_at);
```

### `user_context`
```sql
-- Stores agent's learned context about the user (patterns, preferences, relationships)
CREATE TABLE user_context (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID REFERENCES profiles(id) ON DELETE CASCADE,
  context_key TEXT NOT NULL,          -- e.g. 'email_patterns', 'meeting_prep_style'
  context_value JSONB NOT NULL,
  confidence FLOAT DEFAULT 0.5,       -- How confident the agent is in this context
  source TEXT,                        -- What interaction produced this
  created_at TIMESTAMPTZ DEFAULT now(),
  updated_at TIMESTAMPTZ DEFAULT now(),
  UNIQUE(user_id, context_key)
);

CREATE INDEX idx_user_context_user ON user_context(user_id);
```

### `checkpoints` (LangGraph state persistence)
```sql
CREATE TABLE checkpoints (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  thread_id TEXT NOT NULL,            -- LangGraph thread identifier
  checkpoint_ns TEXT DEFAULT '',      -- Namespace for subgraphs
  checkpoint_data JSONB NOT NULL,     -- Serialized LangGraph state
  metadata JSONB DEFAULT '{}',
  parent_checkpoint_id UUID REFERENCES checkpoints(id),
  created_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_checkpoints_thread ON checkpoints(thread_id, created_at DESC);
CREATE INDEX idx_checkpoints_session ON checkpoints(session_id);
```

### `heartbeat_state`
```sql
CREATE TABLE heartbeat_state (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID REFERENCES profiles(id) ON DELETE CASCADE,
  last_calendar_check TIMESTAMPTZ,
  last_email_check TIMESTAMPTZ,
  last_task_check TIMESTAMPTZ,
  pending_nudges JSONB DEFAULT '[]',  -- Queued proactive messages
  config JSONB DEFAULT '{
    "check_interval_seconds": 60,
    "calendar_lookahead_minutes": 30,
    "email_priority_threshold": "high"
  }',
  created_at TIMESTAMPTZ DEFAULT now(),
  updated_at TIMESTAMPTZ DEFAULT now(),
  UNIQUE(user_id)
);
```

## Supermemory Integration

Semantic memory lives in **Supermemory** (external RAG service), not in Supabase. The division:

| Data | Storage | Why |
|------|---------|-----|
| Sessions, messages, tasks, approvals | Supabase | Operational CRUD, fast lookups, RLS |
| LangGraph checkpoints | Supabase | State persistence for graph resumption |
| Heartbeat state | Supabase | Background loop coordination |
| User preferences (timezone, UI) | Supabase (`profiles`) | Simple key-value, fast access |
| Session summaries, meeting patterns | Supermemory | Semantic search across conversations |
| Learned communication styles | Supermemory | Pattern matching, similarity search |
| Commitments & entity memory | Supermemory | Cross-session context retrieval |

The `user_context` table remains for fast operational lookups. Supermemory handles deeper semantic memory that benefits from vector similarity search.

## Row-Level Security (RLS)

```sql
-- RLS is optional for single-user hackathon demo
-- Enable if deploying for multiple users

-- ALTER TABLE profiles ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE sessions ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE messages ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE tool_calls ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE approvals ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE user_context ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE checkpoints ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE heartbeat_state ENABLE ROW LEVEL SECURITY;

-- Backend uses Supabase service key for all operations.
-- For multi-user: add RLS policies based on user_id column.
```
