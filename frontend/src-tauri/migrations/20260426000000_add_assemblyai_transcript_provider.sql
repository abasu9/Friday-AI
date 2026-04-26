-- Add AssemblyAI hosted streaming transcription API key storage.
ALTER TABLE transcript_settings ADD COLUMN assemblyAiApiKey TEXT;
