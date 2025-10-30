use anyhow::Result;
use async_trait::async_trait;
use bincode;
use chrono::Utc;
use serde_json::Value;
use sqlx::{sqlite::SqlitePool, Column, Row, TypeInfo};
use tracing::info;
use std::path::PathBuf;

use crate::models::ai::AiRole;
use crate::models::database::{
    AiConversation, AiMessage, Configuration, DatabaseStats, McpServerConfig,
    ScanTask, Vulnerability,
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
    async fn set_current_ai_role(&self, role_id: Option<&str>) -> Result<()>;
    async fn get_current_ai_role(&self) -> Result<Option<AiRole>>;
    
    // 扫描任务相关方法
    async fn create_scan_task(&self, task: &ScanTask) -> Result<()>;
    async fn get_scan_tasks(&self, project_id: Option<&str>) -> Result<Vec<ScanTask>>;
    async fn get_scan_task(&self, id: &str) -> Result<Option<ScanTask>>;
    async fn get_scan_tasks_by_target(&self, target: &str) -> Result<Vec<ScanTask>>;
    async fn update_scan_task_status(&self, id: &str, status: &str, progress: Option<f64>) -> Result<()>;
    
    // Agent任务相关方法
    async fn create_agent_task(&self, task: &crate::agents::traits::AgentTask) -> Result<()>;
    async fn get_agent_task(&self, id: &str) -> Result<Option<crate::agents::traits::AgentTask>>;
    async fn get_agent_tasks(&self, user_id: Option<&str>) -> Result<Vec<crate::agents::traits::AgentTask>>;
    async fn update_agent_task_status(&self, id: &str, status: &str, agent_name: Option<&str>, architecture: Option<&str>) -> Result<()>;
    async fn update_agent_task_timing(&self, id: &str, started_at: Option<chrono::DateTime<chrono::Utc>>, completed_at: Option<chrono::DateTime<chrono::Utc>>, execution_time_ms: Option<u64>) -> Result<()>;
    async fn update_agent_task_error(&self, id: &str, error_message: &str) -> Result<()>;
    
    // Agent会话相关方法
    async fn create_agent_session(&self, session_id: &str, task_id: &str, agent_name: &str) -> Result<()>;
    async fn update_agent_session_status(&self, session_id: &str, status: &str) -> Result<()>;
    async fn get_agent_session(&self, session_id: &str) -> Result<Option<crate::agents::session::AgentSessionData>>;
    async fn list_agent_sessions(&self) -> Result<Vec<crate::agents::session::AgentSessionData>>;
    async fn delete_agent_session(&self, session_id: &str) -> Result<()>;
    async fn delete_agent_execution_steps(&self, session_id: &str) -> Result<()>;
    
    // Agent执行日志相关方法
    async fn add_agent_session_log(&self, session_id: &str, level: &str, message: &str, source: &str) -> Result<()>;
    async fn get_agent_session_logs(&self, session_id: &str) -> Result<Vec<crate::agents::traits::SessionLog>>;
    
    // Agent执行结果相关方法
    async fn save_agent_execution_result(&self, session_id: &str, result: &crate::agents::traits::AgentExecutionResult) -> Result<()>;
    async fn get_agent_execution_result(&self, session_id: &str) -> Result<Option<crate::agents::traits::AgentExecutionResult>>;
    
    // Agent执行步骤相关方法
    async fn save_agent_execution_step(&self, session_id: &str, step: &crate::commands::agent_commands::WorkflowStepDetail) -> Result<()>;
    async fn get_agent_execution_steps(&self, session_id: &str) -> Result<Vec<crate::commands::agent_commands::WorkflowStepDetail>>;
    async fn update_agent_execution_step_status(&self, step_id: &str, status: &str, started_at: Option<chrono::DateTime<chrono::Utc>>, completed_at: Option<chrono::DateTime<chrono::Utc>>, duration_ms: Option<u64>, error_message: Option<&str>) -> Result<()>;
}

