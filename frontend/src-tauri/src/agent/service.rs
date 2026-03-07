use crate::agent::types::{
    AgentMeetingContextResponse, AgentMemoryItem, AgentRecommendation,
    AgentRecommendationActionResponse, AgentSettingsPayload, AgentStatusResponse, AgentTask,
    CalendarEventDraftPayload,
};
use crate::calendar::service as calendar_service;
use crate::database::models::{
    AgentMemoryItemModel, AgentRecommendationModel, AgentSettingModel, AgentTaskModel,
};
use crate::database::repositories::agent::{
    AgentMeetingContextRow, AgentRepository, InsertAgentRecommendation, UpsertAgentMemoryItem,
    UpsertAgentTask,
};
use crate::database::repositories::calendar::CalendarRepository;
use crate::database::repositories::setting::SettingsRepository;
use crate::notifications::commands::NotificationManagerState;
use crate::notifications::types::{
    Notification, NotificationPriority, NotificationTimeout, NotificationType,
};
use crate::state::AppState;
use crate::summary::llm_client::{generate_summary, LLMProvider};
use crate::summary::processor::clean_llm_markdown_output;
use chrono::{DateTime, Duration, Local, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::Mutex;

const DEFAULT_PROVIDER: &str = "gemini";
const DEFAULT_MODEL: &str = "gemini-2.5-flash";
const DEFAULT_HEARTBEAT_INTERVAL_MINUTES: u64 = 5;
const MAX_TRANSCRIPT_CHARS: usize = 12_000;
const MAX_MEETINGS_PER_HEARTBEAT: i64 = 3;
const MEETING_LOOKBACK_DAYS: i64 = 7;
const GOOGLE_CALENDAR_WRITE_SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeartbeatMode {
    Automatic,
    Manual,
}

impl HeartbeatMode {
    fn bypass_enabled_guard(self) -> bool {
        matches!(self, Self::Manual)
    }

    fn reprocess_recent_meetings(self) -> bool {
        matches!(self, Self::Manual)
    }
}

#[derive(Debug, Default)]
pub struct AgentRuntimeState {
    pub is_running: bool,
}

#[derive(Clone, Default)]
pub struct AgentManagerState(pub Arc<Mutex<AgentRuntimeState>>);

#[derive(Debug, Deserialize, Default)]
struct AgentModelOutput {
    #[serde(default)]
    memory_items: Vec<AgentModelMemoryItem>,
    #[serde(default)]
    tasks: Vec<AgentModelTask>,
    #[serde(default)]
    recommendations: Vec<AgentModelRecommendation>,
}

#[derive(Debug, Deserialize, Default)]
struct AgentModelMemoryItem {
    memory_type: String,
    title: String,
    body: String,
    #[serde(default)]
    subject_key: Option<String>,
    #[serde(default)]
    subject_json: Option<serde_json::Value>,
    #[serde(default)]
    confidence: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
struct AgentModelTask {
    title: String,
    body: String,
    #[serde(default)]
    owner_kind: Option<String>,
    #[serde(default)]
    due_at: Option<String>,
    #[serde(default)]
    priority: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct AgentModelRecommendation {
    recommendation_type: String,
    title: String,
    body: String,
    rationale: String,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    related_task_title: Option<String>,
    #[serde(default)]
    payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
struct AnalysisOutcome {
    recommendations: Vec<AgentRecommendationModel>,
}

pub async fn get_status<R: Runtime>(app: &AppHandle<R>) -> Result<AgentStatusResponse, String> {
    let settings = load_settings(app).await?;
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(AgentStatusResponse {
            settings,
            api_key_configured: false,
            calendar_connected: false,
            calendar_can_write: false,
            is_running: false,
            last_run_at: None,
            last_success_at: None,
            last_error: None,
            pending_recommendations: 0,
            open_tasks: 0,
        });
    };

    let pool = app_state.db_manager.pool();
    let settings_model = AgentRepository::get_settings(pool)
        .await
        .map_err(|e| format!("Failed to load agent settings: {}", e))?;
    let pending_recommendations = AgentRepository::count_pending_recommendations(pool)
        .await
        .map_err(|e| format!("Failed to count recommendations: {}", e))?
        as usize;
    let open_tasks = AgentRepository::count_open_tasks(pool)
        .await
        .map_err(|e| format!("Failed to count tasks: {}", e))? as usize;
    let api_key_configured = SettingsRepository::get_api_key(pool, "gemini")
        .await
        .map_err(|e| format!("Failed to inspect Gemini API key: {}", e))?
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    let calendar_account = CalendarRepository::get_google_account(pool)
        .await
        .map_err(|e| format!("Failed to inspect calendar account: {}", e))?;
    let calendar_connected = calendar_account
        .as_ref()
        .map(|account| account.connection_status == "connected")
        .unwrap_or(false);
    let calendar_can_write = calendar_account
        .as_ref()
        .map(account_has_calendar_write_scope)
        .unwrap_or(false);

    let is_running = {
        let state = app.state::<AgentManagerState>();
        let guard = state.0.lock().await;
        guard.is_running
    };

    Ok(AgentStatusResponse {
        settings,
        api_key_configured,
        calendar_connected,
        calendar_can_write,
        is_running,
        last_run_at: settings_model
            .as_ref()
            .and_then(|value| value.last_run_at.clone()),
        last_success_at: settings_model
            .as_ref()
            .and_then(|value| value.last_success_at.clone()),
        last_error: settings_model
            .as_ref()
            .and_then(|value| value.last_error.clone()),
        pending_recommendations,
        open_tasks,
    })
}

pub async fn get_settings<R: Runtime>(app: &AppHandle<R>) -> Result<AgentSettingsPayload, String> {
    load_settings(app).await
}

pub async fn set_settings<R: Runtime>(
    app: &AppHandle<R>,
    settings: AgentSettingsPayload,
) -> Result<AgentStatusResponse, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Err("Database is not initialized yet".to_string());
    };

    AgentRepository::save_settings(
        app_state.db_manager.pool(),
        settings.enabled,
        &settings.provider,
        &settings.model,
        settings.notifications_enabled,
        settings.calendar_proposals_enabled,
        settings.heartbeat_interval_minutes as i64,
    )
    .await
    .map_err(|e| format!("Failed to save agent settings: {}", e))?;

    get_status(app).await
}

pub async fn save_gemini_api_key<R: Runtime>(
    app: &AppHandle<R>,
    api_key: &str,
) -> Result<(), String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Err("Database is not initialized yet".to_string());
    };
    SettingsRepository::save_api_key(app_state.db_manager.pool(), "gemini", api_key)
        .await
        .map_err(|e| format!("Failed to save Gemini API key: {}", e))
}

