-- Friday AI Assistant - Extended Schema
-- Additional tables for AI assistant functionality

-- =============================================================================
-- TYPES & ENUMS
-- =============================================================================

-- Intent categories for routing
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'intent_type') THEN
        CREATE TYPE intent_type AS ENUM (
            'chat',           -- General conversation
            'action',         -- Requires tool use (email, calendar, etc.)
            'triage',         -- Overwhelm mode — prioritize and simplify
            'proactive'       -- System-initiated nudge
        );
    END IF;
END$$;

-- Message roles
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'message_role') THEN
        CREATE TYPE message_role AS ENUM ('user', 'assistant', 'system', 'tool');
    END IF;
END$$;

-- Approval states
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'approval_status') THEN
        CREATE TYPE approval_status AS ENUM ('pending', 'approved', 'rejected', 'expired');
    END IF;
END$$;

-- Task priority levels
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'task_priority') THEN
        CREATE TYPE task_priority AS ENUM ('critical', 'high', 'medium', 'low');
    END IF;
END$$;

-- Task status
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'task_status') THEN
        CREATE TYPE task_status AS ENUM ('pending', 'in_progress', 'done', 'dismissed');
    END IF;
END$$;

-- Tool call status
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'tool_call_status') THEN
        CREATE TYPE tool_call_status AS ENUM ('running', 'completed', 'error');
    END IF;
END$$;

-- =============================================================================
-- USERS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    display_name TEXT,
    google_refresh_token TEXT,         -- Encrypted; for MCP Google Workspace
    preferences JSONB DEFAULT '{}',    -- UI prefs, notification settings
    timezone TEXT DEFAULT 'UTC',
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- =============================================================================
-- SESSIONS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT,                         -- Auto-generated from first message
    is_active BOOLEAN DEFAULT true,
    metadata JSONB DEFAULT '{}',        -- Session-level context
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_active ON sessions(user_id, is_active);
CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);

-- =============================================================================
-- MESSAGES TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role message_role NOT NULL,
    content TEXT NOT NULL,
    intent intent_type,                 -- Classified intent (null for non-user msgs)
    token_count INTEGER,                -- For context window management
    metadata JSONB DEFAULT '{}',        -- Extra data (model, latency, etc.)
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_messages_intent ON messages(intent) WHERE intent IS NOT NULL;

