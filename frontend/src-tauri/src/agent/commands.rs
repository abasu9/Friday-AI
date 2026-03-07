use crate::agent::service;
use crate::agent::types::{
    AgentMeetingContextResponse, AgentMemoryItem, AgentRecommendation,
    AgentRecommendationActionResponse, AgentSettingsPayload, AgentStatusResponse, AgentTask,
};
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub async fn agent_get_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<AgentStatusResponse, String> {
    service::get_status(&app).await
}

#[tauri::command]
pub async fn agent_get_settings<R: Runtime>(
    app: AppHandle<R>,
) -> Result<AgentSettingsPayload, String> {
    service::get_settings(&app).await
}

#[tauri::command]
pub async fn agent_set_settings<R: Runtime>(
    app: AppHandle<R>,
    settings: AgentSettingsPayload,
) -> Result<AgentStatusResponse, String> {
    service::set_settings(&app, settings).await
}

#[tauri::command]
pub async fn agent_save_gemini_api_key<R: Runtime>(
    app: AppHandle<R>,
    api_key: String,
) -> Result<(), String> {
    service::save_gemini_api_key(&app, &api_key).await
}

#[tauri::command]
pub async fn agent_clear_gemini_api_key<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    service::clear_gemini_api_key(&app).await
}

#[tauri::command]
pub async fn agent_run_heartbeat_now<R: Runtime>(
    app: AppHandle<R>,
) -> Result<AgentStatusResponse, String> {
    service::run_heartbeat_now(&app).await
}

#[tauri::command]
pub async fn agent_list_recommendations<R: Runtime>(
    app: AppHandle<R>,
    status: Option<String>,
) -> Result<Vec<AgentRecommendation>, String> {
    service::list_recommendations(&app, status.as_deref()).await
}

#[tauri::command]
pub async fn agent_accept_recommendation<R: Runtime>(
    app: AppHandle<R>,
    recommendation_id: String,
) -> Result<AgentRecommendationActionResponse, String> {
    service::accept_recommendation(&app, &recommendation_id).await
}

#[tauri::command]
pub async fn agent_dismiss_recommendation<R: Runtime>(
    app: AppHandle<R>,
    recommendation_id: String,
) -> Result<AgentRecommendation, String> {
    service::dismiss_recommendation(&app, &recommendation_id).await
}

#[tauri::command]
pub async fn agent_list_memory<R: Runtime>(
    app: AppHandle<R>,
    limit: Option<u32>,
) -> Result<Vec<AgentMemoryItem>, String> {
    service::list_memory(&app, limit.unwrap_or(25) as i64).await
}

#[tauri::command]
pub async fn agent_get_meeting_context<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<AgentMeetingContextResponse, String> {
    service::get_meeting_context(&app, &meeting_id).await
}

#[tauri::command]
pub async fn agent_list_tasks<R: Runtime>(
    app: AppHandle<R>,
    status: Option<String>,
) -> Result<Vec<AgentTask>, String> {
    service::list_tasks(&app, status.as_deref()).await
}

#[tauri::command]
pub async fn agent_update_task_status<R: Runtime>(
    app: AppHandle<R>,
    task_id: String,
    status: String,
) -> Result<Vec<AgentTask>, String> {
    service::update_task_status(&app, &task_id, &status).await
}