pub async fn clear_gemini_api_key<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Err("Database is not initialized yet".to_string());
    };
    SettingsRepository::delete_api_key(app_state.db_manager.pool(), "gemini")
        .await
        .map_err(|e| format!("Failed to clear Gemini API key: {}", e))
}

pub async fn run_heartbeat_now<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<AgentStatusResponse, String> {
    run_heartbeat(app, "manual", None, HeartbeatMode::Manual).await?;
    get_status(app).await
}

pub fn start_heartbeat_loop<R: Runtime + 'static>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_heartbeat(&app, "startup", None, HeartbeatMode::Automatic).await {
            log::warn!("Initial agent heartbeat failed: {}", err);
        }

        loop {
            let interval_minutes = heartbeat_interval_minutes(&app).await;
            tokio::time::sleep(std::time::Duration::from_secs(interval_minutes * 60)).await;
            if let Err(err) = run_heartbeat(&app, "interval", None, HeartbeatMode::Automatic).await
            {
                log::warn!("Agent heartbeat failed: {}", err);
            }
        }
    });
}

pub fn trigger_meeting_saved<R: Runtime + 'static>(app: AppHandle<R>, meeting_id: String) {
    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_heartbeat(
            &app,
            "meeting_saved",
            Some(meeting_id),
            HeartbeatMode::Automatic,
        )
        .await
        {
            log::warn!("Agent heartbeat after meeting save failed: {}", err);
        }
    });
}

pub fn trigger_calendar_sync<R: Runtime + 'static>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_heartbeat(&app, "calendar_sync", None, HeartbeatMode::Automatic).await
        {
            log::warn!("Agent heartbeat after calendar sync failed: {}", err);
        }
    });
}

pub async fn list_recommendations<R: Runtime>(
    app: &AppHandle<R>,
    status: Option<&str>,
) -> Result<Vec<AgentRecommendation>, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(Vec::new());
    };
    let items = AgentRepository::list_recommendations(app_state.db_manager.pool(), status)
        .await
        .map_err(|e| format!("Failed to list recommendations: {}", e))?;
    Ok(items.into_iter().map(recommendation_to_response).collect())
}

