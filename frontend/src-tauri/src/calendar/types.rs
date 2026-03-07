use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarAccountSummary {
    pub email: Option<String>,
    pub connection_status: String,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarStatusResponse {
    pub client_configured: bool,
    pub connected: bool,
    pub can_write: bool,
    pub syncing: bool,
    pub account: Option<CalendarAccountSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarSyncResult {
    pub synced_events: usize,
    pub synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarAttendeeSummary {
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub response_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedCalendarEvent {
    pub provider_event_id: String,
    pub title: String,
    pub description: Option<String>,
    pub organizer_email: Option<String>,
    pub organizer_name: Option<String>,
    pub attendees: Vec<CalendarAttendeeSummary>,
    pub start_at: String,
    pub end_at: String,
    pub timezone: Option<String>,
    pub conference_url: Option<String>,
    pub status: String,
    pub html_link: Option<String>,
    pub confidence: f64,
    pub link_method: String,
    pub reason: Option<String>,
    pub linked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarLinkCandidate {
    pub provider_event_id: String,
    pub title: String,
    pub start_at: String,
    pub end_at: String,
    pub organizer_email: Option<String>,
    pub organizer_name: Option<String>,
    pub conference_url: Option<String>,
    pub html_link: Option<String>,
    pub confidence: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpcomingCalendarEvent {
    pub provider_event_id: String,
    pub title: String,
    pub start_at: String,
    pub end_at: String,
    pub organizer_email: Option<String>,
    pub organizer_name: Option<String>,
    pub attendees: Vec<CalendarAttendeeSummary>,
    pub conference_url: Option<String>,
    pub html_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredOAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredAttendee {
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub response_status: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingRecordingLink {
    pub account_id: String,
    pub provider_event_id: String,
    pub confidence: f64,
    pub reason: String,
    pub created_at: String,
}
