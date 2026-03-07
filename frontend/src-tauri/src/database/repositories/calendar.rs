use crate::database::models::{CalendarEventModel, CalendarSyncStateModel, ConnectedAccountModel};
use chrono::Utc;
use sqlx::{FromRow, QueryBuilder, Sqlite, SqlitePool};

#[derive(Debug, Clone)]
pub struct UpsertCalendarEvent {
    pub provider_event_id: String,
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub organizer_email: Option<String>,
    pub organizer_name: Option<String>,
    pub attendees_json: Option<String>,
    pub start_at: String,
    pub end_at: String,
    pub timezone: Option<String>,
    pub conference_url: Option<String>,
    pub status: String,
    pub html_link: Option<String>,
    pub raw_etag: Option<String>,
    pub is_primary_calendar: bool,
}

#[derive(Debug, Clone, FromRow)]
pub struct MeetingLinkedEventRow {
    pub meeting_id: String,
    pub account_id: String,
    pub provider_event_id: String,
    pub confidence: f64,
    pub link_method: String,
    pub reason: Option<String>,
    pub linked_at: String,
    pub title: String,
    pub description: Option<String>,
    pub organizer_email: Option<String>,
    pub organizer_name: Option<String>,
    pub attendees_json: Option<String>,
    pub start_at: String,
    pub end_at: String,
    pub timezone: Option<String>,
    pub conference_url: Option<String>,
    pub status: String,
    pub html_link: Option<String>,
}

pub struct CalendarRepository;