pub async fn accept_recommendation<R: Runtime>(
    app: &AppHandle<R>,
    recommendation_id: &str,
) -> Result<AgentRecommendationActionResponse, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Err("Database is not initialized yet".to_string());
    };
    let pool = app_state.db_manager.pool();
    let recommendation = AgentRepository::get_recommendation(pool, recommendation_id)
        .await
        .map_err(|e| format!("Failed to load recommendation: {}", e))?
        .ok_or_else(|| "Recommendation not found".to_string())?;

    let created_calendar_event = match recommendation.recommendation_type.as_str() {
        "calendar_event_draft" => {
            let payload = recommendation
                .payload_json
                .as_deref()
                .ok_or_else(|| "Calendar draft payload is missing".to_string())
                .and_then(parse_calendar_draft_payload)?;

            let created =
                calendar_service::create_google_calendar_event_from_agent(app, &payload).await?;
            AgentRepository::update_recommendation_status(
                pool,
                recommendation_id,
                "executed",
                None,
                true,
            )
            .await
            .map_err(|e| format!("Failed to update recommendation: {}", e))?;
            Some(created)
        }
        _ => {
            return Err(
                "Only calendar event drafts can be accepted into Google Calendar".to_string(),
            );
        }
    };

    let updated = AgentRepository::get_recommendation(pool, recommendation_id)
        .await
        .map_err(|e| format!("Failed to reload recommendation: {}", e))?
        .ok_or_else(|| "Recommendation disappeared after update".to_string())?;

    Ok(AgentRecommendationActionResponse {
        recommendation: recommendation_to_response(updated),
        created_calendar_event,
    })
}

pub async fn dismiss_recommendation<R: Runtime>(
    app: &AppHandle<R>,
    recommendation_id: &str,
) -> Result<AgentRecommendation, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Err("Database is not initialized yet".to_string());
    };
    let pool = app_state.db_manager.pool();
    let recommendation = AgentRepository::get_recommendation(pool, recommendation_id)
        .await
        .map_err(|e| format!("Failed to load recommendation: {}", e))?
        .ok_or_else(|| "Recommendation not found".to_string())?;

    AgentRepository::update_recommendation_status(pool, recommendation_id, "dismissed", None, true)
        .await
        .map_err(|e| format!("Failed to dismiss recommendation: {}", e))?;

    if let Some(task_id) = recommendation.task_id {
        let _ = AgentRepository::update_task_status(pool, &task_id, "dismissed").await;
    }

    let updated = AgentRepository::get_recommendation(pool, recommendation_id)
        .await
        .map_err(|e| format!("Failed to reload recommendation: {}", e))?
        .ok_or_else(|| "Recommendation disappeared after dismissal".to_string())?;
    Ok(recommendation_to_response(updated))
}

pub async fn list_memory<R: Runtime>(
    app: &AppHandle<R>,
    limit: i64,
) -> Result<Vec<AgentMemoryItem>, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(Vec::new());
    };
    let items = AgentRepository::list_memory_items(app_state.db_manager.pool(), limit)
        .await
        .map_err(|e| format!("Failed to list memory: {}", e))?;
    Ok(items.into_iter().map(memory_to_response).collect())
}

pub async fn get_meeting_context<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
) -> Result<AgentMeetingContextResponse, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(AgentMeetingContextResponse {
            memory_items: Vec::new(),
            tasks: Vec::new(),
            recommendations: Vec::new(),
        });
    };
    let pool = app_state.db_manager.pool();
    let memory_items = AgentRepository::list_meeting_memory_items(pool, meeting_id)
        .await
        .map_err(|e| format!("Failed to load meeting memory: {}", e))?;
    let tasks = AgentRepository::list_meeting_tasks(pool, meeting_id)
        .await
        .map_err(|e| format!("Failed to load meeting tasks: {}", e))?;
    let recommendations = AgentRepository::list_meeting_recommendations(pool, meeting_id)
        .await
        .map_err(|e| format!("Failed to load meeting recommendations: {}", e))?;

    Ok(AgentMeetingContextResponse {
        memory_items: memory_items.into_iter().map(memory_to_response).collect(),
        tasks: tasks.into_iter().map(task_to_response).collect(),
        recommendations: recommendations
            .into_iter()
            .map(recommendation_to_response)
            .collect(),
    })
}

pub async fn list_tasks<R: Runtime>(
    app: &AppHandle<R>,
    status: Option<&str>,
) -> Result<Vec<AgentTask>, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(Vec::new());
    };
    let tasks = AgentRepository::list_tasks(app_state.db_manager.pool(), status)
        .await
        .map_err(|e| format!("Failed to list tasks: {}", e))?;
    Ok(tasks.into_iter().map(task_to_response).collect())
}

pub async fn update_task_status<R: Runtime>(
    app: &AppHandle<R>,
    task_id: &str,
    status: &str,
) -> Result<Vec<AgentTask>, String> {
    let status = match status {
        "open" | "completed" | "dismissed" => status,
        _ => return Err("Unsupported task status".to_string()),
    };
    let Some(app_state) = app.try_state::<AppState>() else {
        return Err("Database is not initialized yet".to_string());
    };
    AgentRepository::update_task_status(app_state.db_manager.pool(), task_id, status)
        .await
        .map_err(|e| format!("Failed to update task: {}", e))?;
    list_tasks(app, None).await
}

