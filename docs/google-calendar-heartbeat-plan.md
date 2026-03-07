# Friday: Google Calendar + Heartbeat Product Plan

## Purpose

This document captures the product and technical requirements to evolve Friday from a meeting note taker into a local-first autonomous meeting assistant.

The target product should:

- continue to capture, transcribe, and summarize meetings locally,
- integrate with Google Calendar first,
- run an always-on "heartbeat" loop inside the desktop app,
- prepare for meetings automatically,
- process meetings after they end,
- draft follow-up actions,
- remain user-controlled for high-impact actions.

This is a planning document only. It does not prescribe exact code changes line by line, but it is specific enough to drive implementation work.

## Product Goal

Friday should move from:

- "record a meeting and summarize it"

to:

- "know what meetings are coming, be ready for them, capture them, understand them, and help the user follow through continuously."

The first external system of record should be Google Calendar.

## Current State In This Repo

The repo already has several foundations we can build on:

- Tauri desktop runtime as the main application host.
- React/Next frontend for settings, meeting views, onboarding, and controls.
- Rust backend inside Tauri for audio capture, transcription, database access, notifications, tray integration, and summary engine management.
- Local SQLite storage for meetings, transcripts, transcript chunks, summary processes, and model settings.
- System tray support for background presence and recording control.
- Notification settings that already mention calendar-based reminders.
- A small Gemini-based `Friday` extraction path in the Python backend that writes structured output to `friday_state.json`.

Relevant existing files:

- [docs/architecture.md](/Users/clarkohlenbusch/friday/docs/architecture.md)
- [frontend/src-tauri/src/lib.rs](/Users/clarkohlenbusch/friday/frontend/src-tauri/src/lib.rs)
- [frontend/src-tauri/src/tray.rs](/Users/clarkohlenbusch/friday/frontend/src-tauri/src/tray.rs)
- [frontend/src-tauri/src/database/models.rs](/Users/clarkohlenbusch/friday/frontend/src-tauri/src/database/models.rs)
- [frontend/src-tauri/src/notifications/settings.rs](/Users/clarkohlenbusch/friday/frontend/src-tauri/src/notifications/settings.rs)
- [backend/app/gemini_processor.py](/Users/clarkohlenbusch/friday/backend/app/gemini_processor.py)

Important limitation of the current model:

- the app understands local meetings and transcripts,
- but it does not yet understand accounts, calendars, events, sync state, background agent runs, or approval workflows.

## Product Direction

### Core product idea

Friday becomes a desktop meeting assistant with three continuous behaviors:

1. Before the meeting
   - sync upcoming Google Calendar events,
   - identify likely meetings,
   - prepare reminders and meeting context,
   - present a ready-to-start meeting surface.

2. During the meeting
   - capture audio,
   - generate transcript,
   - tie the local meeting record to a calendar event when available.

3. After the meeting
   - summarize transcript,
   - extract action items, deadlines, and follow-ups,
   - keep those outputs visible until the user resolves or approves the next action.

### User experience target

The user should feel that Friday is "keeping watch" in the background, but not acting recklessly.

For v1 autonomy, the app should:

- observe,
- sync,
- remind,
- prepare,
- draft,
- suggest.

For v1 autonomy, the app should not:

- send emails automatically,
- edit calendar events automatically,
- create external documents automatically,
- take irreversible external actions without user approval.

## Scope Decisions

### In scope for the first implementation

- Google Calendar integration.
- Desktop local-first runtime.
- Heartbeat loop inside the Tauri app.
- Event sync from Google Calendar to local storage.
- Event-to-meeting linking.
- Meeting reminders based on synced events.
- Upcoming meeting preparation state.
- Post-meeting extraction and action drafting.
- Status surfaces for connection, sync health, and heartbeat activity.

### Explicitly out of scope for the first implementation

- Gmail integration.
- Google Drive or Docs write-back.
- Multi-user collaboration.
- Hosted backend as the source of truth.
- Full autonomous execution across external tools.
- Multiple connected Google accounts.
- Automatic meeting join/bot behavior.

Those can be layered later, but they should not distort the first architecture.

## Integration Decision: Do Not Use Google Workspace CLI As Runtime

We should not build the shipped product around the Google Workspace CLI.

The CLI can be useful for developer experimentation or manual testing, but it is the wrong runtime dependency for Friday because:

- end users would need an external CLI installed and authenticated,
- background sync would depend on shelling out to another tool,
- parsing CLI output is brittle compared with calling Google APIs directly,
- token management becomes split across the app and a separate tool,
- desktop packaging becomes more fragile across macOS and Windows.

Recommended runtime approach:

- use Google OAuth directly,
- call Google Calendar APIs directly,
- store and refresh credentials inside Friday.

## Architecture Direction

### Runtime model

The Tauri app remains the primary runtime.

That means the main orchestration should live in Rust, not in the optional Python backend.

Rust responsibilities should expand to include:

