use crate::agent::types::{CalendarEventDraftPayload, CreatedCalendarEventSummary};
use crate::calendar::types::{
    CalendarAccountSummary, CalendarAttendeeSummary, CalendarLinkCandidate, CalendarStatusResponse,
    CalendarSyncResult, LinkedCalendarEvent, PendingRecordingLink, StoredAttendee,
    StoredOAuthTokens, UpcomingCalendarEvent,
};
use crate::database::models::{CalendarEventModel, ConnectedAccountModel};
use crate::database::repositories::calendar::{
    CalendarRepository, MeetingLinkedEventRow, UpsertCalendarEvent,
};
use crate::state::AppState;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use keyring::{Entry, Error as KeyringError};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use std::collections::HashSet;
use std::process::Command;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use url::Url;

const GOOGLE_PROVIDER: &str = "google";
const GOOGLE_ACCOUNT_ID: &str = "google-primary";
const GOOGLE_SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";
const GOOGLE_WRITE_SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";
const TOKEN_KEYRING_SERVICE: &str = "Friday.GoogleCalendar";
const TOKEN_KEYRING_USERNAME: &str = "google-primary";
const MISSING_KEYCHAIN_TOKENS_ERROR: &str =
    "Google Calendar credentials were not found in the system keychain";
const WINDOW_PAST_HOURS: i64 = 24;
const WINDOW_FUTURE_DAYS: i64 = 30;
const AUTO_MATCH_WINDOW_MINUTES: i64 = 30;
const CANDIDATE_LOOKBACK_HOURS: i64 = 4;
const CANDIDATE_LOOKAHEAD_HOURS: i64 = 2;
const BACKGROUND_SYNC_INTERVAL_SECONDS: u64 = 300;
const MATCH_SYNC_FRESHNESS_MINUTES: i64 = 10;

#[derive(Debug, Default)]
pub struct CalendarRuntimeState {
    pending_recording_link: Option<PendingRecordingLink>,
    is_sync_in_progress: bool,
}

#[derive(Clone, Default)]
pub struct CalendarManagerState(pub Arc<Mutex<CalendarRuntimeState>>);

#[derive(Debug, Deserialize)]
struct GoogleCalendarResource {
    id: String,
}

#[derive(Debug, Deserialize)]
struct GoogleEventsResponse {
    items: Vec<GoogleEvent>,
}

#[derive(Debug, Deserialize)]
struct GoogleEvent {
    id: String,
    summary: Option<String>,
    description: Option<String>,
    organizer: Option<GooglePerson>,
    attendees: Option<Vec<GoogleAttendee>>,
    start: GoogleEventTime,
    end: GoogleEventTime,
    status: Option<String>,
    #[serde(rename = "htmlLink")]
    html_link: Option<String>,
    etag: Option<String>,
    #[serde(rename = "hangoutLink")]
    hangout_link: Option<String>,
    #[serde(rename = "conferenceData")]
    conference_data: Option<GoogleConferenceData>,
}

#[derive(Debug, Deserialize)]
struct GooglePerson {
    email: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleAttendee {
    email: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "responseStatus")]
    response_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleConferenceData {
    #[serde(rename = "entryPoints")]
    entry_points: Option<Vec<GoogleConferenceEntryPoint>>,
}

#[derive(Debug, Deserialize)]
struct GoogleConferenceEntryPoint {
    uri: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleEventTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
    #[serde(rename = "timeZone")]
    time_zone: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct GoogleEventInsertRequest {
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    start: GoogleEventInsertDateTime,
    end: GoogleEventInsertDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    attendees: Option<Vec<GoogleEventInsertAttendee>>,
    #[serde(rename = "conferenceData", skip_serializing_if = "Option::is_none")]
    conference_data: Option<GoogleEventInsertConferenceData>,
}

#[derive(Debug, serde::Serialize)]
struct GoogleEventInsertDateTime {
    #[serde(rename = "dateTime")]
    date_time: String,
    #[serde(rename = "timeZone", skip_serializing_if = "Option::is_none")]
    time_zone: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct GoogleEventInsertAttendee {
    email: String,
}

#[derive(Debug, serde::Serialize)]
struct GoogleEventInsertConferenceData {
    #[serde(rename = "createRequest")]
    create_request: GoogleEventConferenceCreateRequest,
}

#[derive(Debug, serde::Serialize)]
struct GoogleEventConferenceCreateRequest {
    #[serde(rename = "requestId")]
    request_id: String,
    #[serde(rename = "conferenceSolutionKey")]
    conference_solution_key: GoogleConferenceSolutionKey,
}

#[derive(Debug, serde::Serialize)]
struct GoogleConferenceSolutionKey {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct GoogleInsertedEvent {
    id: String,
    summary: Option<String>,
    #[serde(rename = "htmlLink")]
    html_link: Option<String>,
    start: GoogleEventTime,
    end: GoogleEventTime,
}

#[derive(Debug, Clone)]
struct EventMatchCandidate {
    event: CalendarEventModel,
    confidence: f64,
    reason: String,
}

pub async fn get_status<R: Runtime>(app: &AppHandle<R>) -> Result<CalendarStatusResponse, String> {
    let client_configured = google_client_id().is_some();
    let syncing = {
        let state = app.state::<CalendarManagerState>();
        let guard = state.0.lock().await;
        guard.is_sync_in_progress
    };

    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(CalendarStatusResponse {
            client_configured,
            connected: false,
            can_write: false,
            syncing,
            account: None,
        });
    };

