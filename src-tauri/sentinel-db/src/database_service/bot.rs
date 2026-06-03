use crate::core::models::database::{BotAccount, BotExecutionRun, BotMessage, BotPeer};
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::PgPool;
use anyhow::Result;
use chrono::{DateTime, Utc};

impl DatabaseService {
    pub async fn create_bot_schema(&self, pool: &PgPool) -> Result<()> {
        let statements = [
            r#"CREATE TABLE IF NOT EXISTS bot_accounts (
                id TEXT PRIMARY KEY,
                transport TEXT NOT NULL,
                account_id TEXT NOT NULL,
                display_name TEXT,
                status TEXT,
                last_seen_at TIMESTAMP WITH TIME ZONE,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS bot_peers (
                id TEXT PRIMARY KEY,
                transport TEXT NOT NULL,
                account_id TEXT NOT NULL,
                peer_type TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                display_name TEXT,
                last_sender_id TEXT,
                last_message_at TIMESTAMP WITH TIME ZONE,
                last_inbound_message_at TIMESTAMP WITH TIME ZONE,
                last_outbound_message_at TIMESTAMP WITH TIME ZONE,
                message_count BIGINT NOT NULL DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS bot_messages (
                id TEXT PRIMARY KEY,
                transport TEXT NOT NULL,
                account_id TEXT NOT NULL,
                peer_type TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                sender_id TEXT NOT NULL,
                direction TEXT NOT NULL,
                content TEXT NOT NULL,
                transport_message_id TEXT,
                context_token TEXT,
                conversation_id TEXT,
                ai_message_id TEXT,
                linked_execution_run_id TEXT,
                metadata_json TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS bot_execution_runs (
                id TEXT PRIMARY KEY,
                transport TEXT NOT NULL,
                account_id TEXT NOT NULL,
                peer_type TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                sender_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                ai_execution_id TEXT NOT NULL,
                assistant_profile_id TEXT,
                trigger_kind TEXT NOT NULL,
                trigger_bot_message_id TEXT,
                trigger_ai_message_id TEXT,
                task_text TEXT NOT NULL,
                status TEXT NOT NULL,
                result_text TEXT,
                error_message TEXT,
                started_at TIMESTAMP WITH TIME ZONE NOT NULL,
                completed_at TIMESTAMP WITH TIME ZONE,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
            "CREATE INDEX IF NOT EXISTS idx_bot_accounts_transport_account ON bot_accounts(transport, account_id)",
            "CREATE INDEX IF NOT EXISTS idx_bot_peers_transport_account_peer ON bot_peers(transport, account_id, peer_type, peer_id)",
            "CREATE INDEX IF NOT EXISTS idx_bot_peers_last_message_at ON bot_peers(last_message_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_bot_messages_peer_created_at ON bot_messages(transport, account_id, peer_type, peer_id, created_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_bot_messages_execution_run ON bot_messages(linked_execution_run_id, created_at ASC)",
            "CREATE INDEX IF NOT EXISTS idx_bot_execution_runs_peer_started_at ON bot_execution_runs(transport, account_id, peer_type, peer_id, started_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_bot_execution_runs_status_started_at ON bot_execution_runs(status, started_at DESC)",
        ];

        for sql in statements {
            sqlx::query(sql).execute(pool).await?;
        }
        Ok(())
    }

    pub async fn ensure_bot_compat_schema(&self, runtime: &DatabasePool) -> Result<()> {
        let statements = [
            r#"CREATE TABLE IF NOT EXISTS bot_accounts (
                id TEXT PRIMARY KEY,
                transport TEXT NOT NULL,
                account_id TEXT NOT NULL,
                display_name TEXT,
                status TEXT,
                last_seen_at DATETIME,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS bot_peers (
                id TEXT PRIMARY KEY,
                transport TEXT NOT NULL,
                account_id TEXT NOT NULL,
                peer_type TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                display_name TEXT,
                last_sender_id TEXT,
                last_message_at DATETIME,
                last_inbound_message_at DATETIME,
                last_outbound_message_at DATETIME,
                message_count BIGINT NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS bot_messages (
                id TEXT PRIMARY KEY,
                transport TEXT NOT NULL,
                account_id TEXT NOT NULL,
                peer_type TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                sender_id TEXT NOT NULL,
                direction TEXT NOT NULL,
                content TEXT NOT NULL,
                transport_message_id TEXT,
                context_token TEXT,
                conversation_id TEXT,
                ai_message_id TEXT,
                linked_execution_run_id TEXT,
                metadata_json TEXT,
                created_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS bot_execution_runs (
                id TEXT PRIMARY KEY,
                transport TEXT NOT NULL,
                account_id TEXT NOT NULL,
                peer_type TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                sender_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                ai_execution_id TEXT NOT NULL,
                assistant_profile_id TEXT,
                trigger_kind TEXT NOT NULL,
                trigger_bot_message_id TEXT,
                trigger_ai_message_id TEXT,
                task_text TEXT NOT NULL,
                status TEXT NOT NULL,
                result_text TEXT,
                error_message TEXT,
                started_at DATETIME NOT NULL,
                completed_at DATETIME,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            "CREATE INDEX IF NOT EXISTS idx_bot_accounts_transport_account ON bot_accounts(transport, account_id)",
            "CREATE INDEX IF NOT EXISTS idx_bot_peers_transport_account_peer ON bot_peers(transport, account_id, peer_type, peer_id)",
            "CREATE INDEX IF NOT EXISTS idx_bot_peers_last_message_at ON bot_peers(last_message_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_bot_messages_peer_created_at ON bot_messages(transport, account_id, peer_type, peer_id, created_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_bot_messages_execution_run ON bot_messages(linked_execution_run_id, created_at ASC)",
            "CREATE INDEX IF NOT EXISTS idx_bot_execution_runs_peer_started_at ON bot_execution_runs(transport, account_id, peer_type, peer_id, started_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_bot_execution_runs_status_started_at ON bot_execution_runs(status, started_at DESC)",
        ];

        for sql in statements {
            execute_runtime_query(runtime, sql).await?;
        }
        Ok(())
    }

    pub async fn upsert_bot_account_presence(
        &self,
        transport: &str,
        account_id: &str,
        display_name: Option<&str>,
        status: Option<&str>,
        seen_at: DateTime<Utc>,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let id = format!("{transport}:{account_id}");

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_accounts (
                        id, transport, account_id, display_name, status, last_seen_at, created_at, updated_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT (id) DO UPDATE SET
                        display_name = COALESCE(EXCLUDED.display_name, bot_accounts.display_name),
                        status = COALESCE(EXCLUDED.status, bot_accounts.status),
                        last_seen_at = EXCLUDED.last_seen_at,
                        updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(&id)
                .bind(transport)
                .bind(account_id)
                .bind(display_name)
                .bind(status)
                .bind(seen_at)
                .bind(seen_at)
                .bind(seen_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_accounts (
                        id, transport, account_id, display_name, status, last_seen_at, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT (id) DO UPDATE SET
                        display_name = COALESCE(excluded.display_name, bot_accounts.display_name),
                        status = COALESCE(excluded.status, bot_accounts.status),
                        last_seen_at = excluded.last_seen_at,
                        updated_at = excluded.updated_at
                    "#,
                )
                .bind(&id)
                .bind(transport)
                .bind(account_id)
                .bind(display_name)
                .bind(status)
                .bind(seen_at)
                .bind(seen_at)
                .bind(seen_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_accounts (
                        id, transport, account_id, display_name, status, last_seen_at, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    ON DUPLICATE KEY UPDATE
                        display_name = COALESCE(VALUES(display_name), display_name),
                        status = COALESCE(VALUES(status), status),
                        last_seen_at = VALUES(last_seen_at),
                        updated_at = VALUES(updated_at)
                    "#,
                )
                .bind(&id)
                .bind(transport)
                .bind(account_id)
                .bind(display_name)
                .bind(status)
                .bind(seen_at)
                .bind(seen_at)
                .bind(seen_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn record_bot_peer_activity(
        &self,
        transport: &str,
        account_id: &str,
        peer_type: &str,
        peer_id: &str,
        sender_id: Option<&str>,
        direction: &str,
        observed_at: DateTime<Utc>,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let id = format!("{transport}:{account_id}:{peer_type}:{peer_id}");
        let last_inbound_message_at = (direction == "inbound").then_some(observed_at);
        let last_outbound_message_at = (direction == "outbound").then_some(observed_at);

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_peers (
                        id, transport, account_id, peer_type, peer_id, display_name,
                        last_sender_id, last_message_at, last_inbound_message_at,
                        last_outbound_message_at, message_count, created_at, updated_at
                    ) VALUES ($1, $2, $3, $4, $5, NULL, $6, $7, $8, $9, 1, $10, $11)
                    ON CONFLICT (id) DO UPDATE SET
                        last_sender_id = COALESCE(EXCLUDED.last_sender_id, bot_peers.last_sender_id),
                        last_message_at = EXCLUDED.last_message_at,
                        last_inbound_message_at = COALESCE(EXCLUDED.last_inbound_message_at, bot_peers.last_inbound_message_at),
                        last_outbound_message_at = COALESCE(EXCLUDED.last_outbound_message_at, bot_peers.last_outbound_message_at),
                        message_count = bot_peers.message_count + 1,
                        updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(&id)
                .bind(transport)
                .bind(account_id)
                .bind(peer_type)
                .bind(peer_id)
                .bind(sender_id)
                .bind(observed_at)
                .bind(last_inbound_message_at)
                .bind(last_outbound_message_at)
                .bind(observed_at)
                .bind(observed_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_peers (
                        id, transport, account_id, peer_type, peer_id, display_name,
                        last_sender_id, last_message_at, last_inbound_message_at,
                        last_outbound_message_at, message_count, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, 1, ?, ?)
                    ON CONFLICT (id) DO UPDATE SET
                        last_sender_id = COALESCE(excluded.last_sender_id, bot_peers.last_sender_id),
                        last_message_at = excluded.last_message_at,
                        last_inbound_message_at = COALESCE(excluded.last_inbound_message_at, bot_peers.last_inbound_message_at),
                        last_outbound_message_at = COALESCE(excluded.last_outbound_message_at, bot_peers.last_outbound_message_at),
                        message_count = bot_peers.message_count + 1,
                        updated_at = excluded.updated_at
                    "#,
                )
                .bind(&id)
                .bind(transport)
                .bind(account_id)
                .bind(peer_type)
                .bind(peer_id)
                .bind(sender_id)
                .bind(observed_at)
                .bind(last_inbound_message_at)
                .bind(last_outbound_message_at)
                .bind(observed_at)
                .bind(observed_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_peers (
                        id, transport, account_id, peer_type, peer_id, display_name,
                        last_sender_id, last_message_at, last_inbound_message_at,
                        last_outbound_message_at, message_count, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, 1, ?, ?)
                    ON DUPLICATE KEY UPDATE
                        last_sender_id = COALESCE(VALUES(last_sender_id), last_sender_id),
                        last_message_at = VALUES(last_message_at),
                        last_inbound_message_at = COALESCE(VALUES(last_inbound_message_at), last_inbound_message_at),
                        last_outbound_message_at = COALESCE(VALUES(last_outbound_message_at), last_outbound_message_at),
                        message_count = message_count + 1,
                        updated_at = VALUES(updated_at)
                    "#,
                )
                .bind(&id)
                .bind(transport)
                .bind(account_id)
                .bind(peer_type)
                .bind(peer_id)
                .bind(sender_id)
                .bind(observed_at)
                .bind(last_inbound_message_at)
                .bind(last_outbound_message_at)
                .bind(observed_at)
                .bind(observed_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn create_bot_message(&self, message: &BotMessage) -> Result<()> {
        let _permit = self
            .write_semaphore
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_messages (
                        id, transport, account_id, peer_type, peer_id, sender_id, direction,
                        content, transport_message_id, context_token, conversation_id,
                        ai_message_id, linked_execution_run_id, metadata_json, created_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                    "#,
                )
                .bind(&message.id)
                .bind(&message.transport)
                .bind(&message.account_id)
                .bind(&message.peer_type)
                .bind(&message.peer_id)
                .bind(&message.sender_id)
                .bind(&message.direction)
                .bind(&message.content)
                .bind(&message.transport_message_id)
                .bind(&message.context_token)
                .bind(&message.conversation_id)
                .bind(&message.ai_message_id)
                .bind(&message.linked_execution_run_id)
                .bind(&message.metadata_json)
                .bind(message.created_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_messages (
                        id, transport, account_id, peer_type, peer_id, sender_id, direction,
                        content, transport_message_id, context_token, conversation_id,
                        ai_message_id, linked_execution_run_id, metadata_json, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&message.id)
                .bind(&message.transport)
                .bind(&message.account_id)
                .bind(&message.peer_type)
                .bind(&message.peer_id)
                .bind(&message.sender_id)
                .bind(&message.direction)
                .bind(&message.content)
                .bind(&message.transport_message_id)
                .bind(&message.context_token)
                .bind(&message.conversation_id)
                .bind(&message.ai_message_id)
                .bind(&message.linked_execution_run_id)
                .bind(&message.metadata_json)
                .bind(message.created_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_messages (
                        id, transport, account_id, peer_type, peer_id, sender_id, direction,
                        content, transport_message_id, context_token, conversation_id,
                        ai_message_id, linked_execution_run_id, metadata_json, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&message.id)
                .bind(&message.transport)
                .bind(&message.account_id)
                .bind(&message.peer_type)
                .bind(&message.peer_id)
                .bind(&message.sender_id)
                .bind(&message.direction)
                .bind(&message.content)
                .bind(&message.transport_message_id)
                .bind(&message.context_token)
                .bind(&message.conversation_id)
                .bind(&message.ai_message_id)
                .bind(&message.linked_execution_run_id)
                .bind(&message.metadata_json)
                .bind(message.created_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_bot_accounts(&self, transport: Option<&str>) -> Result<Vec<BotAccount>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let records = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, BotAccount>(
                    r#"SELECT
                        id, transport, account_id, display_name, status,
                        last_seen_at, created_at, updated_at
                    FROM bot_accounts
                    WHERE ($1::text IS NULL OR transport = $1)
                    ORDER BY updated_at DESC, account_id ASC"#,
                )
                .bind(transport)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, BotAccount>(
                    r#"SELECT
                        id, transport, account_id, display_name, status,
                        last_seen_at, created_at, updated_at
                    FROM bot_accounts
                    WHERE (? IS NULL OR transport = ?)
                    ORDER BY updated_at DESC, account_id ASC"#,
                )
                .bind(transport)
                .bind(transport)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, BotAccount>(
                    r#"SELECT
                        id, transport, account_id, display_name, status,
                        last_seen_at, created_at, updated_at
                    FROM bot_accounts
                    WHERE (? IS NULL OR transport = ?)
                    ORDER BY updated_at DESC, account_id ASC"#,
                )
                .bind(transport)
                .bind(transport)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(records)
    }

    pub async fn list_bot_peers(
        &self,
        transport: Option<&str>,
        account_id: Option<&str>,
        peer_type: Option<&str>,
        limit: i64,
    ) -> Result<Vec<BotPeer>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let records = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, BotPeer>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, display_name,
                        last_sender_id, last_message_at, last_inbound_message_at,
                        last_outbound_message_at, message_count,
                        COALESCE((
                            SELECT COUNT(*)
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                        ), 0) AS execution_run_count,
                        COALESCE((
                            SELECT COUNT(*)
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                              AND LOWER(ber.status) IN ('failed', 'error')
                        ), 0) AS failed_execution_count,
                        COALESCE((
                            SELECT COUNT(*)
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                              AND LOWER(ber.status) IN ('running', 'in_progress')
                        ), 0) AS running_execution_count,
                        (
                            SELECT ber.status
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                            ORDER BY ber.started_at DESC
                            LIMIT 1
                        ) AS latest_execution_status,
                        (
                            SELECT ber.started_at
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                            ORDER BY ber.started_at DESC
                            LIMIT 1
                        ) AS latest_execution_started_at,
                        created_at, updated_at
                    FROM bot_peers
                    WHERE ($1::text IS NULL OR transport = $1)
                      AND ($2::text IS NULL OR account_id = $2)
                      AND ($3::text IS NULL OR peer_type = $3)
                    ORDER BY last_message_at DESC NULLS LAST, updated_at DESC
                    LIMIT $4"#,
                )
                .bind(transport)
                .bind(account_id)
                .bind(peer_type)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, BotPeer>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, display_name,
                        last_sender_id, last_message_at, last_inbound_message_at,
                        last_outbound_message_at, message_count,
                        COALESCE((
                            SELECT COUNT(*)
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                        ), 0) AS execution_run_count,
                        COALESCE((
                            SELECT COUNT(*)
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                              AND LOWER(ber.status) IN ('failed', 'error')
                        ), 0) AS failed_execution_count,
                        COALESCE((
                            SELECT COUNT(*)
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                              AND LOWER(ber.status) IN ('running', 'in_progress')
                        ), 0) AS running_execution_count,
                        (
                            SELECT ber.status
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                            ORDER BY ber.started_at DESC
                            LIMIT 1
                        ) AS latest_execution_status,
                        (
                            SELECT ber.started_at
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                            ORDER BY ber.started_at DESC
                            LIMIT 1
                        ) AS latest_execution_started_at,
                        created_at, updated_at
                    FROM bot_peers
                    WHERE (? IS NULL OR transport = ?)
                      AND (? IS NULL OR account_id = ?)
                      AND (? IS NULL OR peer_type = ?)
                    ORDER BY (last_message_at IS NULL) ASC, last_message_at DESC, updated_at DESC
                    LIMIT ?"#,
                )
                .bind(transport)
                .bind(transport)
                .bind(account_id)
                .bind(account_id)
                .bind(peer_type)
                .bind(peer_type)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, BotPeer>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, display_name,
                        last_sender_id, last_message_at, last_inbound_message_at,
                        last_outbound_message_at, message_count,
                        COALESCE((
                            SELECT COUNT(*)
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                        ), 0) AS execution_run_count,
                        COALESCE((
                            SELECT COUNT(*)
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                              AND LOWER(ber.status) IN ('failed', 'error')
                        ), 0) AS failed_execution_count,
                        COALESCE((
                            SELECT COUNT(*)
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                              AND LOWER(ber.status) IN ('running', 'in_progress')
                        ), 0) AS running_execution_count,
                        (
                            SELECT ber.status
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                            ORDER BY ber.started_at DESC
                            LIMIT 1
                        ) AS latest_execution_status,
                        (
                            SELECT ber.started_at
                            FROM bot_execution_runs ber
                            WHERE ber.transport = bot_peers.transport
                              AND ber.account_id = bot_peers.account_id
                              AND ber.peer_type = bot_peers.peer_type
                              AND ber.peer_id = bot_peers.peer_id
                            ORDER BY ber.started_at DESC
                            LIMIT 1
                        ) AS latest_execution_started_at,
                        created_at, updated_at
                    FROM bot_peers
                    WHERE (? IS NULL OR transport = ?)
                      AND (? IS NULL OR account_id = ?)
                      AND (? IS NULL OR peer_type = ?)
                    ORDER BY (last_message_at IS NULL) ASC, last_message_at DESC, updated_at DESC
                    LIMIT ?"#,
                )
                .bind(transport)
                .bind(transport)
                .bind(account_id)
                .bind(account_id)
                .bind(peer_type)
                .bind(peer_type)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(records)
    }

    pub async fn list_bot_messages_for_peer(
        &self,
        transport: &str,
        account_id: &str,
        peer_type: &str,
        peer_id: &str,
        limit: i64,
    ) -> Result<Vec<BotMessage>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let records = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, BotMessage>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, direction,
                        content, transport_message_id, context_token, conversation_id,
                        ai_message_id, linked_execution_run_id, metadata_json, created_at
                    FROM bot_messages
                    WHERE transport = $1
                      AND account_id = $2
                      AND peer_type = $3
                      AND peer_id = $4
                    ORDER BY created_at DESC
                    LIMIT $5"#,
                )
                .bind(transport)
                .bind(account_id)
                .bind(peer_type)
                .bind(peer_id)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, BotMessage>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, direction,
                        content, transport_message_id, context_token, conversation_id,
                        ai_message_id, linked_execution_run_id, metadata_json, created_at
                    FROM bot_messages
                    WHERE transport = ?
                      AND account_id = ?
                      AND peer_type = ?
                      AND peer_id = ?
                    ORDER BY created_at DESC
                    LIMIT ?"#,
                )
                .bind(transport)
                .bind(account_id)
                .bind(peer_type)
                .bind(peer_id)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, BotMessage>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, direction,
                        content, transport_message_id, context_token, conversation_id,
                        ai_message_id, linked_execution_run_id, metadata_json, created_at
                    FROM bot_messages
                    WHERE transport = ?
                      AND account_id = ?
                      AND peer_type = ?
                      AND peer_id = ?
                    ORDER BY created_at DESC
                    LIMIT ?"#,
                )
                .bind(transport)
                .bind(account_id)
                .bind(peer_type)
                .bind(peer_id)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(records)
    }

    pub async fn create_bot_execution_run(&self, run: &BotExecutionRun) -> Result<()> {
        let _permit = self
            .write_semaphore
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_execution_runs (
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                        $11, $12, $13, $14, $15, $16, $17, $18, $19, $20
                    )
                    "#,
                )
                .bind(&run.id)
                .bind(&run.transport)
                .bind(&run.account_id)
                .bind(&run.peer_type)
                .bind(&run.peer_id)
                .bind(&run.sender_id)
                .bind(&run.conversation_id)
                .bind(&run.ai_execution_id)
                .bind(&run.assistant_profile_id)
                .bind(&run.trigger_kind)
                .bind(&run.trigger_bot_message_id)
                .bind(&run.trigger_ai_message_id)
                .bind(&run.task_text)
                .bind(&run.status)
                .bind(&run.result_text)
                .bind(&run.error_message)
                .bind(run.started_at)
                .bind(run.completed_at)
                .bind(run.created_at)
                .bind(run.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_execution_runs (
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    ) VALUES (
                        ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                        ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
                    )
                    "#,
                )
                .bind(&run.id)
                .bind(&run.transport)
                .bind(&run.account_id)
                .bind(&run.peer_type)
                .bind(&run.peer_id)
                .bind(&run.sender_id)
                .bind(&run.conversation_id)
                .bind(&run.ai_execution_id)
                .bind(&run.assistant_profile_id)
                .bind(&run.trigger_kind)
                .bind(&run.trigger_bot_message_id)
                .bind(&run.trigger_ai_message_id)
                .bind(&run.task_text)
                .bind(&run.status)
                .bind(&run.result_text)
                .bind(&run.error_message)
                .bind(run.started_at)
                .bind(run.completed_at)
                .bind(run.created_at)
                .bind(run.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO bot_execution_runs (
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    ) VALUES (
                        ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                        ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
                    )
                    "#,
                )
                .bind(&run.id)
                .bind(&run.transport)
                .bind(&run.account_id)
                .bind(&run.peer_type)
                .bind(&run.peer_id)
                .bind(&run.sender_id)
                .bind(&run.conversation_id)
                .bind(&run.ai_execution_id)
                .bind(&run.assistant_profile_id)
                .bind(&run.trigger_kind)
                .bind(&run.trigger_bot_message_id)
                .bind(&run.trigger_ai_message_id)
                .bind(&run.task_text)
                .bind(&run.status)
                .bind(&run.result_text)
                .bind(&run.error_message)
                .bind(run.started_at)
                .bind(run.completed_at)
                .bind(run.created_at)
                .bind(run.updated_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_bot_execution_runs_for_peer(
        &self,
        transport: &str,
        account_id: &str,
        peer_type: &str,
        peer_id: &str,
        limit: i64,
    ) -> Result<Vec<BotExecutionRun>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let records = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, BotExecutionRun>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    FROM bot_execution_runs
                    WHERE transport = $1
                      AND account_id = $2
                      AND peer_type = $3
                      AND peer_id = $4
                    ORDER BY started_at DESC
                    LIMIT $5"#,
                )
                .bind(transport)
                .bind(account_id)
                .bind(peer_type)
                .bind(peer_id)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, BotExecutionRun>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    FROM bot_execution_runs
                    WHERE transport = ?
                      AND account_id = ?
                      AND peer_type = ?
                      AND peer_id = ?
                    ORDER BY started_at DESC
                    LIMIT ?"#,
                )
                .bind(transport)
                .bind(account_id)
                .bind(peer_type)
                .bind(peer_id)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, BotExecutionRun>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    FROM bot_execution_runs
                    WHERE transport = ?
                      AND account_id = ?
                      AND peer_type = ?
                      AND peer_id = ?
                    ORDER BY started_at DESC
                    LIMIT ?"#,
                )
                .bind(transport)
                .bind(account_id)
                .bind(peer_type)
                .bind(peer_id)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(records)
    }

    pub async fn get_bot_execution_run(&self, id: &str) -> Result<Option<BotExecutionRun>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let record = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, BotExecutionRun>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    FROM bot_execution_runs
                    WHERE id = $1 OR ai_execution_id = $1
                    LIMIT 1"#,
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, BotExecutionRun>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    FROM bot_execution_runs
                    WHERE id = ? OR ai_execution_id = ?
                    LIMIT 1"#,
                )
                .bind(id)
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, BotExecutionRun>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    FROM bot_execution_runs
                    WHERE id = ? OR ai_execution_id = ?
                    LIMIT 1"#,
                )
                .bind(id)
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(record)
    }

    pub async fn update_bot_execution_run_result(
        &self,
        id: &str,
        status: &str,
        result_text: Option<&str>,
        error_message: Option<&str>,
        completed_at: DateTime<Utc>,
    ) -> Result<()> {
        let _permit = self
            .write_semaphore
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    UPDATE bot_execution_runs
                    SET status = $1,
                        result_text = $2,
                        error_message = $3,
                        completed_at = $4,
                        updated_at = $4
                    WHERE id = $5
                    "#,
                )
                .bind(status)
                .bind(result_text)
                .bind(error_message)
                .bind(completed_at)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    UPDATE bot_execution_runs
                    SET status = ?,
                        result_text = ?,
                        error_message = ?,
                        completed_at = ?,
                        updated_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(status)
                .bind(result_text)
                .bind(error_message)
                .bind(completed_at)
                .bind(completed_at)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    UPDATE bot_execution_runs
                    SET status = ?,
                        result_text = ?,
                        error_message = ?,
                        completed_at = ?,
                        updated_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(status)
                .bind(result_text)
                .bind(error_message)
                .bind(completed_at)
                .bind(completed_at)
                .bind(id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }
}

async fn execute_runtime_query(runtime: &DatabasePool, sql: &str) -> Result<()> {
    match runtime {
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(sql).execute(pool).await?;
        }
        DatabasePool::SQLite(pool) => {
            sqlx::query(sql).execute(pool).await?;
        }
        DatabasePool::MySQL(pool) => {
            sqlx::query(sql).execute(pool).await?;
        }
    }
    Ok(())
}