async fn run_heartbeat<R: Runtime>(
    app: &AppHandle<R>,
    trigger_type: &str,
    meeting_id: Option<String>,
    mode: HeartbeatMode,
) -> Result<(), String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(());
    };

    if !set_running(app, true).await? {
        return Ok(());
    }

    let pool = app_state.db_manager.pool();
    let settings = match load_settings(app).await {
        Ok(settings) => settings,
        Err(err) => {
            let _ = set_running(app, false).await;
            return Err(err);
        }
    };

    let started_at = Utc::now().to_rfc3339();
    if let Err(err) = AgentRepository::update_run_status(pool, &started_at, None, None).await {
        log::warn!("Failed to mark agent heartbeat start: {}", err);
    }

    let result = async {
        if !settings.enabled && !mode.bypass_enabled_guard() {
            return Ok(());
        }

        if settings.provider != DEFAULT_PROVIDER {
            return Err("Only Gemini is supported for the agent right now".to_string());
        }

        let api_key = SettingsRepository::get_api_key(pool, "gemini")
            .await
            .map_err(|e| format!("Failed to load Gemini API key: {}", e))?
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "Configure a Gemini API key to enable the agent".to_string())?;

        let contexts = if let Some(meeting_id) = meeting_id.clone() {
            AgentRepository::get_meeting_context(pool, &meeting_id)
                .await
                .map_err(|e| format!("Failed to load meeting context: {}", e))?
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            let since = (Utc::now() - Duration::days(MEETING_LOOKBACK_DAYS)).to_rfc3339();
            AgentRepository::list_recent_meetings(
                pool,
                &since,
                MAX_MEETINGS_PER_HEARTBEAT,
                !mode.reprocess_recent_meetings(),
            )
            .await
            .map_err(|e| format!("Failed to load recent meetings: {}", e))?
        };

        let mut new_recommendations = Vec::new();
        for context in contexts {
            if let Some(outcome) =
                analyze_meeting(pool, &settings, &api_key, trigger_type, &context).await?
            {
                new_recommendations.extend(outcome.recommendations);
            }
        }

        if settings.notifications_enabled {
            notify_new_recommendations(app, &new_recommendations).await;
        }

        Ok(())
    }
    .await;

    let finished_at = Utc::now().to_rfc3339();
    match result {
        Ok(_) => {
            let _ =
                AgentRepository::update_run_status(pool, &finished_at, Some(&finished_at), None)
                    .await;
        }
        Err(err) => {
            let _ = AgentRepository::update_run_status(pool, &finished_at, None, Some(&err)).await;
            let _ = set_running(app, false).await;
            return Err(err);
        }
    }

    let _ = set_running(app, false).await;
    Ok(())
}