    let pool = app_state.db_manager.pool();
    let account = CalendarRepository::get_google_account(pool)
        .await
        .map_err(|e| format!("Failed to load calendar account: {}", e))?;

    Ok(CalendarStatusResponse {
        client_configured,
        connected: account
            .as_ref()
            .map(|item| item.connection_status == "connected")
            .unwrap_or(false),
        can_write: account
            .as_ref()
            .map(account_has_write_scope)
            .unwrap_or(false),
        syncing,
        account: account.map(account_to_summary),
    })
}

pub async fn list_upcoming_events<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<Vec<UpcomingCalendarEvent>, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(Vec::new());
    };
    let pool = app_state.db_manager.pool();

    let Some(account) = CalendarRepository::get_google_account(pool)
        .await
        .map_err(|e| format!("Failed to load Google Calendar account: {}", e))?
    else {
        return Ok(Vec::new());
    };

    if !account_allows_cached_reads(&account) {
        return Ok(Vec::new());
    }

    let now = Utc::now();
    let from = (now - Duration::minutes(30)).to_rfc3339();
    let until = (now + Duration::hours(24)).to_rfc3339();

    let events = CalendarRepository::list_upcoming_events(pool, &account.id, &from, &until)
        .await
        .map_err(|e| format!("Failed to load upcoming events: {}", e))?;

    Ok(events
        .into_iter()
        .map(|e| {
            let attendees: Vec<CalendarAttendeeSummary> = e
                .attendees_json
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default();
            UpcomingCalendarEvent {
                provider_event_id: e.provider_event_id,
                title: e.title,
                start_at: e.start_at,
                end_at: e.end_at,
                organizer_email: e.organizer_email,
                organizer_name: e.organizer_name,
                attendees,
                conference_url: e.conference_url,
                html_link: e.html_link,
            }
        })
        .collect())
}

pub async fn connect_google<R: Runtime>(
    app: &AppHandle<R>,
    write_access: bool,
) -> Result<CalendarStatusResponse, String> {
    let client_id = google_client_id()
        .ok_or_else(|| "FRIDAY_GOOGLE_CLIENT_ID is not configured".to_string())?;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to start OAuth callback listener: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to read OAuth callback address: {}", e))?
        .port();

    // Google desktop OAuth expects a loopback redirect on the local listener.
    let redirect_uri = format!("http://127.0.0.1:{}", port);
    let client_secret = google_client_secret();
    let oauth_client =
        build_oauth_client(&client_id, client_secret.as_deref(), Some(&redirect_uri))?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_state) = oauth_client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(requested_google_scopes(write_access))
        .set_pkce_challenge(pkce_challenge)
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .url();

    open_browser(auth_url.as_str())?;

    let (code, received_state) = wait_for_callback(listener).await?;
    if received_state != csrf_state.secret().as_str() {
        return Err("OAuth state validation failed".to_string());
    }

    let token_response = oauth_client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.secret().to_string()))
        .request_async(async_http_client)
        .await
        .map_err(|e| format!("Failed to exchange OAuth code: {:?}", e))?;

    let tokens = StoredOAuthTokens {
        access_token: token_response.access_token().secret().to_string(),
        refresh_token: token_response
            .refresh_token()
            .map(|value| value.secret().to_string()),
        expires_at: token_response
            .expires_in()
            .and_then(|value| chrono::Duration::from_std(value).ok())
            .map(|value| (Utc::now() + value).to_rfc3339()),
    };

    store_tokens(&tokens)?;

    let primary_calendar = fetch_primary_calendar(&tokens.access_token).await?;
    let email = primary_calendar.id;

    let Some(app_state) = app.try_state::<AppState>() else {
        return Err("Database is not initialized yet".to_string());
    };

    CalendarRepository::upsert_account(
        app_state.db_manager.pool(),
        GOOGLE_ACCOUNT_ID,
        GOOGLE_PROVIDER,
        Some(email.as_str()),
        &serde_json::to_string(&google_scope_strings(write_access))
            .unwrap_or_else(|_| "[]".to_string()),
        "connected",
        None,
        None,
    )
    .await
    .map_err(|e| format!("Failed to save calendar account: {}", e))?;

    let _ = sync_google_calendar(app).await?;
    get_status(app).await
}

pub async fn create_google_calendar_event_from_agent<R: Runtime>(
    app: &AppHandle<R>,
    draft: &CalendarEventDraftPayload,
) -> Result<CreatedCalendarEventSummary, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Err("Database is not initialized yet".to_string());
    };
    let pool = app_state.db_manager.pool();
    let Some(account) = CalendarRepository::get_google_account(pool)
        .await
        .map_err(|e| format!("Failed to load Google Calendar account: {}", e))?
    else {
        return Err("Google Calendar is not connected".to_string());
    };

    if account.connection_status != "connected" {
        return Err("Google Calendar is not connected".to_string());
    }
    if !account_has_write_scope(&account) {
        return Err(
            "Reconnect Google Calendar with write access before creating events".to_string(),
        );
    }

    let access_token = ensure_access_token(&account).await?;
    let created = insert_google_calendar_event(&access_token, draft).await?;
    if let Err(err) = sync_google_calendar(app).await {
        log::warn!("Google Calendar sync after event creation failed: {}", err);
    }
    Ok(created)
}

