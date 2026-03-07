# FRIDAY — Product Requirements Document

## Overview & Index

FRIDAY is an AI-native workspace orchestrator for people with ADHD. The core problem isn't productivity — it's **overload recovery**. Built at the Frontiers Hackathon (MIT, Google DeepMind + Breakthrough Ventures).

### PRD Sections

| # | File | Contents |
|---|------|----------|
| 01 | [Vision & User Stories](./01-vision.md) | Problem statement, personas, user stories, demo flow |
| 02 | [System Architecture](./02-architecture.md) | High-level architecture, LangGraph StateGraph, FastAPI, Supabase, gws CLI, Supermemory |
| 03 | [Database Schema](./03-database-schema.md) | Complete Supabase schema — tables, indexes, types, RLS |
| 04 | [LangGraph State Machine](./04-langgraph.md) | Nodes, edges, conditional routing, state definition |
| 05 | [Agent Tools](./05-tools.md) | gws CLI (Google Workspace) + Exa (web search) + Supermemory + custom Supabase tools |
| 06 | [Prompt Engineering](./06-prompts.md) | System prompt, tool descriptions, context injection layers |
| 07 | [Structured Output](./07-schemas.md) | Pydantic models for all agent responses |
| 08 | [API & Streaming](./08-api.md) | FastAPI endpoints, SSE streaming, heartbeat loop |
| 09 | [Implementation Plan](./09-implementation.md) | Build order, verification steps, milestones |

### Architecture Decision: Custom StateGraph

We use a **custom LangGraph StateGraph** (not `create_react_agent`) because:
1. Matches existing InfoSavvy Compass patterns — proven in production
2. Full control over preprocessing, intent routing, human-in-the-loop approval
3. Heartbeat loop integration for proactive ADHD support
4. Layered prompt engineering following OpenCode's patterns

### Tech Stack (Backend — our scope)

| Layer | Technology |
|-------|-----------|
| Agent Framework | LangGraph (custom StateGraph) |
| API Server | FastAPI |
| Database | Supabase Postgres |
| LLM | Gemini (Google DeepMind) |
| Google Tools | `gws` CLI — Google Workspace CLI with 100+ agent skills, dynamic API discovery |
| Internal Tools | Custom Supabase tools (tasks, context, preferences) |
| Web Search | Exa (neural search for real-time web information) |
| Memory/RAG | Supermemory (semantic memory for patterns, entities, commitments) |
| Auth | Google OAuth via `gws auth` (direct, no Supabase Auth needed) |
| Streaming | Server-Sent Events (SSE) |
| Validation | Pydantic v2 |

> **Note**: The frontend is a desktop app owned by another team member. This PRD covers the **backend system** only.