async fn analyze_meeting(
    pool: &sqlx::SqlitePool,
    settings: &AgentSettingsPayload,
    api_key: &str,
    trigger_type: &str,
    context: &AgentMeetingContextRow,
) -> Result<Option<AnalysisOutcome>, String> {
    let transcript = context
        .transcript_text
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();

    let run_id = AgentRepository::insert_run(
        pool,
        trigger_type,
        Some(&context.meeting_id),
        "running",
        &settings.provider,
        &settings.model,
    )
    .await
    .map_err(|e| format!("Failed to record agent run: {}", e))?;

    if transcript.is_empty() {
        let summary = serde_json::json!({
            "meeting_id": context.meeting_id,
            "memory_count": 0,
            "task_count": 0,
            "recommendation_count": 0,
            "note": "No transcript text available"
        });
        AgentRepository::finish_run(pool, &run_id, "completed", Some(&summary.to_string()), None)
            .await
            .map_err(|e| format!("Failed to finish agent run: {}", e))?;
        return Ok(None);
    }

    let prompt = build_agent_prompt(context, &transcript);
    let system_prompt = r#"You are Friday's persistent memory and follow-up agent.
Return strict JSON only. No markdown, no prose before or after the JSON.

Schema:
{
  "memory_items": [
    {
      "memory_type": "person_preference|project_fact|commitment|follow_up|scheduling_constraint",
      "title": "short title",
      "body": "concise durable memory",
      "subject_key": "stable short slug",
      "subject_json": {"optional":"object"},
      "confidence": 0.0
    }
  ],
  "tasks": [
    {
      "title": "action title",
      "body": "what needs to happen",
      "owner_kind": "user|unknown",
      "due_at": "RFC3339 timestamp or null",
      "priority": "low|medium|high"
    }
  ],
  "recommendations": [
    {
      "recommendation_type": "calendar_event_draft",
      "title": "calendar event title",
      "body": "short summary of the event the user should create",
      "rationale": "why this matters now",
      "confidence": 0.0,
      "related_task_title": "matching task title or null",
      "payload": {
        "title": "required only for calendar_event_draft",
        "description": "optional",
        "start_at": "RFC3339",
        "end_at": "RFC3339",
        "timezone": "optional",
        "location": "optional",
        "attendees": ["email@example.com"],
        "conference_request": false
      }
    }
  ]
}

Rules:
- Only include durable memory that would still matter later.
- Recommendations are only for new Google Calendar meetings or time blocks the user can approve.
- If something should be tracked but not scheduled, put it in tasks instead of recommendations.
- Create a calendar_event_draft when the conversation implies scheduling, blocking time, or making social plans (e.g., drinks, lunch, coffee) and a time is mentioned. Infer the date from context when only a time is given. If no end time is stated, default to one hour after start.
- Both start_at and end_at must be valid RFC3339 timestamps in the user's local timezone.
- Keep arrays short and high-signal.
- If there is nothing useful, return empty arrays."#;

    let client = reqwest::Client::new();
    let raw = generate_summary(
        &client,
        &LLMProvider::Gemini,
        &settings.model,
        api_key,
        system_prompt,
        &prompt,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .map_err(|e| format!("Agent model call failed: {}", e))?;

    let model_output = parse_agent_model_output(&raw)?;

    let mut memory_count = 0;
    let mut tasks_by_title = HashMap::new();
    for item in model_output.memory_items.iter().take(6) {
        let title = sanitize_line(&item.title, 120);
        let body = sanitize_line(&item.body, 500);
        if title.is_empty() || body.is_empty() {
            continue;
        }
        let memory = AgentRepository::upsert_memory_item(
            pool,
            &UpsertAgentMemoryItem {
                memory_type: normalize_memory_type(&item.memory_type),
                title,
                body,
                source_meeting_id: Some(context.meeting_id.clone()),
                source_calendar_event_id: context.provider_event_id.clone(),
                subject_key: item
                    .subject_key
                    .clone()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| slugify(item.title.as_str())),
                subject_json: item.subject_json.as_ref().map(|value| value.to_string()),
                confidence: item.confidence.unwrap_or(0.75).clamp(0.0, 1.0),
                status: "active".to_string(),
                run_id: Some(run_id.clone()),
            },
        )
        .await
        .map_err(|e| format!("Failed to save memory: {}", e))?;
        memory_count += 1;
        tasks_by_title.insert(memory.title.clone(), memory.id);
    }

    let mut task_count = 0;
    let mut inserted_tasks = HashMap::new();
    for task in model_output.tasks.iter().take(6) {
        let title = sanitize_line(&task.title, 120);
        let body = sanitize_line(&task.body, 500);
        if title.is_empty() || body.is_empty() {
            continue;
        }
        let saved_task = AgentRepository::upsert_task(
            pool,
            &UpsertAgentTask {
                title: title.clone(),
                body,
                source_meeting_id: Some(context.meeting_id.clone()),
                source_memory_item_id: tasks_by_title.get(&title).cloned(),
                owner_kind: normalize_owner_kind(task.owner_kind.as_deref()),
                due_at: normalize_optional_rfc3339(task.due_at.as_deref()),
                priority: normalize_priority(task.priority.as_deref()),
                status: "open".to_string(),
                run_id: Some(run_id.clone()),
            },
        )
        .await
        .map_err(|e| format!("Failed to save task: {}", e))?;
        task_count += 1;
        inserted_tasks.insert(title, saved_task.id);
    }

    let mut recommendations = Vec::new();
    for recommendation in model_output.recommendations.iter().take(5) {
        let recommendation_type =
            normalize_recommendation_type(&recommendation.recommendation_type);
        let confidence = recommendation.confidence.unwrap_or(0.7).clamp(0.0, 1.0);
        if recommendation_type != "calendar_event_draft" {
            continue;
        }

        let title = sanitize_line(&recommendation.title, 120);
        let body = sanitize_line(&recommendation.body, 500);
        let rationale = sanitize_line(&recommendation.rationale, 500);
        if title.is_empty() || body.is_empty() || rationale.is_empty() {
            continue;
        }

        let payload_json = validated_calendar_recommendation_payload(
            recommendation_type.as_str(),
            recommendation.payload.as_ref(),
            settings.calendar_proposals_enabled,
            confidence,
        );

        if payload_json.is_none() {
            continue;
        }

        if AgentRepository::find_existing_recommendation(
            pool,
            &recommendation_type,
            &title,
            Some(&context.meeting_id),
            context.provider_event_id.as_deref(),
        )
        .await
        .map_err(|e| format!("Failed to dedupe recommendation: {}", e))?
        .is_some()
        {
            continue;
        }

        let inserted = AgentRepository::insert_recommendation(
            pool,
            &InsertAgentRecommendation {
                recommendation_type,
                title,
                body,
                rationale,
                confidence,
                source_meeting_id: Some(context.meeting_id.clone()),
                source_calendar_event_id: context.provider_event_id.clone(),
                task_id: recommendation
                    .related_task_title
                    .as_deref()
                    .and_then(|value| inserted_tasks.get(value).cloned()),
                payload_json,
                status: "pending".to_string(),
            },
        )
        .await
        .map_err(|e| format!("Failed to save recommendation: {}", e))?;
        recommendations.push(inserted);
    }

    let summary = serde_json::json!({
        "meeting_id": context.meeting_id,
        "memory_count": memory_count,
        "task_count": task_count,
        "recommendation_count": recommendations.len(),
    });
    AgentRepository::finish_run(pool, &run_id, "completed", Some(&summary.to_string()), None)
        .await
        .map_err(|e| format!("Failed to finish agent run: {}", e))?;

    Ok(Some(AnalysisOutcome { recommendations }))
}