pub async fn disconnect_google<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<CalendarStatusResponse, String> {
    clear_tokens()?;

    if let Some(app_state) = app.try_state::<AppState>() {
        CalendarRepository::update_account_status(
            app_state.db_manager.pool(),
            GOOGLE_ACCOUNT_ID,
            "disconnected",
            None,
            None,
        )
        .await
        .map_err(|e| format!("Failed to disconnect Google Calendar: {}", e))?;
    }

    {
        let state = app.state::<CalendarManagerState>();
        let mut guard = state.0.lock().await;
        guard.pending_recording_link = None;
    }

    get_status(app).await
}

pub async fn sync_google_calendar<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<CalendarSyncResult, String> {
    set_sync_in_progress(app, true).await?;

    let result = sync_google_calendar_inner(app).await;

    if let Err(err) = set_sync_in_progress(app, false).await {
        log::warn!("Failed to clear calendar sync state: {}", err);
    }

    if result.is_ok() {
        crate::agent::service::trigger_calendar_sync(app.clone());
    }

    result
}

pub fn start_background_sync_loop<R: Runtime + 'static>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            BACKGROUND_SYNC_INTERVAL_SECONDS,
        ));

        loop {
            interval.tick().await;
            match get_status(&app).await {
                Ok(status) if status.connected => {
                    if let Err(err) = sync_google_calendar(&app).await {
                        log::warn!("Background Google Calendar sync failed: {}", err);
                    }
                }
                Ok(_) => {}
                Err(err) => {
                    log::warn!("Failed to inspect Google Calendar status: {}", err);
                }
            }
        }
    });
}

pub async fn prepare_pending_recording_match<R: Runtime>(
    app: &AppHandle<R>,
    meeting_name: Option<&str>,
) -> Result<(), String> {
    {
        let state = app.state::<CalendarManagerState>();
        let mut guard = state.0.lock().await;
        guard.pending_recording_link = None;
    }

    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(());
    };
    let pool = app_state.db_manager.pool();

    let Some(account) = CalendarRepository::get_google_account(pool)
        .await
        .map_err(|e| format!("Failed to load Google Calendar account: {}", e))?
    else {
        return Ok(());
    };

    if account.connection_status != "connected" {
        return Ok(());
    }

    if should_refresh_before_matching(&account) {
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(err) = sync_google_calendar(&app_handle).await {
                log::warn!("Calendar refresh before matching failed: {}", err);
            }
        });
    }

    let reference_time = Utc::now();
    let window_start = (reference_time - Duration::minutes(AUTO_MATCH_WINDOW_MINUTES)).to_rfc3339();
    let window_end = (reference_time + Duration::minutes(AUTO_MATCH_WINDOW_MINUTES)).to_rfc3339();
    let events =
        CalendarRepository::list_events_for_matching(pool, &account.id, &window_start, &window_end)
            .await
            .map_err(|e| format!("Failed to load candidate events: {}", e))?;

    let best_candidate = choose_best_match(meeting_name, &events, reference_time);
    if let Some(candidate) = best_candidate {
        let state = app.state::<CalendarManagerState>();
        let mut guard = state.0.lock().await;
        guard.pending_recording_link = Some(PendingRecordingLink {
            account_id: account.id,
            provider_event_id: candidate.event.provider_event_id,
            confidence: candidate.confidence,
            reason: candidate.reason,
            created_at: reference_time.to_rfc3339(),
        });
    }

    Ok(())
}

pub async fn persist_pending_link_for_meeting<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
) -> Result<(), String> {
    let pending = {
        let state = app.state::<CalendarManagerState>();
        let guard = state.0.lock().await;
        guard.pending_recording_link.clone()
    };

    let Some(pending) = pending else {
        return Ok(());
    };

    let Some(created_at) = parse_rfc3339(&pending.created_at) else {
        clear_pending_link(app).await;
        return Ok(());
    };

    if Utc::now() - created_at > Duration::hours(6) {
        clear_pending_link(app).await;
        return Ok(());
    }

    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(());
    };
    let pool = app_state.db_manager.pool();

    if CalendarRepository::get_event(pool, &pending.account_id, &pending.provider_event_id)
        .await
        .map_err(|e| format!("Failed to validate linked calendar event: {}", e))?
        .is_none()
    {
        clear_pending_link(app).await;
        return Ok(());
    }

    CalendarRepository::set_meeting_link(
        pool,
        meeting_id,
        &pending.account_id,
        &pending.provider_event_id,
        pending.confidence,
        "auto",
        Some(&pending.reason),
    )
    .await
    .map_err(|e| format!("Failed to persist Google Calendar link: {}", e))?;

    clear_pending_link(app).await;
    Ok(())
}

pub async fn get_meeting_link<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
) -> Result<Option<LinkedCalendarEvent>, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(None);
    };

    let row = CalendarRepository::get_meeting_link(app_state.db_manager.pool(), meeting_id)
        .await
        .map_err(|e| format!("Failed to load meeting link: {}", e))?;

    Ok(row.map(row_to_linked_event))
}

