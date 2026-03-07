-- Migration: Add RLS policies for tables with RLS enabled
-- Date: 2026-03-07
-- Description: Add "allow all" policies to tables that have RLS enabled but no policies
--              Fix function search_path security warning

-- Fix function search_path mutable warning
CREATE OR REPLACE FUNCTION public.update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql
SET search_path = '';

-- meetings table
CREATE POLICY "Allow all" ON public.meetings FOR ALL USING (true) WITH CHECK (true);

-- settings table
CREATE POLICY "Allow all" ON public.settings FOR ALL USING (true) WITH CHECK (true);

-- summary_processes table
CREATE POLICY "Allow all" ON public.summary_processes FOR ALL USING (true) WITH CHECK (true);

-- transcript_chunks table
CREATE POLICY "Allow all" ON public.transcript_chunks FOR ALL USING (true) WITH CHECK (true);

-- transcript_settings table
CREATE POLICY "Allow all" ON public.transcript_settings FOR ALL USING (true) WITH CHECK (true);

-- transcripts table
CREATE POLICY "Allow all" ON public.transcripts FOR ALL USING (true) WITH CHECK (true);