#[derive(Debug)]
/// 数据库服务
pub struct DatabaseService {
    pool: Option<SqlitePool>,
    db_path: PathBuf,
    db_client: Option<sentinel_db::DatabaseClient>,
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
            db_client: None,
        }
    }

    /// 初始化数据库连接和架构
    pub async fn initialize(&mut self) -> Result<()> {
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

        // 如果数据库文件不存在，先创建一个空文件
        if !db_exists {
            tracing::info!("Creating database file: {}", self.db_path.display());
            if let Err(e) = std::fs::File::create(&self.db_path) {
                tracing::error!("Failed to create database file: {}", e);
                return Err(anyhow::anyhow!("创建数据库文件失败: {}", e));
            }
        }

        // 创建连接池，使用更安全的连接选项
        let _database_url = format!("sqlite:{}?mode=rwc", self.db_path.display());

        let pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&self.db_path)
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Delete)
                .synchronous(sqlx::sqlite::SqliteSynchronous::Normal),
        )
        .await
        .map_err(|e| anyhow::anyhow!("创建数据库连接池失败: {}", e))?;

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
            self.insert_default_prompts(&pool).await?;
        } 

        self.pool = Some(pool);
        // 初始化数据库客户端
        if let Some(p) = &self.pool {
            self.db_client = Some(sentinel_db::DatabaseClient::new(p.clone()));
        }

        Ok(())
    }

    /// 创建数据库表结构
    async fn create_database_schema(&self, pool: &SqlitePool) -> Result<()> {

        // 使用事务来确保所有表创建成功或全部回滚
        let mut tx = pool.begin().await?;

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
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 提示词模板表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS prompt_templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                architecture TEXT NOT NULL,
                stage TEXT NOT NULL,
                content TEXT NOT NULL,
                is_default INTEGER DEFAULT 0,
                is_active INTEGER DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 用户提示词配置表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_prompt_configs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                architecture TEXT NOT NULL,
                stage TEXT NOT NULL,
                template_id INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(architecture, stage),
                FOREIGN KEY (template_id) REFERENCES prompt_templates(id)
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 提示词分组表（每个架构仅允许一个默认组）
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS prompt_groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                architecture TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                is_default INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 唯一索引：同一架构仅一个默认组
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_prompt_groups_arch_default \
             ON prompt_groups(architecture) \
             WHERE is_default = 1",
        )
        .execute(&mut *tx)
        .await?;

        // 分组阶段-模板映射表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS prompt_group_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER NOT NULL,
                stage TEXT NOT NULL,
                template_id INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(group_id, stage),
                FOREIGN KEY(group_id) REFERENCES prompt_groups(id) ON DELETE CASCADE,
                FOREIGN KEY(template_id) REFERENCES prompt_templates(id) ON DELETE CASCADE
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 提示词历史版本表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS prompt_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                template_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                version INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (template_id) REFERENCES prompt_templates(id)
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建扫描会话表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS scan_sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                target TEXT NOT NULL,
                scan_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                config TEXT NOT NULL DEFAULT '{}',
                progress REAL NOT NULL DEFAULT 0.0,
                current_stage TEXT,
                total_stages INTEGER NOT NULL DEFAULT 0,
                completed_stages INTEGER NOT NULL DEFAULT 0,
                results_summary TEXT,
                error_message TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                started_at DATETIME,
                completed_at DATETIME,
                created_by TEXT NOT NULL
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建扫描阶段表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS scan_stages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                stage_name TEXT NOT NULL,
                stage_order INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                tool_name TEXT NOT NULL,
                config TEXT NOT NULL DEFAULT '{}',
                results TEXT,
                error_message TEXT,
                started_at DATETIME,
                completed_at DATETIME,
                duration_ms INTEGER,
                FOREIGN KEY (session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 索引优化（提示词相关）
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_prompt_templates_arch_stage ON prompt_templates(architecture, stage)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_prompt_templates_is_active ON prompt_templates(is_active)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_prompt_configs_arch_stage ON user_prompt_configs(architecture, stage)")
            .execute(&mut *tx)
            .await?;

        // 创建资产表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS assets (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                asset_type TEXT NOT NULL,
                name TEXT NOT NULL,
                value TEXT NOT NULL,
                description TEXT,
                confidence REAL NOT NULL DEFAULT 1.0,
                status TEXT NOT NULL DEFAULT 'active',
                source TEXT,
                source_scan_id TEXT,
                metadata TEXT,
                tags TEXT,
                risk_level TEXT NOT NULL DEFAULT 'unknown',
                first_seen DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_seen DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                created_by TEXT NOT NULL DEFAULT 'system',
                FOREIGN KEY (project_id) REFERENCES scan_tasks(id) ON DELETE SET NULL
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
                resolution_date DATETIME,
                tags TEXT,
                attachments TEXT,
                notes TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE SET NULL,
                FOREIGN KEY (scan_task_id) REFERENCES scan_tasks(id) ON DELETE SET NULL
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

        // 创建内置工具设置表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS builtin_tool_settings (
                tool_name TEXT PRIMARY KEY,
                enabled BOOLEAN DEFAULT 1,
                updated_at INTEGER NOT NULL
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

        // 创建Agent任务表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_tasks (
                id TEXT PRIMARY KEY,
                description TEXT NOT NULL,
                target TEXT,
                parameters TEXT NOT NULL DEFAULT '{}',
                user_id TEXT NOT NULL,
                priority TEXT NOT NULL DEFAULT 'Normal',
                timeout_seconds INTEGER,
                status TEXT NOT NULL DEFAULT 'Created',
                agent_name TEXT,
                architecture TEXT,
                session_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                started_at DATETIME,
                completed_at DATETIME,
                execution_time_ms INTEGER,
                error_message TEXT
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建Agent会话表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_sessions (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Created',
                agent_name TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (task_id) REFERENCES agent_tasks(id) ON DELETE CASCADE
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建Agent执行日志表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_session_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                level TEXT NOT NULL,
                message TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'agent_session',
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建Agent执行结果表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_execution_results (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                success BOOLEAN NOT NULL,
                data TEXT,
                error_message TEXT,
                execution_time_ms INTEGER NOT NULL,
                resources_used TEXT DEFAULT '{}',
                artifacts TEXT DEFAULT '[]',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
            )",
        )
        .execute(&mut *tx)
        .await?;

        // 创建Agent执行步骤表（用于详细的步骤跟踪）
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_execution_steps (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                step_name TEXT NOT NULL,
                step_order INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'Pending',
                started_at DATETIME,
                completed_at DATETIME,
                duration_ms INTEGER,
                result_data TEXT,
                error_message TEXT,
                retry_count INTEGER DEFAULT 0,
                dependencies TEXT DEFAULT '[]',
                tool_result TEXT,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
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

        // Agent任务索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_tasks_user_id ON agent_tasks(user_id)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_tasks_status ON agent_tasks(status)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_tasks_created_at ON agent_tasks(created_at)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_tasks_agent_name ON agent_tasks(agent_name)")
            .execute(&mut *tx)
            .await?;

        // Agent会话索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_sessions_task_id ON agent_sessions(task_id)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_sessions_status ON agent_sessions(status)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_sessions_agent_name ON agent_sessions(agent_name)")
            .execute(&mut *tx)
            .await?;

        // Agent会话日志索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_session_logs_session_id ON agent_session_logs(session_id)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_session_logs_timestamp ON agent_session_logs(timestamp)")
            .execute(&mut *tx)
            .await?;

        // Agent执行结果索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_execution_results_session_id ON agent_execution_results(session_id)")
            .execute(&mut *tx)
            .await?;

        // Agent执行步骤索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_execution_steps_session_id ON agent_execution_steps(session_id)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_execution_steps_order ON agent_execution_steps(session_id, step_order)")
            .execute(&mut *tx)
            .await?;

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

        // 扫描会话索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_scan_sessions_status ON scan_sessions(status)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_scan_sessions_created_at ON scan_sessions(created_at)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_scan_sessions_created_by ON scan_sessions(created_by)")
            .execute(&mut *tx)
            .await?;

        // 扫描阶段索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_scan_stages_session_id ON scan_stages(session_id)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_scan_stages_status ON scan_stages(status)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_scan_stages_order ON scan_stages(session_id, stage_order)")
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

        // 创建RAG集合表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rag_collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                is_active INTEGER NOT NULL DEFAULT 0,
                document_count INTEGER DEFAULT 0,
                chunk_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        // 迁移：为旧表添加 is_active 列
        // SQLite 不支持 IF NOT EXISTS 列级检查，这里通过 PRAGMA table_info 检查
        let pragma_rows = sqlx::query("PRAGMA table_info(rag_collections)")
            .fetch_all(&mut *tx)
            .await?;
        let has_is_active = pragma_rows
            .iter()
            .any(|row| {
                let name: String = row.get("name");
                name == "is_active"
            });
        if !has_is_active {
            let _ = sqlx::query("ALTER TABLE rag_collections ADD COLUMN is_active INTEGER NOT NULL DEFAULT 0")
                .execute(&mut *tx)
                .await;
        }

        // 创建RAG文档源表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rag_document_sources (
                id TEXT PRIMARY KEY,
                collection_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_name TEXT NOT NULL,
                file_type TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                file_hash TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                chunk_count INTEGER DEFAULT 0,
                ingestion_status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                metadata TEXT,
                FOREIGN KEY (collection_id) REFERENCES rag_collections(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        // 创建RAG文档块表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rag_document_chunks (
                id TEXT PRIMARY KEY,
                collection_id TEXT NOT NULL,
                source_id TEXT NOT NULL,
                content TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                embedding BLOB,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (collection_id) REFERENCES rag_collections(id) ON DELETE CASCADE,
                FOREIGN KEY (source_id) REFERENCES rag_document_sources(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        // 创建RAG查询历史表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rag_query_history (
                id TEXT PRIMARY KEY,
                collection_id TEXT,
                query TEXT NOT NULL,
                response TEXT NOT NULL,
                processing_time_ms INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (collection_id) REFERENCES rag_collections(id) ON DELETE SET NULL
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        // 创建新的RAG表结构
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rag_chunks (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                collection_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                token_count INTEGER,
                char_count INTEGER NOT NULL,
                start_position INTEGER,
                end_position INTEGER,
                page_number INTEGER,
                section_title TEXT,
                embedding_vector BLOB,
                embedding_model TEXT NOT NULL,
                embedding_dimension INTEGER NOT NULL,
                similarity_threshold REAL DEFAULT 0.7,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                FOREIGN KEY (document_id) REFERENCES rag_document_sources(id) ON DELETE CASCADE,
                FOREIGN KEY (collection_id) REFERENCES rag_collections(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        // 创建RAG表索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_collections_name ON rag_collections(name)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_document_sources_collection_id ON rag_document_sources(collection_id)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_document_sources_file_hash ON rag_document_sources(file_hash)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_document_chunks_collection_id ON rag_document_chunks(collection_id)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_document_chunks_source_id ON rag_document_chunks(source_id)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_document_chunks_content_hash ON rag_document_chunks(content_hash)").execute(&mut *tx).await?;
        
        // 新RAG表的索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_chunks_document_id ON rag_chunks(document_id)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_chunks_collection_id ON rag_chunks(collection_id)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_chunks_content_hash ON rag_chunks(content_hash)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_chunks_chunk_index ON rag_chunks(chunk_index)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_chunks_embedding_model ON rag_chunks(embedding_model)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_rag_query_history_collection_id ON rag_query_history(collection_id)").execute(&mut *tx).await?;

        // 创建scenario_agents表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS scenario_agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                enabled BOOLEAN DEFAULT 1,
                profile_json TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"
        )
        .execute(&mut *tx)
        .await?;

        // scenario_agents表的索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_scenario_agents_name ON scenario_agents(name)")
            .execute(&mut *tx)
            .await?;

        // 提交事务
        tx.commit().await?;

        Ok(())
    }

    async fn insert_default_prompts(&self, _pool: &SqlitePool) -> Result<()> {
        // Use centralized DB client
        let db = self.get_db()?;
        db.insert_default_templates().await
    }

    /// 获取数据库连接池
    pub fn get_pool(&self) -> Result<&SqlitePool> {
        self.pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))
    }

    /// 获取数据库客户端
    pub fn get_db(&self) -> Result<&sentinel_db::DatabaseClient> {
        self.db_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库客户端未初始化"))
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



        let scan_tasks_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scan_tasks")
            .fetch_one(pool)
            .await?;

        let vulnerabilities_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vulnerabilities")
            .fetch_one(pool)
            .await?;

        let assets_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets")
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
            scan_tasks_count: scan_tasks_count as f64,
            vulnerabilities_count: vulnerabilities_count as f64,
            assets_count: assets_count as f64,
            conversations_count: conversations_count as f64,
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



    /// 扫描任务相关操作
    pub async fn create_scan_task(&self, task: &ScanTask) -> Result<()> {
        let pool = self.get_pool()?;
        let t = sentinel_core::models::database::ScanTask {
            id: task.id.clone(),
            project_id: task.project_id.clone(),
            name: task.name.clone(),
            description: task.description.clone(),
            target_type: task.target_type.clone(),
            targets: task.targets.clone(),
            scan_type: task.scan_type.clone(),
            tools_config: task.tools_config.clone(),
            status: task.status.clone(),
            progress: task.progress,
            priority: task.priority,
            scheduled_at: task.scheduled_at,
            started_at: task.started_at,
            completed_at: task.completed_at,
            execution_time: task.execution_time,
            results_summary: task.results_summary.clone(),
            error_message: task.error_message.clone(),
            created_by: task.created_by.clone(),
            created_at: task.created_at,
            updated_at: task.updated_at,
        };
        self.get_db()?.create_scan_task(&t).await
    }

    pub async fn get_scan_tasks(&self, project_id: Option<&str>) -> Result<Vec<ScanTask>> {
        let pool = self.get_pool()?;
        let rows = self.get_db()?.get_scan_tasks(project_id).await?;
        let mapped = rows
            .into_iter()
            .map(|t| crate::models::database::ScanTask {
                id: t.id,
                project_id: t.project_id,
                name: t.name,
                description: t.description,
                target_type: t.target_type,
                targets: t.targets,
                scan_type: t.scan_type,
                tools_config: t.tools_config,
                status: t.status,
                progress: t.progress,
                priority: t.priority,
                scheduled_at: t.scheduled_at,
                started_at: t.started_at,
                completed_at: t.completed_at,
                execution_time: t.execution_time,
                results_summary: t.results_summary,
                error_message: t.error_message,
                created_by: t.created_by,
                created_at: t.created_at,
                updated_at: t.updated_at,
            })
            .collect();
        Ok(mapped)
    }

    pub async fn get_scan_task(&self, id: &str) -> Result<Option<ScanTask>> {
        let pool = self.get_pool()?;
        let row = self.get_db()?.get_scan_task(id).await?;
        Ok(row.map(|t| crate::models::database::ScanTask {
            id: t.id,
            project_id: t.project_id,
            name: t.name,
            description: t.description,
            target_type: t.target_type,
            targets: t.targets,
            scan_type: t.scan_type,
            tools_config: t.tools_config,
            status: t.status,
            progress: t.progress,
            priority: t.priority,
            scheduled_at: t.scheduled_at,
            started_at: t.started_at,
            completed_at: t.completed_at,
            execution_time: t.execution_time,
            results_summary: t.results_summary,
            error_message: t.error_message,
            created_by: t.created_by,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }))
    }

    pub async fn update_scan_task_status(
        &self,
        id: &str,
        status: &str,
        progress: Option<f64>,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        self.get_db()?.update_scan_task_status(id, status, progress).await
    }

    /// 漏洞相关操作
    pub async fn create_vulnerability(&self, vuln: &Vulnerability) -> Result<()> {
        let pool = self.get_pool()?;
        let v = sentinel_core::models::database::Vulnerability {
            id: vuln.id.clone(),
            project_id: vuln.project_id.clone(),
            asset_id: vuln.asset_id.clone(),
            scan_task_id: vuln.scan_task_id.clone(),
            title: vuln.title.clone(),
            description: vuln.description.clone(),
            vulnerability_type: vuln.vulnerability_type.clone(),
            severity: vuln.severity.clone(),
            cvss_score: vuln.cvss_score,
            cvss_vector: vuln.cvss_vector.clone(),
            cwe_id: vuln.cwe_id.clone(),
            owasp_category: vuln.owasp_category.clone(),
            proof_of_concept: vuln.proof_of_concept.clone(),
            impact: vuln.impact.clone(),
            remediation: vuln.remediation.clone(),
            references: vuln.references.clone(),
            status: vuln.status.clone(),
            verification_status: vuln.verification_status.clone(),
            resolution_date: vuln.resolution_date,
            tags: vuln.tags.clone(),
            attachments: vuln.attachments.clone(),
            notes: vuln.notes.clone(),
            created_at: vuln.created_at,
            updated_at: vuln.updated_at,
        };
        self.get_db()?.create_vulnerability(&v).await
    }

    pub async fn get_vulnerabilities(
        &self,
        project_id: Option<&str>,
    ) -> Result<Vec<Vulnerability>> {
        let pool = self.get_pool()?;
        let rows = self.get_db()?.get_vulnerabilities(project_id).await?;
        let mapped = rows
            .into_iter()
            .map(|v| crate::models::database::Vulnerability {
                id: v.id,
                project_id: v.project_id,
                asset_id: v.asset_id,
                scan_task_id: v.scan_task_id,
                title: v.title,
                description: v.description,
                vulnerability_type: v.vulnerability_type,
                severity: v.severity,
                cvss_score: v.cvss_score,
                cvss_vector: v.cvss_vector,
                cwe_id: v.cwe_id,
                owasp_category: v.owasp_category,
                proof_of_concept: v.proof_of_concept,
                impact: v.impact,
                remediation: v.remediation,
                references: v.references,
                status: v.status,
                verification_status: v.verification_status,
                resolution_date: v.resolution_date,
                tags: v.tags,
                attachments: v.attachments,
                notes: v.notes,
                created_at: v.created_at,
                updated_at: v.updated_at,
            })
            .collect();
        Ok(mapped)
    }

    pub async fn get_vulnerability(&self, id: &str) -> Result<Option<Vulnerability>> {
        let pool = self.get_pool()?;
        let row = self.get_db()?.get_vulnerability(id).await?;
        Ok(row.map(|v| crate::models::database::Vulnerability {
            id: v.id,
            project_id: v.project_id,
            asset_id: v.asset_id,
            scan_task_id: v.scan_task_id,
            title: v.title,
            description: v.description,
            vulnerability_type: v.vulnerability_type,
            severity: v.severity,
            cvss_score: v.cvss_score,
            cvss_vector: v.cvss_vector,
            cwe_id: v.cwe_id,
            owasp_category: v.owasp_category,
            proof_of_concept: v.proof_of_concept,
            impact: v.impact,
            remediation: v.remediation,
            references: v.references,
            status: v.status,
            verification_status: v.verification_status,
            resolution_date: v.resolution_date,
            tags: v.tags,
            attachments: v.attachments,
            notes: v.notes,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }))
    }

    pub async fn update_vulnerability_status(&self, id: &str, status: &str) -> Result<()> {
        let pool = self.get_pool()?;
        self.get_db()?.update_vulnerability_status(id, status).await
    }

    /// AI对话相关操作
    pub async fn create_conversation(&self, conversation: &AiConversation) -> Result<()> {
        let pool = self.get_pool()?;
        let c = sentinel_core::models::database::AiConversation {
            id: conversation.id.clone(),
            title: conversation.title.clone(),
            service_name: conversation.service_name.clone(),
            model_name: conversation.model_name.clone(),
            model_provider: conversation.model_provider.clone(),
            context_type: conversation.context_type.clone(),
            project_id: conversation.project_id.clone(),
            vulnerability_id: conversation.vulnerability_id.clone(),
            scan_task_id: conversation.scan_task_id.clone(),
            conversation_data: conversation.conversation_data.clone(),
            summary: conversation.summary.clone(),
            total_messages: conversation.total_messages,
            total_tokens: conversation.total_tokens,
            cost: conversation.cost,
            tags: conversation.tags.clone(),
            is_archived: conversation.is_archived,
            created_at: conversation.created_at,
            updated_at: conversation.updated_at,
        };
        self.get_db()?.create_ai_conversation(&c).await
    }

    pub async fn get_conversations(&self) -> Result<Vec<AiConversation>> {
        let pool = self.get_pool()?;
        let rows = self.get_db()?.get_ai_conversations().await?;
        let mapped = rows
            .into_iter()
            .map(|c| crate::models::database::AiConversation {
                id: c.id,
                title: c.title,
                service_name: c.service_name,
                model_name: c.model_name,
                model_provider: c.model_provider,
                context_type: c.context_type,
                project_id: c.project_id,
                vulnerability_id: c.vulnerability_id,
                scan_task_id: c.scan_task_id,
                conversation_data: c.conversation_data,
                summary: c.summary,
                total_messages: c.total_messages,
                total_tokens: c.total_tokens,
                cost: c.cost,
                tags: c.tags,
                is_archived: c.is_archived,
                created_at: c.created_at,
                updated_at: c.updated_at,
            })
            .collect();
        Ok(mapped)
    }

    pub async fn get_conversation(&self, id: &str) -> Result<Option<AiConversation>> {
        let pool = self.get_pool()?;
        let row = self.get_db()?.get_ai_conversation(id).await?;
        Ok(row.map(|c| crate::models::database::AiConversation {
            id: c.id,
            title: c.title,
            service_name: c.service_name,
            model_name: c.model_name,
            model_provider: c.model_provider,
            context_type: c.context_type,
            project_id: c.project_id,
            vulnerability_id: c.vulnerability_id,
            scan_task_id: c.scan_task_id,
            conversation_data: c.conversation_data,
            summary: c.summary,
            total_messages: c.total_messages,
            total_tokens: c.total_tokens,
            cost: c.cost,
            tags: c.tags,
            is_archived: c.is_archived,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }))
    }

    pub async fn update_conversation(&self, conversation: &AiConversation) -> Result<()> {
        let pool = self.get_pool()?;
        let c = sentinel_core::models::database::AiConversation {
            id: conversation.id.clone(),
            title: conversation.title.clone(),
            service_name: conversation.service_name.clone(),
            model_name: conversation.model_name.clone(),
            model_provider: conversation.model_provider.clone(),
            context_type: conversation.context_type.clone(),
            project_id: conversation.project_id.clone(),
            vulnerability_id: conversation.vulnerability_id.clone(),
            scan_task_id: conversation.scan_task_id.clone(),
            conversation_data: conversation.conversation_data.clone(),
            summary: conversation.summary.clone(),
            total_messages: conversation.total_messages,
            total_tokens: conversation.total_tokens,
            cost: conversation.cost,
            tags: conversation.tags.clone(),
            is_archived: conversation.is_archived,
            created_at: conversation.created_at,
            updated_at: Utc::now(),
        };
        self.get_db()?.update_ai_conversation(&c).await
    }

    pub async fn delete_conversation(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        self.get_db()?.delete_ai_conversation(id).await
    }

    /// 更新对话标题
    pub async fn update_conversation_title(&self, id: &str, title: &str) -> Result<()> {
        let pool = self.get_pool()?;
        self.get_db()?.update_conversation_title(id, title).await
    }

    /// 创建AI消息
    pub async fn create_message(&self, message: &AiMessage) -> Result<()> {
        let pool = self.get_pool()?;
        let m = sentinel_core::models::database::AiMessage {
            id: message.id.clone(),
            conversation_id: message.conversation_id.clone(),
            role: message.role.clone(),
            content: message.content.clone(),
            metadata: message.metadata.clone(),
            token_count: message.token_count,
            cost: message.cost,
            tool_calls: message.tool_calls.clone(),
            attachments: message.attachments.clone(),
            timestamp: message.timestamp,
        };
        self.get_db()?.create_ai_message(&m).await
    }

    /// 获取对话的消息列表
    pub async fn get_messages(&self, conversation_id: &str) -> Result<Vec<AiMessage>> {
        let pool = self.get_pool()?;
        let rows = self.get_db()?.get_ai_messages_by_conversation(conversation_id).await?;
        let mapped = rows
            .into_iter()
            .map(|m| crate::models::database::AiMessage {
                id: m.id,
                conversation_id: m.conversation_id,
                role: m.role,
                content: m.content,
                metadata: m.metadata,
                token_count: m.token_count,
                cost: m.cost,
                tool_calls: m.tool_calls,
                attachments: m.attachments,
                timestamp: m.timestamp,
            })
            .collect();
        Ok(mapped)
    }

    /// 配置相关操作
    pub async fn get_configs_by_category(&self, category: &str) -> Result<Vec<Configuration>> {
        let pool = self.get_pool()?;
        let rows = self.get_db()?.get_configs_by_category(category).await?;
        let mapped = rows
            .into_iter()
            .map(|c| crate::models::database::Configuration {
                id: c.id,
                category: c.category,
                key: c.key,
                value: c.value,
                description: c.description,
                is_encrypted: c.is_encrypted,
                created_at: c.created_at,
                updated_at: c.updated_at,
            })
            .collect();
        Ok(mapped)
    }

    pub async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        self.get_db()?.get_config(category, key).await
    }

    pub async fn set_config(
        &self,
        category: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        self.get_db()?.set_config(category, key, value, description).await
    }

    /// RAG配置管理方法
    pub async fn get_rag_config(&self) -> Result<Option<crate::rag::config::RagConfig>> {
        let config_json = self.get_config("rag", "config").await?;
        
        if let Some(json_str) = config_json {
            match serde_json::from_str::<crate::rag::config::RagConfig>(&json_str) {
                Ok(config) => Ok(Some(config)),
                Err(e) => {
                    log::warn!("Failed to parse RAG config from database: {}, using default", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    pub async fn save_rag_config(&self, config: &crate::rag::config::RagConfig) -> Result<()> {
        let config_json = serde_json::to_string(config)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize RAG config: {}", e)))?;
        
        self.set_config(
            "rag",
            "config", 
            &config_json,
            Some("RAG系统配置")
        ).await?;
        
        Ok(())
    }

    /// 子域名字典管理方法
    pub async fn get_subdomain_dictionary(&self) -> Result<Vec<String>> {
        let pool = self.get_pool()?;

        // 1) 优先使用"默认字典"配置：category='dictionary_default', key='subdomain'
        if let Some(default_dict_id) = self
            .get_config("dictionary_default", "subdomain")
            .await?
            .filter(|s| !s.is_empty())
        {
            let words: Vec<String> = sqlx::query_scalar(
                r#"SELECT word FROM dictionary_words 
                   WHERE dictionary_id = ? 
                   ORDER BY COALESCE(weight, 0) DESC, word ASC"#,
            )
            .bind(&default_dict_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();

            if !words.is_empty() {
                return Ok(words);
            }
        }

        // 2) 若未设置默认，或默认字典无词条，则选择一个可用的子域名字典（优先内置且启用的，按更新时间倒序）
        if let Some(candidate_id) = sqlx::query_scalar::<_, String>(
            r#"SELECT id FROM dictionaries 
               WHERE dict_type = 'subdomain' AND is_active = 1 
               ORDER BY is_builtin DESC, updated_at DESC 
               LIMIT 1"#,
        )
        .fetch_optional(pool)
        .await?
        {
            let words: Vec<String> = sqlx::query_scalar(
                r#"SELECT word FROM dictionary_words 
                   WHERE dictionary_id = ? 
                   ORDER BY COALESCE(weight, 0) DESC, word ASC"#,
            )
            .bind(&candidate_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();

            if !words.is_empty() {
                return Ok(words);
            }
        }

        // 3) 仍无可用词条时，退回内置静态默认列表
        Ok(self.get_default_subdomain_dictionary())
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

    /// 根据目标获取扫描任务
    pub async fn get_scan_tasks_by_target(&self, target: &str) -> Result<Vec<ScanTask>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query_as::<_, ScanTask>(
            "SELECT * FROM scan_tasks WHERE targets LIKE ? ORDER BY created_at DESC"
        )
        .bind(format!("%{}%", target))
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    /// 根据项目ID获取漏洞
    pub async fn get_vulnerabilities_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<Vulnerability>> {
        self.get_vulnerabilities(Some(project_id)).await
    }

    /// 根据项目ID获取提交记录


    // --- MCP Server Configs ---

    pub async fn create_mcp_server_config(
        &self,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
    ) -> Result<String> {
        let pool = self.get_pool()?;
        self.get_db()?.create_mcp_server_config(name, description, command, args).await
    }

    pub async fn get_all_mcp_server_configs(&self) -> Result<Vec<McpServerConfig>> {
        let pool = self.get_pool()?;
        let rows = self.get_db()?.get_all_mcp_server_configs().await?;
        let mapped = rows
            .into_iter()
            .map(|m| McpServerConfig {
                id: m.id,
                name: m.name,
                description: m.description,
                url: m.url,
                connection_type: m.connection_type,
                command: m.command,
                args: m.args,
                enabled: m.enabled,
                created_at: m.created_at,
                updated_at: m.updated_at,
            })
            .collect();
        Ok(mapped)
    }

    pub async fn update_mcp_server_config_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let pool = self.get_pool()?;
        self.get_db()?.update_mcp_server_config_enabled(id, enabled).await
    }

    pub async fn delete_mcp_server_config(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        self.get_db()?.delete_mcp_server_config(id).await
    }

    pub async fn get_mcp_server_config_by_name(
        &self,
        name: &str,
    ) -> Result<Option<McpServerConfig>> {
        let pool = self.get_pool()?;
        let row = self.get_db()?.get_mcp_server_config_by_name(name).await?;
        Ok(row.map(|m| McpServerConfig {
            id: m.id,
            name: m.name,
            description: m.description,
            url: m.url,
            connection_type: m.connection_type,
            command: m.command,
            args: m.args,
            enabled: m.enabled,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }))
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
        self.get_db()?.update_mcp_server_config(id, name, description, command, args, enabled).await
    }

    #[allow(unused)]
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

    #[allow(unused)]
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

    #[allow(unused)]
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

    #[allow(unused)]
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
        
        // 验证外键约束 - 检查vulnerability_id是否存在
        let vulnerability_id = if let Some(ref vuln_id) = conversation.vulnerability_id {
            if !vuln_id.is_empty() {
                // 检查vulnerability是否存在
                let exists: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM vulnerabilities WHERE id = ?"
                )
                .bind(vuln_id)
                .fetch_optional(pool)
                .await?;
                
                if exists.is_some() {
                    Some(vuln_id.clone())
                } else {
                    None // 如果不存在，设置为NULL
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // 验证外键约束 - 检查scan_task_id是否存在
        let scan_task_id = if let Some(ref task_id) = conversation.scan_task_id {
            if !task_id.is_empty() {
                // 检查scan_task是否存在
                let exists: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM scan_tasks WHERE id = ?"
                )
                .bind(task_id)
                .fetch_optional(pool)
                .await?;
                
                if exists.is_some() {
                    Some(task_id.clone())
                } else {
                    None // 如果不存在，设置为NULL
                }
            } else {
                None
            }
        } else {
            None
        };
        
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
        .bind(vulnerability_id)
        .bind(scan_task_id)
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
        
        // 验证外键约束 - 检查vulnerability_id是否存在
        let vulnerability_id = if let Some(ref vuln_id) = conversation.vulnerability_id {
            if !vuln_id.is_empty() {
                // 检查vulnerability是否存在
                let exists: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM vulnerabilities WHERE id = ?"
                )
                .bind(vuln_id)
                .fetch_optional(pool)
                .await?;
                
                if exists.is_some() {
                    Some(vuln_id.clone())
                } else {
                    None // 如果不存在，设置为NULL
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // 验证外键约束 - 检查scan_task_id是否存在
        let scan_task_id = if let Some(ref task_id) = conversation.scan_task_id {
            if !task_id.is_empty() {
                // 检查scan_task是否存在
                let exists: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM scan_tasks WHERE id = ?"
                )
                .bind(task_id)
                .fetch_optional(pool)
                .await?;
                
                if exists.is_some() {
                    Some(task_id.clone())
                } else {
                    None // 如果不存在，设置为NULL
                }
            } else {
                None
            }
        } else {
            None
        };
        
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
        .bind(vulnerability_id)
        .bind(scan_task_id)
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
        let rows = self.get_db()?.get_configs_by_category(category).await?;
        let mapped = rows
            .into_iter()
            .map(|c| crate::models::database::Configuration {
                id: c.id,
                category: c.category,
                key: c.key,
                value: c.value,
                description: c.description,
                is_encrypted: c.is_encrypted,
                created_at: c.created_at,
                updated_at: c.updated_at,
            })
            .collect();
        Ok(mapped)
    }

    async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        let value: Option<String> =
            sqlx::query_scalar("SELECT value FROM configurations WHERE category = ? AND key = ?")
                .bind(category)
                .bind(key)
                .fetch_optional(pool)
                .await?;
        Ok(value)
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
            "INSERT INTO configurations (category, key, value, description) VALUES (?, ?, ?, ?)\n             ON CONFLICT(category, key) DO UPDATE SET value = excluded.value, description = excluded.description",
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

    async fn set_current_ai_role(&self, role_id: Option<&str>) -> Result<()> {
        match role_id {
            Some(id) => {
                self.set_config("ai", "current_role_id", id, Some("Current selected AI role ID")).await
            }
            None => {
                // 删除当前角色配置，表示使用默认助手
                let pool = self.get_pool()?;
                sqlx::query("DELETE FROM configurations WHERE category = ? AND key = ?")
                    .bind("ai")
                    .bind("current_role_id")
                    .execute(pool)
                    .await?;
                Ok(())
            }
        }
    }

    async fn get_current_ai_role(&self) -> Result<Option<AiRole>> {
        // 获取当前选中的角色ID
        let role_id = match self.get_config("ai", "current_role_id").await? {
            Some(id) => id,
            None => return Ok(None), // 没有选中角色，使用默认助手
        };

        // 根据ID获取角色信息
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT id, title, description, prompt, is_system, created_at, updated_at FROM ai_roles WHERE id = ?")
            .bind(&role_id)
            .fetch_optional(pool)
            .await?;

        match row {
            Some(row) => Ok(Some(AiRole {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                prompt: row.get("prompt"),
                is_system: row.get("is_system"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
            None => {
                // 角色不存在，清除配置
                self.set_current_ai_role(None).await?;
                Ok(None)
            }
        }
    }

    async fn create_scan_task(&self, task: &ScanTask) -> Result<()> {
        let pool = self.get_pool()?;
        let t = sentinel_core::models::database::ScanTask {
            id: task.id.clone(),
            project_id: task.project_id.clone(),
            name: task.name.clone(),
            description: task.description.clone(),
            target_type: task.target_type.clone(),
            targets: task.targets.clone(),
            scan_type: task.scan_type.clone(),
            tools_config: task.tools_config.clone(),
            status: task.status.clone(),
            progress: task.progress,
            priority: task.priority,
            scheduled_at: task.scheduled_at,
            started_at: task.started_at,
            completed_at: task.completed_at,
            execution_time: task.execution_time,
            results_summary: task.results_summary.clone(),
            error_message: task.error_message.clone(),
            created_by: task.created_by.clone(),
            created_at: task.created_at,
            updated_at: task.updated_at,
        };
        self.get_db()?.create_scan_task(&t).await
    }

    async fn get_scan_tasks(&self, project_id: Option<&str>) -> Result<Vec<ScanTask>> {
        let pool = self.get_pool()?;
        let rows = self.get_db()?.get_scan_tasks(project_id).await?;
        let mapped = rows
            .into_iter()
            .map(|t| crate::models::database::ScanTask {
                id: t.id,
                project_id: t.project_id,
                name: t.name,
                description: t.description,
                target_type: t.target_type,
                targets: t.targets,
                scan_type: t.scan_type,
                tools_config: t.tools_config,
                status: t.status,
                progress: t.progress,
                priority: t.priority,
                scheduled_at: t.scheduled_at,
                started_at: t.started_at,
                completed_at: t.completed_at,
                execution_time: t.execution_time,
                results_summary: t.results_summary,
                error_message: t.error_message,
                created_by: t.created_by,
                created_at: t.created_at,
                updated_at: t.updated_at,
            })
            .collect();
        Ok(mapped)
    }

    async fn get_scan_task(&self, id: &str) -> Result<Option<ScanTask>> {
        let pool = self.get_pool()?;
        let row = self.get_db()?.get_scan_task(id).await?;
        Ok(row.map(|t| crate::models::database::ScanTask {
            id: t.id,
            project_id: t.project_id,
            name: t.name,
            description: t.description,
            target_type: t.target_type,
            targets: t.targets,
            scan_type: t.scan_type,
            tools_config: t.tools_config,
            status: t.status,
            progress: t.progress,
            priority: t.priority,
            scheduled_at: t.scheduled_at,
            started_at: t.started_at,
            completed_at: t.completed_at,
            execution_time: t.execution_time,
            results_summary: t.results_summary,
            error_message: t.error_message,
            created_by: t.created_by,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }))
    }

    async fn get_scan_tasks_by_target(&self, target: &str) -> Result<Vec<ScanTask>> {
        // 仍在本地实现（与 LIKE 拼接强相关），后续可迁移为参数化 DAO
        let pool = self.get_pool()?;
        let rows = sqlx::query_as::<_, ScanTask>(
            "SELECT * FROM scan_tasks WHERE targets LIKE ? ORDER BY created_at DESC",
        )
        .bind(format!("%{}%", target))
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    async fn update_scan_task_status(&self, id: &str, status: &str, progress: Option<f64>) -> Result<()> {
        let pool = self.get_pool()?;
        self.get_db()?.update_scan_task_status(id, status, progress).await
    }
    
    // Agent任务相关方法实现
    async fn create_agent_task(&self, task: &crate::agents::traits::AgentTask) -> Result<()> {
        let pool = self.get_pool()?;
        let parameters_json = serde_json::to_string(&task.parameters)?;
        let priority_str = format!("{:?}", task.priority);
        
        sqlx::query(
            "INSERT INTO agent_tasks (id, description, target, parameters, user_id, priority, timeout_seconds, status, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 'Created', ?)"
        )
        .bind(&task.id)
        .bind(&task.description)
        .bind(&task.target)
        .bind(&parameters_json)
        .bind(&task.user_id)
        .bind(&priority_str)
        .bind(task.timeout.map(|t| t as i64))
        .bind(chrono::Utc::now())
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    async fn get_agent_task(&self, id: &str) -> Result<Option<crate::agents::traits::AgentTask>> {
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT id, description, target, parameters, user_id, priority, timeout_seconds FROM agent_tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
            
        if let Some(row) = row {
            let parameters_json: String = row.get("parameters");
            let parameters = serde_json::from_str(&parameters_json).unwrap_or_default();
            let priority_str: String = row.get("priority");
            let priority = match priority_str.as_str() {
                "Low" => crate::agents::traits::TaskPriority::Low,
                "High" => crate::agents::traits::TaskPriority::High,
                "Critical" => crate::agents::traits::TaskPriority::Critical,
                _ => crate::agents::traits::TaskPriority::Normal,
            };
            let timeout_seconds: Option<i64> = row.get("timeout_seconds");
            
            Ok(Some(crate::agents::traits::AgentTask {
                id: row.get("id"),
                description: row.get("description"),
                target: row.get("target"),
                parameters,
                user_id: row.get("user_id"),
                priority,
                timeout: timeout_seconds.map(|t| t as u64),
            }))
        } else {
            Ok(None)
        }
    }
    
    async fn get_agent_tasks(&self, user_id: Option<&str>) -> Result<Vec<crate::agents::traits::AgentTask>> {
        let pool = self.get_pool()?;
        let rows = if let Some(user_id) = user_id {
            sqlx::query("SELECT id, description, target, parameters, user_id, priority, timeout_seconds FROM agent_tasks WHERE user_id = ? ORDER BY created_at DESC")
                .bind(user_id)
                .fetch_all(pool)
                .await?
        } else {
            sqlx::query("SELECT id, description, target, parameters, user_id, priority, timeout_seconds FROM agent_tasks ORDER BY created_at DESC")
                .fetch_all(pool)
                .await?
        };
        
        let mut tasks = Vec::new();
        for row in rows {
            let parameters_json: String = row.get("parameters");
            let parameters = serde_json::from_str(&parameters_json).unwrap_or_default();
            let priority_str: String = row.get("priority");
            let priority = match priority_str.as_str() {
                "Low" => crate::agents::traits::TaskPriority::Low,
                "High" => crate::agents::traits::TaskPriority::High,
                "Critical" => crate::agents::traits::TaskPriority::Critical,
                _ => crate::agents::traits::TaskPriority::Normal,
            };
            let timeout_seconds: Option<i64> = row.get("timeout_seconds");
            
            tasks.push(crate::agents::traits::AgentTask {
                id: row.get("id"),
                description: row.get("description"),
                target: row.get("target"),
                parameters,
                user_id: row.get("user_id"),
                priority,
                timeout: timeout_seconds.map(|t| t as u64),
            });
        }
        
        Ok(tasks)
    }
    
    async fn update_agent_task_status(&self, id: &str, status: &str, agent_name: Option<&str>, architecture: Option<&str>) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "UPDATE agent_tasks SET status = ?, agent_name = ?, architecture = ? WHERE id = ?"
        )
        .bind(status)
        .bind(agent_name)
        .bind(architecture)
        .bind(id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    async fn update_agent_task_timing(&self, id: &str, started_at: Option<chrono::DateTime<chrono::Utc>>, completed_at: Option<chrono::DateTime<chrono::Utc>>, execution_time_ms: Option<u64>) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "UPDATE agent_tasks SET started_at = ?, completed_at = ?, execution_time_ms = ? WHERE id = ?"
        )
        .bind(started_at)
        .bind(completed_at)
        .bind(execution_time_ms.map(|t| t as i64))
        .bind(id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    async fn update_agent_task_error(&self, id: &str, error_message: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "UPDATE agent_tasks SET error_message = ? WHERE id = ?"
        )
        .bind(error_message)
        .bind(id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    // Agent会话相关方法实现
    async fn create_agent_session(&self, session_id: &str, task_id: &str, agent_name: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "INSERT INTO agent_sessions (id, task_id, status, agent_name, created_at, updated_at)
             VALUES (?, ?, 'Created', ?, ?, ?)"
        )
        .bind(session_id)
        .bind(task_id)
        .bind(agent_name)
        .bind(chrono::Utc::now())
        .bind(chrono::Utc::now())
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    async fn update_agent_session_status(&self, session_id: &str, status: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "UPDATE agent_sessions SET status = ?, updated_at = ? WHERE id = ?"
        )
        .bind(status)
        .bind(chrono::Utc::now())
        .bind(session_id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    async fn get_agent_session(&self, session_id: &str) -> Result<Option<crate::agents::session::AgentSessionData>> {
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT id, task_id, status, agent_name, created_at, updated_at FROM agent_sessions WHERE id = ?")
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            
        if let Some(row) = row {
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
            
            Ok(Some(crate::agents::session::AgentSessionData {
                session_id: row.get("id"),
                task_id: row.get("task_id"),
                status: row.get("status"),
                agent_name: row.get("agent_name"),
                created_at,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }
    
    async fn list_agent_sessions(&self) -> Result<Vec<crate::agents::session::AgentSessionData>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT id, task_id, status, agent_name, created_at, updated_at FROM agent_sessions ORDER BY created_at DESC")
            .fetch_all(pool)
            .await?;
        
        let mut sessions = Vec::new();
        for row in rows {
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
            
            sessions.push(crate::agents::session::AgentSessionData {
                session_id: row.get("id"),
                task_id: row.get("task_id"),
                status: row.get("status"),
                agent_name: row.get("agent_name"),
                created_at,
                updated_at,
            });
        }
        
        Ok(sessions)
    }
    
    async fn delete_agent_session(&self, session_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM agent_sessions WHERE id = ?")
            .bind(session_id)
            .execute(pool)
            .await?;
        Ok(())
    }
    
    async fn delete_agent_execution_steps(&self, session_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM agent_execution_steps WHERE session_id = ?")
            .bind(session_id)
            .execute(pool)
            .await?;
        Ok(())
    }
    
    // Agent执行日志相关方法实现
    async fn add_agent_session_log(&self, session_id: &str, level: &str, message: &str, source: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "INSERT INTO agent_session_logs (session_id, level, message, source, timestamp)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(session_id)
        .bind(level)
        .bind(message)
        .bind(source)
        .bind(chrono::Utc::now())
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    async fn get_agent_session_logs(&self, session_id: &str) -> Result<Vec<crate::agents::traits::SessionLog>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT level, message, source, timestamp FROM agent_session_logs WHERE session_id = ? ORDER BY timestamp ASC")
            .bind(session_id)
            .fetch_all(pool)
            .await?;
        
        let mut logs = Vec::new();
        for row in rows {
            let level_str: String = row.get("level");
            let level = match level_str.as_str() {
                "Debug" => crate::agents::traits::LogLevel::Debug,
                "Info" => crate::agents::traits::LogLevel::Info,
                "Warn" => crate::agents::traits::LogLevel::Warn,
                "Error" => crate::agents::traits::LogLevel::Error,
                _ => crate::agents::traits::LogLevel::Info,
            };
            
            logs.push(crate::agents::traits::SessionLog {
                level,
                message: row.get("message"),
                timestamp: row.get("timestamp"),
                source: row.get("source"),
            });
        }
        
        Ok(logs)
    }
    
    // Agent执行结果相关方法实现
    async fn save_agent_execution_result(&self, session_id: &str, result: &crate::agents::traits::AgentExecutionResult) -> Result<()> {
        let pool = self.get_pool()?;
        let data_json = serde_json::to_string(&result.data)?;
        let resources_json = serde_json::to_string(&result.resources_used)?;
        let artifacts_json = serde_json::to_string(&result.artifacts)?;
        
        sqlx::query(
            "INSERT INTO agent_execution_results (id, session_id, success, data, error_message, execution_time_ms, resources_used, artifacts, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&result.id)
        .bind(session_id)
        .bind(result.success)
        .bind(&data_json)
        .bind(&result.error)
        .bind(result.execution_time_ms as i64)
        .bind(&resources_json)
        .bind(&artifacts_json)
        .bind(chrono::Utc::now())
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    async fn get_agent_execution_result(&self, session_id: &str) -> Result<Option<crate::agents::traits::AgentExecutionResult>> {
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT id, success, data, error_message, execution_time_ms, resources_used, artifacts FROM agent_execution_results WHERE session_id = ?")
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            
        if let Some(row) = row {
            let data_json: String = row.get("data");
            let data = serde_json::from_str(&data_json).ok();
            let resources_json: String = row.get("resources_used");
            let resources_used = serde_json::from_str(&resources_json).unwrap_or_default();
            let artifacts_json: String = row.get("artifacts");
            let artifacts = serde_json::from_str(&artifacts_json).unwrap_or_default();
            let execution_time_ms: i64 = row.get("execution_time_ms");
            
            Ok(Some(crate::agents::traits::AgentExecutionResult {
                id: row.get("id"),
                success: row.get("success"),
                data,
                error: row.get("error_message"),
                execution_time_ms: execution_time_ms as u64,
                resources_used,
                artifacts,
            }))
        } else {
            Ok(None)
        }
    }
    
    // Agent执行步骤相关方法实现
    async fn save_agent_execution_step(&self, session_id: &str, step: &crate::commands::agent_commands::WorkflowStepDetail) -> Result<()> {
        let pool = self.get_pool()?;
        let dependencies_json = serde_json::to_string(&step.dependencies)?;
        let result_data_json = serde_json::to_string(&step.result_data)?;
        let tool_result_json = serde_json::to_string(&step.tool_result)?;
        
        // 从step_id中提取step_order（假设step_id类似于"step_1", "step_2"等）
        let step_order = step.step_id.trim_start_matches("step_")
            .parse::<i32>()
            .unwrap_or(0);
        
        let started_at = step.started_at.as_ref()
            .and_then(|s| s.parse::<i64>().ok())
            .map(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .flatten();
        let completed_at = step.completed_at.as_ref()
            .and_then(|s| s.parse::<i64>().ok())
            .map(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .flatten();
        
        sqlx::query(
            "INSERT OR REPLACE INTO agent_execution_steps 
             (id, session_id, step_name, step_order, status, started_at, completed_at, duration_ms, result_data, error_message, retry_count, dependencies, tool_result)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&step.step_id)
        .bind(session_id)
        .bind(&step.step_name)
        .bind(step_order)
        .bind(&step.status)
        .bind(started_at)
        .bind(completed_at)
        .bind(step.duration_ms as i64)
        .bind(&result_data_json)
        .bind(&step.error)
        .bind(step.retry_count as i64)
        .bind(&dependencies_json)
        .bind(&tool_result_json)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    async fn get_agent_execution_steps(&self, session_id: &str) -> Result<Vec<crate::commands::agent_commands::WorkflowStepDetail>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT id, step_name, status, started_at, completed_at, duration_ms, result_data, error_message, retry_count, dependencies, tool_result FROM agent_execution_steps WHERE session_id = ? ORDER BY step_order ASC")
            .bind(session_id)
            .fetch_all(pool)
            .await?;
        
        let mut steps = Vec::new();
        for row in rows {
            let dependencies_json: String = row.get("dependencies");
            let dependencies = serde_json::from_str(&dependencies_json).unwrap_or_default();
            let result_data_json: String = row.get("result_data");
            let result_data = serde_json::from_str(&result_data_json).ok();
            let tool_result_json: String = row.get("tool_result");
            let tool_result = serde_json::from_str(&tool_result_json).ok();
            
            let started_at: Option<chrono::DateTime<chrono::Utc>> = row.get("started_at");
            let completed_at: Option<chrono::DateTime<chrono::Utc>> = row.get("completed_at");
            let duration_ms: i64 = row.get("duration_ms");
            let retry_count: i64 = row.get("retry_count");
            
            steps.push(crate::commands::agent_commands::WorkflowStepDetail {
                step_id: row.get("id"),
                step_name: row.get("step_name"),
                status: row.get("status"),
                started_at: started_at.map(|dt| dt.timestamp().to_string()),
                completed_at: completed_at.map(|dt| dt.timestamp().to_string()),
                duration_ms: duration_ms as u64,
                result_data,
                error: row.get("error_message"),
                retry_count: retry_count as u32,
                dependencies,
                tool_result,
            });
        }
        
        Ok(steps)
    }
    
    async fn update_agent_execution_step_status(&self, step_id: &str, status: &str, started_at: Option<chrono::DateTime<chrono::Utc>>, completed_at: Option<chrono::DateTime<chrono::Utc>>, duration_ms: Option<u64>, error_message: Option<&str>) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "UPDATE agent_execution_steps SET status = ?, started_at = ?, completed_at = ?, duration_ms = ?, error_message = ? WHERE id = ?"
        )
        .bind(status)
        .bind(started_at)
        .bind(completed_at)
        .bind(duration_ms.map(|d| d as i64))
        .bind(error_message)
        .bind(step_id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
}

impl DatabaseService {
    // RAG-specific methods
    pub async fn create_rag_collection(&self, name: &str, description: Option<&str>) -> Result<String> {
        let pool = self.get_pool()?;
        self.get_db()?.create_rag_collection(name, description).await
    }

    pub async fn get_rag_collections(&self) -> Result<Vec<crate::rag::models::CollectionInfo>> {
        let pool = self.get_pool()?;
        let rows = self.get_db()?.get_rag_collections().await?;
        let mapped = rows
            .into_iter()
            .map(|r| {
                let created_at = chrono::DateTime::parse_from_rfc3339(&r.created_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());
                let updated_at = chrono::DateTime::parse_from_rfc3339(&r.updated_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());
                crate::rag::models::CollectionInfo {
                    id: r.id,
                    name: r.name,
                    description: r.description,
                    is_active: r.is_active != 0,
                    embedding_model: "default".to_string(),
                    document_count: r.document_count as usize,
                    chunk_count: r.chunk_count as usize,
                    created_at,
                    updated_at,
                }
            })
            .collect();
        Ok(mapped)
    }

    pub async fn delete_rag_collection(&self, collection_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        self.get_db()?.delete_rag_collection(collection_id).await
    }

    pub async fn get_rag_collection_by_id(&self, collection_id: &str) -> Result<Option<crate::rag::models::CollectionInfo>> {
        let pool = self.get_pool()?;
        let row = self.get_db()?.get_rag_collection_by_id(collection_id).await?;
        Ok(row.map(|r| {
            let created_at = chrono::DateTime::parse_from_rfc3339(&r.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let updated_at = chrono::DateTime::parse_from_rfc3339(&r.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            crate::rag::models::CollectionInfo {
                id: r.id,
                name: r.name,
                description: r.description,
                is_active: r.is_active != 0,
                embedding_model: "default".to_string(),
                document_count: r.document_count as usize,
                chunk_count: r.chunk_count as usize,
                created_at,
                updated_at,
            }
        }))
    }

    pub async fn get_rag_collection_by_name(&self, collection_name: &str) -> Result<Option<crate::rag::models::CollectionInfo>> {
        let pool = self.get_pool()?;
        let row = self.get_db()?.get_rag_collection_by_name(collection_name).await?;
        Ok(row.map(|r| {
            let created_at = chrono::DateTime::parse_from_rfc3339(&r.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let updated_at = chrono::DateTime::parse_from_rfc3339(&r.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            crate::rag::models::CollectionInfo {
                id: r.id,
                name: r.name,
                description: r.description,
                is_active: r.is_active != 0,
                embedding_model: "default".to_string(),
                document_count: r.document_count as usize,
                chunk_count: r.chunk_count as usize,
                created_at,
                updated_at,
            }
        }))
    }

    pub async fn set_rag_collection_active(&self, collection_id: &str, active: bool) -> Result<()> {
        let pool = self.get_pool()?;
        self.get_db()?.set_rag_collection_active(collection_id, active).await
    }

    /// 向量相似度搜索RAG文档块
    pub async fn search_rag_chunks_by_vector(
        &self,
        collection_id: &str,
        query_embedding: &[f32],
        embedding_model: &str,
        limit: usize,
        similarity_threshold: f32,
    ) -> Result<Vec<(f32, crate::rag::models::DocumentChunk)>> {
        let pool = self.get_pool()?;
        
        info!("向量搜索RAG文档块: collection_id={}, embedding_dim={}, limit={}, threshold={}", 
              collection_id, query_embedding.len(), limit, similarity_threshold);

        // 获取所有有嵌入向量的文档块
        let rows = sqlx::query(
            r#"
            SELECT id, document_id, content, content_hash, chunk_index, char_count,
                   embedding_vector, embedding_model, embedding_dimension, metadata,
                   created_at
            FROM rag_chunks 
            WHERE collection_id = ? AND embedding_vector IS NOT NULL AND embedding_model = ?
            ORDER BY created_at DESC
            "#
        )
        .bind(collection_id)
        .bind(embedding_model)
        .fetch_all(pool)
        .await?;

        let mut scored_chunks = Vec::new();
        
        for row in rows {
            let embedding_bytes: Option<Vec<u8>> = row.get("embedding_vector");
            
            if let Some(bytes) = embedding_bytes {
                // 将字节转换为f32向量
                if bytes.len() % 4 == 0 {
                    let embedding: Vec<f32> = bytes
                        .chunks_exact(4)
                        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                        .collect();
                    
                    // 计算余弦相似度
                    let similarity = self.cosine_similarity(query_embedding, &embedding);
                    
                    // 只保留超过阈值的结果
                    if similarity >= similarity_threshold {
                        let chunk = crate::rag::models::DocumentChunk {
                            id: row.get("id"),
                            source_id: row.get("document_id"),
                            content: row.get("content"),
                            content_hash: row.get("content_hash"),
                            chunk_index: row.get::<i64, _>("chunk_index") as usize,
                            embedding: Some(embedding),
                            metadata: {
                                let metadata_json: String = row.get("metadata");
                                if metadata_json.trim().is_empty() || metadata_json.trim() == "{}" {
                                    crate::rag::models::ChunkMetadata {
                                        file_path: "unknown".to_string(),
                                        file_name: "unknown".to_string(),
                                        file_type: "unknown".to_string(),
                                        file_size: 0,
                                        chunk_start_char: 0,
                                        chunk_end_char: row.get::<i64, _>("char_count") as usize,
                                        page_number: None,
                                        section_title: None,
                                        custom_fields: std::collections::HashMap::new(),
                                    }
                                } else {
                                    serde_json::from_str(&metadata_json).unwrap_or_else(|_| {
                                        crate::rag::models::ChunkMetadata {
                                            file_path: "unknown".to_string(),
                                            file_name: "unknown".to_string(),
                                            file_type: "unknown".to_string(),
                                            file_size: 0,
                                            chunk_start_char: 0,
                                            chunk_end_char: row.get::<i64, _>("char_count") as usize,
                                            page_number: None,
                                            section_title: None,
                                            custom_fields: std::collections::HashMap::new(),
                                        }
                                    })
                                }
                            },
                            created_at: {
                                let timestamp: i64 = row.get("created_at");
                                chrono::DateTime::from_timestamp(timestamp, 0)
                                    .unwrap_or_else(|| chrono::Utc::now())
                            },
                        };
                        
                        scored_chunks.push((similarity, chunk));
                    }
                }
            }
        }

        // 按相似度降序排序并限制结果数量
        scored_chunks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored_chunks.truncate(limit);
        
        info!("向量搜索完成，找到 {} 个相关文档块", scored_chunks.len());
        Ok(scored_chunks)
    }

    /// 余弦相似度计算
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot_product / (norm_a * norm_b)
    }

    pub async fn search_rag_chunks_by_id(&self, collection_id: &str, query: &str, limit: usize) -> Result<Vec<crate::rag::models::DocumentChunk>> {
        let pool = self.get_pool()?;

        // 朴素文本检索：将查询拆为多个词项（英文/数字/点 与 中文分别为词），以 AND 串联
        use regex::Regex;
        let token_re = Regex::new(r"([A-Za-z0-9\.]+|[\u4e00-\u9fff]+)").unwrap();
        let mut tokens: Vec<String> = Vec::new();
        for cap in token_re.captures_iter(query) {
            if let Some(m) = cap.get(0) {
                let t = m.as_str().trim();
                if !t.is_empty() { tokens.push(t.to_string()); }
            }
        }

        // 若拆不出词，回退使用原查询
        if tokens.is_empty() {
            tokens.push(query.trim().to_string());
        }

        info!("搜索RAG文本块: collection_id={}, tokens={:?}, limit={}", collection_id, tokens, limit);

        // 动态拼装 AND 条件：content LIKE ? AND content LIKE ? ...
        let mut sql = String::from(
            "SELECT id, document_id, content, content_hash, chunk_index, char_count, \
             embedding_vector, embedding_model, embedding_dimension, metadata, \
             created_at, updated_at FROM rag_chunks WHERE collection_id = ?"
        );
        for _ in &tokens { sql.push_str(" AND content LIKE ?"); }
        sql.push_str(" ORDER BY created_at DESC LIMIT ?");

        let mut q = sqlx::query(&sql).bind(collection_id.to_string());
        for t in &tokens { q = q.bind(format!("%{}%", t)); }
        q = q.bind(limit as i64);

        let rows = q.fetch_all(pool).await?;

        info!("找到 {} 个匹配的文本块", rows.len());

        let mut chunks = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let document_id: String = row.get("document_id");
            let content: String = row.get("content");
            let content_hash: String = row.get("content_hash");
            let chunk_index: i32 = row.get("chunk_index");
            let metadata_json: String = row.get("metadata");
            let created_at: i64 = row.get("created_at");

            // 解析metadata - 如果是空JSON，创建默认的ChunkMetadata
            let metadata = if metadata_json.trim() == "{}" {
                crate::rag::models::ChunkMetadata {
                    file_path: "unknown".to_string(),
                    file_name: "unknown".to_string(),
                    file_type: "unknown".to_string(),
                    file_size: 0,
                    chunk_start_char: 0,
                    chunk_end_char: content.chars().count(),
                    page_number: None,
                    section_title: None,
                    custom_fields: std::collections::HashMap::new(),
                }
            } else {
                match serde_json::from_str::<crate::rag::models::ChunkMetadata>(&metadata_json) {
                    Ok(meta) => meta,
                    Err(_) => {
                        // 尝试解析为HashMap，然后转换
                        let meta_map: std::collections::HashMap<String, String> = 
                            serde_json::from_str(&metadata_json).unwrap_or_default();
                        crate::rag::models::ChunkMetadata {
                            file_path: meta_map.get("file_path").unwrap_or(&"unknown".to_string()).clone(),
                            file_name: meta_map.get("file_name").unwrap_or(&"unknown".to_string()).clone(),
                            file_type: meta_map.get("file_type").unwrap_or(&"unknown".to_string()).clone(),
                            file_size: meta_map.get("file_size").unwrap_or(&"0".to_string()).parse().unwrap_or(0),
                            chunk_start_char: meta_map.get("chunk_start_char").unwrap_or(&"0".to_string()).parse().unwrap_or(0),
                            chunk_end_char: meta_map.get("chunk_end_char").unwrap_or(&content.chars().count().to_string()).parse().unwrap_or(content.chars().count()),
                            page_number: meta_map.get("page_number").and_then(|s| s.parse().ok()),
                            section_title: meta_map.get("section_title").cloned(),
                            custom_fields: meta_map,
                        }
                    }
                }
            };

            let created_at_datetime = chrono::DateTime::from_timestamp(created_at, 0)
                .unwrap_or_else(|| chrono::Utc::now());

            chunks.push(crate::rag::models::DocumentChunk {
                id,
                source_id: document_id, // document_id acts as source_id
                content,
                content_hash,
                chunk_index: chunk_index as usize,
                metadata,
                embedding: None, // TODO: 从数据库加载嵌入向量
                created_at: created_at_datetime,
            });
        }

        info!("返回 {} 个文本块", chunks.len());

        Ok(chunks)
    }

    /// 读取包含嵌入向量的chunks（用于内存向量搜索）
    pub async fn fetch_chunks_with_embeddings(
        &self,
        collection_id: &str,
        embedding_model: &str,
        embedding_dimension: i32,
        limit: usize,
    ) -> Result<Vec<crate::rag::models::DocumentChunk>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query(
            r#"
            SELECT id, document_id, content, content_hash, chunk_index, metadata, created_at, embedding_vector
            FROM rag_chunks
            WHERE collection_id = ? AND embedding_vector IS NOT NULL AND embedding_model = ? AND embedding_dimension = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#
        )
        .bind(collection_id)
        .bind(embedding_model)
        .bind(embedding_dimension)
        .bind(limit as i64)
        .fetch_all(pool)
        .await?;

        let mut chunks = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let document_id: String = row.get("document_id");
            let content: String = row.get("content");
            let content_hash: String = row.get("content_hash");
            let chunk_index: i64 = row.get("chunk_index");
            let metadata_json: String = row.get("metadata");
            let created_at: i64 = row.get("created_at");
            let embedding_bytes: Option<Vec<u8>> = row.get("embedding_vector");

            let metadata = if metadata_json.trim().is_empty() || metadata_json.trim() == "{}" {
                crate::rag::models::ChunkMetadata {
                    file_path: "unknown".to_string(),
                    file_name: "unknown".to_string(),
                    file_type: "unknown".to_string(),
                    file_size: 0,
                    chunk_start_char: 0,
                    chunk_end_char: content.chars().count(),
                    page_number: None,
                    section_title: None,
                    custom_fields: std::collections::HashMap::new(),
                }
            } else {
                serde_json::from_str::<crate::rag::models::ChunkMetadata>(&metadata_json)
                    .unwrap_or_else(|_| crate::rag::models::ChunkMetadata {
                        file_path: "unknown".to_string(),
                        file_name: "unknown".to_string(),
                        file_type: "unknown".to_string(),
                        file_size: 0,
                        chunk_start_char: 0,
                        chunk_end_char: content.chars().count(),
                        page_number: None,
                        section_title: None,
                        custom_fields: std::collections::HashMap::new(),
                    })
            };

            let created_at_datetime = chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_else(|| chrono::Utc::now());

            let embedding: Option<Vec<f32>> = if let Some(bytes) = embedding_bytes {
                bincode::deserialize::<Vec<f32>>(&bytes).ok()
            } else { None };

            chunks.push(crate::rag::models::DocumentChunk {
                id,
                source_id: document_id,
                content,
                content_hash,
                chunk_index: chunk_index as usize,
                metadata,
                embedding,
                created_at: created_at_datetime,
            });
        }

        Ok(chunks)
    }

    pub async fn search_rag_chunks(&self, collection_name: &str, query: &str, limit: usize) -> Result<Vec<crate::rag::models::DocumentChunk>> {
        let pool = self.get_pool()?;
        
        // For now, do a simple text search. In a real implementation, you'd use vector similarity
        let rows = sqlx::query(
            r#"
            SELECT c.id, c.source_id, c.content, c.content_hash, c.chunk_index, c.metadata, c.created_at
            FROM rag_document_chunks c
            JOIN rag_collections col ON c.collection_id = col.id
            WHERE col.name = ? AND c.content LIKE ?
            ORDER BY c.chunk_index
            LIMIT ?
            "#
        )
        .bind(collection_name)
        .bind(format!("%{}%", query))
        .bind(limit as i64)
        .fetch_all(pool)
        .await?;

        let mut chunks = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let source_id: String = row.get("source_id");
            let content: String = row.get("content");
            let content_hash: String = row.get("content_hash");
            let chunk_index: i64 = row.get("chunk_index");
            let metadata_json: String = row.get("metadata");
            let created_at: String = row.get("created_at");

            let metadata: crate::rag::models::ChunkMetadata = serde_json::from_str(&metadata_json)?;

            chunks.push(crate::rag::models::DocumentChunk {
                id,
                source_id,
                content,
                content_hash,
                chunk_index: chunk_index as usize,
                metadata,
                embedding: None,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&chrono::Utc),
            });
        }

        Ok(chunks)
    }

    pub async fn save_rag_query(&self, collection_name: Option<&str>, query: &str, response: &str, processing_time_ms: u64) -> Result<()> {
        let pool = self.get_pool()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Get collection_id if collection_name is provided
        let collection_id = if let Some(name) = collection_name {
            let row = sqlx::query("SELECT id FROM rag_collections WHERE name = ?")
                .bind(name)
                .fetch_optional(pool)
                .await?;
            row.map(|r| r.get::<String, _>("id"))
        } else {
            None
        };

        sqlx::query(
            "INSERT INTO rag_query_history (id, collection_id, query, response, processing_time_ms, created_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(collection_id)
        .bind(query)
        .bind(response)
        .bind(processing_time_ms as i64)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_rag_chunks(&self, collection_name: &str) -> Result<Vec<crate::rag::models::DocumentChunk>> {
        let pool = self.get_pool()?;
        
        let rows = sqlx::query(
            r#"
            SELECT c.id, c.source_id, c.content, c.content_hash, c.chunk_index, c.metadata, c.created_at
            FROM rag_document_chunks c
            JOIN rag_collections col ON c.collection_id = col.id
            WHERE col.name = ?
            ORDER BY c.chunk_index
            "#
        )
        .bind(collection_name)
        .fetch_all(pool)
        .await?;

        let mut chunks = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let source_id: String = row.get("source_id");
            let content: String = row.get("content");
            let content_hash: String = row.get("content_hash");
            let chunk_index: i64 = row.get("chunk_index");
            let metadata_json: String = row.get("metadata");
            let created_at: String = row.get("created_at");

            let metadata: crate::rag::models::ChunkMetadata = serde_json::from_str(&metadata_json)?;

            chunks.push(crate::rag::models::DocumentChunk {
                id,
                source_id,
                content,
                content_hash,
                chunk_index: chunk_index as usize,
                metadata,
                embedding: None,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&chrono::Utc),
            });
        }

        Ok(chunks)
    }

    pub async fn get_rag_query_history(&self, _collection_name: Option<&str>, limit: Option<i32>) -> Result<Vec<crate::rag::models::QueryResult>> {
        let _pool = self.get_pool()?;
        let _limit = limit.unwrap_or(50);
        
        // For now, return empty results since we need to implement proper query result storage
        // This is a placeholder implementation to fix compilation
        Ok(Vec::new())
    }

    pub async fn delete_rag_document(&self, document_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        let collection_id = self.get_db()?.get_collection_id_by_document_id(document_id).await?;
        self.get_db()?.delete_document_cascade(document_id).await?;
        if let Some(cid) = collection_id.as_deref() {
            let _ = self.update_collection_stats(cid).await;
        }
        Ok(())
    }

    pub async fn get_rag_documents(&self, collection_name: &str) -> Result<Vec<crate::rag::models::DocumentSource>> {
        let pool = self.get_pool()?;
        let rows = self.get_db()?.get_documents_by_collection_name(collection_name).await?;
        let mut documents = Vec::new();
        for r in rows {
            let metadata: std::collections::HashMap<String, String> = serde_json::from_str(&r.metadata)?;
            documents.push(crate::rag::models::DocumentSource {
                id: r.id,
                file_path: r.file_path,
                file_name: r.file_name,
                file_type: r.file_type,
                file_size: r.file_size as u64,
                file_hash: r.content_hash,
                chunk_count: 0,
                ingestion_status: crate::rag::models::IngestionStatusEnum::Completed,
                created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at)?.with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at)?.with_timezone(&chrono::Utc),
                metadata,
            });
        }
        Ok(documents)
    }

    /// 根据集合ID获取文档列表
    pub async fn get_rag_documents_by_collection_id(&self, collection_id: &str) -> Result<Vec<crate::rag::models::DocumentSource>> {
        let pool = self.get_pool()?;
        let rows = self.get_db()?.get_documents_by_collection_id(collection_id).await?;
        let mut documents = Vec::new();
        for r in rows {
            let metadata: std::collections::HashMap<String, String> = serde_json::from_str(&r.metadata)?;
            documents.push(crate::rag::models::DocumentSource {
                id: r.id,
                file_path: r.file_path,
                file_name: r.file_name,
                file_type: r.file_type,
                file_size: r.file_size as u64,
                file_hash: r.content_hash,
                chunk_count: 0,
                ingestion_status: crate::rag::models::IngestionStatusEnum::Completed,
                created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at)?.with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at)?.with_timezone(&chrono::Utc),
                metadata,
            });
        }
        Ok(documents)
    }

    /// 根据文档ID获取其所有文本块（来自新表 rag_chunks）
    pub async fn get_rag_chunks_by_document_id(&self, document_id: &str) -> Result<Vec<crate::rag::models::DocumentChunk>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query(
            r#"
            SELECT id, document_id, content, content_hash, chunk_index, metadata, created_at
            FROM rag_chunks
            WHERE document_id = ?
            ORDER BY chunk_index ASC
            "#
        )
        .bind(document_id)
        .fetch_all(pool)
        .await?;

        let mut chunks = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let document_id_val: String = row.get("document_id");
            let content: String = row.get("content");
            let content_hash: String = row.get("content_hash");
            let chunk_index: i64 = row.get("chunk_index");
            let metadata_json: String = row.get("metadata");
            let created_at_ts: i64 = row.get("created_at");

            // 解析metadata，容错空/无效JSON
            let metadata = if metadata_json.trim().is_empty() || metadata_json.trim() == "{}" {
                crate::rag::models::ChunkMetadata {
                    file_path: "unknown".to_string(),
                    file_name: "unknown".to_string(),
                    file_type: "unknown".to_string(),
                    file_size: 0,
                    chunk_start_char: 0,
                    chunk_end_char: content.chars().count(),
                    page_number: None,
                    section_title: None,
                    custom_fields: std::collections::HashMap::new(),
                }
            } else {
                serde_json::from_str::<crate::rag::models::ChunkMetadata>(&metadata_json)
                    .unwrap_or_else(|_| crate::rag::models::ChunkMetadata {
                        file_path: "unknown".to_string(),
                        file_name: "unknown".to_string(),
                        file_type: "unknown".to_string(),
                        file_size: 0,
                        chunk_start_char: 0,
                        chunk_end_char: content.chars().count(),
                        page_number: None,
                        section_title: None,
                        custom_fields: std::collections::HashMap::new(),
                    })
            };

            let created_at = chrono::DateTime::from_timestamp(created_at_ts, 0)
                .unwrap_or_else(|| chrono::Utc::now());

            chunks.push(crate::rag::models::DocumentChunk {
                id,
                source_id: document_id_val,
                content,
                content_hash,
                chunk_index: chunk_index as usize,
                metadata,
                embedding: None,
                created_at,
            });
        }

        Ok(chunks)
    }

    /// 通过文档ID获取其所属集合ID
    pub async fn get_collection_id_by_document_id(&self, document_id: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        self.get_db()?.get_collection_id_by_document_id(document_id).await
    }

    pub async fn create_rag_document(
        &self,
        collection_id: &str,
        file_path: &str,
        file_name: &str,
        content: &str,
        metadata: &str,
    ) -> Result<String> {
        let pool = self.get_pool()?;
        let document_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        
        // Calculate file hash and size based on content string
        // 若content为空（部分旧流程未填充），退化为根据文件路径读取实际大小与内容hash
        let (file_hash, file_size) = if content.is_empty() {
            // Try to read file for accurate metadata; ignore errors and fallback
            match std::fs::metadata(file_path) {
                Ok(meta) => {
                    let size = meta.len() as i64;
                    let hash = std::fs::read(file_path)
                        .map(|bytes| format!("{:x}", md5::compute(&bytes)))
                        .unwrap_or_else(|_| format!("{:x}", md5::compute(file_path.as_bytes())));
                    (hash, size)
                }
                Err(_) => (format!("{:x}", md5::compute(file_path.as_bytes())), 0),
            }
        } else {
            (format!("{:x}", md5::compute(content.as_bytes())), content.len() as i64)
        };
        
        // DAO insert
        self.get_db()?.insert_document_source(
            &document_id,
            collection_id,
            file_path,
            file_name,
            "text",
            file_size,
            &file_hash,
            &file_hash,
            metadata,
            &now.to_rfc3339(),
            &now.to_rfc3339(),
        )
        .await?;

        Ok(document_id)
    }

    pub async fn create_rag_chunk(
        &self,
        document_id: &str,
        collection_id: &str,
        content: &str,
        chunk_index: i32,
        embedding: Option<&[f32]>,
        embedding_model: &str,
        embedding_dimension: i32,
        metadata_json: &str,
    ) -> Result<String> {
        let pool = self.get_pool()?;
        let chunk_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let now_timestamp = now.timestamp();
        
        // Convert embedding to bytes if provided
        let embedding_bytes = if let Some(emb) = embedding {
            Some(bincode::serialize(emb)?)
        } else {
            None
        };
        
        // Generate content hash
        let content_hash = format!("{:x}", md5::compute(content.as_bytes()));
        let char_count = content.chars().count() as i32;
        
        // DAO insert
        self.get_db()?.insert_chunk(
            &chunk_id,
            document_id,
            collection_id,
            content,
            &content_hash,
            chunk_index,
            char_count,
            embedding_bytes,
            embedding_model,
            embedding_dimension,
            metadata_json,
            now_timestamp,
            now_timestamp,
        )
        .await?;

        Ok(chunk_id)
    }

    /// 更新集合的文档和块统计
    pub async fn update_collection_stats(&self, collection_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        self.get_db()?.update_collection_stats(collection_id).await
    }

    // Scenario agent methods
    pub async fn list_scenario_agents(&self) -> Result<Vec<crate::commands::ai_commands::ScenarioAgentProfile>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT * FROM scenario_agents ORDER BY name")
            .fetch_all(pool)
            .await?;

        let mut agents = Vec::new();
        for row in rows {
            let profile_json: String = row.get("profile_json");
            let profile: crate::commands::ai_commands::ScenarioAgentProfile = 
                serde_json::from_str(&profile_json)?;
            agents.push(profile);
        }
        Ok(agents)
    }

    pub async fn upsert_scenario_agent(&self, profile: &crate::commands::ai_commands::ScenarioAgentProfile) -> Result<()> {
        let pool = self.get_pool()?;
        let profile_json = serde_json::to_string(profile)?;
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT OR REPLACE INTO scenario_agents (id, name, profile_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&profile.id)
        .bind(&profile.name)
        .bind(&profile_json)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_scenario_agent(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM scenario_agents WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn set_current_ai_role(&self, role_id: Option<&str>) -> Result<()> {
        match role_id {
            Some(id) => {
                self.set_config("ai", "current_role_id", id, Some("Current selected AI role ID")).await
            }
            None => {
                // 删除当前角色配置，表示使用默认助手
                let pool = self.get_pool()?;
                sqlx::query("DELETE FROM configurations WHERE category = ? AND key = ?")
                    .bind("ai")
                    .bind("current_role_id")
                    .execute(pool)
                    .await?;
                Ok(())
            }
        }
    }

    async fn get_current_ai_role(&self) -> Result<Option<AiRole>> {
        // 获取当前选中的角色ID
        let role_id = match self.get_config("ai", "current_role_id").await? {
            Some(id) => id,
            None => return Ok(None), // 没有选中角色，使用默认助手
        };

        // 根据ID获取角色信息
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT id, title, description, prompt, is_system, created_at, updated_at FROM ai_roles WHERE id = ?")
            .bind(&role_id)
            .fetch_optional(pool)
            .await?;

        match row {
            Some(row) => Ok(Some(AiRole {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                prompt: row.get("prompt"),
                is_system: row.get("is_system"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
            None => {
                // 角色不存在，清除配置
                self.set_current_ai_role(None).await?;
                Ok(None)
            }
        }
    }
}

impl Default for DatabaseService {
    fn default() -> Self {
        Self::new()
    }
}