-- =============================================================================
-- TOOL CALLS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS tool_calls (
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

CREATE INDEX IF NOT EXISTS idx_tool_calls_message ON tool_calls(message_id);
CREATE INDEX IF NOT EXISTS idx_tool_calls_session ON tool_calls(session_id, created_at);

-- =============================================================================
-- APPROVALS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS approvals (
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

CREATE INDEX IF NOT EXISTS idx_approvals_session_pending ON approvals(session_id, status)
    WHERE status = 'pending';

-- =============================================================================
-- TASKS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
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

CREATE INDEX IF NOT EXISTS idx_tasks_user_status ON tasks(user_id, status);
CREATE INDEX IF NOT EXISTS idx_tasks_user_priority ON tasks(user_id, priority, due_at);

-- =============================================================================
-- USER CONTEXT TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS user_context (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    context_key TEXT NOT NULL,          -- e.g. 'email_patterns', 'meeting_prep_style'
    context_value JSONB NOT NULL,
    confidence FLOAT DEFAULT 0.5,       -- How confident the agent is in this context
    source TEXT,                        -- What interaction produced this
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE(user_id, context_key)
);

CREATE INDEX IF NOT EXISTS idx_user_context_user ON user_context(user_id);

-- =============================================================================
-- CHECKPOINTS TABLE (LangGraph state persistence)
-- =============================================================================
CREATE TABLE IF NOT EXISTS checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    thread_id TEXT NOT NULL,            -- LangGraph thread identifier
    checkpoint_ns TEXT DEFAULT '',      -- Namespace for subgraphs
    checkpoint_data JSONB NOT NULL,     -- Serialized LangGraph state
    metadata JSONB DEFAULT '{}',
    parent_checkpoint_id UUID REFERENCES checkpoints(id),
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_checkpoints_thread ON checkpoints(thread_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_checkpoints_session ON checkpoints(session_id);

-- =============================================================================
-- HEARTBEAT STATE TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS heartbeat_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
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

-- =============================================================================
-- AUTO-UPDATE TRIGGERS
-- =============================================================================

-- Apply updated_at triggers to new tables
DROP TRIGGER IF EXISTS update_users_updated_at ON users;
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_sessions_updated_at ON sessions;
CREATE TRIGGER update_sessions_updated_at
    BEFORE UPDATE ON sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_tasks_updated_at ON tasks;
CREATE TRIGGER update_tasks_updated_at
    BEFORE UPDATE ON tasks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_user_context_updated_at ON user_context;
CREATE TRIGGER update_user_context_updated_at
    BEFORE UPDATE ON user_context
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_heartbeat_state_updated_at ON heartbeat_state;
CREATE TRIGGER update_heartbeat_state_updated_at
    BEFORE UPDATE ON heartbeat_state
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- =============================================================================
-- ROW LEVEL SECURITY (RLS)
-- =============================================================================

-- Enable RLS on all new tables
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE tool_calls ENABLE ROW LEVEL SECURITY;
ALTER TABLE approvals ENABLE ROW LEVEL SECURITY;
ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_context ENABLE ROW LEVEL SECURITY;
ALTER TABLE checkpoints ENABLE ROW LEVEL SECURITY;
ALTER TABLE heartbeat_state ENABLE ROW LEVEL SECURITY;

-- Policies: users can only access their own data
-- Note: Backend uses service key for agent operations

-- Users table policy
DROP POLICY IF EXISTS "Users own data" ON users;
CREATE POLICY "Users own data" ON users
    FOR ALL USING (auth.uid() = id);

-- Sessions policy
DROP POLICY IF EXISTS "Users own sessions" ON sessions;
CREATE POLICY "Users own sessions" ON sessions
    FOR ALL USING (user_id = auth.uid());

-- Messages policy
DROP POLICY IF EXISTS "Users own messages" ON messages;
CREATE POLICY "Users own messages" ON messages
    FOR ALL USING (
        session_id IN (SELECT id FROM sessions WHERE user_id = auth.uid())
    );

-- Tool calls policy
DROP POLICY IF EXISTS "Users own tool_calls" ON tool_calls;
CREATE POLICY "Users own tool_calls" ON tool_calls
    FOR ALL USING (
        session_id IN (SELECT id FROM sessions WHERE user_id = auth.uid())
    );

-- Approvals policy
DROP POLICY IF EXISTS "Users own approvals" ON approvals;
CREATE POLICY "Users own approvals" ON approvals
    FOR ALL USING (
        session_id IN (SELECT id FROM sessions WHERE user_id = auth.uid())
    );

-- Tasks policy
DROP POLICY IF EXISTS "Users own tasks" ON tasks;
CREATE POLICY "Users own tasks" ON tasks
    FOR ALL USING (user_id = auth.uid());

-- User context policy
DROP POLICY IF EXISTS "Users own context" ON user_context;
CREATE POLICY "Users own context" ON user_context
    FOR ALL USING (user_id = auth.uid());

-- Checkpoints policy
DROP POLICY IF EXISTS "Users own checkpoints" ON checkpoints;
CREATE POLICY "Users own checkpoints" ON checkpoints
    FOR ALL USING (
        session_id IN (SELECT id FROM sessions WHERE user_id = auth.uid())
    );

-- Heartbeat state policy
DROP POLICY IF EXISTS "Users own heartbeat" ON heartbeat_state;
CREATE POLICY "Users own heartbeat" ON heartbeat_state
    FOR ALL USING (user_id = auth.uid());
