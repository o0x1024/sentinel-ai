use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use tracing::info;
use chrono::Utc;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    pub async fn create_database_schema(&self, pool: &SqlitePool) -> Result<()> {
        info!("Creating database schema...");
        
        // 核心配置表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS configurations (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT,
                description TEXT,
                is_encrypted BOOLEAN DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
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
                cost REAL DEFAULT 0.0,
                tags TEXT,
                tool_config TEXT,
                is_archived BOOLEAN DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
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
                cost REAL,
                tool_calls TEXT,
                attachments TEXT,
                timestamp DATETIME NOT NULL,
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
                progress REAL DEFAULT 0.0,
                priority INTEGER DEFAULT 1,
                scheduled_at DATETIME,
                started_at DATETIME,
                completed_at DATETIME,
                execution_time INTEGER,
                results_summary TEXT,
                error_message TEXT,
                created_by TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
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
                progress REAL DEFAULT 0.0,
                current_stage TEXT NOT NULL,
                total_stages INTEGER DEFAULT 0,
                completed_stages INTEGER DEFAULT 0,
                results_summary TEXT,
                error_message TEXT,
                created_at DATETIME NOT NULL,
                started_at DATETIME,
                completed_at DATETIME,
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
                started_at DATETIME,
                completed_at DATETIME,
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
                cvss_score REAL,
                cvss_vector TEXT,
                cwe_id TEXT,
                owasp_category TEXT,
                proof_of_concept TEXT,
                impact TEXT,
                remediation TEXT,
                references_json TEXT,
                status TEXT NOT NULL,
                verification_status TEXT NOT NULL,
                resolution_date DATETIME,
                tags TEXT,
                attachments TEXT,
                notes TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
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
                started_at DATETIME NOT NULL,
                completed_at DATETIME,
                error_message TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS workflow_run_steps (
                run_id TEXT NOT NULL,
                step_id TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at DATETIME NOT NULL,
                completed_at DATETIME,
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
                is_template BOOLEAN DEFAULT 0,
                is_tool BOOLEAN DEFAULT 0,
                category TEXT,
                tags TEXT,
                version TEXT NOT NULL,
                created_by TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
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
                progress REAL DEFAULT 0.0,
                start_time DATETIME,
                end_time DATETIME,
                execution_time INTEGER,
                output TEXT,
                error_output TEXT,
                exit_code INTEGER,
                resource_usage TEXT,
                artifacts TEXT,
                metadata TEXT,
                created_at DATETIME NOT NULL
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
                started_at DATETIME,
                completed_at DATETIME,
                execution_time_ms INTEGER,
                error_message TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_sessions (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
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
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
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
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(session_id) REFERENCES agent_sessions(id)
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_execution_steps (
                step_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                step_name TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at DATETIME,
                completed_at DATETIME,
                duration_ms INTEGER,
                result_data TEXT,
                error_message TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(session_id) REFERENCES agent_sessions(id)
            )"#
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
                enabled BOOLEAN NOT NULL DEFAULT 0,
                metadata TEXT NOT NULL DEFAULT '{}',
                code TEXT NOT NULL DEFAULT '',
                plugin_code TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                quality_score REAL,
                validation_status TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS plugin_favorites (
                plugin_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
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
                is_encrypted BOOLEAN DEFAULT 0,
                enabled BOOLEAN DEFAULT 1,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
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
                is_enabled BOOLEAN DEFAULT 1,
                auto_connect BOOLEAN DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#
        ).execute(pool).await?;

        // AI 角色和 Prompt 模板表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS ai_roles (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                prompt TEXT NOT NULL,
                is_system BOOLEAN DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS prompt_templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                content TEXT NOT NULL,
                is_default BOOLEAN DEFAULT 0,
                is_active BOOLEAN DEFAULT 1,
                category TEXT,
                template_type TEXT,
                is_system BOOLEAN DEFAULT 0,
                priority INTEGER DEFAULT 50,
                tags TEXT,
                variables TEXT,
                version TEXT DEFAULT '1.0.0',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#
        ).execute(pool).await?;

        // AI 用量统计表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS ai_usage_stats (
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                cost REAL DEFAULT 0.0,
                last_used DATETIME,
                PRIMARY KEY(provider, model)
            )"#
        ).execute(pool).await?;

        // Plan-and-Execute 架构相关表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS execution_plans (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                estimated_duration INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                metadata TEXT NOT NULL DEFAULT '{}'
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS execution_sessions (
                id TEXT PRIMARY KEY,
                plan_id TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at DATETIME NOT NULL,
                completed_at DATETIME,
                current_step INTEGER,
                progress REAL DEFAULT 0.0,
                context TEXT NOT NULL DEFAULT '{}',
                metadata TEXT NOT NULL DEFAULT '{}',
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
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
                confidence REAL DEFAULT 1.0,
                status TEXT NOT NULL,
                source TEXT,
                source_scan_id TEXT,
                metadata TEXT,
                tags TEXT,
                risk_level TEXT,
                last_seen DATETIME NOT NULL,
                first_seen DATETIME NOT NULL,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
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
                confidence REAL DEFAULT 1.0,
                metadata TEXT,
                created_at DATETIME NOT NULL,
                created_by TEXT NOT NULL,
                FOREIGN KEY(source_asset_id) REFERENCES assets(id),
                FOREIGN KEY(target_asset_id) REFERENCES assets(id)
            )"#
        ).execute(pool).await?;

        // 能力组表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS ability_groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT NOT NULL DEFAULT '',
                instructions TEXT NOT NULL DEFAULT '',
                tool_ids TEXT NOT NULL DEFAULT '[]',
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
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
                enabled BOOLEAN DEFAULT 1,
                sort_order INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS proxifier_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                enabled BOOLEAN DEFAULT 1,
                applications TEXT,
                target_hosts TEXT,
                target_ports TEXT,
                action TEXT NOT NULL,
                proxy_id TEXT,
                sort_order INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(proxy_id) REFERENCES proxifier_proxies(id)
            )"#
        ).execute(pool).await?;

        // RAG 相关表
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS rag_collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                is_active BOOLEAN DEFAULT 0,
                document_count INTEGER DEFAULT 0,
                chunk_count INTEGER DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS rag_document_sources (
                id TEXT PRIMARY KEY,
                collection_id TEXT NOT NULL,
                file_path TEXT,
                file_name TEXT NOT NULL,
                file_type TEXT,
                file_size INTEGER,
                file_hash TEXT,
                content_hash TEXT,
                metadata TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                FOREIGN KEY(collection_id) REFERENCES rag_collections(id)
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
                embedding BLOB,
                embedding_model TEXT,
                embedding_dimension INTEGER,
                metadata TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY(document_id) REFERENCES rag_document_sources(id),
                FOREIGN KEY(collection_id) REFERENCES rag_collections(id)
            )"#
        ).execute(pool).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS rag_queries (
                id TEXT PRIMARY KEY,
                collection_id TEXT,
                query TEXT NOT NULL,
                response TEXT NOT NULL,
                processing_time_ms INTEGER,
                created_at DATETIME NOT NULL,
                FOREIGN KEY(collection_id) REFERENCES rag_collections(id)
            )"#
        ).execute(pool).await?;

        info!("Database schema creation completed");
        Ok(())
    }

    pub async fn insert_default_data(&self, pool: &SqlitePool) -> Result<()> {
        info!("Inserting default data...");
        
        // 初始化默认AI角色
        self.initialize_default_roles(pool).await?;
        
        info!("Default data insertion completed");
        Ok(())
    }

    pub async fn initialize_default_roles(&self, pool: &SqlitePool) -> Result<()> {
        let roles = vec![
            ("security-analyst", "安全分析师", "分析安全漏洞和威胁", "你是一个专业的安全分析师..."),
            ("penetration-tester", "渗透测试专家", "模拟黑客攻击", "你是一个资深的渗透测试专家..."),
        ];

        for (id, title, description, prompt) in roles {
            let now = Utc::now();
            sqlx::query(
                "INSERT OR IGNORE INTO ai_roles (id, title, description, prompt, is_system, created_at, updated_at) VALUES (?, ?, ?, ?, 1, ?, ?)"
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
