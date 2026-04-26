-- Move existing installs from the old default to the new hosted streaming default.
UPDATE transcript_settings
SET provider = 'assemblyAI',
    model = 'universal-streaming-english'
WHERE provider = 'parakeet'
  AND model = 'parakeet-tdt-0.6b-v3-int8';
