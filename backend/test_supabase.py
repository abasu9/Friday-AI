#!/usr/bin/env python3
"""Test Supabase connection."""

import os
from dotenv import load_dotenv

# Load environment variables
load_dotenv("temp.env")

SUPABASE_URL = os.getenv("SUPABASE_URL")
SUPABASE_KEY = os.getenv("SUPABASE_KEY")

print(f"SUPABASE_URL: {SUPABASE_URL}")
print(f"SUPABASE_KEY: {SUPABASE_KEY[:20]}..." if SUPABASE_KEY else "SUPABASE_KEY: None")

if not SUPABASE_URL or not SUPABASE_KEY:
    print("\n❌ Missing environment variables!")
    print("Make sure temp.env has SUPABASE_URL and SUPABASE_KEY")
    exit(1)

try:
    from supabase import create_client, Client
    
    print("\n🔄 Connecting to Supabase...")
    supabase: Client = create_client(SUPABASE_URL, SUPABASE_KEY)
    
    # Test by querying the settings table (should exist from schema)
    print("🔄 Testing database query...")
    response = supabase.table("settings").select("*").execute()
    
    print(f"\n✅ Supabase connection successful!")
    print(f"   Settings data: {response.data}")
    
    # Also verify other tables exist
    print("\n🔄 Checking all tables...")
    tables = {
        # Original meeting tables
        "meetings": "id",
        "transcripts": "id", 
        "summary_processes": "meeting_id",
        "transcript_chunks": "meeting_id",
        "transcript_settings": "id",
        "settings": "id",
        # AI Assistant tables
        "users": "id",
        "sessions": "id",
        "messages": "id",
        "tool_calls": "id",
        "approvals": "id",
        "tasks": "id",
        "user_context": "id",
        "checkpoints": "id",
        "heartbeat_state": "id"
    }
    for table, pk in tables.items():
        try:
            supabase.table(table).select(pk).limit(1).execute()
            print(f"   ✅ {table}")
        except Exception as e:
            print(f"   ❌ {table}: {e}")
    
except Exception as e:
    print(f"\n❌ Connection failed: {e}")
    exit(1)
