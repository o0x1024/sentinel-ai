use anyhow::Result;
use crate::database_service::service::DatabaseService;
use crate::core::models::database::{ConversationSegment, GlobalSummary};

impl DatabaseService {
    pub async fn ensure_sliding_window_tables_exist_internal(&self) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS conversation_segments (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                segment_index INTEGER NOT NULL,
                start_message_index INTEGER NOT NULL,
                end_message_index INTEGER NOT NULL,
                summary TEXT NOT NULL,
                summary_tokens INTEGER NOT NULL,
                created_at BIGINT NOT NULL
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_segments_conv ON conversation_segments(conversation_id, segment_index)"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS conversation_global_summaries (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL UNIQUE,
                summary TEXT NOT NULL,
                summary_tokens INTEGER NOT NULL,
                covers_up_to_index INTEGER NOT NULL,
                updated_at BIGINT NOT NULL
            )"#
        ).execute(pool).await?;

        Ok(())
    }

    pub async fn get_sliding_window_summaries_internal(
        &self,
        conversation_id: &str,
    ) -> Result<(Option<GlobalSummary>, Vec<ConversationSegment>)> {
        let pool = self.get_pool()?;
        
        let global_summary = sqlx::query_as::<_, GlobalSummary>(
            "SELECT * FROM conversation_global_summaries WHERE conversation_id = $1"
        )
        .bind(conversation_id)
        .fetch_optional(pool)
        .await?;

        let segments = sqlx::query_as::<_, ConversationSegment>(
            "SELECT * FROM conversation_segments WHERE conversation_id = $1 ORDER BY segment_index ASC"
        )
        .bind(conversation_id)
        .fetch_all(pool)
        .await?;

        Ok((global_summary, segments))
    }

    pub async fn save_conversation_segment_internal(&self, segment: &ConversationSegment) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query(
            r#"INSERT INTO conversation_segments 
            (id, conversation_id, segment_index, start_message_index, end_message_index, summary, summary_tokens, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#
        )
        .bind(&segment.id)
        .bind(&segment.conversation_id)
        .bind(segment.segment_index)
        .bind(segment.start_message_index)
        .bind(segment.end_message_index)
        .bind(&segment.summary)
        .bind(segment.summary_tokens)
        .bind(segment.created_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn upsert_global_summary_internal(&self, summary: &GlobalSummary) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query(
            r#"INSERT INTO conversation_global_summaries (id, conversation_id, summary, summary_tokens, covers_up_to_index, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT(conversation_id) DO UPDATE SET
            summary = excluded.summary,
            summary_tokens = excluded.summary_tokens,
            covers_up_to_index = excluded.covers_up_to_index,
            updated_at = excluded.updated_at"#
        )
        .bind(&summary.id)
        .bind(&summary.conversation_id)
        .bind(&summary.summary)
        .bind(summary.summary_tokens)
        .bind(summary.covers_up_to_index)
        .bind(summary.updated_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_conversation_segments_internal(&self, segment_ids: &[String]) -> Result<()> {
        let pool = self.get_pool()?;
        
        for id in segment_ids {
            sqlx::query("DELETE FROM conversation_segments WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await?;
        }
        
        Ok(())
    }
}