- Google account connection state,
- token lifecycle,
- event sync,
- heartbeat scheduling,
- local job state,
- meeting-event matching,
- reminder scheduling,
- post-meeting orchestration.

The Python backend can remain optional for specific AI processing paths, but it should not become the source of truth for app behavior.

### Why the heartbeat belongs in the desktop app

The desktop app already has:

- startup hooks,
- tray presence,
- notification support,
- local database access,
- meeting capture lifecycle.

That makes it the natural place to host an always-on scheduler.

## Required New Product Concepts

The current storage model is centered on local meetings and transcripts. We need first-class entities for external context and agent state.

### New persistent concepts

- connected account
  - provider,
  - account email,
  - granted scopes,
  - connection status,
  - token metadata,
  - last successful auth/refresh,
  - last auth error.

- calendar event
  - provider event id,
  - title,
  - description,
  - organizer,
  - attendees,
  - start time,
  - end time,
  - timezone,
  - conferencing URL,
  - status,
  - source calendar,
  - sync metadata.

- meeting link
  - local meeting id,
  - external event id,
  - link confidence,
  - linked-by rule,
  - linked-at time.

- heartbeat state
  - last run start,
  - last run end,
  - last successful sync,
  - next scheduled run,
  - current status,
  - latest error.

- agent task / run
  - task type,
  - task payload,
  - status,
  - retry count,
  - created at,
  - started at,
  - finished at,
  - result summary,
  - error summary.

- action draft
  - related meeting id,
  - related event id,
  - draft type,
  - content,
  - confidence,
  - status,
  - user approval state.

## Required New Capabilities

### 1. Google account connection

Friday needs a proper Google account connection flow:

- initiate OAuth from desktop app,
- receive callback safely,
- exchange auth code for tokens,
- persist tokens securely,
- refresh tokens,
- detect revoked or expired sessions,
- let the user disconnect cleanly.

Requirements:

- only request the minimum Google Calendar scopes needed for the first release,
- clearly explain to the user why access is needed,
- surface connection errors in a recoverable way.

### 2. Secure credential storage

Google credentials must not be stored like ordinary model settings.

We need a separate credential boundary for:

- access token,
- refresh token,
- token expiry,
- granted scopes,
- account identity metadata.

Design requirement:

- prefer OS-secure storage,
- if fallback storage is ever necessary, it must be explicit and documented,
- never store Google refresh tokens in plain settings rows.

### 3. Calendar sync engine

Friday needs a sync subsystem that:

- fetches upcoming events,
- normalizes them into local records,
- updates changed events,
- handles canceled events,
- removes or archives stale state,
- tracks sync checkpoints and failures.

Sync behavior should be:

- incremental where possible,
- idempotent,
- safe to rerun frequently,
- tolerant of offline periods.

### 4. Meeting-to-event matching

The app needs deterministic and explainable matching logic between a calendar event and a local meeting.

Candidate signals:

- event start/end time,
- meeting title,
- conferencing URL,
- organizer,
- attendee overlap,
- manual user selection.

The app should support:

- automatic match when confidence is high,
- user confirmation or correction when confidence is uncertain,
- manual linking after the fact.

### 5. Heartbeat service

The heartbeat is the core autonomous loop.

Its job is to wake up regularly and do lightweight coordination work without user intervention.

Responsibilities:

- refresh tokens when needed,
- sync calendar events,
- compute upcoming reminders,
- mark meetings as "ready soon",
- create preparation context,
- trigger post-meeting extraction when recording has ended,
- queue follow-up draft generation,
- store state and errors for display.

Heartbeat design requirements:

- only one active loop at a time,
- survives app restarts,
- stores last run results,
- no duplicate work from repeated wakes,
- no dependence on the UI being open to function.

### 6. Reminder and preparation flow

The app already has notification support. That should be extended so reminders become calendar-backed.

Needed behaviors:

- remind user at configured intervals before a synced meeting,
- include meeting title and start time,
- offer a quick path to open Friday and start capture,
- support dismiss/snooze later if added in future versions.

### 7. Post-meeting processing

When a recording ends, the app should do more than summarize.

It should:

- ensure the meeting is linked to an event when possible,
- produce summary,
- extract action items and deadlines,
- create action drafts,
- surface the result in a review state,
- preserve enough metadata to revisit or regenerate later.

The existing Gemini `Friday` extraction path is useful as prior art, but the long-term orchestration should live in the desktop app's state model.

## UX Requirements

### New settings and surfaces

The app needs new user-facing surfaces for:

- Google account connection,
- Calendar sync status,
- heartbeat on/off state,
- next heartbeat run,
- last successful sync,
- recent sync or auth errors,
- reminder preferences,
- autonomy mode,
- approval-required policy for external actions.

### New main app surfaces

The app should gain a lightweight planning surface, such as:

- "Upcoming meetings",
- "Today",
- or "Ready soon".

That surface should show:

- event title,
- start time,
- linked/not linked state,
- quick start action,
- sync freshness,
- draft follow-up status after meeting completion.

### User trust requirements

The app should always make clear:

