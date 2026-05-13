use anyhow::Result;
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet};

struct SqliteIndex {
    table: &'static str,
    columns: &'static [&'static str],
    sql: &'static str,
}

pub async fn configure_sqlite_connection(
    conn: &mut sqlx::SqliteConnection,
    enable_wal: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&mut *conn)
        .await?;
    sqlx::query("PRAGMA busy_timeout = 10000")
        .execute(&mut *conn)
        .await?;

    if enable_wal {
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&mut *conn)
            .await?;
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&mut *conn)
            .await?;
        sqlx::query("PRAGMA wal_autocheckpoint = 1000")
            .execute(&mut *conn)
            .await?;
        sqlx::query("PRAGMA journal_size_limit = 67108864")
            .execute(&mut *conn)
            .await?;
    }

    sqlx::query("PRAGMA temp_store = MEMORY")
        .execute(&mut *conn)
        .await?;
    sqlx::query("PRAGMA cache_size = -20000")
        .execute(&mut *conn)
        .await?;
    sqlx::query("PRAGMA mmap_size = 268435456")
        .execute(&mut *conn)
        .await?;
    sqlx::query("PRAGMA analysis_limit = 1000")
        .execute(&mut *conn)
        .await?;

    Ok(())
}

pub async fn ensure_sqlite_crud_performance(pool: &sqlx::SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;
    let mut column_cache = BTreeMap::new();

    for index in SQLITE_CRUD_INDEXES {
        let columns = table_columns(&mut tx, &mut column_cache, index.table).await?;
        if columns.is_empty() || !index.columns.iter().all(|column| columns.contains(*column)) {
            continue;
        }

        sqlx::query(index.sql).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    sqlx::query("PRAGMA optimize").execute(pool).await?;

    Ok(())
}

async fn table_columns(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    cache: &mut BTreeMap<&'static str, BTreeSet<String>>,
    table: &'static str,
) -> Result<BTreeSet<String>> {
    if let Some(columns) = cache.get(table) {
        return Ok(columns.clone());
    }

    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(&mut **tx)
        .await?;
    let columns = rows
        .into_iter()
        .filter_map(|row| row.try_get::<String, _>("name").ok())
        .collect::<BTreeSet<_>>();
    cache.insert(table, columns.clone());
    Ok(columns)
}

const SQLITE_CRUD_INDEXES: &[SqliteIndex] = &[
    SqliteIndex {
        table: "ai_conversations",
        columns: &["created_at", "updated_at", "service_name", "context_type"],
        sql: "CREATE INDEX IF NOT EXISTS idx_ai_conversations_visible_created_updated ON ai_conversations(created_at DESC, updated_at DESC) WHERE service_name != 'subagent' AND (context_type IS NULL OR context_type != 'subagent')",
    },
    SqliteIndex {
        table: "ai_conversations",
        columns: &["project_id", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_ai_conversations_project_created ON ai_conversations(project_id, created_at DESC)",
    },
    SqliteIndex {
        table: "ai_conversations",
        columns: &["vulnerability_id", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_ai_conversations_vulnerability_created ON ai_conversations(vulnerability_id, created_at DESC)",
    },
    SqliteIndex {
        table: "ai_conversations",
        columns: &["scan_task_id", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_ai_conversations_scan_task_created ON ai_conversations(scan_task_id, created_at DESC)",
    },
    SqliteIndex {
        table: "ai_messages",
        columns: &["conversation_id", "timestamp", "id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_ai_messages_conversation_timestamp_id ON ai_messages(conversation_id, timestamp ASC, id ASC)",
    },
    SqliteIndex {
        table: "ai_subagent_runs",
        columns: &["parent_execution_id", "started_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_ai_subagent_runs_parent_started ON ai_subagent_runs(parent_execution_id, started_at DESC)",
    },
    SqliteIndex {
        table: "ai_subagent_messages",
        columns: &["subagent_run_id", "timestamp"],
        sql: "CREATE INDEX IF NOT EXISTS idx_ai_subagent_messages_run_timestamp ON ai_subagent_messages(subagent_run_id, timestamp ASC)",
    },
    SqliteIndex {
        table: "scan_tasks",
        columns: &["project_id", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_scan_tasks_project_created ON scan_tasks(project_id, created_at DESC)",
    },
    SqliteIndex {
        table: "scan_tasks",
        columns: &["status", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_scan_tasks_status_created ON scan_tasks(status, created_at DESC)",
    },
    SqliteIndex {
        table: "vulnerabilities",
        columns: &["project_id", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_vulnerabilities_project_created ON vulnerabilities(project_id, created_at DESC)",
    },
    SqliteIndex {
        table: "vulnerabilities",
        columns: &["scan_task_id", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_vulnerabilities_scan_task_created ON vulnerabilities(scan_task_id, created_at DESC)",
    },
    SqliteIndex {
        table: "vulnerabilities",
        columns: &["asset_id", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_vulnerabilities_asset_created ON vulnerabilities(asset_id, created_at DESC)",
    },
    SqliteIndex {
        table: "assets",
        columns: &["project_id", "asset_type", "status", "last_seen"],
        sql: "CREATE INDEX IF NOT EXISTS idx_assets_project_type_status_seen ON assets(project_id, asset_type, status, last_seen DESC)",
    },
    SqliteIndex {
        table: "workflow_definitions",
        columns: &["is_template", "category", "updated_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_workflow_definitions_template_category_updated ON workflow_definitions(is_template, category, updated_at DESC)",
    },
    SqliteIndex {
        table: "plugin_registry",
        columns: &["main_category", "category", "enabled", "updated_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_plugin_registry_category_enabled_updated ON plugin_registry(main_category, category, enabled, updated_at DESC)",
    },
    SqliteIndex {
        table: "traffic_vulnerabilities",
        columns: &["status", "severity", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_traffic_vulnerabilities_status_severity_created ON traffic_vulnerabilities(status, severity, created_at DESC)",
    },
    SqliteIndex {
        table: "traffic_vulnerabilities",
        columns: &["session_id", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_traffic_vulnerabilities_session_created ON traffic_vulnerabilities(session_id, created_at DESC)",
    },
    SqliteIndex {
        table: "traffic_vulnerabilities",
        columns: &["last_seen_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_traffic_vulnerabilities_last_seen ON traffic_vulnerabilities(last_seen_at DESC)",
    },
    SqliteIndex {
        table: "traffic_evidence",
        columns: &["vuln_id", "timestamp"],
        sql: "CREATE INDEX IF NOT EXISTS idx_traffic_evidence_vuln_timestamp ON traffic_evidence(vuln_id, timestamp DESC)",
    },
    SqliteIndex {
        table: "proxy_requests",
        columns: &["host", "timestamp", "id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_proxy_requests_host_timestamp_id ON proxy_requests(host, timestamp DESC, id DESC)",
    },
    SqliteIndex {
        table: "proxy_requests",
        columns: &["scheme", "method", "status_code", "timestamp"],
        sql: "CREATE INDEX IF NOT EXISTS idx_proxy_requests_scheme_method_status_time ON proxy_requests(scheme, method, status_code, timestamp DESC)",
    },
    SqliteIndex {
        table: "proxy_requests",
        columns: &["origin_kind", "origin_ref_id", "timestamp"],
        sql: "CREATE INDEX IF NOT EXISTS idx_proxy_requests_origin_timestamp ON proxy_requests(origin_kind, origin_ref_id, timestamp DESC)",
    },
    SqliteIndex {
        table: "memory_records",
        columns: &["status", "scope", "kind", "updated_at_ms"],
        sql: "CREATE INDEX IF NOT EXISTS idx_memory_records_status_scope_kind_updated ON memory_records(status, scope, kind, updated_at_ms DESC)",
    },
    SqliteIndex {
        table: "memory_projection_state",
        columns: &["lexical_indexed", "vector_indexed", "skill_projected", "updated_at_ms"],
        sql: "CREATE INDEX IF NOT EXISTS idx_memory_projection_state_pending_updated ON memory_projection_state(lexical_indexed, vector_indexed, skill_projected, updated_at_ms)",
    },
    SqliteIndex {
        table: "execution_tasks",
        columns: &["execution_id", "status", "item_index"],
        sql: "CREATE INDEX IF NOT EXISTS idx_execution_tasks_execution_status_index ON execution_tasks(execution_id, status, item_index)",
    },
    SqliteIndex {
        table: "agent_sessions",
        columns: &["task_id", "status", "updated_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_agent_sessions_task_status_updated ON agent_sessions(task_id, status, updated_at DESC)",
    },
    SqliteIndex {
        table: "agent_session_logs",
        columns: &["session_id", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_agent_session_logs_session_created ON agent_session_logs(session_id, created_at ASC)",
    },
    SqliteIndex {
        table: "dictionary_words",
        columns: &["dictionary_id", "word"],
        sql: "CREATE INDEX IF NOT EXISTS idx_dictionary_words_dictionary_word ON dictionary_words(dictionary_id, word)",
    },
    SqliteIndex {
        table: "dictionaries",
        columns: &["dict_type", "is_active", "updated_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_dictionaries_type_active_updated ON dictionaries(dict_type, is_active, updated_at DESC)",
    },
    SqliteIndex {
        table: "rag_document_sources",
        columns: &["collection_id", "status", "updated_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_rag_document_sources_collection_status_updated ON rag_document_sources(collection_id, status, updated_at DESC)",
    },
    SqliteIndex {
        table: "rag_chunks",
        columns: &["collection_id", "document_id", "chunk_index"],
        sql: "CREATE INDEX IF NOT EXISTS idx_rag_chunks_collection_document_index ON rag_chunks(collection_id, document_id, chunk_index)",
    },
    SqliteIndex {
        table: "system_agent_bindings",
        columns: &["event_name", "enabled", "priority"],
        sql: "CREATE INDEX IF NOT EXISTS idx_system_agent_bindings_event_enabled_priority ON system_agent_bindings(event_name, enabled, priority DESC)",
    },
    SqliteIndex {
        table: "system_agent_runs",
        columns: &["profile_id", "status", "started_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_system_agent_runs_profile_status_started ON system_agent_runs(profile_id, status, started_at DESC)",
    },
    SqliteIndex {
        table: "surface_org_assets",
        columns: &["asset_id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_org_assets_asset_id ON surface_org_assets(asset_id)",
    },
    SqliteIndex {
        table: "surface_domain_assets",
        columns: &["asset_id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_domain_assets_asset_id ON surface_domain_assets(asset_id)",
    },
    SqliteIndex {
        table: "surface_ip_assets",
        columns: &["asset_id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_ip_assets_asset_id ON surface_ip_assets(asset_id)",
    },
    SqliteIndex {
        table: "surface_host_assets",
        columns: &["asset_id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_host_assets_asset_id ON surface_host_assets(asset_id)",
    },
    SqliteIndex {
        table: "surface_port_assets",
        columns: &["asset_id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_port_assets_asset_id ON surface_port_assets(asset_id)",
    },
    SqliteIndex {
        table: "surface_service_assets",
        columns: &["asset_id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_service_assets_asset_id ON surface_service_assets(asset_id)",
    },
    SqliteIndex {
        table: "surface_web_assets",
        columns: &["asset_id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_web_assets_asset_id ON surface_web_assets(asset_id)",
    },
    SqliteIndex {
        table: "surface_cert_assets",
        columns: &["asset_id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_cert_assets_asset_id ON surface_cert_assets(asset_id)",
    },
    SqliteIndex {
        table: "surface_assets",
        columns: &["program_id", "status", "risk_score", "last_seen_at", "id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_assets_program_status_risk_seen ON surface_assets(program_id, status, risk_score DESC, last_seen_at DESC, id)",
    },
    SqliteIndex {
        table: "surface_service_assets",
        columns: &["application_service_name", "protocol_name", "asset_id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_service_assets_service_asset ON surface_service_assets(application_service_name, protocol_name, asset_id)",
    },
    SqliteIndex {
        table: "surface_web_assets",
        columns: &["http_status_code", "asset_id"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_web_assets_status_asset ON surface_web_assets(http_status_code, asset_id)",
    },
    SqliteIndex {
        table: "surface_seeds",
        columns: &["program_id", "seed_type"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_seeds_program_type ON surface_seeds(program_id, seed_type)",
    },
    SqliteIndex {
        table: "surface_seed_candidates",
        columns: &["program_id", "source_kind", "status", "observed_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_surface_seed_candidates_program_source_status_seen ON surface_seed_candidates(program_id, source_kind, status, observed_at DESC)",
    },
    SqliteIndex {
        table: "bot_messages",
        columns: &["linked_conversation_id", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_bot_messages_conversation_created ON bot_messages(linked_conversation_id, created_at ASC)",
    },
    SqliteIndex {
        table: "bot_execution_runs",
        columns: &["linked_conversation_id", "started_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_bot_execution_runs_conversation_started ON bot_execution_runs(linked_conversation_id, started_at DESC)",
    },
    SqliteIndex {
        table: "missions",
        columns: &["owner_kind", "owner_ref", "status", "next_run_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_missions_owner_status_next_run ON missions(owner_kind, owner_ref, status, next_run_at)",
    },
    SqliteIndex {
        table: "mission_runs",
        columns: &["mission_id", "status", "created_at"],
        sql: "CREATE INDEX IF NOT EXISTS idx_mission_runs_mission_status_created ON mission_runs(mission_id, status, created_at DESC)",
    },
];