pub async fn get_link_candidates<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
) -> Result<Vec<CalendarLinkCandidate>, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(Vec::new());
    };
    let pool = app_state.db_manager.pool();

    let Some(account) = CalendarRepository::get_google_account(pool)
        .await
        .map_err(|e| format!("Failed to load Google Calendar account: {}", e))?
    else {
        return Ok(Vec::new());
    };

    let Some((meeting_title, meeting_created_at)) =
        CalendarRepository::get_meeting_context(pool, meeting_id)
            .await
            .map_err(|e| format!("Failed to load meeting context: {}", e))?
    else {
        return Ok(Vec::new());
    };

    let Some(reference_time) = parse_rfc3339(&meeting_created_at) else {
        return Ok(Vec::new());
    };

    let window_start = (reference_time - Duration::hours(CANDIDATE_LOOKBACK_HOURS)).to_rfc3339();
    let window_end = (reference_time + Duration::hours(CANDIDATE_LOOKAHEAD_HOURS)).to_rfc3339();
    let events =
        CalendarRepository::list_events_for_matching(pool, &account.id, &window_start, &window_end)
            .await
            .map_err(|e| format!("Failed to load link candidates: {}", e))?;

    let mut candidates = events
        .into_iter()
        .map(|event| {
            let (confidence, reason) =
                score_event_match(Some(&meeting_title), &event, reference_time);
            CalendarLinkCandidate {
                provider_event_id: event.provider_event_id,
                title: event.title,
                start_at: event.start_at,
                end_at: event.end_at,
                organizer_email: event.organizer_email,
                organizer_name: event.organizer_name,
                conference_url: event.conference_url,
                html_link: event.html_link,
                confidence,
                reason,
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| right.confidence.total_cmp(&left.confidence));
    candidates.truncate(8);
    Ok(candidates)
}

pub async fn set_meeting_link<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
    provider_event_id: &str,
) -> Result<Option<LinkedCalendarEvent>, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(None);
    };
    let pool = app_state.db_manager.pool();

    let Some(account) = CalendarRepository::get_google_account(pool)
        .await
        .map_err(|e| format!("Failed to load Google Calendar account: {}", e))?
    else {
        return Ok(None);
    };

    let Some(event) = CalendarRepository::get_event(pool, &account.id, provider_event_id)
        .await
        .map_err(|e| format!("Failed to load selected calendar event: {}", e))?
    else {
        return Err("Selected calendar event was not found in the local cache".to_string());
    };

    let meeting_context = CalendarRepository::get_meeting_context(pool, meeting_id)
        .await
        .map_err(|e| format!("Failed to load meeting context: {}", e))?;
    let (meeting_title, reference_time) = meeting_context
        .and_then(|(title, created_at)| parse_rfc3339(&created_at).map(|time| (title, time)))
        .unwrap_or_else(|| ("Meeting".to_string(), Utc::now()));
    let (confidence, reason) = score_event_match(Some(&meeting_title), &event, reference_time);

    CalendarRepository::set_meeting_link(
        pool,
        meeting_id,
        &account.id,
        provider_event_id,
        confidence,
        "manual",
        Some(&reason),
    )
    .await
    .map_err(|e| format!("Failed to save meeting link: {}", e))?;

    get_meeting_link(app, meeting_id).await
}

pub async fn clear_meeting_link_for_meeting<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
) -> Result<(), String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(());
    };

    CalendarRepository::clear_meeting_link(app_state.db_manager.pool(), meeting_id)
        .await
        .map_err(|e| format!("Failed to clear meeting link: {}", e))
}

fn google_client_id() -> Option<String> {
    std::env::var("FRIDAY_GOOGLE_CLIENT_ID")
        .ok()
        .or_else(|| std::env::var("GOOGLE_CALENDAR_CLIENT_ID").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn google_client_secret() -> Option<String> {
    std::env::var("FRIDAY_GOOGLE_CLIENT_SECRET")
        .ok()
        .or_else(|| std::env::var("GOOGLE_CALENDAR_CLIENT_SECRET").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn requested_google_scopes(write_access: bool) -> Vec<Scope> {
    google_scope_strings(write_access)
        .into_iter()
        .map(Scope::new)
        .collect()
}

fn google_scope_strings(write_access: bool) -> Vec<String> {
    let mut scopes = vec![GOOGLE_SCOPE.to_string()];
    if write_access {
        scopes.push(GOOGLE_WRITE_SCOPE.to_string());
    }
    scopes
}

fn build_oauth_client(
    client_id: &str,
    client_secret: Option<&str>,
    redirect_uri: Option<&str>,
) -> Result<BasicClient, String> {
    let client = BasicClient::new(
        ClientId::new(client_id.to_string()),
        client_secret.map(|value| ClientSecret::new(value.to_string())),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| format!("Invalid Google auth URL: {}", e))?,
        Some(
            TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                .map_err(|e| format!("Invalid Google token URL: {}", e))?,
        ),
    );

    if let Some(redirect_uri) = redirect_uri {
        client
            .set_redirect_uri(
                RedirectUrl::new(redirect_uri.to_string())
                    .map_err(|e| format!("Invalid Google redirect URL: {}", e))?,
            )
            .pipe(Ok)
    } else {
        Ok(client)
    }
}

async fn wait_for_callback(listener: TcpListener) -> Result<(String, String), String> {
    let (mut socket, _) = listener
        .accept()
        .await
        .map_err(|e| format!("Failed to receive OAuth callback: {}", e))?;

    let mut buffer = [0_u8; 4096];
    let bytes_read = socket
        .read(&mut buffer)
        .await
        .map_err(|e| format!("Failed to read OAuth callback: {}", e))?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| "Google OAuth callback request was malformed".to_string())?;

    let parsed_url = Url::parse(&format!("http://localhost{}", path))
        .map_err(|e| format!("Failed to parse OAuth callback: {}", e))?;
    let code = parsed_url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.to_string())
        .ok_or_else(|| "Google OAuth callback did not include a code".to_string())?;
    let state = parsed_url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .ok_or_else(|| "Google OAuth callback did not include a state".to_string())?;

    let response = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Type: text/html; charset=utf-8\r\n",
        "Connection: close\r\n\r\n",
        "<html><body><h2>Friday connected Google Calendar.</h2>",
        "<p>You can close this window and return to the app.</p></body></html>"
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;

    Ok((code, state))
}

fn open_browser(url: &str) -> Result<(), String> {
    let result = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", url]).output()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).output()
    } else {
        Command::new("xdg-open").arg(url).output()
    };

    result
        .map(|_| ())
        .map_err(|e| format!("Failed to open Google OAuth in the browser: {}", e))
}