async fn notify_new_recommendations<R: Runtime>(
    app: &AppHandle<R>,
    recommendations: &[AgentRecommendationModel],
) {
    let Some(app_state) = app.try_state::<AppState>() else {
        return;
    };
    let pool = app_state.db_manager.pool();
    let manager_state = app.state::<NotificationManagerState<R>>();

    for recommendation in recommendations {
        let already_logged =
            match AgentRepository::has_notification_log(pool, &recommendation.id).await {
                Ok(result) => result,
                Err(err) => {
                    log::warn!("Failed to inspect notification log: {}", err);
                    false
                }
            };
        if already_logged {
            continue;
        }

        let notification = Notification::new(
            "Friday",
            format!("Draft calendar event ready: {}", recommendation.title),
            NotificationType::AgentCalendarProposal,
        )
        .with_priority(NotificationPriority::High)
        .with_timeout(NotificationTimeout::Seconds(8));

        let result = {
            let manager = manager_state.read().await;
            if let Some(manager) = manager.as_ref() {
                manager.show_notification(notification).await
            } else {
                Ok(())
            }
        };

        let delivery_status = if result.is_ok() { "shown" } else { "failed" };
        if let Err(err) = AgentRepository::insert_notification_log(
            pool,
            &recommendation.id,
            delivery_status,
            if result.is_ok() {
                "system_notification"
            } else {
                "inbox_only"
            },
        )
        .await
        {
            log::warn!("Failed to log agent notification: {}", err);
        }
    }
}

async fn load_settings<R: Runtime>(app: &AppHandle<R>) -> Result<AgentSettingsPayload, String> {
    let Some(app_state) = app.try_state::<AppState>() else {
        return Ok(default_settings());
    };
    let settings = AgentRepository::get_settings(app_state.db_manager.pool())
        .await
        .map_err(|e| format!("Failed to load agent settings: {}", e))?;
    Ok(settings
        .map(settings_model_to_payload)
        .unwrap_or_else(default_settings))
}

fn default_settings() -> AgentSettingsPayload {
    AgentSettingsPayload {
        enabled: false,
        provider: DEFAULT_PROVIDER.to_string(),
        model: DEFAULT_MODEL.to_string(),
        notifications_enabled: true,
        calendar_proposals_enabled: false,
        heartbeat_interval_minutes: DEFAULT_HEARTBEAT_INTERVAL_MINUTES as u32,
    }
}

async fn heartbeat_interval_minutes<R: Runtime>(app: &AppHandle<R>) -> u64 {
    load_settings(app)
        .await
        .map(|settings| settings.heartbeat_interval_minutes.max(1) as u64)
        .unwrap_or(DEFAULT_HEARTBEAT_INTERVAL_MINUTES)
}

async fn set_running<R: Runtime>(app: &AppHandle<R>, value: bool) -> Result<bool, String> {
    let state = app.state::<AgentManagerState>();
    let mut guard = state.0.lock().await;
    if value && guard.is_running {
        return Ok(false);
    }
    guard.is_running = value;
    Ok(true)
}