impl CalendarRepository {
    pub async fn upsert_account(
        pool: &SqlitePool,
        id: &str,
        provider: &str,
        email: Option<&str>,
        scopes_json: &str,
        connection_status: &str,
        last_sync_at: Option<&str>,
        last_error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO connected_accounts (
                id, provider, email, scopes_json, connection_status, last_sync_at, last_error, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                email = excluded.email,
                scopes_json = excluded.scopes_json,
                connection_status = excluded.connection_status,
                last_sync_at = excluded.last_sync_at,
                last_error = excluded.last_error,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(id)
        .bind(provider)
        .bind(email)
        .bind(scopes_json)
        .bind(connection_status)
        .bind(last_sync_at)
        .bind(last_error)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_google_account(
        pool: &SqlitePool,
    ) -> Result<Option<ConnectedAccountModel>, sqlx::Error> {
        sqlx::query_as::<_, ConnectedAccountModel>(
            "SELECT * FROM connected_accounts WHERE provider = 'google' LIMIT 1",
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn update_account_status(
        pool: &SqlitePool,
        id: &str,
        connection_status: &str,
        last_error: Option<&str>,
        last_sync_at: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE connected_accounts
            SET connection_status = ?,
                last_error = ?,
                last_sync_at = COALESCE(?, last_sync_at),
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(connection_status)
        .bind(last_error)
        .bind(last_sync_at)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn upsert_sync_state(
        pool: &SqlitePool,
        account_id: &str,
        window_start: Option<&str>,
        window_end: Option<&str>,
        last_sync_started_at: Option<&str>,
        last_sync_finished_at: Option<&str>,
        last_success_at: Option<&str>,
        last_error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO calendar_sync_state (
                account_id,
                sync_token,
                window_start,
                window_end,
                last_sync_started_at,
                last_sync_finished_at,
                last_success_at,
                last_error
            )
            VALUES (?, NULL, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(account_id) DO UPDATE SET
                window_start = excluded.window_start,
                window_end = excluded.window_end,
                last_sync_started_at = excluded.last_sync_started_at,
                last_sync_finished_at = excluded.last_sync_finished_at,
                last_success_at = excluded.last_success_at,
                last_error = excluded.last_error
            "#,
        )
        .bind(account_id)
        .bind(window_start)
        .bind(window_end)
        .bind(last_sync_started_at)
        .bind(last_sync_finished_at)
        .bind(last_success_at)
        .bind(last_error)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_sync_state(
        pool: &SqlitePool,
        account_id: &str,
    ) -> Result<Option<CalendarSyncStateModel>, sqlx::Error> {
        sqlx::query_as::<_, CalendarSyncStateModel>(
            "SELECT * FROM calendar_sync_state WHERE account_id = ? LIMIT 1",
        )
        .bind(account_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn upsert_events_for_window(
        pool: &SqlitePool,
        account_id: &str,
        window_start: &str,
        window_end: &str,
        events: &[UpsertCalendarEvent],
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        let mut delete_query =
            QueryBuilder::<Sqlite>::new("DELETE FROM calendar_events WHERE account_id = ");
        delete_query
            .push_bind(account_id)
            .push(" AND start_at >= ")
            .push_bind(window_start)
            .push(" AND start_at <= ")
            .push_bind(window_end);

        if !events.is_empty() {
            delete_query.push(" AND provider_event_id NOT IN (");
            let mut separated = delete_query.separated(", ");
            for event in events {
                separated.push_bind(&event.provider_event_id);
            }
            separated.push_unseparated(")");
        }

        delete_query.build().execute(&mut *tx).await?;

        for event in events {
            sqlx::query(
                r#"
                INSERT INTO calendar_events (
                    account_id,
                    provider_event_id,
                    calendar_id,
                    title,
                    description,
                    organizer_email,
                    organizer_name,
                    attendees_json,
                    start_at,
                    end_at,
                    timezone,
                    conference_url,
                    status,
                    html_link,
                    raw_etag,
                    is_primary_calendar,
                    updated_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(account_id, provider_event_id) DO UPDATE SET
                    calendar_id = excluded.calendar_id,
                    title = excluded.title,
                    description = excluded.description,
                    organizer_email = excluded.organizer_email,
                    organizer_name = excluded.organizer_name,
                    attendees_json = excluded.attendees_json,
                    start_at = excluded.start_at,
                    end_at = excluded.end_at,
                    timezone = excluded.timezone,
                    conference_url = excluded.conference_url,
                    status = excluded.status,
                    html_link = excluded.html_link,
                    raw_etag = excluded.raw_etag,
                    is_primary_calendar = excluded.is_primary_calendar,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(account_id)
            .bind(&event.provider_event_id)
            .bind(&event.calendar_id)
            .bind(&event.title)
            .bind(&event.description)
            .bind(&event.organizer_email)
            .bind(&event.organizer_name)
            .bind(&event.attendees_json)
            .bind(&event.start_at)
            .bind(&event.end_at)
            .bind(&event.timezone)
            .bind(&event.conference_url)
            .bind(&event.status)
            .bind(&event.html_link)
            .bind(&event.raw_etag)
            .bind(i64::from(event.is_primary_calendar))
            .bind(Utc::now().to_rfc3339())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_events_for_matching(
        pool: &SqlitePool,
        account_id: &str,
        window_start: &str,
        window_end: &str,
    ) -> Result<Vec<CalendarEventModel>, sqlx::Error> {
        sqlx::query_as::<_, CalendarEventModel>(
            r#"
            SELECT * FROM calendar_events
            WHERE account_id = ?
              AND status IN ('confirmed', 'tentative')
              AND end_at >= ?
              AND start_at <= ?
            ORDER BY start_at ASC
            "#,
        )
        .bind(account_id)
        .bind(window_start)
        .bind(window_end)
        .fetch_all(pool)
        .await
    }

    pub async fn list_upcoming_events(
        pool: &SqlitePool,
        account_id: &str,
        from: &str,
        until: &str,
    ) -> Result<Vec<CalendarEventModel>, sqlx::Error> {
        sqlx::query_as::<_, CalendarEventModel>(
            r#"
            SELECT * FROM calendar_events
            WHERE account_id = ?
              AND status IN ('confirmed', 'tentative')
              AND end_at >= ?
              AND start_at <= ?
            ORDER BY start_at ASC
            "#,
        )
        .bind(account_id)
        .bind(from)
        .bind(until)
        .fetch_all(pool)
        .await
    }

    pub async fn get_event(
        pool: &SqlitePool,
        account_id: &str,
        provider_event_id: &str,
    ) -> Result<Option<CalendarEventModel>, sqlx::Error> {
        sqlx::query_as::<_, CalendarEventModel>(
            "SELECT * FROM calendar_events WHERE account_id = ? AND provider_event_id = ? LIMIT 1",
        )
        .bind(account_id)
        .bind(provider_event_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn set_meeting_link(
        pool: &SqlitePool,
        meeting_id: &str,
        account_id: &str,
        provider_event_id: &str,
        confidence: f64,
        link_method: &str,
        reason: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO meeting_event_links (
                meeting_id,
                account_id,
                provider_event_id,
                confidence,
                link_method,
                reason,
                linked_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(meeting_id) DO UPDATE SET
                account_id = excluded.account_id,
                provider_event_id = excluded.provider_event_id,
                confidence = excluded.confidence,
                link_method = excluded.link_method,
                reason = excluded.reason,
                linked_at = excluded.linked_at
            "#,
        )
        .bind(meeting_id)
        .bind(account_id)
        .bind(provider_event_id)
        .bind(confidence)
        .bind(link_method)
        .bind(reason)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn clear_meeting_link(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM meeting_event_links WHERE meeting_id = ?")
            .bind(meeting_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_meeting_link(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<MeetingLinkedEventRow>, sqlx::Error> {
        sqlx::query_as::<_, MeetingLinkedEventRow>(
            r#"
            SELECT
                l.meeting_id,
                l.account_id,
                l.provider_event_id,
                l.confidence,
                l.link_method,
                l.reason,
                l.linked_at,
                e.title,
                e.description,
                e.organizer_email,
                e.organizer_name,
                e.attendees_json,
                e.start_at,
                e.end_at,
                e.timezone,
                e.conference_url,
                e.status,
                e.html_link
            FROM meeting_event_links l
            JOIN calendar_events e
              ON e.account_id = l.account_id
             AND e.provider_event_id = l.provider_event_id
            WHERE l.meeting_id = ?
            LIMIT 1
            "#,
        )
        .bind(meeting_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn get_meeting_context(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<(String, String)>, sqlx::Error> {
        sqlx::query_as::<_, (String, String)>(
            "SELECT title, created_at FROM meetings WHERE id = ? LIMIT 1",
        )
        .bind(meeting_id)
        .fetch_optional(pool)
        .await
    }
}
