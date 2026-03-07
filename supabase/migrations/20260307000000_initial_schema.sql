-- Friday Meeting Assistant - Supabase Schema
-- Run this in Supabase SQL Editor: https://supabase.com/dashboard/project/lprzfiaicvrcffyfzvli/sql

-- =============================================================================
-- MEETINGS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS meetings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    folder_path TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Index for faster queries
CREATE INDEX IF NOT EXISTS idx_meetings_created_at ON meetings(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_meetings_updated_at ON meetings(updated_at DESC);

-- =============================================================================
-- TRANSCRIPTS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS transcripts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    meeting_id UUID NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    transcript TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
    summary TEXT,
    action_items JSONB,
    key_points JSONB,
    audio_start_time REAL,
    audio_end_time REAL,
    duration REAL
);

-- Index for faster queries by meeting
CREATE INDEX IF NOT EXISTS idx_transcripts_meeting_id ON transcripts(meeting_id);
CREATE INDEX IF NOT EXISTS idx_transcripts_timestamp ON transcripts(timestamp DESC);

-- =============================================================================
-- SUMMARY PROCESSES TABLE
-- =============================================================================

-- Handle type creation gracefully
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'summary_status') THEN
        CREATE TYPE summary_status AS ENUM ('PENDING', 'PROCESSING', 'COMPLETED', 'FAILED');
    END IF;
END$$;

CREATE TABLE IF NOT EXISTS summary_processes (
    meeting_id UUID PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'PENDING',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    error TEXT,
    result JSONB,
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ,
    chunk_count INTEGER DEFAULT 0,
    processing_time REAL DEFAULT 0.0,
    metadata JSONB DEFAULT '{}'
);

-- Index for status queries
CREATE INDEX IF NOT EXISTS idx_summary_processes_status ON summary_processes(status);

-- =============================================================================
-- TRANSCRIPT CHUNKS TABLE (for processing)
-- =============================================================================
CREATE TABLE IF NOT EXISTS transcript_chunks (
    meeting_id UUID PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    meeting_name TEXT,
    transcript_text TEXT NOT NULL,
    model TEXT NOT NULL,
    model_name TEXT NOT NULL,
    chunk_size INTEGER,
    overlap INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- =============================================================================
-- SETTINGS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS settings (
    id TEXT PRIMARY KEY DEFAULT 'default',
    provider TEXT NOT NULL DEFAULT 'ollama',
    model TEXT NOT NULL DEFAULT 'llama3.2',
    whisper_model TEXT NOT NULL DEFAULT 'base',
    groq_api_key TEXT,
    openai_api_key TEXT,
    anthropic_api_key TEXT,
    ollama_api_key TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- =============================================================================
-- TRANSCRIPT SETTINGS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS transcript_settings (
    id TEXT PRIMARY KEY DEFAULT 'default',
    provider TEXT NOT NULL DEFAULT 'local',
    model TEXT NOT NULL DEFAULT 'whisper',
    whisper_api_key TEXT,
    deepgram_api_key TEXT,
    eleven_labs_api_key TEXT,
    groq_api_key TEXT,
    openai_api_key TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- =============================================================================
-- ROW LEVEL SECURITY (RLS)
-- Enable if you want per-user data isolation
-- =============================================================================

-- Enable RLS on all tables (optional - uncomment if needed)
-- ALTER TABLE meetings ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE transcripts ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE summary_processes ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE transcript_chunks ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE settings ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE transcript_settings ENABLE ROW LEVEL SECURITY;

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply triggers to tables with updated_at
DROP TRIGGER IF EXISTS update_meetings_updated_at ON meetings;
CREATE TRIGGER update_meetings_updated_at
    BEFORE UPDATE ON meetings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_summary_processes_updated_at ON summary_processes;
CREATE TRIGGER update_summary_processes_updated_at
    BEFORE UPDATE ON summary_processes
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_settings_updated_at ON settings;
CREATE TRIGGER update_settings_updated_at
    BEFORE UPDATE ON settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_transcript_settings_updated_at ON transcript_settings;
CREATE TRIGGER update_transcript_settings_updated_at
    BEFORE UPDATE ON transcript_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- =============================================================================
-- SEED DATA (optional default settings)
-- =============================================================================
INSERT INTO settings (id, provider, model, whisper_model)
VALUES ('default', 'ollama', 'llama3.2', 'base')
ON CONFLICT (id) DO NOTHING;

INSERT INTO transcript_settings (id, provider, model)
VALUES ('default', 'local', 'whisper')
ON CONFLICT (id) DO NOTHING;
