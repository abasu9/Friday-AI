CREATE TABLE IF NOT EXISTS agent_settings (
    id TEXT PRIMARY KEY NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 0,
    provider TEXT NOT NULL DEFAULT 'gemini',
    model TEXT NOT NULL DEFAULT 'gemini-2.5-flash',
    notifications_enabled INTEGER NOT NULL DEFAULT 1,
    calendar_proposals_enabled INTEGER NOT NULL DEFAULT 0,
    heartbeat_interval_minutes INTEGER NOT NULL DEFAULT 5,
    last_run_at TEXT,
    last_success_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT INTO agent_settings (
    id,
    enabled,
    provider,
    model,
    notifications_enabled,
    calendar_proposals_enabled,
    heartbeat_interval_minutes,
    created_at,
    updated_at
)
VALUES (
    'default',
    0,
    'gemini',
    'gemini-2.5-flash',
    1,
    0,
    5,
    CURRENT_TIMESTAMP,
    CURRENT_TIMESTAMP
)
ON CONFLICT(id) DO NOTHING;

CREATE TABLE IF NOT EXISTS agent_runs (
    id TEXT PRIMARY KEY NOT NULL,
    trigger_type TEXT NOT NULL,
    trigger_ref TEXT,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    model_provider TEXT NOT NULL,
    model_name TEXT NOT NULL,
    summary_json TEXT,
    error TEXT
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_trigger_ref
    ON agent_runs(trigger_ref);

CREATE INDEX IF NOT EXISTS idx_agent_runs_started_at
    ON agent_runs(started_at DESC);

CREATE TABLE IF NOT EXISTS agent_memory_items (
    id TEXT PRIMARY KEY NOT NULL,
    memory_type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    source_meeting_id TEXT,
    source_calendar_event_id TEXT,
    subject_key TEXT NOT NULL,
    subject_json TEXT,
    confidence REAL NOT NULL,
    status TEXT NOT NULL,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    created_run_id TEXT,
    updated_run_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_agent_memory_source_meeting
    ON agent_memory_items(source_meeting_id);

CREATE INDEX IF NOT EXISTS idx_agent_memory_status
    ON agent_memory_items(status);

CREATE INDEX IF NOT EXISTS idx_agent_memory_subject_key
    ON agent_memory_items(subject_key);

CREATE TABLE IF NOT EXISTS agent_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    source_meeting_id TEXT,
    source_memory_item_id TEXT,
    owner_kind TEXT NOT NULL,
    due_at TEXT,
    priority TEXT NOT NULL,
    status TEXT NOT NULL,
    last_suggested_at TEXT NOT NULL,
    created_run_id TEXT,
    updated_run_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_agent_tasks_source_meeting
    ON agent_tasks(source_meeting_id);

CREATE INDEX IF NOT EXISTS idx_agent_tasks_status
    ON agent_tasks(status);

CREATE TABLE IF NOT EXISTS agent_recommendations (
    id TEXT PRIMARY KEY NOT NULL,
    recommendation_type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    rationale TEXT NOT NULL,
    confidence REAL NOT NULL,
    source_meeting_id TEXT,
    source_calendar_event_id TEXT,
    task_id TEXT,
    payload_json TEXT,
    status TEXT NOT NULL,
    surfaced_at TEXT NOT NULL,
    acted_at TEXT,
    error TEXT
);

CREATE INDEX IF NOT EXISTS idx_agent_recommendations_status
    ON agent_recommendations(status);

CREATE INDEX IF NOT EXISTS idx_agent_recommendations_type
    ON agent_recommendations(recommendation_type);

CREATE INDEX IF NOT EXISTS idx_agent_recommendations_source_meeting
    ON agent_recommendations(source_meeting_id);

CREATE TABLE IF NOT EXISTS agent_notification_log (
    id TEXT PRIMARY KEY NOT NULL,
    recommendation_id TEXT NOT NULL,
    shown_at TEXT NOT NULL,
    delivery_status TEXT NOT NULL,
    channel TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_notification_recommendation
    ON agent_notification_log(recommendation_id);