- what it knows from Calendar,
- what it inferred,
- what it is suggesting,
- what it has not done automatically.

Autonomy without clear boundaries will reduce trust.

## Data Model Work Needed

Current persisted models are not enough. New schema will be required.

### Existing persistent areas

- meetings
- transcripts
- transcript chunks
- summary processes
- settings
- transcript settings

### New schema areas to add

- connected accounts
- calendar events
- meeting event links
- heartbeat state
- agent runs
- action drafts

### Migration requirements

- migrations must preserve existing user meeting data,
- new tables must be additive,
- failures in Google features must not break the existing local meeting flow,
- startup should remain functional for users who never connect Google.

## Runtime Changes Needed

### App startup

Current startup already initializes:

- tray,
- notifications,
- models,
- database,
- templates.

We need to add startup steps for:

- secure credential store initialization,
- Google account state restoration,
- heartbeat scheduler startup,
- initial sync scheduling after app ready.

### New internal services to introduce

- `SecureCredentialStore`
- `GoogleCalendarAuthService`
- `GoogleCalendarSyncService`
- `MeetingEventLinkService`
- `HeartbeatService`
- `ReminderService`
- `PostMeetingAgentService`

These names are conceptual. Exact symbol names can change, but the responsibilities should remain separate.

## Security And Privacy Requirements

Friday's privacy-first positioning makes this part critical.

### Requirements

- local notes, transcripts, and summaries remain local-first,
- Google is used as context input for scheduling and event metadata,
- access scopes are minimal and documented,
- token storage uses the strongest secure local mechanism available,
- disconnection fully removes active account access,
- logs must avoid sensitive token leakage,
- user should be able to understand what data comes from Google and what remains fully local.

### Privacy boundary to preserve

Adding Calendar should not quietly turn Friday into a cloud-first app.

The app should still function as a local meeting recorder and note taker even with no Google account connected.

## Reliability Requirements

### Offline behavior

The app should continue to work locally when:

- no internet is available,
- Google API calls fail,
- token refresh fails,
- sync is temporarily rate-limited.

### Error handling

We need first-class handling for:

- expired tokens,
- revoked tokens,
- network failures,
- malformed or partially updated event data,
- duplicate event sync,
- missed heartbeat runs,
- crashes during background work.

### Idempotency rules

The following operations must be safe to retry:

- event sync,
- meeting-event link computation,
- reminder evaluation,
- post-meeting task scheduling,
- action draft generation.

## Suggested Rollout Phases

### Phase 1: Calendar foundation

- Google OAuth flow.
- Secure credential storage.
- Calendar event fetch and local persistence.
- Settings/status UI for connection and sync.

Deliverable:

- user can connect Google Calendar and see upcoming meetings in Friday.

### Phase 2: Heartbeat and reminders

- Background heartbeat service.
- Sync loop scheduling.
- Reminder generation from synced events.
- Ready-to-start meeting preparation state.

Deliverable:

- app stays aware of upcoming meetings and can prompt the user at the right time.

### Phase 3: Meeting linking and post-meeting intelligence

- event-to-meeting linking,
- structured action extraction,
- draft follow-up objects,
- review UI for outputs.

Deliverable:

- every captured meeting can be tied to a calendar context and produce follow-up drafts.

### Phase 4: Controlled autonomy

- policy controls,
- approval workflow for external actions,
- later integration points for Gmail, Docs, or Tasks.

Deliverable:

- app can prepare external actions, but user controls execution.

## Acceptance Criteria For The First Real Milestone

The first milestone should be considered complete when:

- a user can connect a Google account from Friday,
- Friday stores the connection securely,
- Friday syncs upcoming calendar events to local storage,
- the UI exposes those events and sync health,
- the heartbeat runs in the background without duplicating work,
- upcoming meeting reminders are based on synced events,
- disconnecting the account cleanly removes access without breaking local meeting features.

## Open Risks

### Product risks

- adding Google too aggressively could weaken the local-first story,
- users may expect automatic meeting joining if calendar integration appears,
- trust will drop if the app behaves autonomously without clear approval boundaries.

### Technical risks

- desktop OAuth callback handling across macOS and Windows,
- secure token storage portability,
- background scheduling reliability when the app window is closed,
- matching meetings to events incorrectly,
- overloading the main runtime with too much orchestration logic too early.

## Recommended Defaults

- one Google account only for the first version,
- Google Calendar only for the first external integration,
- local-first desktop runtime remains primary,
- heartbeat runs on a fixed interval plus event-driven triggers,
- high-impact external actions require explicit user confirmation,
- local meeting capture flow must remain usable even with no Google integration configured.

## Summary

The right first expansion for Friday is not "integrate with everything Google."

It is:

- integrate with Google Calendar well,
- add a trustworthy heartbeat loop,
- add the missing state model for sync and autonomy,
- keep the app local-first,
- make suggestions and drafts before allowing external actions.

If we do those pieces correctly, Gmail, Docs, Drive, and broader agent workflows can be added later without rebuilding the foundation.