async fn sync_google_calendar_inner<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<CalendarSyncResult, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Err("Database is not initialized yet".to_string());
    };
    let pool = app_state.db_manager.pool();
    let Some(account) = CalendarRepository::get_google_account(pool)
        .await
        .map_err(|e| format!("Failed to load Google Calendar account: {}", e))?
    else {
        return Err("Google Calendar is not connected".to_string());
    };

    if account.connection_status != "connected" {
        return Err("Google Calendar is not connected".to_string());
    }

    let now = Utc::now();
    let window_start = (now - Duration::hours(WINDOW_PAST_HOURS)).to_rfc3339();
    let window_end = (now + Duration::days(WINDOW_FUTURE_DAYS)).to_rfc3339();
    let sync_started_at = now.to_rfc3339();

    CalendarRepository::upsert_sync_state(
        pool,
        &account.id,
        Some(&window_start),
        Some(&window_end),
        Some(&sync_started_at),
        None,
        None,
        None,
    )
    .await
    .map_err(|e| format!("Failed to mark calendar sync as started: {}", e))?;

    let access_token = match ensure_access_token(&account).await {
        Ok(token) => token,
        Err(err) => {
            let (next_status, next_error, user_error) = if is_missing_keychain_tokens_error(&err) {
                (
                    "disconnected",
                    None,
                    "Google Calendar session was missing. Reconnect Google Calendar.".to_string(),
                )
            } else {
                ("error", Some(err.as_str()), err.clone())
            };

            CalendarRepository::update_account_status(
                pool,
                &account.id,
                next_status,
                next_error,
                None,
            )
            .await
            .map_err(|e| format!("Failed to persist calendar auth error: {}", e))?;
            CalendarRepository::upsert_sync_state(
                pool,
                &account.id,
                Some(&window_start),
                Some(&window_end),
                Some(&sync_started_at),
                Some(&Utc::now().to_rfc3339()),
                None,
                Some(&user_error),
            )
            .await
            .map_err(|e| format!("Failed to persist calendar sync error: {}", e))?;
            return Err(user_error);
        }
    };

    let events = match fetch_events(&access_token, &window_start, &window_end).await {
        Ok(events) => events,
        Err(err) => {
            CalendarRepository::update_account_status(pool, &account.id, "error", Some(&err), None)
                .await
                .map_err(|e| format!("Failed to persist calendar fetch error: {}", e))?;
            CalendarRepository::upsert_sync_state(
                pool,
                &account.id,
                Some(&window_start),
                Some(&window_end),
                Some(&sync_started_at),
                Some(&Utc::now().to_rfc3339()),
                None,
                Some(&err),
            )
            .await
            .map_err(|e| format!("Failed to persist calendar sync error: {}", e))?;
            return Err(err);
        }
    };

    CalendarRepository::upsert_events_for_window(
        pool,
        &account.id,
        &window_start,
        &window_end,
        &events,
    )
    .await
    .map_err(|e| format!("Failed to store synced calendar events: {}", e))?;

    let synced_at = Utc::now().to_rfc3339();
    CalendarRepository::update_account_status(
        pool,
        &account.id,
        "connected",
        None,
        Some(&synced_at),
    )
    .await
    .map_err(|e| format!("Failed to update calendar sync status: {}", e))?;
    CalendarRepository::upsert_sync_state(
        pool,
        &account.id,
        Some(&window_start),
        Some(&window_end),
        Some(&sync_started_at),
        Some(&synced_at),
        Some(&synced_at),
        None,
    )
    .await
    .map_err(|e| format!("Failed to update calendar sync state: {}", e))?;

    Ok(CalendarSyncResult {
        synced_events: events.len(),
        synced_at,
    })
}

async fn ensure_access_token(account: &ConnectedAccountModel) -> Result<String, String> {
    let mut stored_tokens =
        load_tokens()?.ok_or_else(|| MISSING_KEYCHAIN_TOKENS_ERROR.to_string())?;

    let expires_at = stored_tokens.expires_at.as_deref().and_then(parse_rfc3339);
    let still_valid = expires_at
        .map(|value| value > Utc::now() + Duration::seconds(60))
        .unwrap_or(true);

    if still_valid {
        return Ok(stored_tokens.access_token);
    }

    let refresh_token = stored_tokens.refresh_token.clone().ok_or_else(|| {
        "Google Calendar token expired and no refresh token is available".to_string()
    })?;
    let client_id = google_client_id()
        .ok_or_else(|| "FRIDAY_GOOGLE_CLIENT_ID is not configured".to_string())?;
    let client_secret = google_client_secret();
    let oauth_client = build_oauth_client(&client_id, client_secret.as_deref(), None)?;
    let token_response = oauth_client
        .exchange_refresh_token(&RefreshToken::new(refresh_token))
        .request_async(async_http_client)
        .await
        .map_err(|e| format!("Failed to refresh Google Calendar token: {:?}", e))?;

    stored_tokens.access_token = token_response.access_token().secret().to_string();
    stored_tokens.refresh_token = token_response
        .refresh_token()
        .map(|value| value.secret().to_string())
        .or(stored_tokens.refresh_token);
    stored_tokens.expires_at = token_response
        .expires_in()
        .and_then(|value| chrono::Duration::from_std(value).ok())
        .map(|value| (Utc::now() + value).to_rfc3339());

    store_tokens(&stored_tokens)?;

    let _ = account;
    Ok(stored_tokens.access_token)
}

