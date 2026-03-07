CREATE TABLE IF NOT EXISTS connected_accounts (
    id TEXT PRIMARY KEY NOT NULL,
    provider TEXT NOT NULL UNIQUE,
    email TEXT,
    scopes_json TEXT NOT NULL,
    connection_status TEXT NOT NULL,
    last_sync_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS calendar_events (
    account_id TEXT NOT NULL,
    provider_event_id TEXT NOT NULL,
    calendar_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    organizer_email TEXT,
    organizer_name TEXT,
    attendees_json TEXT,
    start_at TEXT NOT NULL,
    end_at TEXT NOT NULL,
    timezone TEXT,
    conference_url TEXT,
    status TEXT NOT NULL,
    html_link TEXT,
    raw_etag TEXT,
    is_primary_calendar INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (account_id, provider_event_id),
    FOREIGN KEY (account_id) REFERENCES connected_accounts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS meeting_event_links (
    meeting_id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    provider_event_id TEXT NOT NULL,
    confidence REAL NOT NULL,
    link_method TEXT NOT NULL,
    reason TEXT,
    linked_at TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, provider_event_id)
        REFERENCES calendar_events(account_id, provider_event_id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS calendar_sync_state (
    account_id TEXT PRIMARY KEY NOT NULL,
    sync_token TEXT,
    window_start TEXT,
    window_end TEXT,
    last_sync_started_at TEXT,
    last_sync_finished_at TEXT,
    last_success_at TEXT,
    last_error TEXT,
    FOREIGN KEY (account_id) REFERENCES connected_accounts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_calendar_events_time
    ON calendar_events(account_id, start_at, end_at);

CREATE INDEX IF NOT EXISTS idx_calendar_events_status
    ON calendar_events(account_id, status);

CREATE INDEX IF NOT EXISTS idx_meeting_event_links_event
    ON meeting_event_links(account_id, provider_event_id);
