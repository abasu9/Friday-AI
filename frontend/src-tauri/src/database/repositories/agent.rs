use crate::database::models::{
    AgentMemoryItemModel, AgentRecommendationModel, AgentSettingModel, AgentTaskModel,
};
use chrono::Utc;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct AgentMeetingContextRow {
    pub meeting_id: String,
    pub meeting_title: String,
    pub created_at: String,
    pub transcript_text: Option<String>,
    pub provider_event_id: Option<String>,
    pub calendar_title: Option<String>,
    pub calendar_description: Option<String>,
    pub calendar_start_at: Option<String>,
    pub calendar_end_at: Option<String>,
    pub organizer_email: Option<String>,
    pub organizer_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpsertAgentMemoryItem {
    pub memory_type: String,
    pub title: String,
    pub body: String,
    pub source_meeting_id: Option<String>,
    pub source_calendar_event_id: Option<String>,
    pub subject_key: String,
    pub subject_json: Option<String>,
    pub confidence: f64,
    pub status: String,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpsertAgentTask {
    pub title: String,
    pub body: String,
    pub source_meeting_id: Option<String>,
    pub source_memory_item_id: Option<String>,
    pub owner_kind: String,
    pub due_at: Option<String>,
    pub priority: String,
    pub status: String,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InsertAgentRecommendation {
    pub recommendation_type: String,
    pub title: String,
    pub body: String,
    pub rationale: String,
    pub confidence: f64,
    pub source_meeting_id: Option<String>,
    pub source_calendar_event_id: Option<String>,
    pub task_id: Option<String>,
    pub payload_json: Option<String>,
    pub status: String,
}

pub struct AgentRepository;

impl AgentRepository {
    pub async fn get_settings(pool: &SqlitePool) -> Result<Option<AgentSettingModel>, sqlx::Error> {
        sqlx::query_as::<_, AgentSettingModel>("SELECT * FROM agent_settings WHERE id = 'default'")
            .fetch_optional(pool)
            .await
    }

    pub async fn save_settings(
        pool: &SqlitePool,
        enabled: bool,
        provider: &str,
        model: &str,
        notifications_enabled: bool,
        calendar_proposals_enabled: bool,
        heartbeat_interval_minutes: i64,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO agent_settings (
                id,
                enabled,
                provider,
                model,
                notifications_enabled,
                calendar_proposals_enabled,
                heartbeat_interval_minutes,
                created_at,
                updated_at
            )
            VALUES ('default', ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                enabled = excluded.enabled,
                provider = excluded.provider,
                model = excluded.model,
                notifications_enabled = excluded.notifications_enabled,
                calendar_proposals_enabled = excluded.calendar_proposals_enabled,
                heartbeat_interval_minutes = excluded.heartbeat_interval_minutes,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(i64::from(enabled))
        .bind(provider)
        .bind(model)
        .bind(i64::from(notifications_enabled))
        .bind(i64::from(calendar_proposals_enabled))
        .bind(heartbeat_interval_minutes)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_run_status(
        pool: &SqlitePool,
        last_run_at: &str,
        last_success_at: Option<&str>,
        last_error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE agent_settings
            SET last_run_at = ?,
                last_success_at = COALESCE(?, last_success_at),
                last_error = ?,
                updated_at = ?
            WHERE id = 'default'
            "#,
        )
        .bind(last_run_at)
        .bind(last_success_at)
        .bind(last_error)
        .bind(last_run_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn insert_run(
        pool: &SqlitePool,
        trigger_type: &str,
        trigger_ref: Option<&str>,
        status: &str,
        model_provider: &str,
        model_name: &str,
    ) -> Result<String, sqlx::Error> {
        let id = format!("agent-run-{}", Uuid::new_v4());
        let started_at = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO agent_runs (
                id,
                trigger_type,
                trigger_ref,
                status,
                started_at,
                model_provider,
                model_name
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(trigger_type)
        .bind(trigger_ref)
        .bind(status)
        .bind(started_at)
        .bind(model_provider)
        .bind(model_name)
        .execute(pool)
        .await?;

        Ok(id)
    }

    pub async fn finish_run(
        pool: &SqlitePool,
        run_id: &str,
        status: &str,
        summary_json: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let finished_at = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE agent_runs
            SET status = ?,
                finished_at = ?,
                summary_json = ?,
                error = ?
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(&finished_at)
        .bind(summary_json)
        .bind(error)
        .bind(run_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn count_pending_recommendations(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM agent_recommendations
            WHERE status = 'pending'
              AND recommendation_type = 'calendar_event_draft'
            "#,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn count_open_tasks(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks WHERE status = 'open'")
            .fetch_one(pool)
            .await
    }

    pub async fn list_recent_meetings(
        pool: &SqlitePool,
        since: &str,
        limit: i64,
        only_unprocessed: bool,
    ) -> Result<Vec<AgentMeetingContextRow>, sqlx::Error> {
        let query = if only_unprocessed {
            r#"
            SELECT
                m.id AS meeting_id,
                m.title AS meeting_title,
                m.created_at AS created_at,
                GROUP_CONCAT(t.transcript, CHAR(10)) AS transcript_text,
                l.provider_event_id AS provider_event_id,
                e.title AS calendar_title,
                e.description AS calendar_description,
                e.start_at AS calendar_start_at,
                e.end_at AS calendar_end_at,
                e.organizer_email AS organizer_email,
                e.organizer_name AS organizer_name
            FROM meetings m
            LEFT JOIN transcripts t ON t.meeting_id = m.id
            LEFT JOIN meeting_event_links l ON l.meeting_id = m.id
            LEFT JOIN calendar_events e
              ON e.account_id = l.account_id
             AND e.provider_event_id = l.provider_event_id
            WHERE m.created_at >= ?
              AND NOT EXISTS (
                SELECT 1
                FROM agent_runs r
                WHERE r.trigger_ref = m.id
                  AND r.status = 'completed'
              )
            GROUP BY
                m.id,
                m.title,
                m.created_at,
                l.provider_event_id,
                e.title,
                e.description,
                e.start_at,
                e.end_at,
                e.organizer_email,
                e.organizer_name
            ORDER BY m.created_at DESC
            LIMIT ?
            "#
        } else {
            r#"
            SELECT
                m.id AS meeting_id,
                m.title AS meeting_title,
                m.created_at AS created_at,
                GROUP_CONCAT(t.transcript, CHAR(10)) AS transcript_text,
                l.provider_event_id AS provider_event_id,
                e.title AS calendar_title,
                e.description AS calendar_description,
                e.start_at AS calendar_start_at,
                e.end_at AS calendar_end_at,
                e.organizer_email AS organizer_email,
                e.organizer_name AS organizer_name
            FROM meetings m
            LEFT JOIN transcripts t ON t.meeting_id = m.id
            LEFT JOIN meeting_event_links l ON l.meeting_id = m.id
            LEFT JOIN calendar_events e
              ON e.account_id = l.account_id
             AND e.provider_event_id = l.provider_event_id
            WHERE m.created_at >= ?
            GROUP BY
                m.id,
                m.title,
                m.created_at,
                l.provider_event_id,
                e.title,
                e.description,
                e.start_at,
                e.end_at,
                e.organizer_email,
                e.organizer_name
            ORDER BY m.created_at DESC
            LIMIT ?
            "#
        };

        sqlx::query_as::<_, AgentMeetingContextRow>(query)
            .bind(since)
            .bind(limit)
            .fetch_all(pool)
            .await
    }

    pub async fn get_meeting_context(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<AgentMeetingContextRow>, sqlx::Error> {
        sqlx::query_as::<_, AgentMeetingContextRow>(
            r#"
            SELECT
                m.id AS meeting_id,
                m.title AS meeting_title,
                m.created_at AS created_at,
                GROUP_CONCAT(t.transcript, CHAR(10)) AS transcript_text,
                l.provider_event_id AS provider_event_id,
                e.title AS calendar_title,
                e.description AS calendar_description,
                e.start_at AS calendar_start_at,
                e.end_at AS calendar_end_at,
                e.organizer_email AS organizer_email,
                e.organizer_name AS organizer_name
            FROM meetings m
            LEFT JOIN transcripts t ON t.meeting_id = m.id
            LEFT JOIN meeting_event_links l ON l.meeting_id = m.id
            LEFT JOIN calendar_events e
              ON e.account_id = l.account_id
             AND e.provider_event_id = l.provider_event_id
            WHERE m.id = ?
            GROUP BY
                m.id,
                m.title,
                m.created_at,
                l.provider_event_id,
                e.title,
                e.description,
                e.start_at,
                e.end_at,
                e.organizer_email,
                e.organizer_name
            LIMIT 1
            "#,
        )
        .bind(meeting_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn upsert_memory_item(
        pool: &SqlitePool,
        item: &UpsertAgentMemoryItem,
    ) -> Result<AgentMemoryItemModel, sqlx::Error> {
        if let Some(existing) = sqlx::query_as::<_, AgentMemoryItemModel>(
            r#"
            SELECT *
            FROM agent_memory_items
            WHERE memory_type = ?
              AND title = ?
              AND COALESCE(source_meeting_id, '') = COALESCE(?, '')
            LIMIT 1
            "#,
        )
        .bind(&item.memory_type)
        .bind(&item.title)
        .bind(&item.source_meeting_id)
        .fetch_optional(pool)
        .await?
        {
            let now = Utc::now().to_rfc3339();
            sqlx::query(
                r#"
                UPDATE agent_memory_items
                SET body = ?,
                    source_calendar_event_id = ?,
                    subject_key = ?,
                    subject_json = ?,
                    confidence = ?,
                    status = ?,
                    last_seen_at = ?,
                    updated_run_id = ?
                WHERE id = ?
                "#,
            )
            .bind(&item.body)
            .bind(&item.source_calendar_event_id)
            .bind(&item.subject_key)
            .bind(&item.subject_json)
            .bind(item.confidence)
            .bind(&item.status)
            .bind(&now)
            .bind(&item.run_id)
            .bind(&existing.id)
            .execute(pool)
            .await?;

            return sqlx::query_as::<_, AgentMemoryItemModel>(
                "SELECT * FROM agent_memory_items WHERE id = ? LIMIT 1",
            )
            .bind(existing.id)
            .fetch_one(pool)
            .await;
        }

        let id = format!("agent-memory-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO agent_memory_items (
                id,
                memory_type,
                title,
                body,
                source_meeting_id,
                source_calendar_event_id,
                subject_key,
                subject_json,
                confidence,
                status,
                first_seen_at,
                last_seen_at,
                created_run_id,
                updated_run_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&item.memory_type)
        .bind(&item.title)
        .bind(&item.body)
        .bind(&item.source_meeting_id)
        .bind(&item.source_calendar_event_id)
        .bind(&item.subject_key)
        .bind(&item.subject_json)
        .bind(item.confidence)
        .bind(&item.status)
        .bind(&now)
        .bind(&now)
        .bind(&item.run_id)
        .bind(&item.run_id)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, AgentMemoryItemModel>(
            "SELECT * FROM agent_memory_items WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn upsert_task(
        pool: &SqlitePool,
        task: &UpsertAgentTask,
    ) -> Result<AgentTaskModel, sqlx::Error> {
        if let Some(existing) = sqlx::query_as::<_, AgentTaskModel>(
            r#"
            SELECT *
            FROM agent_tasks
            WHERE title = ?
              AND COALESCE(source_meeting_id, '') = COALESCE(?, '')
              AND status IN ('open', 'completed', 'dismissed')
            LIMIT 1
            "#,
        )
        .bind(&task.title)
        .bind(&task.source_meeting_id)
        .fetch_optional(pool)
        .await?
        {
            let now = Utc::now().to_rfc3339();
            sqlx::query(
                r#"
                UPDATE agent_tasks
                SET body = ?,
                    source_memory_item_id = ?,
                    owner_kind = ?,
                    due_at = ?,
                    priority = ?,
                    last_suggested_at = ?,
                    updated_run_id = ?
                WHERE id = ?
                "#,
            )
            .bind(&task.body)
            .bind(&task.source_memory_item_id)
            .bind(&task.owner_kind)
            .bind(&task.due_at)
            .bind(&task.priority)
            .bind(&now)
            .bind(&task.run_id)
            .bind(&existing.id)
            .execute(pool)
            .await?;

            return sqlx::query_as::<_, AgentTaskModel>(
                "SELECT * FROM agent_tasks WHERE id = ? LIMIT 1",
            )
            .bind(existing.id)
            .fetch_one(pool)
            .await;
        }

        let id = format!("agent-task-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO agent_tasks (
                id,
                title,
                body,
                source_meeting_id,
                source_memory_item_id,
                owner_kind,
                due_at,
                priority,
                status,
                last_suggested_at,
                created_run_id,
                updated_run_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&task.title)
        .bind(&task.body)
        .bind(&task.source_meeting_id)
        .bind(&task.source_memory_item_id)
        .bind(&task.owner_kind)
        .bind(&task.due_at)
        .bind(&task.priority)
        .bind(&task.status)
        .bind(&now)
        .bind(&task.run_id)
        .bind(&task.run_id)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, AgentTaskModel>("SELECT * FROM agent_tasks WHERE id = ? LIMIT 1")
            .bind(id)
            .fetch_one(pool)
            .await
    }

    pub async fn find_existing_recommendation(
        pool: &SqlitePool,
        recommendation_type: &str,
        title: &str,
        source_meeting_id: Option<&str>,
        source_calendar_event_id: Option<&str>,
    ) -> Result<Option<AgentRecommendationModel>, sqlx::Error> {
        sqlx::query_as::<_, AgentRecommendationModel>(
            r#"
            SELECT *
            FROM agent_recommendations
            WHERE recommendation_type = ?
              AND title = ?
              AND COALESCE(source_meeting_id, '') = COALESCE(?, '')
              AND COALESCE(source_calendar_event_id, '') = COALESCE(?, '')
              AND status IN ('pending', 'accepted', 'executed')
            LIMIT 1
            "#,
        )
        .bind(recommendation_type)
        .bind(title)
        .bind(source_meeting_id)
        .bind(source_calendar_event_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn insert_recommendation(
        pool: &SqlitePool,
        recommendation: &InsertAgentRecommendation,
    ) -> Result<AgentRecommendationModel, sqlx::Error> {
        let id = format!("agent-rec-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO agent_recommendations (
                id,
                recommendation_type,
                title,
                body,
                rationale,
                confidence,
                source_meeting_id,
                source_calendar_event_id,
                task_id,
                payload_json,
                status,
                surfaced_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&recommendation.recommendation_type)
        .bind(&recommendation.title)
        .bind(&recommendation.body)
        .bind(&recommendation.rationale)
        .bind(recommendation.confidence)
        .bind(&recommendation.source_meeting_id)
        .bind(&recommendation.source_calendar_event_id)
        .bind(&recommendation.task_id)
        .bind(&recommendation.payload_json)
        .bind(&recommendation.status)
        .bind(now)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, AgentRecommendationModel>(
            "SELECT * FROM agent_recommendations WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn list_recommendations(
        pool: &SqlitePool,
        status: Option<&str>,
    ) -> Result<Vec<AgentRecommendationModel>, sqlx::Error> {
        if matches!(status, Some("pending")) {
            sqlx::query_as::<_, AgentRecommendationModel>(
                r#"
                SELECT *
                FROM agent_recommendations
                WHERE status = 'pending'
                  AND recommendation_type = 'calendar_event_draft'
                ORDER BY surfaced_at DESC
                "#,
            )
            .fetch_all(pool)
            .await
        } else if let Some(status) = status {
            sqlx::query_as::<_, AgentRecommendationModel>(
                "SELECT * FROM agent_recommendations WHERE status = ? ORDER BY surfaced_at DESC",
            )
            .bind(status)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as::<_, AgentRecommendationModel>(
                "SELECT * FROM agent_recommendations ORDER BY surfaced_at DESC",
            )
            .fetch_all(pool)
            .await
        }
    }

    pub async fn get_recommendation(
        pool: &SqlitePool,
        recommendation_id: &str,
    ) -> Result<Option<AgentRecommendationModel>, sqlx::Error> {
        sqlx::query_as::<_, AgentRecommendationModel>(
            "SELECT * FROM agent_recommendations WHERE id = ? LIMIT 1",
        )
        .bind(recommendation_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn update_recommendation_status(
        pool: &SqlitePool,
        recommendation_id: &str,
        status: &str,
        error: Option<&str>,
        acted: bool,
    ) -> Result<(), sqlx::Error> {
        let acted_at = acted.then(|| Utc::now().to_rfc3339());
        sqlx::query(
            r#"
            UPDATE agent_recommendations
            SET status = ?,
                acted_at = COALESCE(?, acted_at),
                error = ?
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(acted_at)
        .bind(error)
        .bind(recommendation_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list_tasks(
        pool: &SqlitePool,
        status: Option<&str>,
    ) -> Result<Vec<AgentTaskModel>, sqlx::Error> {
        if let Some(status) = status {
            sqlx::query_as::<_, AgentTaskModel>(
                "SELECT * FROM agent_tasks WHERE status = ? ORDER BY last_suggested_at DESC",
            )
            .bind(status)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as::<_, AgentTaskModel>(
                "SELECT * FROM agent_tasks ORDER BY last_suggested_at DESC",
            )
            .fetch_all(pool)
            .await
        }
    }

    pub async fn update_task_status(
        pool: &SqlitePool,
        task_id: &str,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE agent_tasks SET status = ? WHERE id = ?")
            .bind(status)
            .bind(task_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn list_memory_items(
        pool: &SqlitePool,
        limit: i64,
    ) -> Result<Vec<AgentMemoryItemModel>, sqlx::Error> {
        sqlx::query_as::<_, AgentMemoryItemModel>(
            "SELECT * FROM agent_memory_items WHERE status = 'active' ORDER BY last_seen_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn list_meeting_memory_items(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Vec<AgentMemoryItemModel>, sqlx::Error> {
        sqlx::query_as::<_, AgentMemoryItemModel>(
            "SELECT * FROM agent_memory_items WHERE source_meeting_id = ? ORDER BY last_seen_at DESC",
        )
        .bind(meeting_id)
        .fetch_all(pool)
        .await
    }

    pub async fn list_meeting_tasks(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Vec<AgentTaskModel>, sqlx::Error> {
        sqlx::query_as::<_, AgentTaskModel>(
            "SELECT * FROM agent_tasks WHERE source_meeting_id = ? ORDER BY last_suggested_at DESC",
        )
        .bind(meeting_id)
        .fetch_all(pool)
        .await
    }

    pub async fn list_meeting_recommendations(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Vec<AgentRecommendationModel>, sqlx::Error> {
        sqlx::query_as::<_, AgentRecommendationModel>(
            "SELECT * FROM agent_recommendations WHERE source_meeting_id = ? ORDER BY surfaced_at DESC",
        )
        .bind(meeting_id)
        .fetch_all(pool)
        .await
    }

    pub async fn has_notification_log(
        pool: &SqlitePool,
        recommendation_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_notification_log WHERE recommendation_id = ?",
        )
        .bind(recommendation_id)
        .fetch_one(pool)
        .await?;
        Ok(exists > 0)
    }

    pub async fn insert_notification_log(
        pool: &SqlitePool,
        recommendation_id: &str,
        delivery_status: &str,
        channel: &str,
    ) -> Result<(), sqlx::Error> {
        let id = format!("agent-notif-{}", Uuid::new_v4());
        let shown_at = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO agent_notification_log (
                id,
                recommendation_id,
                shown_at,
                delivery_status,
                channel
            )
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(recommendation_id)
        .bind(shown_at)
        .bind(delivery_status)
        .bind(channel)
        .execute(pool)
        .await?;
        Ok(())
    }
}
