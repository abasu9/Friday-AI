use crate::calendar::service;
use crate::calendar::types::{
    CalendarLinkCandidate, CalendarStatusResponse, CalendarSyncResult, LinkedCalendarEvent,
    UpcomingCalendarEvent,
};
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub async fn calendar_get_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<CalendarStatusResponse, String> {
    service::get_status(&app).await
}

#[tauri::command]
pub async fn calendar_list_upcoming<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<UpcomingCalendarEvent>, String> {
    service::list_upcoming_events(&app).await
}

#[tauri::command]
pub async fn calendar_connect_google<R: Runtime>(
    app: AppHandle<R>,
    write_access: Option<bool>,
) -> Result<CalendarStatusResponse, String> {
    service::connect_google(&app, write_access.unwrap_or(false)).await
}

#[tauri::command]
pub async fn calendar_upgrade_google_access<R: Runtime>(
    app: AppHandle<R>,
) -> Result<CalendarStatusResponse, String> {
    service::connect_google(&app, true).await
}

#[tauri::command]
pub async fn calendar_disconnect_google<R: Runtime>(
    app: AppHandle<R>,
) -> Result<CalendarStatusResponse, String> {
    service::disconnect_google(&app).await
}

#[tauri::command]
pub async fn calendar_sync_now<R: Runtime>(
    app: AppHandle<R>,
) -> Result<CalendarSyncResult, String> {
    service::sync_google_calendar(&app).await
}

#[tauri::command]
pub async fn calendar_get_meeting_link<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<Option<LinkedCalendarEvent>, String> {
    service::get_meeting_link(&app, &meeting_id).await
}

#[tauri::command]
pub async fn calendar_get_link_candidates<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<Vec<CalendarLinkCandidate>, String> {
    service::get_link_candidates(&app, &meeting_id).await
}

#[tauri::command]
pub async fn calendar_set_meeting_link<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    provider_event_id: String,
) -> Result<Option<LinkedCalendarEvent>, String> {
    service::set_meeting_link(&app, &meeting_id, &provider_event_id).await
}

#[tauri::command]
pub async fn calendar_clear_meeting_link<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<(), String> {
    service::clear_meeting_link_for_meeting(&app, &meeting_id).await
}
