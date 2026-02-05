use anyhow::Result;
use sqlx::postgres::PgPool;
use tracing::info;
use chrono::Utc;
use crate::database_service::service::DatabaseService;
use crate::database_service::migrations::{
    AgentTodosMigration,
    SubagentMessagesMigration,
    SubagentRunsMigration,
    TaskToolIntegrationMigration,
    AsmEnhancementMigration,
    FloatTypeMigration,
    TimestampTypeMigration,
    IntegerTypeMigration,
};

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;
        

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_sessions (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(task_id) REFERENCES agent_tasks(id)
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_session_logs (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                level TEXT NOT NULL,
                message TEXT NOT NULL,
                source TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(session_id) REFERENCES agent_sessions(id)
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_execution_results (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                success BOOLEAN NOT NULL,
                data TEXT,
                error TEXT,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(session_id) REFERENCES agent_sessions(id)
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_run_states (
                execution_id TEXT PRIMARY KEY,
                state_json TEXT NOT NULL,
                updated_at BIGINT NOT NULL
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_memory_executions_created_at
               ON memory_executions(created_at)"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS plugin_favorites (
                plugin_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(plugin_id, user_id),
                FOREIGN KEY(plugin_id) REFERENCES plugin_registry(id)
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS mcp_server_configs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                url TEXT NOT NULL,
                connection_type TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT NOT NULL,
                is_enabled BOOLEAN DEFAULT TRUE,
                auto_connect BOOLEAN DEFAULT FALSE,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"#
        ).execute(pool).await?;

        // AI 角色和 Prompt 模板表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS ai_roles (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                prompt TEXT NOT NULL,
                is_system BOOLEAN DEFAULT FALSE,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

        // Drop legacy ability_groups table if exists
        sqlx::query("DROP TABLE IF EXISTS ability_groups").execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_dictionary_words_dict_id 
               ON dictionary_words(dictionary_id)"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_dictionary_words_word 
               ON dictionary_words(word)"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_cache_storage_type 
               ON cache_storage(cache_type)"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_cache_storage_expires 
               ON cache_storage(expires_at)"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

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
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

        // Deduplication index table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS traffic_dedupe_index (
                signature TEXT PRIMARY KEY,
                vuln_id TEXT NOT NULL,
                first_hit TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_hit TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (vuln_id) REFERENCES traffic_vulnerabilities(id) ON DELETE CASCADE
            )"#
        ).execute(pool).await?;

        // Proxy request history table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS proxy_requests (
                id SERIAL PRIMARY KEY,
                url TEXT NOT NULL,
                host TEXT NOT NULL,
                protocol TEXT NOT NULL,
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
            )"#
        ).execute(pool).await?;
        
        // 添加压缩标记列（如果表已存在）
        let _ = sqlx::query(
            "ALTER TABLE proxy_requests ADD COLUMN request_body_compressed BOOLEAN NOT NULL DEFAULT FALSE"
        ).execute(pool).await;
        
        let _ = sqlx::query(
            "ALTER TABLE proxy_requests ADD COLUMN response_body_compressed BOOLEAN NOT NULL DEFAULT FALSE"
        ).execute(pool).await;
        
        // 创建索引以优化查询性能
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_host ON proxy_requests(host)"
        ).execute(pool).await?;
        
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_timestamp ON proxy_requests(timestamp DESC)"
        ).execute(pool).await?;
        
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_protocol ON proxy_requests(protocol)"
        ).execute(pool).await?;
        
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_proxy_requests_status ON proxy_requests(status_code)"
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

        // Proxy config table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS proxy_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#
        ).execute(pool).await?;

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
            )"#
        ).execute(pool).await?;

        // Create traffic-related indices
        let traffic_indices = vec![
            "CREATE INDEX IF NOT EXISTS idx_traffic_vulns_plugin ON traffic_vulnerabilities(plugin_id)",
            "CREATE INDEX IF NOT EXISTS idx_traffic_vulns_severity ON traffic_vulnerabilities(severity)",
            "CREATE INDEX IF NOT EXISTS idx_traffic_vulns_status ON traffic_vulnerabilities(status)",
            "CREATE INDEX IF NOT EXISTS idx_traffic_vulns_created ON traffic_vulnerabilities(created_at DESC)",
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
            sqlx::query(index_sql)
                .execute(pool)
                .await?;
        }

        // Bug Bounty tables
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS bounty_programs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                organization TEXT NOT NULL,
                platform TEXT NOT NULL DEFAULT 'private',
                platform_handle TEXT,
                url TEXT,
                program_type TEXT NOT NULL DEFAULT 'public',
                status TEXT NOT NULL DEFAULT 'active',
                description TEXT,
                rewards_json TEXT,
                response_sla_days INTEGER,
                resolution_sla_days INTEGER,
                rules_json TEXT,
                tags_json TEXT,
                metadata_json TEXT,
                priority_score DOUBLE PRECISION DEFAULT 0.0,
                total_submissions INTEGER DEFAULT 0,
                accepted_submissions INTEGER DEFAULT 0,
                total_earnings DOUBLE PRECISION DEFAULT 0.0,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_activity_at TIMESTAMP WITH TIME ZONE
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS bounty_scopes (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                scope_type TEXT NOT NULL DEFAULT 'in_scope',
                target_type TEXT NOT NULL,
                target TEXT NOT NULL,
                description TEXT,
                allowed_tests_json TEXT,
                instructions_json TEXT,
                requires_auth BOOLEAN DEFAULT FALSE,
                test_accounts_json TEXT,
                asset_count INTEGER DEFAULT 0,
                finding_count INTEGER DEFAULT 0,
                priority DOUBLE PRECISION DEFAULT 0.0,
                metadata_json TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(program_id) REFERENCES bounty_programs(id) ON DELETE CASCADE
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS bounty_findings (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                scope_id TEXT,
                asset_id TEXT,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                finding_type TEXT NOT NULL,
                severity TEXT NOT NULL DEFAULT 'medium',
                status TEXT NOT NULL DEFAULT 'new',
                confidence TEXT NOT NULL DEFAULT 'medium',
                cvss_score DOUBLE PRECISION,
                cwe_id TEXT,
                affected_url TEXT,
                affected_parameter TEXT,
                reproduction_steps_json TEXT,
                impact TEXT,
                remediation TEXT,
                evidence_ids_json TEXT,
                tags_json TEXT,
                metadata_json TEXT,
                fingerprint TEXT NOT NULL,
                duplicate_of TEXT,
                first_seen_at TIMESTAMP WITH TIME ZONE NOT NULL,
                last_seen_at TIMESTAMP WITH TIME ZONE NOT NULL,
                verified_at TIMESTAMP WITH TIME ZONE,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                created_by TEXT NOT NULL,
                FOREIGN KEY(program_id) REFERENCES bounty_programs(id) ON DELETE CASCADE,
                FOREIGN KEY(scope_id) REFERENCES bounty_scopes(id) ON DELETE SET NULL
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS bounty_submissions (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                finding_id TEXT NOT NULL,
                platform_submission_id TEXT,
                title TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'draft',
                priority TEXT NOT NULL DEFAULT 'medium',
                vulnerability_type TEXT NOT NULL,
                severity TEXT NOT NULL DEFAULT 'medium',
                cvss_score DOUBLE PRECISION,
                cwe_id TEXT,
                description TEXT NOT NULL,
                reproduction_steps_json TEXT,
                impact TEXT NOT NULL,
                remediation TEXT,
                evidence_ids_json TEXT,
                platform_url TEXT,
                reward_amount DOUBLE PRECISION,
                reward_currency TEXT,
                bonus_amount DOUBLE PRECISION,
                response_time_hours INTEGER,
                resolution_time_hours INTEGER,
                requires_retest BOOLEAN DEFAULT FALSE,
                retest_at TEXT,
                last_retest_at TEXT,
                communications_json TEXT,
                timeline_json TEXT,
                tags_json TEXT,
                metadata_json TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                submitted_at TIMESTAMP WITH TIME ZONE,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                closed_at TIMESTAMP WITH TIME ZONE,
                created_by TEXT NOT NULL,
                FOREIGN KEY(program_id) REFERENCES bounty_programs(id) ON DELETE CASCADE,
                FOREIGN KEY(finding_id) REFERENCES bounty_findings(id) ON DELETE CASCADE
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS bounty_evidence (
                id TEXT PRIMARY KEY,
                finding_id TEXT NOT NULL,
                evidence_type TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                file_path TEXT,
                file_url TEXT,
                content TEXT,
                mime_type TEXT,
                file_size BIGINT,
                http_request_json TEXT,
                http_response_json TEXT,
                diff TEXT,
                tags_json TEXT,
                metadata_json TEXT,
                display_order INTEGER DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(finding_id) REFERENCES bounty_findings(id) ON DELETE CASCADE
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS bounty_change_events (
                id TEXT PRIMARY KEY,
                program_id TEXT,
                asset_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                severity TEXT NOT NULL DEFAULT 'medium',
                status TEXT NOT NULL DEFAULT 'new',
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                old_value TEXT,
                new_value TEXT,
                diff TEXT,
                affected_scope TEXT,
                detection_method TEXT NOT NULL,
                triggered_workflows_json TEXT,
                generated_findings_json TEXT,
                tags_json TEXT,
                metadata_json TEXT,
                risk_score DOUBLE PRECISION DEFAULT 0.0,
                auto_trigger_enabled BOOLEAN DEFAULT FALSE,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                resolved_at TIMESTAMP WITH TIME ZONE,
                FOREIGN KEY(program_id) REFERENCES bounty_programs(id) ON DELETE SET NULL
            )"#
        ).execute(pool).await?;

        // Bounty assets table (P1-B3: Asset consolidation)
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS bounty_assets (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                scope_id TEXT,
                asset_type TEXT NOT NULL DEFAULT 'url',
                canonical_url TEXT NOT NULL,
                original_urls_json TEXT,
                hostname TEXT,
                port INTEGER,
                path TEXT,
                protocol TEXT,
                ip_addresses_json TEXT,
                dns_records_json TEXT,
                tech_stack_json TEXT,
                fingerprint TEXT,
                tags_json TEXT,
                labels_json TEXT,
                priority_score DOUBLE PRECISION DEFAULT 0.0,
                risk_score DOUBLE PRECISION DEFAULT 0.0,
                is_alive BOOLEAN DEFAULT TRUE,
                last_checked_at TIMESTAMP WITH TIME ZONE,
                first_seen_at TIMESTAMP WITH TIME ZONE NOT NULL,
                last_seen_at TIMESTAMP WITH TIME ZONE NOT NULL,
                findings_count INTEGER DEFAULT 0,
                change_events_count INTEGER DEFAULT 0,
                metadata_json TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                
                -- P0: Core ASM Attributes
                -- IP Asset Attributes
                ip_version TEXT,
                asn INTEGER,
                asn_org TEXT,
                isp TEXT,
                country TEXT,
                city TEXT,
                latitude DOUBLE PRECISION,
                longitude DOUBLE PRECISION,
                is_cloud BOOLEAN,
                cloud_provider TEXT,
                
                -- Port/Service Attributes
                service_name TEXT,
                service_version TEXT,
                service_product TEXT,
                banner TEXT,
                transport_protocol TEXT,
                cpe TEXT,
                
                -- Domain Attributes
                domain_registrar TEXT,
                registration_date TEXT,
                expiration_date TEXT,
                nameservers_json TEXT,
                mx_records_json TEXT,
                txt_records_json TEXT,
                whois_data_json TEXT,
                is_wildcard BOOLEAN,
                parent_domain TEXT,
                
                -- Web/URL Attributes
                http_status INTEGER,
                response_time_ms INTEGER,
                content_length INTEGER,
                content_type TEXT,
                title TEXT,
                favicon_hash TEXT,
                headers_json TEXT,
                waf_detected TEXT,
                cdn_detected TEXT,
                screenshot_path TEXT,
                body_hash TEXT,
                
                -- Certificate Attributes
                certificate_id TEXT,
                ssl_enabled BOOLEAN,
                certificate_subject TEXT,
                certificate_issuer TEXT,
                certificate_valid_from TEXT,
                certificate_valid_to TEXT,
                certificate_san_json TEXT,
                
                -- Attack Surface & Risk
                exposure_level TEXT,
                attack_surface_score DOUBLE PRECISION,
                vulnerability_count INTEGER DEFAULT 0,
                cvss_max_score DOUBLE PRECISION,
                exploit_available BOOLEAN,
                
                -- Asset Classification
                asset_category TEXT,
                asset_owner TEXT,
                business_unit TEXT,
                criticality TEXT,
                
                -- Discovery & Monitoring
                discovery_method TEXT,
                data_sources_json TEXT,
                confidence_score DOUBLE PRECISION,
                monitoring_enabled BOOLEAN DEFAULT FALSE,
                scan_frequency TEXT,
                last_scan_type TEXT,
                
                -- Asset Relationships
                parent_asset_id TEXT,
                related_assets_json TEXT,
                
                FOREIGN KEY(program_id) REFERENCES bounty_programs(id) ON DELETE CASCADE,
                FOREIGN KEY(scope_id) REFERENCES bounty_scopes(id) ON DELETE SET NULL,
                FOREIGN KEY(parent_asset_id) REFERENCES bounty_assets(id) ON DELETE SET NULL
            )"#
        ).execute(pool).await?;

        // Bounty workflow templates (built-in for bug bounty)
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS bounty_workflow_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                category TEXT NOT NULL DEFAULT 'recon',
                workflow_definition_id TEXT,
                steps_json TEXT NOT NULL,
                input_schema_json TEXT,
                output_schema_json TEXT,
                tags_json TEXT,
                is_built_in BOOLEAN DEFAULT FALSE,
                estimated_duration_mins INTEGER,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#
        ).execute(pool).await?;

        // Bounty workflow bindings (link templates to programs/scopes)
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS bounty_workflow_bindings (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                scope_id TEXT,
                workflow_template_id TEXT NOT NULL,
                is_enabled BOOLEAN DEFAULT TRUE,
                auto_run_on_change BOOLEAN DEFAULT FALSE,
                trigger_conditions_json TEXT,
                schedule_cron TEXT,
                last_run_at TIMESTAMP WITH TIME ZONE,
                last_run_status TEXT,
                run_count INTEGER DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(program_id) REFERENCES bounty_programs(id) ON DELETE CASCADE,
                FOREIGN KEY(workflow_template_id) REFERENCES bounty_workflow_templates(id)
            )"#
        ).execute(pool).await?;

        // Bug Bounty indices
        let bounty_indices = vec![
            "CREATE INDEX IF NOT EXISTS idx_bounty_programs_platform ON bounty_programs(platform)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_programs_status ON bounty_programs(status)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_programs_priority ON bounty_programs(priority_score DESC)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_scopes_program ON bounty_scopes(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_scopes_type ON bounty_scopes(scope_type)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_findings_program ON bounty_findings(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_findings_status ON bounty_findings(status)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_findings_severity ON bounty_findings(severity)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_findings_fingerprint ON bounty_findings(fingerprint)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_submissions_program ON bounty_submissions(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_submissions_status ON bounty_submissions(status)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_submissions_finding ON bounty_submissions(finding_id)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_evidence_finding ON bounty_evidence(finding_id)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_change_events_program ON bounty_change_events(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_change_events_status ON bounty_change_events(status)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_change_events_severity ON bounty_change_events(severity)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_workflow_templates_category ON bounty_workflow_templates(category)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_workflow_bindings_program ON bounty_workflow_bindings(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_workflow_bindings_template ON bounty_workflow_bindings(workflow_template_id)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_program ON bounty_assets(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_scope ON bounty_assets(scope_id)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_hostname ON bounty_assets(hostname)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_canonical_url ON bounty_assets(canonical_url)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_fingerprint ON bounty_assets(fingerprint)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_priority ON bounty_assets(priority_score DESC)",
        ];

        for index_sql in bounty_indices {
            sqlx::query(index_sql).execute(pool).await?;
        }

        info!("Database schema creation completed");
        
        // Run migrations
        TaskToolIntegrationMigration::apply(pool).await?;
        SubagentRunsMigration::apply(pool).await?;
        SubagentMessagesMigration::apply(pool).await?;
        AgentTodosMigration::apply(pool).await?;
        
        // Run ASM enhancement migration
        AsmEnhancementMigration::apply(pool).await?;
        
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
        
        info!("Default data insertion completed");
        Ok(())
    }

    pub async fn initialize_default_roles(&self, pool: &PgPool) -> Result<()> {
        let roles = vec![
            ("security-analyst", "安全分析师", "分析安全漏洞和威胁", "你是一个专业的安全分析师..."),
            ("penetration-tester", "渗透测试专家", "模拟黑客攻击", "你是一个资深的渗透测试专家..."),
        ];

        for (id, title, description, prompt) in roles {
            let now = Utc::now();
            sqlx::query(
                "INSERT INTO ai_roles (id, title, description, prompt, is_system, created_at, updated_at) VALUES ($1, $2, $3, $4, true, $5, $6) ON CONFLICT (id) DO NOTHING"
            )
            .bind(id)
            .bind(title)
            .bind(description)
            .bind(prompt)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
        Ok(())
    }
}