fn settings_model_to_payload(model: AgentSettingModel) -> AgentSettingsPayload {
    AgentSettingsPayload {
        enabled: model.enabled != 0,
        provider: model.provider,
        model: model.model,
        notifications_enabled: model.notifications_enabled != 0,
        calendar_proposals_enabled: model.calendar_proposals_enabled != 0,
        heartbeat_interval_minutes: model.heartbeat_interval_minutes.max(1) as u32,
    }
}

fn memory_to_response(item: AgentMemoryItemModel) -> AgentMemoryItem {
    AgentMemoryItem {
        id: item.id,
        memory_type: item.memory_type,
        title: item.title,
        body: item.body,
        source_meeting_id: item.source_meeting_id,
        source_calendar_event_id: item.source_calendar_event_id,
        subject_key: item.subject_key,
        confidence: item.confidence,
        status: item.status,
        first_seen_at: item.first_seen_at,
        last_seen_at: item.last_seen_at,
    }
}

fn task_to_response(task: AgentTaskModel) -> AgentTask {
    AgentTask {
        id: task.id,
        title: task.title,
        body: task.body,
        source_meeting_id: task.source_meeting_id,
        source_memory_item_id: task.source_memory_item_id,
        owner_kind: task.owner_kind,
        due_at: task.due_at,
        priority: task.priority,
        status: task.status,
        last_suggested_at: task.last_suggested_at,
    }
}

fn recommendation_to_response(recommendation: AgentRecommendationModel) -> AgentRecommendation {
    AgentRecommendation {
        id: recommendation.id,
        recommendation_type: recommendation.recommendation_type,
        title: recommendation.title,
        body: recommendation.body,
        rationale: recommendation.rationale,
        confidence: recommendation.confidence,
        source_meeting_id: recommendation.source_meeting_id,
        source_calendar_event_id: recommendation.source_calendar_event_id,
        task_id: recommendation.task_id,
        payload: recommendation
            .payload_json
            .as_deref()
            .and_then(|value| serde_json::from_str(value).ok()),
        status: recommendation.status,
        surfaced_at: recommendation.surfaced_at,
        acted_at: recommendation.acted_at,
        error: recommendation.error,
    }
}

fn build_agent_prompt(context: &AgentMeetingContextRow, transcript: &str) -> String {
    let truncated_transcript = transcript
        .chars()
        .take(MAX_TRANSCRIPT_CHARS)
        .collect::<String>();
    let local_now = Local::now();
    format!(
        "Current time: {now}\nUser timezone: {tz}\nMeeting title: {meeting_title}\nMeeting created at: {created_at}\nLinked calendar event: {calendar_title}\nCalendar event start: {calendar_start_at}\nCalendar event end: {calendar_end_at}\nOrganizer: {organizer}\nTranscript:\n{transcript}",
        now = local_now.to_rfc3339(),
        tz = local_now.format("%Z (UTC%:z)"),
        meeting_title = context.meeting_title,
        created_at = context.created_at,
        calendar_title = context.calendar_title.as_deref().unwrap_or("None"),
        calendar_start_at = context.calendar_start_at.as_deref().unwrap_or("None"),
        calendar_end_at = context.calendar_end_at.as_deref().unwrap_or("None"),
        organizer = context
            .organizer_name
            .as_deref()
            .or(context.organizer_email.as_deref())
            .unwrap_or("Unknown"),
        transcript = truncated_transcript,
    )
}

fn parse_agent_model_output(raw: &str) -> Result<AgentModelOutput, String> {
    let cleaned = clean_llm_markdown_output(raw);
    let json_slice = extract_json_object(&cleaned)
        .or_else(|| extract_json_object(raw))
        .ok_or_else(|| "Agent model output did not contain valid JSON".to_string())?;
    serde_json::from_str::<AgentModelOutput>(json_slice)
        .map_err(|e| format!("Failed to parse agent model output: {}", e))
}

fn extract_json_object(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    raw.get(start..=end)
}

