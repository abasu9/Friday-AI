# 01 — Vision & User Stories

## Hackathon Context

Built at the **Frontiers Hackathon (MIT, Google DeepMind + Breakthrough Ventures)**. FRIDAY extends an existing local-first meeting notetaker (Meetly) into an AI-native workspace orchestrator for people with ADHD.

**Why Gemini**: Native Google ecosystem integration (Gmail, Calendar, Docs, Drive), fast structured JSON output via `gemini-3.0-flash`, and BYOK (Bring Your Own Key) model — aligning with the Google DeepMind partnership at this hackathon.

## Problem Statement

People with ADHD don't need another productivity app. They need **overload recovery** — the ability to go from "everything is on fire, I can't even start" to "okay, here's what matters right now."

Current tools fail because they:
- Require manual organization (the thing ADHD brains can't do under stress)
- Present flat lists without prioritization
- Don't connect scattered information across email, calendar, docs
- Provide no proactive nudges or ambient awareness

## Target User Persona

**Alex, 28, Software Engineer with ADHD**
- 47 unread emails, 3 overdue tasks, a meeting in 20 minutes they forgot about
- Has Google Workspace (Gmail, Calendar, Docs, Drive) as primary tools
- Knows what they *should* do but can't break through the wall of overwhelm
- Needs an agent that says: "Hey, you have a meeting in 20 min. Here's the 1 thing you need to prep. I drafted it."

## User Stories

### Core (MVP — Hackathon Demo)

1. **As Alex**, I open FRIDAY and it immediately shows me what matters right now — no setup, no configuration.
2. **As Alex**, I say "what's going on today?" and FRIDAY pulls my calendar, flags urgent emails, and gives me a prioritized summary.
3. **As Alex**, I say "draft a reply to that email from Sarah" and FRIDAY reads the email, drafts a contextual reply, and waits for my approval before sending.
4. **As Alex**, I get a proactive nudge: "You have a 1:1 with your manager in 30 min. Last time you discussed the API migration — want me to pull up your notes?"
5. **As Alex**, I say "I'm overwhelmed" and FRIDAY switches to triage mode: surfaces the 3 most important things and hides everything else.

### Stretch

6. **As Alex**, FRIDAY notices I haven't responded to a high-priority email in 2 days and gently reminds me.
7. **As Alex**, I can approve/reject any action FRIDAY wants to take on my behalf (send email, create event, etc).
8. **As Alex**, FRIDAY learns my patterns over time — which emails I always ignore, which meetings I prep for.

## Demo Flow (3-minute hackathon pitch)

```
0. [Pre-demo] User authenticates Google Workspace via `gws auth login` (one-time)
   - FRIDAY gets full workspace context: Calendar, Gmail, Drive, Docs, Tasks, and more

1. [0:00] Open FRIDAY → "Good morning! Here's your day."
   - Shows: 2 urgent emails, 3 meetings, 1 overdue task
   - Proactive: "Your 1:1 with Manager is in 45 min"

2. [0:45] User: "What's the most important thing right now?"
   - FRIDAY triages → highlights the client email that needs a response
   - Shows email context inline

3. [1:15] User: "Draft a reply"
   - FRIDAY reads full email thread
   - Generates contextual reply
   - Shows draft for approval: [Send] [Edit] [Discard]

4. [1:45] User approves → FRIDAY sends via Gmail API
   - Confirms: "Sent! Next up: prep for your 1:1"

5. [2:00] FRIDAY proactively: "For your 1:1 — last meeting notes mention the API migration. Want me to pull up the doc?"
   - User: "Yes"
   - FRIDAY fetches Google Doc, summarizes key points

6. [2:30] User: "I'm feeling overwhelmed"
   - FRIDAY enters triage mode
   - Hides noise, shows only: "Here are your 3 priorities for today"

7. [2:50] Wrap: "FRIDAY — your AI workspace that gets ADHD"
```

## Success Metrics (Hackathon)

- End-to-end demo works without errors
- < 3 second response for conversational queries
- < 5 second response for tool-calling queries (email read, calendar fetch)
- Human-in-the-loop approval works for email sending
- At least 1 proactive nudge fires correctly