fn is_missing_keychain_tokens_error(err: &str) -> bool {
    err.contains(MISSING_KEYCHAIN_TOKENS_ERROR)
}

async fn fetch_primary_calendar(access_token: &str) -> Result<GoogleCalendarResource, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://www.googleapis.com/calendar/v3/calendars/primary")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch primary Google Calendar: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read response body>".to_string());
        return Err(format!(
            "Failed to fetch primary Google Calendar: HTTP {}: {}",
            status, body
        ));
    }

    response
        .json::<GoogleCalendarResource>()
        .await
        .map_err(|e| format!("Failed to parse primary Google Calendar response: {}", e))
}

async fn fetch_events(
    access_token: &str,
    window_start: &str,
    window_end: &str,
) -> Result<Vec<UpsertCalendarEvent>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
        .bearer_auth(access_token)
        .query(&[
            ("singleEvents", "true"),
            ("orderBy", "startTime"),
            ("timeMin", window_start),
            ("timeMax", window_end),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Google Calendar events: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read response body>".to_string());
        return Err(format!(
            "Failed to fetch Google Calendar events: HTTP {}: {}",
            status, body
        ));
    }

    let parsed = response
        .json::<GoogleEventsResponse>()
        .await
        .map_err(|e| format!("Failed to parse Google Calendar events response: {}", e))?;

    let mut seen_ids = HashSet::new();
    let mut events = Vec::new();
    for event in parsed.items {
        if event.status.as_deref() == Some("cancelled") {
            continue;
        }

        let Some(start_at) = normalize_google_event_time(&event.start) else {
            continue;
        };
        let Some(end_at) = normalize_google_event_time(&event.end) else {
            continue;
        };

        if !seen_ids.insert(event.id.clone()) {
            continue;
        }

        let attendees = event
            .attendees
            .unwrap_or_default()
            .into_iter()
            .map(|attendee| StoredAttendee {
                email: attendee.email,
                display_name: attendee.display_name,
                response_status: attendee.response_status,
            })
            .collect::<Vec<_>>();

        let conference_url = event.hangout_link.or_else(|| {
            event.conference_data.and_then(|data| {
                data.entry_points
                    .unwrap_or_default()
                    .into_iter()
                    .find_map(|entry| entry.uri)
            })
        });

        events.push(UpsertCalendarEvent {
            provider_event_id: event.id,
            calendar_id: "primary".to_string(),
            title: event
                .summary
                .unwrap_or_else(|| "Untitled Event".to_string()),
            description: event.description,
            organizer_email: event
                .organizer
                .as_ref()
                .and_then(|value| value.email.clone()),
            organizer_name: event
                .organizer
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            attendees_json: serde_json::to_string(&attendees).ok(),
            start_at,
            end_at,
            timezone: event.start.time_zone.or(event.end.time_zone),
            conference_url,
            status: event.status.unwrap_or_else(|| "confirmed".to_string()),
            html_link: event.html_link,
            raw_etag: event.etag,
            is_primary_calendar: true,
        });
    }

    Ok(events)
}

async fn insert_google_calendar_event(
    access_token: &str,
    draft: &CalendarEventDraftPayload,
) -> Result<CreatedCalendarEventSummary, String> {
    let request = GoogleEventInsertRequest {
        summary: draft.title.clone(),
        description: draft.description.clone(),
        location: draft.location.clone(),
        start: GoogleEventInsertDateTime {
            date_time: draft.start_at.clone(),
            time_zone: draft.timezone.clone(),
        },
        end: GoogleEventInsertDateTime {
            date_time: draft.end_at.clone(),
            time_zone: draft.timezone.clone(),
        },
        attendees: (!draft.attendees.is_empty()).then(|| {
            draft
                .attendees
                .iter()
                .cloned()
                .map(|email| GoogleEventInsertAttendee { email })
                .collect::<Vec<_>>()
        }),
        conference_data: draft
            .conference_request
            .then(|| GoogleEventInsertConferenceData {
                create_request: GoogleEventConferenceCreateRequest {
                    request_id: format!("friday-{}", uuid::Uuid::new_v4()),
                    conference_solution_key: GoogleConferenceSolutionKey {
                        kind: "hangoutsMeet".to_string(),
                    },
                },
            }),
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://www.googleapis.com/calendar/v3/calendars/primary/events")
        .bearer_auth(access_token)
        .query(&[(
            "conferenceDataVersion",
            if draft.conference_request { "1" } else { "0" },
        )])
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Failed to create Google Calendar event: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read response body>".to_string());
        return Err(format!(
            "Failed to create Google Calendar event: HTTP {}: {}",
            status, body
        ));
    }

    let created = response.json::<GoogleInsertedEvent>().await.map_err(|e| {
        format!(
            "Failed to parse Google Calendar event creation response: {}",
            e
        )
    })?;

    Ok(CreatedCalendarEventSummary {
        provider_event_id: created.id,
        title: created.summary.unwrap_or_else(|| draft.title.clone()),
        start_at: normalize_google_event_time(&created.start)
            .unwrap_or_else(|| draft.start_at.clone()),
        end_at: normalize_google_event_time(&created.end).unwrap_or_else(|| draft.end_at.clone()),
        html_link: created.html_link,
    })
}

