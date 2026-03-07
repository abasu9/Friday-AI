use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSettingsPayload {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub notifications_enabled: bool,
    pub calendar_proposals_enabled: bool,
    pub heartbeat_interval_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusResponse {
    pub settings: AgentSettingsPayload,
    pub api_key_configured: bool,
    pub calendar_connected: bool,
    pub calendar_can_write: bool,
    pub is_running: bool,
    pub last_run_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_error: Option<String>,
    pub pending_recommendations: usize,
    pub open_tasks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemoryItem {
    pub id: String,
    pub memory_type: String,
    pub title: String,
    pub body: String,
    pub source_meeting_id: Option<String>,
    pub source_calendar_event_id: Option<String>,
    pub subject_key: String,
    pub confidence: f64,
    pub status: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub title: String,
    pub body: String,
    pub source_meeting_id: Option<String>,
    pub source_memory_item_id: Option<String>,
    pub owner_kind: String,
    pub due_at: Option<String>,
    pub priority: String,
    pub status: String,
    pub last_suggested_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEventDraftPayload {
    pub title: String,
    pub description: Option<String>,
    pub start_at: String,
    pub end_at: String,
    pub timezone: Option<String>,
    pub location: Option<String>,
    pub attendees: Vec<String>,
    pub conference_request: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecommendation {
    pub id: String,
    pub recommendation_type: String,
    pub title: String,
    pub body: String,
    pub rationale: String,
    pub confidence: f64,
    pub source_meeting_id: Option<String>,
    pub source_calendar_event_id: Option<String>,
    pub task_id: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub status: String,
    pub surfaced_at: String,
    pub acted_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMeetingContextResponse {
    pub memory_items: Vec<AgentMemoryItem>,
    pub tasks: Vec<AgentTask>,
    pub recommendations: Vec<AgentRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedCalendarEventSummary {
    pub provider_event_id: String,
    pub title: String,
    pub start_at: String,
    pub end_at: String,
    pub html_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecommendationActionResponse {
    pub recommendation: AgentRecommendation,
    pub created_calendar_event: Option<CreatedCalendarEventSummary>,
}