fn sanitize_line(value: &str, max_len: usize) -> String {
    value
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn slugify(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    slug.split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn normalize_memory_type(value: &str) -> String {
    match value {
        "person_preference"
        | "project_fact"
        | "commitment"
        | "follow_up"
        | "scheduling_constraint" => value.to_string(),
        _ => "project_fact".to_string(),
    }
}

fn normalize_owner_kind(value: Option<&str>) -> String {
    match value {
        Some("user") => "user".to_string(),
        _ => "unknown".to_string(),
    }
}

fn normalize_priority(value: Option<&str>) -> String {
    match value {
        Some("high") => "high".to_string(),
        Some("low") => "low".to_string(),
        _ => "medium".to_string(),
    }
}

fn normalize_recommendation_type(value: &str) -> String {
    match value {
        "task_followup" | "calendar_event_draft" | "prep_prompt" | "general_suggestion" => {
            value.to_string()
        }
        _ => "general_suggestion".to_string(),
    }
}

fn normalize_optional_rfc3339(value: Option<&str>) -> Option<String> {
    value.and_then(parse_rfc3339).map(|date| date.to_rfc3339())
}

fn validate_calendar_draft_payload(value: &serde_json::Value) -> Option<serde_json::Value> {
    let payload: CalendarEventDraftPayload = serde_json::from_value(value.clone()).ok()?;
    if parse_rfc3339(&payload.start_at).is_none() || parse_rfc3339(&payload.end_at).is_none() {
        return None;
    }
    Some(serde_json::to_value(payload).ok()?)
}

fn validated_calendar_recommendation_payload(
    recommendation_type: &str,
    payload: Option<&serde_json::Value>,
    calendar_proposals_enabled: bool,
    confidence: f64,
) -> Option<String> {
    if recommendation_type != "calendar_event_draft"
        || !calendar_proposals_enabled
        || confidence < 0.60
    {
        return None;
    }

    payload
        .and_then(validate_calendar_draft_payload)
        .map(|value| value.to_string())
}

fn parse_calendar_draft_payload(raw: &str) -> Result<CalendarEventDraftPayload, String> {
    serde_json::from_str(raw).map_err(|e| format!("Invalid calendar draft payload: {}", e))
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn account_has_calendar_write_scope(
    account: &crate::database::models::ConnectedAccountModel,
) -> bool {
    serde_json::from_str::<Vec<String>>(&account.scopes_json)
        .unwrap_or_default()
        .iter()
        .any(|scope| scope == GOOGLE_CALENDAR_WRITE_SCOPE)
}

#[cfg(test)]
mod tests {
    use super::{
        parse_agent_model_output, validate_calendar_draft_payload,
        validated_calendar_recommendation_payload, HeartbeatMode,
    };
    use serde_json::json;

    #[test]
    fn parses_agent_json_wrapped_in_markdown() {
        let raw = r#"
```json
{
  "memory_items": [{"memory_type":"project_fact","title":"Launch target","body":"Team wants to launch in April","confidence":0.8}],
  "tasks": [{"title":"Send recap","body":"Send the recap to the team","owner_kind":"user","priority":"medium"}],
  "recommendations": []
}
```
"#;

        let parsed = parse_agent_model_output(raw).expect("agent output should parse");
        assert_eq!(parsed.memory_items.len(), 1);
        assert_eq!(parsed.tasks.len(), 1);
    }

    #[test]
    fn rejects_invalid_calendar_payload_without_timestamps() {
        let payload = json!({
            "title": "Block focus time",
            "description": "Draft focus block",
            "attendees": [],
            "conference_request": false
        });
        assert!(validate_calendar_draft_payload(&payload).is_none());
    }

    #[test]
    fn rejects_non_calendar_recommendations_for_action_queue() {
        let payload = json!({
            "title": "Follow up",
            "start_at": "2026-03-07T10:00:00Z",
            "end_at": "2026-03-07T10:30:00Z",
            "attendees": [],
            "conference_request": false
        });
        assert!(validated_calendar_recommendation_payload(
            "task_followup",
            Some(&payload),
            true,
            0.95
        )
        .is_none());
    }

    #[test]
    fn requires_calendar_proposals_enabled_for_action_queue() {
        let payload = json!({
            "title": "Follow up review",
            "start_at": "2026-03-07T10:00:00Z",
            "end_at": "2026-03-07T10:30:00Z",
            "attendees": [],
            "conference_request": false
        });
        assert!(validated_calendar_recommendation_payload(
            "calendar_event_draft",
            Some(&payload),
            false,
            0.95
        )
        .is_none());
    }

    #[test]
    fn accepts_valid_calendar_draft_for_action_queue() {
        let payload = json!({
            "title": "Follow up review",
            "start_at": "2026-03-07T10:00:00Z",
            "end_at": "2026-03-07T10:30:00Z",
            "attendees": [],
            "conference_request": false
        });
        assert!(validated_calendar_recommendation_payload(
            "calendar_event_draft",
            Some(&payload),
            true,
            0.95
        )
        .is_some());
    }

    #[test]
    fn manual_heartbeat_bypasses_background_enabled_guard() {
        assert!(HeartbeatMode::Manual.bypass_enabled_guard());
        assert!(!HeartbeatMode::Automatic.bypass_enabled_guard());
    }

    #[test]
    fn manual_heartbeat_reprocesses_recent_meetings() {
        assert!(HeartbeatMode::Manual.reprocess_recent_meetings());
        assert!(!HeartbeatMode::Automatic.reprocess_recent_meetings());
    }
}
