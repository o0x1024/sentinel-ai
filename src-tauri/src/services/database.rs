use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::{sqlite::SqlitePool, Column, Row, TypeInfo};
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

        // 提交事务
        tx.commit().await?;

        Ok(())
    }

    async fn insert_default_prompts(&self, pool: &SqlitePool) -> Result<()> {
        let defaults: &[(&str, &str, &str, &str)] = &[
            // arch, stage, name, content
            ("rewoo", "planner", "ReWOO Planner", "你是一个智能任务规划器。请根据用户的问题，制定一个详细的执行计划。\n\n用户问题：{user_question}\n\n请按照以下格式输出计划：\nPlan: 步骤描述\n#E1 = 工具名称[参数]\n#E2 = 工具名称[参数, #E1]"),
            ("rewoo", "worker", "ReWOO Worker", "你是一个工具执行器。请根据计划中的步骤，调用相应的工具并返回结果。\n当前步骤：{current_step}\n可用工具：{available_tools}"),
            ("rewoo", "solver", "ReWOO Solver", "你是一个结果整合器。请根据执行结果，为用户提供最终答案。\n原始问题：{original_question}\n执行计划：{plan}\n执行结果：{results}"),
            ("llmcompiler", "planning", "LLMC Planning", "你是一个并行任务规划器。请将复杂任务分解为可并行执行的子任务。\n用户任务：{user_task}"),
            ("llmcompiler", "execution", "LLMC Execution", "你是一个任务执行器。请执行指定的子任务。\n当前任务：{current_task}\n依赖结果：{dependencies}"),
            ("llmcompiler", "replan", "LLMC Replan", "你是一个重新规划器。当执行出现问题时，请调整执行计划。\n原始计划：{original_plan}\n执行状态：{execution_status}\n错误信息：{error_info}"),
            ("planexecute", "planning", "P&E Planning", "你是一个策略规划师。请为用户的目标制定详细的执行计划。\n用户目标：{user_goal}"),
            ("planexecute", "execution", "P&E Execution", "你是一个执行专家。请执行计划中的当前步骤。\n执行计划：{plan}\n当前步骤：{current_step}\n前置结果：{previous_results}"),
            ("planexecute", "replan", "P&E Replan", "你是一个重新规划师。请评估执行结果并在必要时调整计划。\n执行计划：{plan}\n执行结果：{results}\n目标达成情况：{goal_achievement}"),
        ];

        for (arch, stage, name, content) in defaults {
            sqlx::query(r#"INSERT INTO prompt_templates (name, description, architecture, stage, content, is_default, is_active) VALUES (?, ?, ?, ?, ?, 1, 1)"#)
                .bind(*name)
                .bind(Option::<&str>::None)
                .bind(*arch)
                .bind(*stage)
                .bind(*content)
                .execute(pool)
                .await?;
        }
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
            scan_tasks_count: scan_tasks_count as u64,
            vulnerabilities_count: vulnerabilities_count as u64,
            assets_count: assets_count as u64,
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
        let mut _bind_count = 1;

        if progress.is_some() {
            query.push_str(", progress = ?");
            _bind_count += 1;
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
                status, verification_status, tags, attachments, notes
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

    async fn create_scan_task(&self, task: &ScanTask) -> Result<()> {
        self.create_scan_task(task).await
    }

    async fn get_scan_tasks(&self, project_id: Option<&str>) -> Result<Vec<ScanTask>> {
        self.get_scan_tasks(project_id).await
    }

    async fn get_scan_task(&self, id: &str) -> Result<Option<ScanTask>> {
        self.get_scan_task(id).await
    }

    async fn get_scan_tasks_by_target(&self, target: &str) -> Result<Vec<ScanTask>> {
        self.get_scan_tasks_by_target(target).await
    }

    async fn update_scan_task_status(&self, id: &str, status: &str, progress: Option<f64>) -> Result<()> {
        self.update_scan_task_status(id, status, progress).await
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

impl Default for DatabaseService {
    fn default() -> Self {
        Self::new()
    }
}
