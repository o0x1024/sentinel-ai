use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::{sqlite::SqlitePool, Column, Row, TypeInfo};
use std::fs;
use std::path::PathBuf;

use crate::models::ai::AiRole;
use crate::models::database::{
    AiConversation, AiMessage, BountyProject, Configuration, DatabaseStats, McpServerConfig,
    ScanTask, Submission, Vulnerability,
};

#[async_trait]
pub trait Database: Send + Sync + std::fmt::Debug {
    async fn create_ai_conversation(&self, conversation: &AiConversation) -> Result<()>;
    async fn get_ai_conversations(&self) -> Result<Vec<AiConversation>>;
    async fn get_ai_conversation(&self, id: &str) -> Result<Option<AiConversation>>;
    async fn update_ai_conversation(&self, conversation: &AiConversation) -> Result<()>;
    async fn delete_ai_conversation(&self, id: &str) -> Result<()>;
    async fn update_ai_conversation_title(&self, id: &str, title: &str) -> Result<()>;
    async fn archive_ai_conversation(&self, id: &str) -> Result<()>;
    async fn create_ai_message(&self, message: &AiMessage) -> Result<()>;
    async fn get_ai_messages_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<AiMessage>>;
    async fn get_configs_by_category(&self, category: &str) -> Result<Vec<Configuration>>;
    async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>>;
    async fn set_config(
        &self,
        category: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()>;
    async fn get_ai_roles(&self) -> Result<Vec<AiRole>>;
    async fn create_ai_role(&self, role: &AiRole) -> Result<()>;
    async fn update_ai_role(&self, role: &AiRole) -> Result<()>;
    async fn delete_ai_role(&self, role_id: &str) -> Result<()>;
}

#[derive(Debug)]
/// 数据库服务
pub struct DatabaseService {
    pool: Option<SqlitePool>,
    db_path: PathBuf,
}

impl DatabaseService {
    pub fn new() -> Self {
        let db_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai")
            .join("database.db");

        Self {
            pool: None,
            db_path,
        }
    }

    /// 初始化数据库连接和架构
    pub async fn initialize(&mut self) -> Result<()> {
        tracing::info!("Starting database initialization...");
        tracing::info!("Database path: {}", self.db_path.display());

        // 确保数据库目录存在
        if let Some(parent) = self.db_path.parent() {
            if !parent.exists() {
                tracing::info!("Creating database directory: {}", parent.display());
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow::anyhow!("创建数据库目录失败: {}", e))?;
            }
        }

        // 检查数据库文件是否存在
        let db_exists = self.db_path.exists();
        tracing::info!("Database file exists: {}", db_exists);

        // 如果数据库文件不存在，先创建一个空文件
        if !db_exists {
            tracing::info!("Creating database file: {}", self.db_path.display());
            if let Err(e) = std::fs::File::create(&self.db_path) {
                tracing::error!("Failed to create database file: {}", e);
                return Err(anyhow::anyhow!("创建数据库文件失败: {}", e));
            }
        }

        // 创建连接池，使用更安全的连接选项
        let database_url = format!("sqlite:{}?mode=rwc", self.db_path.display());
        tracing::debug!("Database connection string: {}", database_url);

