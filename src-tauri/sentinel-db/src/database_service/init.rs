use crate::database_service::migrations::{
    AgentTeamMigration, ExecutionTasksMigration, FloatTypeMigration, IntegerTypeMigration,
    SubagentMessagesMigration, SubagentRunsMigration, TaskToolIntegrationMigration,
    TimestampTypeMigration,
};
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::PgPool;
use anyhow::Result;
use chrono::Utc;
use tracing::info;

impl DatabaseService {
    pub async fn create_database_schema(&self, pool: &PgPool) -> Result<()> {
        info!("Creating database schema...");

        // 核心配置表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS configurations (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT,
                description TEXT,
                is_encrypted BOOLEAN DEFAULT FALSE,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(category, key)
            )"#,
        )
        .execute(pool)
        .await?;

        // LLM 测试套件表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS llm_test_suites (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL DEFAULT '1.0.0',
                description TEXT DEFAULT '',
                cases TEXT NOT NULL DEFAULT '[]',
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        // AI 对话和消息表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS ai_conversations (
                id TEXT PRIMARY KEY,
                title TEXT,
                service_name TEXT NOT NULL,
                model_name TEXT NOT NULL,
                model_provider TEXT,
                context_type TEXT,
                project_id TEXT,
                vulnerability_id TEXT,
                scan_task_id TEXT,
                conversation_data TEXT,
                summary TEXT,
                total_messages INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                cost DOUBLE PRECISION DEFAULT 0.0,
                tags TEXT,
                tool_config TEXT,
                is_archived BOOLEAN DEFAULT FALSE,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS ai_messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                token_count INTEGER,
                cost DOUBLE PRECISION,
                tool_calls TEXT,
                attachments TEXT,
                reasoning_content TEXT,
                timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
                architecture_type TEXT,
                architecture_meta TEXT,
                structured_data TEXT,
                FOREIGN KEY(conversation_id) REFERENCES ai_conversations(id)
            )"#,
        )
        .execute(pool)
        .await?;

        // 扫描任务和漏洞表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS scan_tasks (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                name TEXT NOT NULL,
                description TEXT,
                target_type TEXT NOT NULL,
                targets TEXT NOT NULL,
                scan_type TEXT NOT NULL,
                tools_config TEXT,
                status TEXT NOT NULL,
                progress DOUBLE PRECISION DEFAULT 0.0,
                priority INTEGER DEFAULT 1,
                scheduled_at TIMESTAMP WITH TIME ZONE,
                started_at TIMESTAMP WITH TIME ZONE,
                completed_at TIMESTAMP WITH TIME ZONE,
                execution_time INTEGER,
                results_summary TEXT,
                error_message TEXT,
                created_by TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        // 扫描会话和阶段表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS scan_sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                target TEXT NOT NULL,
                scan_type TEXT NOT NULL,
                status TEXT NOT NULL,
                config TEXT NOT NULL,
                progress DOUBLE PRECISION DEFAULT 0.0,
                current_stage TEXT NOT NULL,
                total_stages INTEGER DEFAULT 0,
                completed_stages INTEGER DEFAULT 0,
                results_summary TEXT,
                error_message TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                started_at TIMESTAMP WITH TIME ZONE,
                completed_at TIMESTAMP WITH TIME ZONE,
                created_by TEXT
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS scan_stages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                stage_name TEXT NOT NULL,
                stage_order INTEGER NOT NULL,
                status TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                config TEXT NOT NULL,
                results TEXT,
                error_message TEXT,
                started_at TIMESTAMP WITH TIME ZONE,
                completed_at TIMESTAMP WITH TIME ZONE,
                duration_ms INTEGER,
                FOREIGN KEY(session_id) REFERENCES scan_sessions(id)
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS vulnerabilities (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                asset_id TEXT,
                scan_task_id TEXT,
                title TEXT NOT NULL,
                description TEXT,
                vulnerability_type TEXT,
                severity TEXT NOT NULL,
                cvss_score DOUBLE PRECISION,
                cvss_vector TEXT,
                cwe_id TEXT,
                owasp_category TEXT,
                proof_of_concept TEXT,
                impact TEXT,
                remediation TEXT,
                references_json TEXT,
                status TEXT NOT NULL,
                verification_status TEXT NOT NULL,
                resolution_date TIMESTAMP WITH TIME ZONE,
                tags TEXT,
                attachments TEXT,
                notes TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        // Workflow 任务和定义表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS workflow_runs (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                workflow_name TEXT NOT NULL,
                version TEXT NOT NULL,
                status TEXT NOT NULL,
                progress INTEGER DEFAULT 0,
                completed_steps INTEGER DEFAULT 0,
                total_steps INTEGER DEFAULT 0,
                started_at TIMESTAMP WITH TIME ZONE NOT NULL,
                completed_at TIMESTAMP WITH TIME ZONE,
                error_message TEXT,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS workflow_run_steps (
                run_id TEXT NOT NULL,
                step_id TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TIMESTAMP WITH TIME ZONE NOT NULL,
                completed_at TIMESTAMP WITH TIME ZONE,
                result_json TEXT,
                error_message TEXT,
                PRIMARY KEY(run_id, step_id),
                FOREIGN KEY(run_id) REFERENCES workflow_runs(id)
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS workflow_definitions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                graph_data TEXT NOT NULL,
                is_template BOOLEAN DEFAULT FALSE,
                is_tool BOOLEAN DEFAULT FALSE,
                category TEXT,
                tags TEXT,
                version TEXT NOT NULL,
                created_by TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        // 工具执行日志表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS tool_executions (
                id TEXT PRIMARY KEY,
                tool_id TEXT NOT NULL,
                scan_task_id TEXT,
                command TEXT,
                arguments TEXT,
                status TEXT NOT NULL,
                progress DOUBLE PRECISION DEFAULT 0.0,
                start_time TIMESTAMP WITH TIME ZONE,
                end_time TIMESTAMP WITH TIME ZONE,
                execution_time INTEGER,
                output TEXT,
                error_output TEXT,
                exit_code INTEGER,
                resource_usage TEXT,
                artifacts TEXT,
                metadata TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        // Agent 任务和会话表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_tasks (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                description TEXT NOT NULL,
                target TEXT,
                parameters TEXT,
                priority TEXT NOT NULL,
                timeout INTEGER,
                status TEXT DEFAULT 'pending',
                started_at TIMESTAMP WITH TIME ZONE,
                completed_at TIMESTAMP WITH TIME ZONE,
                execution_time_ms INTEGER,
                error_message TEXT,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_sessions (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(task_id) REFERENCES agent_tasks(id)
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_session_logs (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                level TEXT NOT NULL,
                message TEXT NOT NULL,
                source TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(session_id) REFERENCES agent_sessions(id)
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_execution_results (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                success BOOLEAN NOT NULL,
                data TEXT,
                error TEXT,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(session_id) REFERENCES agent_sessions(id)
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_execution_steps (
                step_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                step_name TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TIMESTAMP WITH TIME ZONE,
                completed_at TIMESTAMP WITH TIME ZONE,
                duration_ms INTEGER,
                result_data TEXT,
                error_message TEXT,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(session_id) REFERENCES agent_sessions(id)
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_run_states (
                execution_id TEXT PRIMARY KEY,
                state_json TEXT NOT NULL,
                updated_at BIGINT NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        // Memory executions (agent memory persistence)
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS memory_executions (
                id TEXT PRIMARY KEY,
                task TEXT NOT NULL,
                environment TEXT,
                tool_calls TEXT,
                success BOOLEAN NOT NULL,
                error TEXT,
                response_excerpt TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_memory_executions_created_at
               ON memory_executions(created_at)"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS memory_records (
                id TEXT PRIMARY KEY,
                title TEXT,
                text TEXT NOT NULL,
                kind TEXT NOT NULL,
                tier TEXT NOT NULL,
                scope TEXT NOT NULL,
                stability TEXT NOT NULL,
                source TEXT NOT NULL,
                confidence DOUBLE PRECISION NOT NULL DEFAULT 0.7,
                importance INTEGER NOT NULL DEFAULT 3,
                tags_json TEXT NOT NULL DEFAULT '[]',
                origin_execution_id TEXT,
                supersedes_memory_id TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                created_at_ms BIGINT NOT NULL,
                updated_at_ms BIGINT NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS memory_projection_state (
                memory_id TEXT PRIMARY KEY,
                lexical_indexed BOOLEAN NOT NULL DEFAULT FALSE,
                vector_indexed BOOLEAN NOT NULL DEFAULT FALSE,
                skill_projected BOOLEAN NOT NULL DEFAULT FALSE,
                last_error TEXT,
                updated_at_ms BIGINT NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_memory_records_status_updated
               ON memory_records(status, updated_at_ms DESC)"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_memory_records_kind_updated
               ON memory_records(kind, updated_at_ms DESC)"#,
        )
        .execute(pool)
        .await?;

        // 插件和收藏表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS plugin_registry (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL DEFAULT '',
                version TEXT NOT NULL DEFAULT '1.0.0',
                author TEXT,
                main_category TEXT NOT NULL DEFAULT 'traffic',
                category TEXT NOT NULL DEFAULT 'vulnerability',
                description TEXT,
                default_severity TEXT NOT NULL DEFAULT 'medium',
                tags TEXT,
                enabled BOOLEAN NOT NULL DEFAULT FALSE,
                metadata TEXT NOT NULL DEFAULT '{}',
                code TEXT NOT NULL DEFAULT '',
                plugin_code TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                quality_score DOUBLE PRECISION,
                validation_status TEXT,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS plugin_favorites (
                plugin_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(plugin_id, user_id),
                FOREIGN KEY(plugin_id) REFERENCES plugin_registry(id)
            )"#,
        )
        .execute(pool)
        .await?;

        // 通知和 MCP 配置
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS notification_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                channel TEXT NOT NULL,
                config TEXT,
                is_encrypted BOOLEAN DEFAULT FALSE,
                enabled BOOLEAN DEFAULT TRUE,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS mcp_server_configs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                url TEXT NOT NULL,
                connection_type TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT NOT NULL,
                headers_json TEXT,
                is_enabled BOOLEAN DEFAULT TRUE,
                auto_connect BOOLEAN DEFAULT FALSE,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;
        let _ = sqlx::query("ALTER TABLE mcp_server_configs ADD COLUMN headers_json TEXT")
            .execute(pool)
            .await;

        // AI 角色和 Prompt 模板表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS ai_roles (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                prompt TEXT NOT NULL,
                capabilities_json TEXT NOT NULL DEFAULT '[]',
                is_system BOOLEAN DEFAULT FALSE,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;
        let _ = sqlx::query(
            "ALTER TABLE ai_roles ADD COLUMN capabilities_json TEXT NOT NULL DEFAULT '[]'",
        )
        .execute(pool)
        .await;

        // Fix for migration issues where created_at/updated_at might be TEXT in PostgreSQL
        let _ = sqlx::query("ALTER TABLE ai_roles ALTER COLUMN created_at TYPE TIMESTAMP WITH TIME ZONE USING created_at::TIMESTAMP WITH TIME ZONE").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE ai_roles ALTER COLUMN updated_at TYPE TIMESTAMP WITH TIME ZONE USING updated_at::TIMESTAMP WITH TIME ZONE").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE ai_conversations ALTER COLUMN created_at TYPE TIMESTAMP WITH TIME ZONE USING created_at::TIMESTAMP WITH TIME ZONE").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE ai_conversations ALTER COLUMN updated_at TYPE TIMESTAMP WITH TIME ZONE USING updated_at::TIMESTAMP WITH TIME ZONE").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE ai_messages ALTER COLUMN timestamp TYPE TIMESTAMP WITH TIME ZONE USING timestamp::TIMESTAMP WITH TIME ZONE").execute(pool).await;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS prompt_templates (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                content TEXT NOT NULL,
                is_default BOOLEAN DEFAULT FALSE,
                is_active BOOLEAN DEFAULT TRUE,
                category TEXT,
                template_type TEXT,
                is_system BOOLEAN DEFAULT FALSE,
                priority INTEGER DEFAULT 50,
                tags TEXT,
                variables TEXT,
                version TEXT DEFAULT '1.0.0',
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        let _ = sqlx::query("ALTER TABLE prompt_templates ALTER COLUMN created_at TYPE TIMESTAMP WITH TIME ZONE USING created_at::TIMESTAMP WITH TIME ZONE").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE prompt_templates ALTER COLUMN updated_at TYPE TIMESTAMP WITH TIME ZONE USING updated_at::TIMESTAMP WITH TIME ZONE").execute(pool).await;

        // AI 用量统计表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS ai_usage_stats (
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                cost DOUBLE PRECISION DEFAULT 0.0,
                last_used TIMESTAMP WITH TIME ZONE,
                PRIMARY KEY(provider, model)
            )"#,
        )
        .execute(pool)
        .await?;

        // Fix for migration issues where last_used might be TEXT in PostgreSQL
        let _ = sqlx::query("ALTER TABLE ai_usage_stats ALTER COLUMN last_used TYPE TIMESTAMP WITH TIME ZONE USING last_used::TIMESTAMP WITH TIME ZONE")
            .execute(pool)
            .await;

        // Plan-and-Execute 架构相关表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS execution_plans (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                estimated_duration INTEGER DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                metadata TEXT NOT NULL DEFAULT '{}'
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS execution_sessions (
                id TEXT PRIMARY KEY,
                plan_id TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TIMESTAMP WITH TIME ZONE NOT NULL,
                completed_at TIMESTAMP WITH TIME ZONE,
                current_step INTEGER,
                progress DOUBLE PRECISION DEFAULT 0.0,
                context TEXT NOT NULL DEFAULT '{}',
                metadata TEXT NOT NULL DEFAULT '{}',
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(plan_id) REFERENCES execution_plans(id)
            )"#,
        )
        .execute(pool)
        .await?;

        // 资产和关系表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS assets (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                asset_type TEXT NOT NULL,
                name TEXT NOT NULL,
                value TEXT NOT NULL,
                description TEXT,
                confidence DOUBLE PRECISION DEFAULT 1.0,
                status TEXT NOT NULL,
                source TEXT,
                source_scan_id TEXT,
                metadata TEXT,
                tags TEXT,
                risk_level TEXT,
                last_seen TIMESTAMP WITH TIME ZONE NOT NULL,
                first_seen TIMESTAMP WITH TIME ZONE NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
                created_by TEXT NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS asset_relationships (
                id TEXT PRIMARY KEY,
                source_asset_id TEXT NOT NULL,
                target_asset_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                description TEXT,
                confidence DOUBLE PRECISION DEFAULT 1.0,
                metadata TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                created_by TEXT NOT NULL,
                FOREIGN KEY(source_asset_id) REFERENCES assets(id),
                FOREIGN KEY(target_asset_id) REFERENCES assets(id)
            )"#,
        )
        .execute(pool)
        .await?;

        // Drop legacy ability_groups table if exists
        sqlx::query("DROP TABLE IF EXISTS ability_groups")
            .execute(pool)
            .await?;

        // Skills 表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT NOT NULL DEFAULT '',
                source_path TEXT NOT NULL DEFAULT '',
                content TEXT NOT NULL DEFAULT '',
                argument_hint TEXT NOT NULL DEFAULT '',
                disable_model_invocation BOOLEAN NOT NULL DEFAULT FALSE,
                user_invocable BOOLEAN NOT NULL DEFAULT TRUE,
                allowed_tools TEXT NOT NULL DEFAULT '[]',
                model TEXT NOT NULL DEFAULT '',
                context TEXT NOT NULL DEFAULT '',
                agent TEXT NOT NULL DEFAULT '',
                hooks TEXT NOT NULL DEFAULT '{}',
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        // Proxifier 代理和规则表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS proxifier_proxies (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER NOT NULL,
                proxy_type TEXT NOT NULL,
                username TEXT,
                password TEXT,
                enabled BOOLEAN DEFAULT TRUE,
                sort_order INTEGER DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS proxifier_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                enabled BOOLEAN DEFAULT TRUE,
                applications TEXT,
                target_hosts TEXT,
                target_ports TEXT,
                action TEXT NOT NULL,
                proxy_id TEXT,
                sort_order INTEGER DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(proxy_id) REFERENCES proxifier_proxies(id)
            )"#,
        )
        .execute(pool)
        .await?;

        // Dictionary tables
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS dictionaries (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                dict_type TEXT NOT NULL,
                service_type TEXT,
                category TEXT,
                is_builtin BOOLEAN DEFAULT FALSE,
                is_active BOOLEAN DEFAULT TRUE,
                word_count BIGINT DEFAULT 0,
                file_size BIGINT DEFAULT 0,
                checksum TEXT,
                version TEXT DEFAULT '1.0.0',
                author TEXT,
                source_url TEXT,
                tags TEXT,
                metadata TEXT,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS dictionary_words (
                id TEXT PRIMARY KEY,
                dictionary_id TEXT NOT NULL,
                word TEXT NOT NULL,
                weight DOUBLE PRECISION DEFAULT 1.0,
                category TEXT,
                metadata TEXT,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(dictionary_id) REFERENCES dictionaries(id) ON DELETE CASCADE
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_dictionary_words_dict_id 
               ON dictionary_words(dictionary_id)"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_dictionaries_created_at
               ON dictionaries(created_at DESC)"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_dictionaries_dict_type_created_at
               ON dictionaries(dict_type, created_at DESC)"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_dictionaries_service_type_created_at
               ON dictionaries(service_type, created_at DESC)"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_dictionary_words_word 
               ON dictionary_words(word)"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS dictionary_sets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                service_type TEXT,
                scenario TEXT,
                is_active BOOLEAN DEFAULT TRUE,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS dictionary_set_relations (
                id TEXT PRIMARY KEY,
                set_id TEXT NOT NULL,
                dictionary_id TEXT NOT NULL,
                priority INTEGER DEFAULT 0,
                is_enabled BOOLEAN DEFAULT TRUE,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(set_id) REFERENCES dictionary_sets(id) ON DELETE CASCADE,
                FOREIGN KEY(dictionary_id) REFERENCES dictionaries(id) ON DELETE CASCADE
            )"#,
        )
        .execute(pool)
        .await?;

        // Cache storage table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS cache_storage (
                cache_key TEXT PRIMARY KEY,
                cache_value TEXT NOT NULL,
                cache_type TEXT NOT NULL,
                version TEXT DEFAULT '1.0',
                expires_at TIMESTAMP WITH TIME ZONE,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_cache_storage_type 
               ON cache_storage(cache_type)"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_cache_storage_expires 
               ON cache_storage(expires_at)"#,
        )
        .execute(pool)
        .await?;

        // RAG 相关表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS rag_collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                is_active BOOLEAN DEFAULT FALSE,
                document_count BIGINT DEFAULT 0,
                chunk_count BIGINT DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS rag_document_sources (
                id TEXT PRIMARY KEY,
                collection_id TEXT NOT NULL,
                file_path TEXT,
                file_name TEXT NOT NULL,
                file_type TEXT,
                file_size BIGINT,
                file_hash TEXT,
                content_hash TEXT,
                status TEXT DEFAULT 'Pending',
                chunk_count BIGINT DEFAULT 0,
                metadata TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
                FOREIGN KEY(collection_id) REFERENCES rag_collections(id) ON DELETE CASCADE
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS rag_chunks (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                collection_id TEXT NOT NULL,
                content TEXT NOT NULL,
                content_hash TEXT,
                chunk_index INTEGER,
                char_count INTEGER,
                embedding BYTEA,
                metadata TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
                FOREIGN KEY(document_id) REFERENCES rag_document_sources(id) ON DELETE CASCADE,
                FOREIGN KEY(collection_id) REFERENCES rag_collections(id) ON DELETE CASCADE
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS rag_queries (
                id TEXT PRIMARY KEY,
                collection_id TEXT,
                conversation_id TEXT,
                query TEXT NOT NULL,
                response TEXT NOT NULL,
                processing_time_ms INTEGER,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                FOREIGN KEY(collection_id) REFERENCES rag_collections(id) ON DELETE CASCADE
            )"#,
        )
        .execute(pool)
        .await?;

        // Vulnerability table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS traffic_vulnerabilities (
                id TEXT PRIMARY KEY,
                plugin_id TEXT NOT NULL,
                vuln_type TEXT NOT NULL,
                severity TEXT NOT NULL,
                confidence TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                cwe TEXT,
                owasp TEXT,
                remediation TEXT,
                status TEXT NOT NULL DEFAULT 'open',
                signature TEXT NOT NULL,
                first_seen_at TIMESTAMP WITH TIME ZONE NOT NULL,
                last_seen_at TIMESTAMP WITH TIME ZONE NOT NULL,
                hit_count INTEGER NOT NULL DEFAULT 1,
                session_id TEXT,
                viewed_at TIMESTAMP WITH TIME ZONE,
                viewed_by TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        // Evidence table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS traffic_evidence (
                id TEXT PRIMARY KEY,
                vuln_id TEXT NOT NULL,
                url TEXT NOT NULL,
                method TEXT NOT NULL,
                location TEXT,
                evidence_snippet TEXT,
                request_headers TEXT,
                request_body TEXT,
                response_status INTEGER,
                response_headers TEXT,
                response_body TEXT,
                timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (vuln_id) REFERENCES traffic_vulnerabilities(id) ON DELETE CASCADE
            )"#,
        )
        .execute(pool)
        .await?;

        // Deduplication index table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS traffic_dedupe_index (
                signature TEXT PRIMARY KEY,
                vuln_id TEXT NOT NULL,
                first_hit TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_hit TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (vuln_id) REFERENCES traffic_vulnerabilities(id) ON DELETE CASCADE
            )"#,
        )
        .execute(pool)
        .await?;

        // Proxy request history table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS proxy_requests (
                id SERIAL PRIMARY KEY,
                origin_kind TEXT,
                origin_ref_id TEXT,
                parent_request_id BIGINT,
                source_draft_revision_id TEXT,
                url TEXT NOT NULL,
                host TEXT NOT NULL,
                scheme TEXT NOT NULL,
                http_version_observed TEXT,
                method TEXT NOT NULL,
                status_code INTEGER NOT NULL,
                request_headers TEXT,
                request_body TEXT,
                response_headers TEXT,
                response_body TEXT,
                response_size INTEGER NOT NULL DEFAULT 0,
                response_time INTEGER NOT NULL DEFAULT 0,
                timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                request_body_compressed BOOLEAN NOT NULL DEFAULT FALSE,
                response_body_compressed BOOLEAN NOT NULL DEFAULT FALSE
            )"#,
        )
        .execute(pool)
        .await?;

        // 添加压缩标记列（如果表已存在）
        let _ = sqlx::query(
            "ALTER TABLE proxy_requests ADD COLUMN request_body_compressed BOOLEAN NOT NULL DEFAULT FALSE"
        ).execute(pool).await;

        let _ = sqlx::query(
            "ALTER TABLE proxy_requests ADD COLUMN response_body_compressed BOOLEAN NOT NULL DEFAULT FALSE"
        ).execute(pool).await;

        let _ = sqlx::query(
            "ALTER TABLE proxy_requests ADD COLUMN scheme TEXT NOT NULL DEFAULT 'http'",
        )
        .execute(pool)
        .await;

        let _ = sqlx::query("ALTER TABLE proxy_requests ADD COLUMN http_version_observed TEXT")
            .execute(pool)
            .await;
        let _ = sqlx::query("ALTER TABLE proxy_requests ADD COLUMN origin_kind TEXT")
            .execute(pool)
            .await;
        let _ = sqlx::query("ALTER TABLE proxy_requests ADD COLUMN origin_ref_id TEXT")
            .execute(pool)
            .await;
        let _ = sqlx::query("ALTER TABLE proxy_requests ADD COLUMN parent_request_id BIGINT")
            .execute(pool)
            .await;
        let _ = sqlx::query("ALTER TABLE proxy_requests ADD COLUMN source_draft_revision_id TEXT")
            .execute(pool)
            .await;

        let _ = sqlx::query(
            "UPDATE proxy_requests SET scheme = LOWER(protocol) WHERE (scheme IS NULL OR scheme = '') AND protocol IS NOT NULL"
        ).execute(pool).await;

        // 创建索引以优化查询性能
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_proxy_requests_host ON proxy_requests(host)")
            .execute(pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_timestamp ON proxy_requests(timestamp DESC)"
        ).execute(pool).await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_scheme ON proxy_requests(scheme)",
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_status ON proxy_requests(status_code)",
        )
        .execute(pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_parent_request_id ON proxy_requests(parent_request_id)",
        )
        .execute(pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_source_draft_revision_id ON proxy_requests(source_draft_revision_id)",
        )
        .execute(pool)
        .await?;

        // Plugin registry table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS plugin_registry (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                author TEXT,
                main_category TEXT NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                default_severity TEXT NOT NULL,
                tags TEXT,
                enabled BOOLEAN NOT NULL DEFAULT FALSE,
                plugin_code TEXT,
                quality_score DOUBLE PRECISION,
                validation_status TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        // Proxy config table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS proxy_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        // Repeater tabs table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS repeater_tabs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                target_host TEXT NOT NULL,
                target_port INTEGER NOT NULL,
                use_tls BOOLEAN NOT NULL DEFAULT TRUE,
                override_sni BOOLEAN NOT NULL DEFAULT FALSE,
                sni_host TEXT,
                raw_request TEXT NOT NULL,
                request_tab TEXT NOT NULL DEFAULT 'pretty',
                response_tab TEXT NOT NULL DEFAULT 'pretty',
                sort_order INTEGER DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;

        // Create traffic-related indices
        let traffic_indices = vec![
            "CREATE INDEX IF NOT EXISTS idx_traffic_vulns_plugin ON traffic_vulnerabilities(plugin_id)",
            "CREATE INDEX IF NOT EXISTS idx_traffic_vulns_severity ON traffic_vulnerabilities(severity)",
            "CREATE INDEX IF NOT EXISTS idx_traffic_vulns_status ON traffic_vulnerabilities(status)",
            "CREATE INDEX IF NOT EXISTS idx_traffic_vulns_created ON traffic_vulnerabilities(created_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_traffic_vulns_viewed ON traffic_vulnerabilities(viewed_at)",
            "CREATE INDEX IF NOT EXISTS idx_traffic_evidence_vuln ON traffic_evidence(vuln_id)",
            "CREATE INDEX IF NOT EXISTS idx_traffic_evidence_timestamp ON traffic_evidence(timestamp DESC)",
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_timestamp ON proxy_requests(timestamp DESC)",
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_host ON proxy_requests(host)",
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_method ON proxy_requests(method)",
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_status ON proxy_requests(status_code)",
            "CREATE INDEX IF NOT EXISTS idx_plugin_registry_category ON plugin_registry(main_category, category)",
            "CREATE INDEX IF NOT EXISTS idx_plugin_registry_enabled ON plugin_registry(enabled)",
            "CREATE INDEX IF NOT EXISTS idx_plugin_registry_created ON plugin_registry(created_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_repeater_tabs_sort_order ON repeater_tabs(sort_order)",
            "CREATE INDEX IF NOT EXISTS idx_repeater_tabs_updated ON repeater_tabs(updated_at DESC)",
        ];

        for index_sql in traffic_indices {
            sqlx::query(index_sql).execute(pool).await?;
        }

        self.create_bot_schema(pool).await?;

        info!("Database schema creation completed");

        // Run migrations
        TaskToolIntegrationMigration::apply(pool).await?;
        SubagentRunsMigration::apply(pool).await?;
        SubagentMessagesMigration::apply(pool).await?;
        ExecutionTasksMigration::apply(pool).await?;

        // Run float type migration (REAL -> DOUBLE PRECISION)
        FloatTypeMigration::apply(pool).await?;

        // Run timestamp type migration (TEXT -> TIMESTAMP WITH TIME ZONE)
        TimestampTypeMigration::apply(pool).await?;

        // Run integer type migration (INTEGER -> BIGINT)
        IntegerTypeMigration::apply(pool).await?;

        Ok(())
    }

    pub async fn insert_default_data(&self, pool: &PgPool) -> Result<()> {
        info!("Inserting default data...");

        // 初始化默认AI角色
        self.initialize_default_roles(pool).await?;

        // 初始化预置 OWASP LLM Top 10 (2025) 测试套件
        self.initialize_default_llm_suites(pool).await?;

        info!("Default data insertion completed");
        Ok(())
    }

    pub async fn initialize_default_llm_suites(&self, pool: &PgPool) -> Result<()> {
        let suites_json = Self::default_llm_suites_json();
        let suites: Vec<serde_json::Value> = serde_json::from_str(&suites_json)?;

        for suite in &suites {
            let id = suite["id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("default LLM suite id missing"))?;
            let name = suite["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("default LLM suite name missing"))?;
            let version = suite["version"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("default LLM suite version missing"))?;
            let description = suite["description"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("default LLM suite description missing"))?;
            let cases = serde_json::to_string(
                suite["cases"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("default LLM suite cases missing"))?,
            )?;

            sqlx::query(
                "INSERT INTO llm_test_suites (id, name, version, description, cases, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP) \
                 ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name, version = EXCLUDED.version, \
                 description = EXCLUDED.description, cases = EXCLUDED.cases, updated_at = CURRENT_TIMESTAMP",
            )
            .bind(id)
            .bind(name)
            .bind(version)
            .bind(description)
            .bind(&cases)
            .execute(pool)
            .await?;
        }

        info!("Default LLM test suites seeded (or already present)");
        Ok(())
    }

    pub async fn initialize_default_roles(&self, pool: &PgPool) -> Result<()> {
        let roles = vec![
            (
                "security-analyst",
                "安全分析师",
                "分析安全漏洞和威胁",
                "你是一个专业的安全分析师...",
                r#"[]"#,
            ),
            (
                "penetration-tester",
                "渗透测试专家",
                "模拟黑客攻击",
                "你是一个资深的渗透测试专家...",
                r#"[]"#,
            ),
        ];

        for (id, title, description, prompt, capabilities_json) in roles {
            let now = Utc::now();
            sqlx::query(
                "INSERT INTO ai_roles (id, title, description, prompt, capabilities_json, is_system, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, true, $6, $7) ON CONFLICT (id) DO NOTHING"
            )
            .bind(id)
            .bind(title)
            .bind(description)
            .bind(prompt)
            .bind(capabilities_json)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn apply_agent_team_migration(&self, pool: &PgPool) -> Result<()> {
        AgentTeamMigration::apply(pool).await
    }
}