fn normalize_google_event_time(event_time: &GoogleEventTime) -> Option<String> {
    if let Some(date_time) = event_time.date_time.as_deref() {
        return parse_rfc3339(date_time).map(|value| value.to_rfc3339());
    }

    let date = event_time.date.as_deref()?;
    let naive_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
    Some(DateTime::<Utc>::from_naive_utc_and_offset(naive_datetime, Utc).to_rfc3339())
}

fn choose_best_match(
    meeting_name: Option<&str>,
    events: &[CalendarEventModel],
    reference_time: DateTime<Utc>,
) -> Option<EventMatchCandidate> {
    let mut candidates = events
        .iter()
        .cloned()
        .map(|event| {
            let (confidence, reason) = score_event_match(meeting_name, &event, reference_time);
            EventMatchCandidate {
                event,
                confidence,
                reason,
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| right.confidence.total_cmp(&left.confidence));
    let best = candidates.first()?.clone();
    let second_best_score = candidates.get(1).map(|item| item.confidence).unwrap_or(0.0);

    if best.confidence >= 0.60 && (best.confidence - second_best_score) >= 0.15 {
        Some(best)
    } else {
        None
    }
}

fn score_event_match(
    meeting_name: Option<&str>,
    event: &CalendarEventModel,
    reference_time: DateTime<Utc>,
) -> (f64, String) {
    let start_at = parse_rfc3339(&event.start_at).unwrap_or(reference_time);
    let end_at = parse_rfc3339(&event.end_at).unwrap_or(reference_time);

    let temporal_score = if reference_time >= start_at && reference_time <= end_at {
        1.0
    } else {
        let diff_minutes = (start_at - reference_time).num_minutes().abs() as f64;
        (1.0 - (diff_minutes / AUTO_MATCH_WINDOW_MINUTES as f64)).clamp(0.0, 1.0)
    };

    let title_score = if is_generic_meeting_name(meeting_name) {
        0.0
    } else {
        title_similarity(meeting_name.unwrap_or_default(), event.title.as_str())
    };

    let score = (temporal_score * 0.75) + (title_score * 0.25);
    let temporal_reason = if reference_time >= start_at && reference_time <= end_at {
        "Event is currently in progress"
    } else if start_at > reference_time {
        "Event starts close to the recording time"
    } else {
        "Event ended recently and overlaps the recording window"
    };

    let reason = if title_score >= 0.50 {
        format!(
            "{}, and the title closely matches the recording name",
            temporal_reason
        )
    } else {
        temporal_reason.to_string()
    };

    (score, reason)
}

fn title_similarity(left: &str, right: &str) -> f64 {
    let left_tokens = tokenize_title(left);
    let right_tokens = tokenize_title(right);

    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }

    let left_set = left_tokens.into_iter().collect::<HashSet<_>>();
    let right_set = right_tokens.into_iter().collect::<HashSet<_>>();
    let intersection = left_set.intersection(&right_set).count() as f64;
    let union = left_set.union(&right_set).count() as f64;

    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn tokenize_title(value: &str) -> Vec<String> {
    value
        .split(|char: char| !char.is_ascii_alphanumeric())
        .map(|token| token.trim().to_lowercase())
        .filter(|token| token.len() > 1)
        .collect()
}

fn is_generic_meeting_name(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return true;
    };
    let trimmed = value.trim();
    trimmed.is_empty()
        || (trimmed.starts_with("Meeting ") && trimmed.contains('_'))
        || trimmed == "+ New Call"
        || trimmed == "New Meeting"
}

fn should_refresh_before_matching(account: &ConnectedAccountModel) -> bool {
    account
        .last_sync_at
        .as_deref()
        .and_then(parse_rfc3339)
        .map(|value| Utc::now() - value > Duration::minutes(MATCH_SYNC_FRESHNESS_MINUTES))
        .unwrap_or(true)
}

fn account_to_summary(account: ConnectedAccountModel) -> CalendarAccountSummary {
    CalendarAccountSummary {
        email: account.email,
        connection_status: account.connection_status,
        last_sync_at: account.last_sync_at,
        last_error: account.last_error,
        scopes: serde_json::from_str::<Vec<String>>(&account.scopes_json).unwrap_or_default(),
    }
}

fn account_has_write_scope(account: &ConnectedAccountModel) -> bool {
    serde_json::from_str::<Vec<String>>(&account.scopes_json)
        .unwrap_or_default()
        .into_iter()
        .any(|scope| scope == GOOGLE_WRITE_SCOPE)
}

fn account_allows_cached_reads(account: &ConnectedAccountModel) -> bool {
    account.connection_status != "disconnected"
}

fn row_to_linked_event(row: MeetingLinkedEventRow) -> LinkedCalendarEvent {
    LinkedCalendarEvent {
        provider_event_id: row.provider_event_id,
        title: row.title,
        description: row.description,
        organizer_email: row.organizer_email,
        organizer_name: row.organizer_name,
        attendees: parse_attendees(row.attendees_json),
        start_at: row.start_at,
        end_at: row.end_at,
        timezone: row.timezone,
        conference_url: row.conference_url,
        status: row.status,
        html_link: row.html_link,
        confidence: row.confidence,
        link_method: row.link_method,
        reason: row.reason,
        linked_at: row.linked_at,
    }
}

fn parse_attendees(raw: Option<String>) -> Vec<CalendarAttendeeSummary> {
    raw.and_then(|value| serde_json::from_str::<Vec<StoredAttendee>>(&value).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|attendee| CalendarAttendeeSummary {
            email: attendee.email,
            display_name: attendee.display_name,
            response_status: attendee.response_status,
        })
        .collect()
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn keyring_entry() -> Result<Entry, String> {
    Entry::new(TOKEN_KEYRING_SERVICE, TOKEN_KEYRING_USERNAME)
        .map_err(|e| format!("Failed to access the system keychain: {}", e))
}

fn load_tokens() -> Result<Option<StoredOAuthTokens>, String> {
    let entry = keyring_entry()?;
    match entry.get_password() {
        Ok(value) => serde_json::from_str(&value)
            .map(Some)
            .map_err(|e| format!("Failed to parse Google Calendar tokens: {}", e)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(err) => Err(format!("Failed to read Google Calendar tokens: {}", err)),
    }
}

fn store_tokens(tokens: &StoredOAuthTokens) -> Result<(), String> {
    let entry = keyring_entry()?;
    let serialized = serde_json::to_string(tokens)
        .map_err(|e| format!("Failed to serialize Google Calendar tokens: {}", e))?;
    entry
        .set_password(&serialized)
        .map_err(|e| format!("Failed to store Google Calendar tokens: {}", e))
}

fn clear_tokens() -> Result<(), String> {
    let entry = keyring_entry()?;
    match entry.delete_credential() {
        Ok(_) | Err(KeyringError::NoEntry) => Ok(()),
        Err(err) => Err(format!("Failed to clear Google Calendar tokens: {}", err)),
    }
}

async fn clear_pending_link<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<CalendarManagerState>();
    let mut guard = state.0.lock().await;
    guard.pending_recording_link = None;
}

async fn set_sync_in_progress<R: Runtime>(
    app: &AppHandle<R>,
    in_progress: bool,
) -> Result<(), String> {
    let state = app.state::<CalendarManagerState>();
    let mut guard = state.0.lock().await;
    if in_progress && guard.is_sync_in_progress {
        return Err("Google Calendar sync is already running".to_string());
    }
    guard.is_sync_in_progress = in_progress;
    Ok(())
}

trait Pipe: Sized {
    fn pipe<T>(self, op: impl FnOnce(Self) -> T) -> T {
        op(self)
    }
}

impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(title: &str, start_at: &str, end_at: &str) -> CalendarEventModel {
        CalendarEventModel {
            account_id: GOOGLE_ACCOUNT_ID.to_string(),
            provider_event_id: title.to_string(),
            calendar_id: "primary".to_string(),
            title: title.to_string(),
            description: None,
            organizer_email: None,
            organizer_name: None,
            attendees_json: None,
            start_at: start_at.to_string(),
            end_at: end_at.to_string(),
            timezone: None,
            conference_url: None,
            status: "confirmed".to_string(),
            html_link: None,
            raw_etag: None,
            is_primary_calendar: 1,
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn title_similarity_prefers_shared_words() {
        let score = title_similarity("Customer Weekly Sync", "Weekly Customer Sync");
        assert!(score > 0.5);
    }

    #[test]
    fn generic_title_drops_title_score() {
        let reference = Utc::now();
        let event = event(
            "Weekly Customer Sync",
            &(reference - Duration::minutes(5)).to_rfc3339(),
            &(reference + Duration::minutes(25)).to_rfc3339(),
        );
        let with_generic =
            score_event_match(Some("Meeting 2026-03-07_09-00-00"), &event, reference).0;
        let with_real_title = score_event_match(Some("Weekly Customer Sync"), &event, reference).0;
        assert!(with_real_title > with_generic);
    }

    #[test]
    fn best_match_requires_clear_margin() {
        let reference = Utc::now();
        let events = vec![
            event(
                "Weekly Customer Sync",
                &(reference - Duration::minutes(2)).to_rfc3339(),
                &(reference + Duration::minutes(28)).to_rfc3339(),
            ),
            event(
                "Internal Weekly Customer Sync",
                &(reference - Duration::minutes(1)).to_rfc3339(),
                &(reference + Duration::minutes(29)).to_rfc3339(),
            ),
        ];
        assert!(choose_best_match(Some("Weekly Customer Sync"), &events, reference).is_none());
    }

    #[test]
    fn cached_reads_are_allowed_while_account_is_in_error_state() {
        let account = ConnectedAccountModel {
            id: GOOGLE_ACCOUNT_ID.to_string(),
            provider: GOOGLE_PROVIDER.to_string(),
            email: Some("person@example.com".to_string()),
            scopes_json: "[]".to_string(),
            connection_status: "error".to_string(),
            last_sync_at: None,
            last_error: Some("Temporary keychain failure".to_string()),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        assert!(account_allows_cached_reads(&account));
    }

    #[test]
    fn cached_reads_are_blocked_after_disconnect() {
        let account = ConnectedAccountModel {
            id: GOOGLE_ACCOUNT_ID.to_string(),
            provider: GOOGLE_PROVIDER.to_string(),
            email: Some("person@example.com".to_string()),
            scopes_json: "[]".to_string(),
            connection_status: "disconnected".to_string(),
            last_sync_at: None,
            last_error: None,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        assert!(!account_allows_cached_reads(&account));
    }
}