        let pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&self.db_path)
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .synchronous(sqlx::sqlite::SqliteSynchronous::Normal),
        )
        .await
        .map_err(|e| anyhow::anyhow!("连接数据库失败: {}", e))?;

        // 启用外键约束
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .map_err(|e| anyhow::anyhow!("启用外键约束失败: {}", e))?;

        // 创建数据库表结构
        self.create_database_schema(&pool).await?;

        // 如果是新数据库，插入默认数据
        if !db_exists {
            tracing::info!("New database detected, inserting default data...");
            self.insert_default_data(&pool).await?;
        } else {
            tracing::info!("Existing database detected, skipping data insertion.");
        }

        self.pool = Some(pool);
        tracing::info!(
            "Database initialization completed: {}",
            self.db_path.display()
        );

        Ok(())
    }

    /// 创建数据库表结构
    async fn create_database_schema(&self, pool: &SqlitePool) -> Result<()> {
        tracing::info!("Creating database schema...");

        // 使用事务来确保所有表创建成功或全部回滚
        let mut tx = pool.begin().await?;

        // 创建赏金项目表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS bounty_projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                platform TEXT NOT NULL,
                url TEXT,
                scope_domains TEXT,
                scope_ips TEXT,
                out_of_scope TEXT,
                reward_range TEXT,
                difficulty_level INTEGER DEFAULT 1,
                priority INTEGER DEFAULT 1,
                status TEXT DEFAULT 'active',
                last_activity_at DATETIME,
                roi_score REAL DEFAULT 0.0,
                success_rate REAL DEFAULT 0.0,
                competition_level INTEGER DEFAULT 1,
                tags TEXT,
                notes TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建扫描任务表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS scan_tasks (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                name TEXT NOT NULL,
                description TEXT,
                target_type TEXT NOT NULL,
                targets TEXT NOT NULL,
                scan_type TEXT NOT NULL,
                tools_config TEXT,
                status TEXT DEFAULT 'pending',
                progress REAL DEFAULT 0.0,
                priority INTEGER DEFAULT 1,
                scheduled_at DATETIME,
                started_at DATETIME,
                completed_at DATETIME,
                execution_time INTEGER,
                results_summary TEXT,
                error_message TEXT,
                created_by TEXT DEFAULT 'user',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES bounty_projects(id) ON DELETE SET NULL
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建资产表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS assets (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                scan_task_id TEXT,
                asset_type TEXT NOT NULL,
                value TEXT NOT NULL,
                parent_id TEXT,
                metadata TEXT,
                status TEXT DEFAULT 'active',
                confidence_score REAL DEFAULT 1.0,
                risk_level TEXT DEFAULT 'info',
                tags TEXT,
                notes TEXT,
                first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES bounty_projects(id) ON DELETE CASCADE,
                FOREIGN KEY (scan_task_id) REFERENCES scan_tasks(id) ON DELETE SET NULL,
                FOREIGN KEY (parent_id) REFERENCES assets(id) ON DELETE SET NULL
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建漏洞表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS vulnerabilities (
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
                reference_links TEXT,
                status TEXT DEFAULT 'open',
                verification_status TEXT DEFAULT 'unverified',
                submission_status TEXT DEFAULT 'not_submitted',
                reward_amount REAL,
                submission_date DATETIME,
                resolution_date DATETIME,
                tags TEXT,
                attachments TEXT,
                notes TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES bounty_projects(id) ON DELETE CASCADE,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE SET NULL,
                FOREIGN KEY (scan_task_id) REFERENCES scan_tasks(id) ON DELETE SET NULL
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建提交记录表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS submissions (
                id TEXT PRIMARY KEY,
                vulnerability_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                submission_id TEXT,
                title TEXT NOT NULL,
                description TEXT,
                severity TEXT NOT NULL,
                status TEXT DEFAULT 'submitted',
                reward_amount REAL,
                bonus_amount REAL,
                currency TEXT DEFAULT 'USD',
                submitted_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                triaged_at DATETIME,
                resolved_at DATETIME,
                feedback TEXT,
                response_time INTEGER,
                resolution_time INTEGER,
                collaborators TEXT,
                attachments TEXT,
                notes TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (vulnerability_id) REFERENCES vulnerabilities(id) ON DELETE CASCADE,
                FOREIGN KEY (project_id) REFERENCES bounty_projects(id) ON DELETE CASCADE
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建MCP工具表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS mcp_tools (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                display_name TEXT,
                description TEXT,
                version TEXT,
                category TEXT,
                tool_type TEXT NOT NULL,
                executable_path TEXT,
                install_command TEXT,
                config_schema TEXT,
                default_config TEXT,
                capabilities TEXT,
                supported_platforms TEXT,
                requirements TEXT,
                status TEXT DEFAULT 'available',
                installation_status TEXT,
                last_used DATETIME,
                usage_count INTEGER DEFAULT 0,
                success_rate REAL DEFAULT 1.0,
                average_execution_time INTEGER,
                tags TEXT,
                author TEXT,
                license TEXT,
                documentation_url TEXT,
                source_url TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建MCP连接表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS mcp_connections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                connection_type TEXT NOT NULL,
                endpoint TEXT,
                config TEXT,
                status TEXT DEFAULT 'disconnected',
                capabilities TEXT,
                server_info TEXT,
                tools_count INTEGER DEFAULT 0,
                last_ping DATETIME,
                connected_at DATETIME,
                error_message TEXT,
                retry_count INTEGER DEFAULT 0,
                max_retries INTEGER DEFAULT 3,
                auto_reconnect BOOLEAN DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建MCP服务器配置表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS mcp_server_configs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                url TEXT NOT NULL,
                connection_type TEXT NOT NULL,
                auth_type TEXT,
                auth_config TEXT,
                command TEXT,
                args TEXT,
                is_enabled BOOLEAN DEFAULT 1,
                is_default BOOLEAN DEFAULT 0,
                auto_connect BOOLEAN DEFAULT 1,
                retry_count INTEGER DEFAULT 3,
                retry_delay INTEGER DEFAULT 5000,
                timeout INTEGER DEFAULT 30000,
                metadata TEXT,
                last_connected DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建工具执行记录表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tool_executions (
                id TEXT PRIMARY KEY,
                tool_id TEXT NOT NULL,
                scan_task_id TEXT,
                command TEXT NOT NULL,
                arguments TEXT,
                status TEXT DEFAULT 'pending',
                progress REAL DEFAULT 0.0,
                start_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                end_time DATETIME,
                execution_time INTEGER,
                output TEXT,
                error_output TEXT,
                exit_code INTEGER,
                resource_usage TEXT,
                artifacts TEXT,
                metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (tool_id) REFERENCES mcp_tools(id) ON DELETE CASCADE,
                FOREIGN KEY (scan_task_id) REFERENCES scan_tasks(id) ON DELETE CASCADE
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建AI对话记录表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ai_conversations (
                id TEXT PRIMARY KEY,
                title TEXT,
                service_name TEXT DEFAULT 'default',
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
                is_archived BOOLEAN DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES bounty_projects(id) ON DELETE SET NULL,
                FOREIGN KEY (vulnerability_id) REFERENCES vulnerabilities(id) ON DELETE SET NULL,
                FOREIGN KEY (scan_task_id) REFERENCES scan_tasks(id) ON DELETE SET NULL
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建AI消息表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ai_messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                token_count INTEGER,
                cost REAL,
                tool_calls TEXT,
                attachments TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (conversation_id) REFERENCES ai_conversations(id) ON DELETE CASCADE
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建AI角色表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ai_roles (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                prompt TEXT NOT NULL,
                is_system BOOLEAN DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建收益统计表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS earnings (
                id TEXT PRIMARY KEY,
                submission_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                amount REAL NOT NULL,
                currency TEXT DEFAULT 'USD',
                earning_type TEXT,
                payment_status TEXT DEFAULT 'pending',
                payment_date DATETIME,
                payment_method TEXT,
                tax_info TEXT,
                notes TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (submission_id) REFERENCES submissions(id) ON DELETE CASCADE,
                FOREIGN KEY (project_id) REFERENCES bounty_projects(id) ON DELETE CASCADE
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建配置表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS configurations (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT,
                description TEXT,
                is_encrypted BOOLEAN DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(category, key)
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建字典表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS dictionaries (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                dict_type TEXT NOT NULL,
                service_type TEXT,
                category TEXT,
                is_builtin BOOLEAN DEFAULT 0,
                is_active BOOLEAN DEFAULT 1,
                word_count INTEGER DEFAULT 0,
                file_size INTEGER DEFAULT 0,
                checksum TEXT,
                version TEXT DEFAULT '1.0.0',
                author TEXT,
                source_url TEXT,
                tags TEXT,
                metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建字典词条表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS dictionary_words (
                id TEXT PRIMARY KEY,
                dictionary_id TEXT NOT NULL,
                word TEXT NOT NULL,
                weight REAL DEFAULT 1.0,
                category TEXT,
                metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (dictionary_id) REFERENCES dictionaries(id) ON DELETE CASCADE
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建字典集合表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS dictionary_sets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                service_type TEXT,
                scenario TEXT,
                is_active BOOLEAN DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建字典集合关系表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS dictionary_set_relations (
                id TEXT PRIMARY KEY,
                set_id TEXT NOT NULL,
                dictionary_id TEXT NOT NULL,
                priority INTEGER DEFAULT 0,
                is_enabled BOOLEAN DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (set_id) REFERENCES dictionary_sets(id) ON DELETE CASCADE,
                FOREIGN KEY (dictionary_id) REFERENCES dictionaries(id) ON DELETE CASCADE,
                UNIQUE(set_id, dictionary_id)
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建索引
        // 赏金项目索引
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_bounty_projects_platform ON bounty_projects(platform)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_bounty_projects_status ON bounty_projects(status)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_bounty_projects_roi_score ON bounty_projects(roi_score DESC)").execute(&mut *tx).await?;

        // 扫描任务索引
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_scan_tasks_project_id ON scan_tasks(project_id)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_scan_tasks_status ON scan_tasks(status)")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_scan_tasks_created_at ON scan_tasks(created_at DESC)",
        )
        .execute(&mut *tx)
        .await?;

        // 资产索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_assets_project_id ON assets(project_id)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_assets_type ON assets(asset_type)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_assets_risk_level ON assets(risk_level)")
            .execute(&mut *tx)
            .await?;

        // 漏洞索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_vulnerabilities_project_id ON vulnerabilities(project_id)").execute(&mut *tx).await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_vulnerabilities_severity ON vulnerabilities(severity)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_vulnerabilities_status ON vulnerabilities(status)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_vulnerabilities_type ON vulnerabilities(vulnerability_type)").execute(&mut *tx).await?;

        // 提交记录索引
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_submissions_project_id ON submissions(project_id)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_submissions_status ON submissions(status)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_submissions_submitted_at ON submissions(submitted_at DESC)").execute(&mut *tx).await?;

        // MCP工具索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tools_category ON mcp_tools(category)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tools_status ON mcp_tools(status)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tools_name ON mcp_tools(name)")
            .execute(&mut *tx)
            .await?;

        // 工具执行记录索引
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tool_executions_tool_id ON tool_executions(tool_id)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tool_executions_status ON tool_executions(status)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tool_executions_start_time ON tool_executions(start_time DESC)").execute(&mut *tx).await?;

        // AI对话索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ai_conversations_model_name ON ai_conversations(model_name)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ai_conversations_context_type ON ai_conversations(context_type)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ai_conversations_created_at ON ai_conversations(created_at DESC)").execute(&mut *tx).await?;

        // AI消息索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ai_messages_conversation_id ON ai_messages(conversation_id)").execute(&mut *tx).await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_ai_messages_timestamp ON ai_messages(timestamp DESC)",
        )
        .execute(&mut *tx)
        .await?;

        // 收益索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_earnings_project_id ON earnings(project_id)")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_earnings_payment_status ON earnings(payment_status)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_earnings_created_at ON earnings(created_at DESC)",
        )
        .execute(&mut *tx)
        .await?;

        // 配置索引
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_configurations_category ON configurations(category)",
        )
        .execute(&mut *tx)
        .await?;

        // 字典索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionaries_type ON dictionaries(dict_type)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionaries_service_type ON dictionaries(service_type)").execute(&mut *tx).await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_dictionaries_category ON dictionaries(category)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_dictionaries_builtin ON dictionaries(is_builtin)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_dictionaries_active ON dictionaries(is_active)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionaries_name ON dictionaries(name)")
            .execute(&mut *tx)
            .await?;

        // 字典词条索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionary_words_dict_id ON dictionary_words(dictionary_id)").execute(&mut *tx).await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_dictionary_words_word ON dictionary_words(word)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionary_words_category ON dictionary_words(category)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionary_words_weight ON dictionary_words(weight DESC)").execute(&mut *tx).await?;

        // 字典集合索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionary_sets_service_type ON dictionary_sets(service_type)").execute(&mut *tx).await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_dictionary_sets_scenario ON dictionary_sets(scenario)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_dictionary_sets_active ON dictionary_sets(is_active)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionary_sets_name ON dictionary_sets(name)")
            .execute(&mut *tx)
            .await?;

        // 字典集合关系索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionary_set_relations_set_id ON dictionary_set_relations(set_id)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionary_set_relations_dict_id ON dictionary_set_relations(dictionary_id)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionary_set_relations_priority ON dictionary_set_relations(priority DESC)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dictionary_set_relations_enabled ON dictionary_set_relations(is_enabled)").execute(&mut *tx).await?;

        // 创建MCP服务器配置索引
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_mcp_server_configs_name ON mcp_server_configs(name)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_server_configs_enabled ON mcp_server_configs(is_enabled)").execute(&mut *tx).await?;

        // 提交事务
        tx.commit().await?;

        tracing::info!("Database schema created successfully");
        Ok(())
    }

    /// 获取初始数据库架构SQL语句 - 不再需要，直接在create_database_schema中实现
    fn get_initial_schema_statements(&self) -> Vec<String> {
        vec![]
    }

    /// 运行数据库迁移 - 不再需要，直接在initialize中创建表结构
    async fn run_migrations(&self, _pool: &SqlitePool) -> Result<()> {
        Ok(())
    }

    /// 获取数据库连接池
    pub fn get_pool(&self) -> Result<&SqlitePool> {
        self.pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))
    }

    /// 执行自定义查询
    pub async fn execute_query(&self, query: &str) -> Result<Vec<Value>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query(query).fetch_all(pool).await?;

        let mut results = Vec::new();
        for row in rows {
            let mut obj = serde_json::Map::new();

            for (i, column) in row.columns().iter().enumerate() {
                let column_name = column.name();
                let value: Value = match column.type_info().name() {
                    "TEXT" => {
                        let val: Option<String> = row.try_get(i)?;
                        val.map(Value::String).unwrap_or(Value::Null)
                    }
                    "INTEGER" => {
                        let val: Option<i64> = row.try_get(i)?;
                        val.map(|v| Value::Number(v.into())).unwrap_or(Value::Null)
                    }
                    "REAL" => {
                        let val: Option<f64> = row.try_get(i)?;
                        val.map(|v| {
                            Value::Number(
                                serde_json::Number::from_f64(v).unwrap_or_else(|| 0.into()),
                            )
                        })
                        .unwrap_or(Value::Null)
                    }
                    _ => Value::Null,
                };
                obj.insert(column_name.to_string(), value);
            }

            results.push(Value::Object(obj));
        }

        Ok(results)
    }

    /// 获取数据库统计信息
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let pool = self.get_pool()?;

        let projects_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bounty_projects")
            .fetch_one(pool)
            .await?;

        let scan_tasks_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scan_tasks")
            .fetch_one(pool)
            .await?;

        let vulnerabilities_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vulnerabilities")
            .fetch_one(pool)
            .await?;

        let assets_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets")
            .fetch_one(pool)
            .await?;

        let submissions_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM submissions")
            .fetch_one(pool)
            .await?;

        let conversations_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_conversations")
            .fetch_one(pool)
            .await?;

        // 获取数据库文件大小
        let db_size = std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(DatabaseStats {
            projects_count: projects_count as u64,
            scan_tasks_count: scan_tasks_count as u64,
            vulnerabilities_count: vulnerabilities_count as u64,
            assets_count: assets_count as u64,
            submissions_count: submissions_count as u64,
            conversations_count: conversations_count as u64,
            db_size_bytes: db_size,
            last_backup: None, // TODO: 实现备份跟踪
        })
    }

    /// 备份数据库
    pub async fn backup(&self, backup_path: Option<PathBuf>) -> Result<PathBuf> {
        let backup_path = backup_path.unwrap_or_else(|| {
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            self.db_path
                .parent()
                .unwrap_or(&PathBuf::from("."))
                .join(format!("backup_{}.db", timestamp))
        });

        // 简单的文件复制备份
        std::fs::copy(&self.db_path, &backup_path)?;

        tracing::info!("Database backup completed: {}", backup_path.display());
        Ok(backup_path)
    }

    /// 恢复数据库
    pub async fn restore(&self, backup_path: PathBuf) -> Result<()> {
        // 注意：由于这是不可变引用，我们不能直接修改 pool
        // 在实际实现中，可能需要使用 Arc<Mutex<>> 或者重新设计架构

        // 复制备份文件
        std::fs::copy(&backup_path, &self.db_path)?;

        // 注意：这里需要重新初始化，但由于是不可变引用，暂时跳过
        // 在实际应用中，应该重新设计架构来处理这种情况

        tracing::info!("Database restoration completed: {}", backup_path.display());
        Ok(())
    }

    /// 项目相关操作
    pub async fn create_project(&self, project: &BountyProject) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            INSERT INTO bounty_projects (
                id, name, platform, url, scope_domains, scope_ips, out_of_scope,
                reward_range, difficulty_level, priority, status, roi_score,
                success_rate, competition_level, tags, notes
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&project.id)
        .bind(&project.name)
        .bind(&project.platform)
        .bind(&project.url)
        .bind(&project.scope_domains)
        .bind(&project.scope_ips)
        .bind(&project.out_of_scope)
        .bind(&project.reward_range)
        .bind(project.difficulty_level)
        .bind(project.priority)
        .bind(&project.status)
        .bind(project.roi_score)
        .bind(project.success_rate)
        .bind(project.competition_level)
        .bind(&project.tags)
        .bind(&project.notes)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_projects(&self) -> Result<Vec<BountyProject>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query_as::<_, BountyProject>(
            "SELECT * FROM bounty_projects ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn get_project(&self, id: &str) -> Result<BountyProject> {
        let pool = self.get_pool()?;

        let row = sqlx::query_as::<_, BountyProject>("SELECT * FROM bounty_projects WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await?;

        Ok(row)
    }

    pub async fn update_project(&self, project: &BountyProject) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            UPDATE bounty_projects SET
                name = ?, platform = ?, url = ?, scope_domains = ?, scope_ips = ?,
                out_of_scope = ?, reward_range = ?, difficulty_level = ?, priority = ?,
                status = ?, roi_score = ?, success_rate = ?, competition_level = ?,
                tags = ?, notes = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
        "#,
        )
        .bind(&project.name)
        .bind(&project.platform)
        .bind(&project.url)
        .bind(&project.scope_domains)
        .bind(&project.scope_ips)
        .bind(&project.out_of_scope)
        .bind(&project.reward_range)
        .bind(project.difficulty_level)
        .bind(project.priority)
        .bind(&project.status)
        .bind(project.roi_score)
        .bind(project.success_rate)
        .bind(project.competition_level)
        .bind(&project.tags)
        .bind(&project.notes)
        .bind(&project.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_project(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query("DELETE FROM bounty_projects WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 扫描任务相关操作
    pub async fn create_scan_task(&self, task: &ScanTask) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            INSERT INTO scan_tasks (
                id, project_id, name, description, target_type, targets,
                scan_type, tools_config, status, progress, priority,
                scheduled_at, created_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&task.id)
        .bind(&task.project_id)
        .bind(&task.name)
        .bind(&task.description)
        .bind(&task.target_type)
        .bind(&task.targets)
        .bind(&task.scan_type)
        .bind(&task.tools_config)
        .bind(&task.status)
        .bind(task.progress)
        .bind(task.priority)
        .bind(&task.scheduled_at)
        .bind(&task.created_by)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_scan_tasks(&self, project_id: Option<&str>) -> Result<Vec<ScanTask>> {
        let pool = self.get_pool()?;

        let rows = if let Some(project_id) = project_id {
            sqlx::query_as::<_, ScanTask>(
                "SELECT * FROM scan_tasks WHERE project_id = ? ORDER BY created_at DESC",
            )
            .bind(project_id)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks ORDER BY created_at DESC")
                .fetch_all(pool)
                .await?
        };

        Ok(rows)
    }

    pub async fn get_scan_task(&self, id: &str) -> Result<Option<ScanTask>> {
        let pool = self.get_pool()?;

        let row = sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(row)
    }

    pub async fn update_scan_task_status(
        &self,
        id: &str,
        status: &str,
        progress: Option<f64>,
    ) -> Result<()> {
        let pool = self.get_pool()?;

        let mut query =
            "UPDATE scan_tasks SET status = ?, updated_at = CURRENT_TIMESTAMP".to_string();
        let mut bind_count = 1;

        if progress.is_some() {
            query.push_str(", progress = ?");
            bind_count += 1;
        }

        if status == "running" {
            query.push_str(", started_at = CURRENT_TIMESTAMP");
        } else if status == "completed" || status == "failed" || status == "cancelled" {
            query.push_str(", completed_at = CURRENT_TIMESTAMP");
        }

        query.push_str(" WHERE id = ?");

        let mut q = sqlx::query(&query).bind(status);

        if let Some(p) = progress {
            q = q.bind(p);
        }

        q.bind(id).execute(pool).await?;

        Ok(())
    }

    /// 漏洞相关操作
    pub async fn create_vulnerability(&self, vuln: &Vulnerability) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            INSERT INTO vulnerabilities (
                id, project_id, asset_id, scan_task_id, title, description,
                vulnerability_type, severity, cvss_score, cvss_vector, cwe_id,
                owasp_category, proof_of_concept, impact, remediation, references,
                status, verification_status, submission_status, tags, attachments, notes
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&vuln.id)
        .bind(&vuln.project_id)
        .bind(&vuln.asset_id)
        .bind(&vuln.scan_task_id)
        .bind(&vuln.title)
        .bind(&vuln.description)
        .bind(&vuln.vulnerability_type)
        .bind(&vuln.severity)
        .bind(vuln.cvss_score)
        .bind(&vuln.cvss_vector)
        .bind(&vuln.cwe_id)
        .bind(&vuln.owasp_category)
        .bind(&vuln.proof_of_concept)
        .bind(&vuln.impact)
        .bind(&vuln.remediation)
        .bind(&vuln.references)
        .bind(&vuln.status)
        .bind(&vuln.verification_status)
        .bind(&vuln.submission_status)
        .bind(&vuln.tags)
        .bind(&vuln.attachments)
        .bind(&vuln.notes)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_vulnerabilities(
        &self,
        project_id: Option<&str>,
    ) -> Result<Vec<Vulnerability>> {
        let pool = self.get_pool()?;

        let rows = if let Some(project_id) = project_id {
            sqlx::query_as::<_, Vulnerability>(
                "SELECT * FROM vulnerabilities WHERE project_id = ? ORDER BY created_at DESC",
            )
            .bind(project_id)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, Vulnerability>(
                "SELECT * FROM vulnerabilities ORDER BY created_at DESC",
            )
            .fetch_all(pool)
            .await?
        };

        Ok(rows)
    }

    pub async fn get_vulnerability(&self, id: &str) -> Result<Option<Vulnerability>> {
        let pool = self.get_pool()?;

        let row = sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(row)
    }

    pub async fn update_vulnerability_status(&self, id: &str, status: &str) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            "UPDATE vulnerabilities SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// AI对话相关操作
    pub async fn create_conversation(&self, conversation: &AiConversation) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            INSERT INTO ai_conversations (id, title, service_name, model_name, model_provider, context_type, project_id, vulnerability_id, scan_task_id, conversation_data, summary, total_messages, total_tokens, cost, tags, is_archived, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&conversation.id)
        .bind(&conversation.title)
        .bind(&conversation.service_name)
        .bind(&conversation.model_name)
        .bind(&conversation.model_provider)
        .bind(&conversation.context_type)
        .bind(&conversation.project_id)
        .bind(&conversation.vulnerability_id)
        .bind(&conversation.scan_task_id)
        .bind(&conversation.conversation_data)
        .bind(&conversation.summary)
        .bind(conversation.total_messages)
        .bind(conversation.total_tokens)
        .bind(conversation.cost)
        .bind(serde_json::to_string(&conversation.tags).unwrap_or_default())
        .bind(conversation.is_archived)
        .bind(conversation.created_at)
        .bind(conversation.updated_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_conversations(&self) -> Result<Vec<AiConversation>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query_as::<_, AiConversation>(
            "SELECT * FROM ai_conversations WHERE is_archived = 0 ORDER BY updated_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn get_conversation(&self, id: &str) -> Result<Option<AiConversation>> {
        let pool = self.get_pool()?;

        let row =
            sqlx::query_as::<_, AiConversation>("SELECT * FROM ai_conversations WHERE id = ?")
                .bind(id)
                .fetch_optional(pool)
                .await?;

        Ok(row)
    }

    pub async fn update_conversation(&self, conversation: &AiConversation) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            UPDATE ai_conversations
            SET title = ?, service_name = ?, model_name = ?, model_provider = ?, context_type = ?, project_id = ?, vulnerability_id = ?, scan_task_id = ?, conversation_data = ?, summary = ?, total_messages = ?, total_tokens = ?, cost = ?, tags = ?, is_archived = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&conversation.title)
        .bind(&conversation.service_name)
        .bind(&conversation.model_name)
        .bind(&conversation.model_provider)
        .bind(&conversation.context_type)
        .bind(&conversation.project_id)
        .bind(&conversation.vulnerability_id)
        .bind(&conversation.scan_task_id)
        .bind(&conversation.conversation_data)
        .bind(&conversation.summary)
        .bind(conversation.total_messages)
        .bind(conversation.total_tokens)
        .bind(conversation.cost)
        .bind(serde_json::to_string(&conversation.tags).unwrap_or_default())
        .bind(conversation.is_archived)
        .bind(Utc::now())
        .bind(&conversation.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_conversation(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;

        // 先删除相关的消息
        sqlx::query("DELETE FROM ai_messages WHERE conversation_id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        // 再删除对话
        sqlx::query("DELETE FROM ai_conversations WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 更新对话标题
    pub async fn update_conversation_title(&self, id: &str, title: &str) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query("UPDATE ai_conversations SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 创建AI消息
    pub async fn create_message(&self, message: &AiMessage) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            INSERT INTO ai_messages (
                id, conversation_id, role, content, metadata,
                token_count, cost, tool_calls, attachments, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&message.id)
        .bind(&message.conversation_id)
        .bind(&message.role)
        .bind(&message.content)
        .bind(&message.metadata)
        .bind(message.token_count)
        .bind(message.cost)
        .bind(&message.tool_calls)
        .bind(&message.attachments)
        .bind(message.timestamp)
        .execute(pool)
        .await?;

        // 更新对话的updated_at和消息计数
        sqlx::query("UPDATE ai_conversations SET updated_at = ?, total_messages = total_messages + 1 WHERE id = ?")
            .bind(Utc::now())
            .bind(&message.conversation_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 获取对话的消息列表
    pub async fn get_messages(&self, conversation_id: &str) -> Result<Vec<AiMessage>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query_as::<_, AiMessage>(
            "SELECT * FROM ai_messages WHERE conversation_id = ? ORDER BY timestamp ASC",
        )
        .bind(conversation_id)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    /// 配置相关操作
    pub async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;

        let value: Option<String> =
            sqlx::query_scalar("SELECT value FROM configurations WHERE category = ? AND key = ?")
                .bind(category)
                .bind(key)
                .fetch_optional(pool)
                .await?;

        Ok(value)
    }

    pub async fn set_config(
        &self,
        category: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "INSERT INTO configurations (category, key, value, description) VALUES (?, ?, ?, ?)
             ON CONFLICT(category, key) DO UPDATE SET value = excluded.value, description = excluded.description"
        )
        .bind(category)
        .bind(key)
        .bind(value)
        .bind(description)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_configs_by_category(&self, category: &str) -> Result<Vec<Configuration>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query_as::<_, Configuration>(
            "SELECT * FROM configurations WHERE category = ? ORDER BY key",
        )
        .bind(category)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    /// 子域名字典管理方法
    pub async fn get_subdomain_dictionary(&self) -> Result<Vec<String>> {
        let pool = self.get_pool()?;

        // 尝试从数据库获取字典
        let dictionary_json: Option<String> = sqlx::query_scalar(
            "SELECT value FROM configurations WHERE category = 'subdomain_scanner' AND key = 'dictionary'"
        )
        .fetch_optional(pool)
        .await?;

        if let Some(json_str) = dictionary_json {
            // 解析JSON字符串为字符串数组
            let dictionary: Vec<String> = serde_json::from_str(&json_str)
                .unwrap_or_else(|_| self.get_default_subdomain_dictionary());
            Ok(dictionary)
        } else {
            // 如果数据库中没有，使用默认字典并保存到数据库
            let default_dict = self.get_default_subdomain_dictionary();
            self.set_subdomain_dictionary(&default_dict).await?;
            Ok(default_dict)
        }
    }

    pub async fn set_subdomain_dictionary(&self, dictionary: &[String]) -> Result<()> {
        let dictionary_json = serde_json::to_string(dictionary)?;
        self.set_config(
            "subdomain_scanner",
            "dictionary",
            &dictionary_json,
            Some("子域名扫描字典"),
        )
        .await?;
        Ok(())
    }

    pub async fn add_subdomain_words(&self, words: &[String]) -> Result<()> {
        let mut current_dict = self.get_subdomain_dictionary().await?;

        // 添加新词汇，去重
        for word in words {
            if !current_dict.contains(word) {
                current_dict.push(word.clone());
            }
        }

        // 排序并保存
        current_dict.sort();
        self.set_subdomain_dictionary(&current_dict).await?;
        Ok(())
    }

    pub async fn remove_subdomain_words(&self, words: &[String]) -> Result<()> {
        let mut current_dict = self.get_subdomain_dictionary().await?;

        // 移除指定词汇
        current_dict.retain(|word| !words.contains(word));

        self.set_subdomain_dictionary(&current_dict).await?;
        Ok(())
    }

    fn get_default_subdomain_dictionary(&self) -> Vec<String> {
        vec![
            "www".to_string(),
            "mail".to_string(),
            "ftp".to_string(),
            "localhost".to_string(),
            "webmail".to_string(),
            "smtp".to_string(),
            "pop".to_string(),
            "ns1".to_string(),
            "webdisk".to_string(),
            "ns2".to_string(),
            "cpanel".to_string(),
            "whm".to_string(),
            "autodiscover".to_string(),
            "autoconfig".to_string(),
            "m".to_string(),
            "imap".to_string(),
            "test".to_string(),
            "ns".to_string(),
            "blog".to_string(),
            "pop3".to_string(),
            "dev".to_string(),
            "www2".to_string(),
            "admin".to_string(),
            "forum".to_string(),
            "news".to_string(),
            "vpn".to_string(),
            "ns3".to_string(),
            "mail2".to_string(),
            "new".to_string(),
            "mysql".to_string(),
            "old".to_string(),
            "lists".to_string(),
            "support".to_string(),
            "mobile".to_string(),
            "static".to_string(),
            "docs".to_string(),
            "beta".to_string(),
            "shop".to_string(),
            "sql".to_string(),
            "secure".to_string(),
            "demo".to_string(),
            "cp".to_string(),
            "calendar".to_string(),
            "wiki".to_string(),
            "web".to_string(),
            "media".to_string(),
            "email".to_string(),
            "images".to_string(),
            "img".to_string(),
            "www1".to_string(),
            "intranet".to_string(),
            "portal".to_string(),
            "video".to_string(),
            "sip".to_string(),
            "dns2".to_string(),
            "api".to_string(),
            "cdn".to_string(),
            "stats".to_string(),
            "dns1".to_string(),
            "ns4".to_string(),
            "www3".to_string(),
            "dns".to_string(),
            "search".to_string(),
            "staging".to_string(),
            "server".to_string(),
            "mx".to_string(),
            "chat".to_string(),
            "en".to_string(),
            "wap".to_string(),
            "redmine".to_string(),
            "ftp2".to_string(),
            "db".to_string(),
            "erp".to_string(),
            "explore".to_string(),
            "download".to_string(),
            "ww1".to_string(),
            "catalog".to_string(),
            "ssh".to_string(),
            "management".to_string(),
            "www4".to_string(),
        ]
    }

    /// 根据项目ID获取扫描任务
    pub async fn get_scan_tasks_by_project(&self, project_id: &str) -> Result<Vec<ScanTask>> {
        self.get_scan_tasks(Some(project_id)).await
    }

    /// 根据项目ID获取漏洞
    pub async fn get_vulnerabilities_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<Vulnerability>> {
        self.get_vulnerabilities(Some(project_id)).await
    }

    /// 根据项目ID获取提交记录
    pub async fn get_submissions_by_project(&self, project_id: &str) -> Result<Vec<Submission>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query_as::<_, Submission>(
            "SELECT * FROM submissions WHERE project_id = ? ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    // --- MCP Server Configs ---

    pub async fn create_mcp_server_config(
        &self,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
    ) -> Result<String> {
        let args_json = serde_json::to_string(args)?;
        let pool = self.get_pool()?;
        let mut conn = pool.acquire().await?;
        let id = uuid::Uuid::new_v4().to_string();

        // 根据命令生成默认URL
        let url = format!("http://localhost:8080");
        let connection_type = "stdio";

        sqlx::query(
            r#"
            INSERT INTO mcp_server_configs (id, name, description, url, connection_type, command, args)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(&url)
        .bind(connection_type)
        .bind(command)
        .bind(args_json)
        .execute(&mut *conn)
        .await?;
        Ok(id)
    }

    pub async fn get_all_mcp_server_configs(&self) -> Result<Vec<McpServerConfig>> {
        let pool = self.get_pool()?;
        let configs = sqlx::query_as::<_, McpServerConfig>(
            "SELECT id, name, description, url, connection_type, command, args, is_enabled as enabled, created_at, updated_at FROM mcp_server_configs"
        )
        .fetch_all(pool)
        .await?;
        Ok(configs)
    }

    pub async fn update_mcp_server_config_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE mcp_server_configs SET is_enabled = ? WHERE id = ?")
            .bind(enabled)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete_mcp_server_config(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM mcp_server_configs WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_mcp_server_config_by_name(
        &self,
        name: &str,
    ) -> Result<Option<McpServerConfig>> {
        let pool = self.get_pool()?;
        let config = sqlx::query_as::<_, McpServerConfig>(
            "SELECT id, name, description, url, connection_type, command, args, is_enabled as enabled, created_at, updated_at FROM mcp_server_configs WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;
        Ok(config)
    }

    pub async fn update_mcp_server_config(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
        enabled: bool,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        let args_json = serde_json::to_string(args)?;

        // 获取现有配置以保留url和connection_type
        let existing = self.get_mcp_server_config_by_name(name).await?;

        // 如果找不到现有配置，使用默认值
        let url = existing
            .as_ref()
            .map(|c| c.url.clone())
            .unwrap_or_else(|| "http://localhost:8080".to_string());
        let connection_type = existing
            .as_ref()
            .map(|c| c.connection_type.clone())
            .unwrap_or_else(|| "stdio".to_string());

        sqlx::query(
            "UPDATE mcp_server_configs SET name = ?, description = ?, url = ?, connection_type = ?, command = ?, args = ?, is_enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(name)
        .bind(description)
        .bind(&url)
        .bind(&connection_type)
        .bind(command)
        .bind(&args_json)
        .bind(enabled)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn get_ai_roles(&self) -> Result<Vec<AiRole>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT id, title, description, prompt, is_system, created_at, updated_at FROM ai_roles ORDER BY created_at DESC")
            .fetch_all(pool)
            .await?;

        let mut roles = Vec::with_capacity(rows.len());
        for row in rows {
            roles.push(AiRole {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                prompt: row.get("prompt"),
                is_system: row.get("is_system"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(roles)
    }

    async fn create_ai_role(&self, role: &AiRole) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("INSERT INTO ai_roles (id, title, description, prompt, is_system, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(&role.id)
            .bind(&role.title)
            .bind(&role.description)
            .bind(&role.prompt)
            .bind(role.is_system)
            .bind(role.created_at)
            .bind(role.updated_at)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn update_ai_role(&self, role: &AiRole) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE ai_roles SET title = ?, description = ?, prompt = ?, updated_at = ? WHERE id = ?")
            .bind(&role.title)
            .bind(&role.description)
            .bind(&role.prompt)
            .bind(Utc::now())
            .bind(&role.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn delete_ai_role(&self, role_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        tracing::info!("Executing SQL to delete role with ID: {}", role_id);

        let result = sqlx::query("DELETE FROM ai_roles WHERE id = ?")
            .bind(role_id)
            .execute(pool)
            .await?;

        let rows_affected = result.rows_affected();
        tracing::info!(
            "Delete operation affected {} rows for role ID: {}",
            rows_affected,
            role_id
        );

        if rows_affected == 0 {
            tracing::warn!(
                "No rows were affected when deleting role ID: {}. Role might not exist.",
                role_id
            );
        }

        Ok(())
    }

    /// 插入默认配置和数据
    async fn insert_default_data(&self, pool: &SqlitePool) -> Result<()> {
        tracing::info!("Starting to insert default data...");

        // 创建默认配置
        let default_configs = vec![
            (
                "cfg_ai_default_model",
                "ai",
                "default_model",
                "\"gpt-4\"",
                "AI默认模型",
            ),
            (
                "cfg_ai_max_tokens",
                "ai",
                "max_tokens",
                "4000",
                "AI最大令牌数",
            ),
            (
                "cfg_ai_temperature",
                "ai",
                "temperature",
                "0.7",
                "AI温度参数",
            ),
            (
                "cfg_scan_default_timeout",
                "scan",
                "default_timeout",
                "3600",
                "扫描默认超时时间（秒）",
            ),
            (
                "cfg_scan_max_concurrent",
                "scan",
                "max_concurrent_tasks",
                "5",
                "最大并发扫描任务数",
            ),
            (
                "cfg_mcp_auto_connect",
                "mcp",
                "auto_connect_servers",
                "true",
                "是否自动连接MCP服务器",
            ),
            (
                "cfg_notification_enabled",
                "notification",
                "enabled",
                "true",
                "是否启用通知",
            ),
            (
                "cfg_general_theme",
                "general",
                "theme",
                "\"dark\"",
                "界面主题",
            ),
        ];

        for (id, category, key, value, description) in default_configs {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO configurations (id, category, key, value, description) 
                VALUES (?, ?, ?, ?, ?)
            "#,
            )
            .bind(id)
            .bind(category)
            .bind(key)
            .bind(value)
            .bind(description)
            .execute(pool)
            .await?;
        }

        // 插入默认MCP工具
        let default_tools = vec![
            (
                "tool_subfinder",
                "subfinder",
                "Subfinder",
                "快速子域名发现工具",
                "2.6.3",
                "reconnaissance",
                "builtin",
                r#"{"type":"object","properties":{"domain":{"type":"string"},"silent":{"type":"boolean"},"recursive":{"type":"boolean"}}}"#,
                r#"{"silent":true,"recursive":false}"#,
                r#"["subdomain_enumeration","passive_recon"]"#,
                r#"["linux","windows","darwin"]"#,
            ),
            (
                "tool_nmap",
                "nmap",
                "Nmap",
                "网络发现和安全审计工具",
                "7.94",
                "reconnaissance",
                "builtin",
                r#"{"type":"object","properties":{"target":{"type":"string"},"ports":{"type":"string"},"scan_type":{"type":"string"}}}"#,
                r#"{"ports":"1-1000","scan_type":"syn"}"#,
                r#"["port_scanning","service_detection","os_detection"]"#,
                r#"["linux","windows","darwin"]"#,
            ),
            (
                "tool_nuclei",
                "nuclei",
                "Nuclei",
                "基于模板的漏洞扫描器",
                "3.1.0",
                "vulnerability_scanning",
                "builtin",
                r#"{"type":"object","properties":{"target":{"type":"string"},"templates":{"type":"array"},"severity":{"type":"string"}}}"#,
                r#"{"severity":"medium,high,critical"}"#,
                r#"["vulnerability_scanning","template_based_scanning"]"#,
                r#"["linux","windows","darwin"]"#,
            ),
            (
                "tool_httpx",
                "httpx",
                "Httpx",
                "HTTP工具包",
                "1.3.7",
                "reconnaissance",
                "builtin",
                r#"{"type":"object","properties":{"target":{"type":"string"},"probe":{"type":"boolean"},"tech_detect":{"type":"boolean"}}}"#,
                r#"{"probe":true,"tech_detect":true}"#,
                r#"["http_probing","technology_detection","web_analysis"]"#,
                r#"["linux","windows","darwin"]"#,
            ),
        ];

        for (
            id,
            name,
            display_name,
            description,
            version,
            category,
            tool_type,
            config_schema,
            default_config,
            capabilities,
            supported_platforms,
        ) in default_tools
        {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO mcp_tools (
                    id, name, display_name, description, version, category, tool_type,
                    config_schema, default_config, capabilities, supported_platforms, status
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'available')
            "#,
            )
            .bind(id)
            .bind(name)
            .bind(display_name)
            .bind(description)
            .bind(version)
            .bind(category)
            .bind(tool_type)
            .bind(config_schema)
            .bind(default_config)
            .bind(capabilities)
            .bind(supported_platforms)
            .execute(pool)
            .await?;
        }

        // 创建默认MCP服务器配置
        let default_mcp_server = (
            "default_mcp_server",
            "本地MCP服务器",
            "默认的本地MCP服务器配置",
            "http://localhost:8080",
            "http",
            "none",
            r#"{"username":"","password":"","token":""}"#,
            "mcp-server",
            r#"["--port", "8080"]"#,
            true,
            true,
            true,
            3,
            5000,
            30000,
            r#"{}"#,
        );

        let (
            id,
            name,
            description,
            url,
            connection_type,
            auth_type,
            auth_config,
            command,
            args,
            is_enabled,
            is_default,
            auto_connect,
            retry_count,
            retry_delay,
            timeout,
            metadata,
        ) = default_mcp_server;

        sqlx::query(r#"
            INSERT OR IGNORE INTO mcp_server_configs (
                id, name, description, url, connection_type, auth_type, auth_config,
                command, args, is_enabled, is_default, auto_connect, retry_count, retry_delay, timeout, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(url)
        .bind(connection_type)
        .bind(auth_type)
        .bind(auth_config)
        .bind(command)
        .bind(args)
        .bind(is_enabled)
        .bind(is_default)
        .bind(auto_connect)
        .bind(retry_count)
        .bind(retry_delay)
        .bind(timeout)
        .bind(metadata)
        .execute(pool)
        .await?;

        // 初始化默认AI角色
        self.initialize_default_roles(pool).await?;

        tracing::info!("Default data insertion completed");
        Ok(())
    }

    /// 初始化默认AI角色
    async fn initialize_default_roles(&self, pool: &SqlitePool) -> Result<()> {
        let roles = vec![
            ("writer", "写作导师", "帮助用户改进文章、润色文字。", "你是一个经验丰富的写作导师，精通各种文体的写作技巧。你可以帮助用户改进文章结构、提升表达能力、润色文字、纠正语法错误。请提供建设性的反馈和具体的改进建议。"),
            ("study-buddy", "学习伙伴", "用简单易懂的方式解释复杂概念。", "你是一个耐心的学习伙伴，善于用简单易懂的方式解释复杂概念。你可以帮助用户理解各种学科知识、回答疑问、提供学习建议。请用循序渐进的方式进行讲解，确保用户能够跟上。"),
            ("creative-advisor", "创意顾问", "帮助用户产生新的想法，进行头脑风暴。", "你是一个富有创意的顾问，擅长发散思维和创新思考。你可以帮助用户产生新的想法、解决创意问题、进行头脑风暴、提供不同角度的思考方式。请保持开放的心态，鼓励创新和实验。"),
            ("translation-expert", "翻译专家", "提供精准、地道的翻译。", "你是一个专业的翻译专家，精通多种语言之间的准确翻译。你不仅能够进行字面翻译，还能考虑文化背景、语境和表达习惯，提供自然流畅的翻译结果。请确保翻译的准确性和地道性。"),
            // 安全相关角色
            ("security-analyst", "安全分析师", "分析安全漏洞和威胁，提供专业建议。", "你是一位经验丰富的网络安全分析师，专长于识别、分析和评估安全漏洞与威胁。你熟悉各类安全标准、最佳实践和常见攻击手段。在回答问题时，请提供深入的技术分析，同时确保建议符合行业最佳实践。你应该帮助用户理解风险的严重性和潜在影响，并提供切实可行的缓解策略。"),
            ("penetration-tester", "渗透测试专家", "模拟黑客攻击，发现系统安全漏洞。", "你是一位资深的渗透测试专家，擅长通过模拟黑客攻击来发现系统安全漏洞。你熟悉各种渗透测试方法、工具和技术，包括但不限于OWASP Top 10漏洞、网络扫描、社会工程学等。在回答问题时，请提供专业的技术建议，同时强调道德黑客的原则和法律边界。你应该帮助用户理解如何进行负责任的安全测试，并提供有关如何修复发现的漏洞的建议。"),
            ("malware-analyst", "恶意软件分析师", "分析和解释恶意软件的行为和特征。", "你是一位专业的恶意软件分析师，擅长分析和解释各种恶意软件的行为和特征。你熟悉静态和动态分析技术、逆向工程、沙箱环境和各种恶意软件家族的特征。在回答问题时，请提供详细的技术分析，解释恶意软件的工作原理、感染途径、影响范围和防御方法。你应该帮助用户理解恶意软件的危害性，并提供有关如何检测和移除恶意软件的建议。"),
            ("incident-responder", "安全事件响应专家", "处理安全事件和数据泄露，提供应急响应建议。", "你是一位经验丰富的安全事件响应专家，擅长处理各类安全事件和数据泄露。你熟悉事件响应流程、取证技术、威胁情报分析和灾难恢复策略。在回答问题时，请提供冷静、有条理的建议，帮助用户在安全事件发生时采取正确的步骤。你应该强调证据保全、根本原因分析和有效沟通的重要性，并提供有关如何防止类似事件再次发生的建议。"),
            ("compliance-advisor", "合规顾问", "提供安全合规和法规遵从方面的建议。", "你是一位专业的安全合规顾问，熟悉各种安全标准、法规和框架，如GDPR、HIPAA、PCI DSS、ISO 27001等。你了解如何将这些要求转化为实际的安全控制和流程。在回答问题时，请提供清晰、准确的合规建议，解释相关法规的要求和实施策略。你应该帮助用户理解合规的重要性，并提供有关如何建立和维护有效的合规计划的建议。"),
            ("secure-coder", "安全编码专家", "提供安全编码实践和代码审计建议。", "你是一位安全编码专家，精通各种编程语言的安全最佳实践和常见漏洞。你熟悉OWASP安全编码标准、静态和动态代码分析技术，以及安全开发生命周期。在回答问题时，请提供具体的代码示例和安全编码建议，帮助用户编写更安全的代码。你应该强调安全性和功能性的平衡，并提供有关如何在开发过程中集成安全实践的建议。"),
        ];

        for (id, title, description, prompt) in roles {
            let now = Utc::now();
            sqlx::query(
                "INSERT OR IGNORE INTO ai_roles (id, title, description, prompt, is_system, created_at, updated_at) VALUES (?, ?, ?, ?, 1, ?, ?)",
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

#[async_trait]
impl Database for DatabaseService {
    async fn create_ai_conversation(&self, conversation: &AiConversation) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"
            INSERT INTO ai_conversations (id, title, service_name, model_name, model_provider, context_type, project_id, vulnerability_id, scan_task_id, conversation_data, summary, total_messages, total_tokens, cost, tags, is_archived, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&conversation.id)
        .bind(&conversation.title)
        .bind(&conversation.service_name)
        .bind(&conversation.model_name)
        .bind(&conversation.model_provider)
        .bind(&conversation.context_type)
        .bind(&conversation.project_id)
        .bind(&conversation.vulnerability_id)
        .bind(&conversation.scan_task_id)
        .bind(&conversation.conversation_data)
        .bind(&conversation.summary)
        .bind(conversation.total_messages)
        .bind(conversation.total_tokens)
        .bind(conversation.cost)
        .bind(serde_json::to_string(&conversation.tags).unwrap_or_default())
        .bind(conversation.is_archived)
        .bind(conversation.created_at)
        .bind(conversation.updated_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn get_ai_conversations(&self) -> Result<Vec<AiConversation>> {
        let pool = self.get_pool()?;
        let conversations = sqlx::query_as::<_, AiConversation>(
            "SELECT * FROM ai_conversations ORDER BY updated_at DESC",
        )
        .fetch_all(pool)
        .await?;
        Ok(conversations)
    }

    async fn get_ai_conversation(&self, id: &str) -> Result<Option<AiConversation>> {
        let pool = self.get_pool()?;
        let conversation =
            sqlx::query_as::<_, AiConversation>("SELECT * FROM ai_conversations WHERE id = ?")
                .bind(id)
                .fetch_optional(pool)
                .await?;
        Ok(conversation)
    }

    async fn update_ai_conversation(&self, conversation: &AiConversation) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"
            UPDATE ai_conversations
            SET title = ?, service_name = ?, model_name = ?, model_provider = ?, context_type = ?, project_id = ?, vulnerability_id = ?, scan_task_id = ?, conversation_data = ?, summary = ?, total_messages = ?, total_tokens = ?, cost = ?, tags = ?, is_archived = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&conversation.title)
        .bind(&conversation.service_name)
        .bind(&conversation.model_name)
        .bind(&conversation.model_provider)
        .bind(&conversation.context_type)
        .bind(&conversation.project_id)
        .bind(&conversation.vulnerability_id)
        .bind(&conversation.scan_task_id)
        .bind(&conversation.conversation_data)
        .bind(&conversation.summary)
        .bind(conversation.total_messages)
        .bind(conversation.total_tokens)
        .bind(conversation.cost)
        .bind(serde_json::to_string(&conversation.tags).unwrap_or_default())
        .bind(conversation.is_archived)
        .bind(Utc::now())
        .bind(&conversation.id)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn delete_ai_conversation(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        let mut tx = pool.begin().await?;
        // First, delete related messages
        sqlx::query("DELETE FROM ai_messages WHERE conversation_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        // Then, delete the conversation
        sqlx::query("DELETE FROM ai_conversations WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn update_ai_conversation_title(&self, id: &str, title: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE ai_conversations SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn archive_ai_conversation(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE ai_conversations SET is_archived = 1, updated_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn create_ai_message(&self, message: &AiMessage) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "INSERT INTO ai_messages (id, conversation_id, role, content, metadata, token_count, cost, tool_calls, attachments, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&message.id)
        .bind(&message.conversation_id)
        .bind(&message.role)
        .bind(&message.content)
        .bind(&message.metadata)
        .bind(message.token_count)
        .bind(message.cost)
        .bind(&message.tool_calls)
        .bind(&message.attachments)
        .bind(message.timestamp)
        .execute(pool)
        .await?;

        // 更新对话的updated_at和消息计数
        sqlx::query("UPDATE ai_conversations SET updated_at = ?, total_messages = total_messages + 1 WHERE id = ?")
            .bind(Utc::now())
            .bind(&message.conversation_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    async fn get_ai_messages_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<AiMessage>> {
        let pool = self.get_pool()?;
        let messages = sqlx::query_as::<_, AiMessage>(
            "SELECT * FROM ai_messages WHERE conversation_id = ? ORDER BY timestamp ASC",
        )
        .bind(conversation_id)
        .fetch_all(pool)
        .await?;
        Ok(messages)
    }

    async fn get_configs_by_category(&self, category: &str) -> Result<Vec<Configuration>> {
        let pool = self.get_pool()?;
        let configs =
            sqlx::query_as::<_, Configuration>("SELECT * FROM configurations WHERE category = ?")
                .bind(category)
                .fetch_all(pool)
                .await?;
        Ok(configs)
    }

    async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        let result: Option<(String,)> = sqlx::query_as::<_, (String,)>(
            "SELECT value FROM configurations WHERE category = ? AND key = ?",
        )
        .bind(category)
        .bind(key)
        .fetch_optional(pool)
        .await?;
        Ok(result.map(|(value,)| value))
    }

    async fn set_config(
        &self,
        category: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "INSERT INTO configurations (category, key, value, description) VALUES (?, ?, ?, ?)
             ON CONFLICT(category, key) DO UPDATE SET value = excluded.value, description = excluded.description"
        )
        .bind(category)
        .bind(key)
        .bind(value)
        .bind(description)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn get_ai_roles(&self) -> Result<Vec<AiRole>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT id, title, description, prompt, is_system, created_at, updated_at FROM ai_roles ORDER BY created_at DESC")
            .fetch_all(pool)
            .await?;

        let mut roles = Vec::with_capacity(rows.len());
        for row in rows {
            roles.push(AiRole {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                prompt: row.get("prompt"),
                is_system: row.get("is_system"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(roles)
    }

    async fn create_ai_role(&self, role: &AiRole) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("INSERT INTO ai_roles (id, title, description, prompt, is_system, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(&role.id)
            .bind(&role.title)
            .bind(&role.description)
            .bind(&role.prompt)
            .bind(role.is_system)
            .bind(role.created_at)
            .bind(role.updated_at)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn update_ai_role(&self, role: &AiRole) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE ai_roles SET title = ?, description = ?, prompt = ?, updated_at = ? WHERE id = ?")
            .bind(&role.title)
            .bind(&role.description)
            .bind(&role.prompt)
            .bind(Utc::now())
            .bind(&role.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn delete_ai_role(&self, role_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        tracing::info!("Executing SQL to delete role with ID: {}", role_id);

        let result = sqlx::query("DELETE FROM ai_roles WHERE id = ?")
            .bind(role_id)
            .execute(pool)
            .await?;

        let rows_affected = result.rows_affected();
        tracing::info!(
            "Delete operation affected {} rows for role ID: {}",
            rows_affected,
            role_id
        );

        if rows_affected == 0 {
            tracing::warn!(
                "No rows were affected when deleting role ID: {}. Role might not exist.",
                role_id
            );
        }

        Ok(())
    }
}

impl Default for DatabaseService {
    fn default() -> Self {
        Self::new()
    }
}
