//! 数据库操作模块
//!
//! 负责流量分析相关的数据持久化：
//! - 漏洞存储与查询
//! - 证据存储
//! - 插件注册表
//! - 扫描会话管理

use crate::{Finding, TrafficError, PluginMetadata, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, SqlitePool};
use tracing::{debug, info};

/// 数据库服务
pub struct TrafficDatabaseService {
    pool: Pool<Sqlite>,
}

impl TrafficDatabaseService {
    /// 获取数据库连接池（用于测试）
    #[allow(dead_code)]
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// 创建数据库服务
    pub async fn new(database_url: &str) -> Result<Self> {
        // 解析数据库文件路径
        let db_path = database_url
            .strip_prefix("sqlite://")
            .ok_or_else(|| TrafficError::Database("Invalid database URL".to_string()))?;
        
        // 确保数据库目录存在
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| TrafficError::Database(format!("Failed to create database directory: {}", e)))?;
        }

        // 创建连接池，自动创建数据库文件
        let pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(db_path)
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Delete)
                .synchronous(sqlx::sqlite::SqliteSynchronous::Normal),
        )
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to connect: {}", e)))?;

        let service = Self { pool };
        
        // 自动运行表结构创建
        service.run_migrations().await?;
        
        Ok(service)
    }

    /// 迁移旧表名\n    
    async fn migrate_old_tables(&self) -> Result<()> 
    {
        let old_new_tables = vec![
            ("passive_vulnerabilities", "traffic_vulnerabilities"),
            ("passive_evidence", "traffic_evidence"),
            ("passive_dedupe_index", "traffic_dedupe_index"),
        ];

        for (old_table, new_table) in old_new_tables {
            let check_query = format!("SELECT count(*) FROM sqlite_master WHERE type='table' AND name='{}'", old_table);
            let count: i64 = sqlx::query_scalar(&check_query)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| TrafficError::Database(format!("Failed to check for old table {}: {}", old_table, e)))?;

            if count > 0 {
                info!("Migrating old table {} to {}", old_table, new_table);
                let rename_query = format!("ALTER TABLE {} RENAME TO {}", old_table, new_table);
                sqlx::query(&rename_query)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| TrafficError::Database(format!("Failed to rename table {} to {}: {}", old_table, new_table, e)))?;
            }
        }

        // 迁移 main_category 字段值
        let update_category_query = "UPDATE plugin_registry SET main_category = 'traffic' WHERE main_category = 'traffic'";
        let _ = sqlx::query(update_category_query)
            .execute(&self.pool)
            .await;

        Ok(())
    }

    /// 创建数据库表结构
    pub async fn run_migrations(&self) -> Result<()> {
        // 检查并迁移旧表名
        self.migrate_old_tables().await?;

        // 漏洞表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS traffic_vulnerabilities (
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
                first_seen_at DATETIME NOT NULL,
                last_seen_at DATETIME NOT NULL,
                hit_count INTEGER NOT NULL DEFAULT 1,
                session_id TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to create traffic_vulnerabilities table: {}", e)))?;

        // 证据表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS traffic_evidence (
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
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (vuln_id) REFERENCES traffic_vulnerabilities(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to create traffic_evidence table: {}", e)))?;

        // 去重索引表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS traffic_dedupe_index (
                signature TEXT PRIMARY KEY,
                vuln_id TEXT NOT NULL,
                first_hit DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_hit DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (vuln_id) REFERENCES traffic_vulnerabilities(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to create traffic_dedupe_index table: {}", e)))?;

        // 代理请求历史表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS proxy_requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
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
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to create proxy_requests table: {}", e)))?;

        // 插件注册表 (用于存储AI生成的插件和手动创建的插件)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS plugin_registry (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                author TEXT,
                main_category TEXT NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                default_severity TEXT NOT NULL,
                tags TEXT,
                enabled BOOLEAN NOT NULL DEFAULT 0,
                plugin_code TEXT,
                quality_score REAL,
                validation_status TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to create plugin_registry table: {}", e)))?;

        // 配置表 (用于存储代理配置)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS proxy_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to create proxy_config table: {}", e)))?;

        // 创建索引
        let indices = vec![
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
        ];

        for index_sql in indices {
            sqlx::query(index_sql)
                .execute(&self.pool)
                .await
                .map_err(|e| TrafficError::Database(format!("Failed to create index: {}", e)))?;
        }
        
        info!("Traffic scan database schema created successfully");
        Ok(())
    }

    // ============================================================
    // 漏洞操作
    // ============================================================

    /// 插入新漏洞
    pub async fn insert_vulnerability(&self, finding: &Finding) -> Result<()> {
        let signature = finding.calculate_signature();

        debug!("Inserting vulnerability: title='{}', description='{}'", 
              finding.title, finding.description);

        sqlx::query(
            r#"
            INSERT INTO traffic_vulnerabilities (
                id, plugin_id, vuln_type, severity, confidence, title, description,
                cwe, owasp, remediation, signature, first_seen_at, last_seen_at, session_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)
            "#,
        )
        .bind(&finding.id)
        .bind(&finding.plugin_id)
        .bind(&finding.vuln_type)
        .bind(format!("{}", finding.severity))
        .bind(format!("{:?}", finding.confidence))
        .bind(&finding.title)
        .bind(&finding.description)
        .bind(&finding.cwe)
        .bind(&finding.owasp)
        .bind(&finding.remediation)
        .bind(&signature)
        .bind(&finding.created_at)
        .bind(&finding.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to insert vulnerability: {}", e)))?;

        // 插入去重索引
        sqlx::query(
            r#"
            INSERT INTO traffic_dedupe_index (signature, vuln_id) VALUES (?, ?)
            "#,
        )
        .bind(&signature)
        .bind(&finding.id)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to insert dedupe index: {}", e)))?;

        // 插入证据记录（保存原始请求/响应）
        let evidence = EvidenceRecord {
            id: format!("{}-evidence-{}", finding.id, chrono::Utc::now().timestamp_millis()),
            vuln_id: finding.id.clone(),
            url: finding.url.clone(),
            method: finding.method.clone(),
            location: finding.location.clone(),
            evidence_snippet: finding.evidence.clone(),
            request_headers: finding.request_headers.clone(),
            request_body: finding.request_body.clone(),
            response_status: finding.response_status,
            response_headers: finding.response_headers.clone(),
            response_body: finding.response_body.clone(),
            timestamp: finding.created_at,
        };
        
        self.insert_evidence(&evidence).await?;

        debug!("Vulnerability inserted with evidence: {} - {}", finding.id, finding.title);
        Ok(())
    }

    /// 更新漏洞命中次数
    pub async fn update_vulnerability_hit(&self, signature: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE traffic_vulnerabilities 
            SET hit_count = hit_count + 1, 
                last_seen_at = ?,
                updated_at = ?
            WHERE signature = ?
            "#,
        )
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(signature)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to update hit count: {}", e)))?;

        Ok(())
    }

    /// 检查签名是否已存在
    pub async fn check_signature_exists(&self, signature: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM traffic_dedupe_index WHERE signature = ?
            "#,
        )
        .bind(signature)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to check signature: {}", e)))?;

        Ok(count > 0)
    }

    /// 列出漏洞（带分页和筛选）
    pub async fn list_vulnerabilities(
        &self,
        filters: VulnerabilityFilters,
    ) -> Result<Vec<VulnerabilityRecord>> {
        let mut query = String::from(
            r#"
            SELECT id, plugin_id, vuln_type, severity, confidence, title, description,
                   cwe, owasp, remediation, status, signature, first_seen_at, last_seen_at,
                   hit_count, session_id, created_at, updated_at
            FROM traffic_vulnerabilities
            WHERE 1=1
            "#,
        );

        // 动态拼接筛选条件
        if let Some(ref vuln_type) = filters.vuln_type {
            query.push_str(&format!(" AND vuln_type = '{}'", vuln_type));
        }
        if let Some(ref severity) = filters.severity {
            query.push_str(&format!(" AND severity = '{}'", severity));
        }
        if let Some(ref status) = filters.status {
            query.push_str(&format!(" AND status = '{}'", status));
        }
        if let Some(ref plugin_id) = filters.plugin_id {
            query.push_str(&format!(" AND plugin_id = '{}'", plugin_id));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let records = sqlx::query_as::<_, VulnerabilityRecord>(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| TrafficError::Database(format!("Failed to list vulnerabilities: {}", e)))?;

        Ok(records)
    }

    /// 列出漏洞（带证据和分页）
    pub async fn list_vulnerabilities_with_evidence(
        &self,
        filters: VulnerabilityFilters,
    ) -> Result<Vec<VulnerabilityWithEvidence>> {
        // 先获取漏洞记录
        let vulnerabilities = self.list_vulnerabilities(filters).await?;
        
        // 为每个漏洞获取证据
        let mut results = Vec::new();
        for vuln in vulnerabilities {
            let evidence = self.get_evidence_by_vuln_id(&vuln.id).await?;
            
            // 从第一条证据中提取 URL 和方法
            let (url, method) = evidence.first()
                .map(|e| (Some(e.url.clone()), Some(e.method.clone())))
                .unwrap_or((None, None));
            
            results.push(VulnerabilityWithEvidence {
                vulnerability: vuln,
                evidence,
                url,
                method,
            });
        }
        
        Ok(results)
    }

    /// 统计漏洞数量
    pub async fn count_vulnerabilities(&self, filters: VulnerabilityFilters) -> Result<i64> {
        let mut query = String::from(
            r#"
            SELECT COUNT(*) as count
            FROM traffic_vulnerabilities
            WHERE 1=1
            "#,
        );

        // 动态拼接筛选条件
        if let Some(ref vuln_type) = filters.vuln_type {
            query.push_str(&format!(" AND vuln_type = '{}'", vuln_type));
        }
        if let Some(ref severity) = filters.severity {
            query.push_str(&format!(" AND severity = '{}'", severity));
        }
        if let Some(ref status) = filters.status {
            query.push_str(&format!(" AND status = '{}'", status));
        }
        if let Some(ref plugin_id) = filters.plugin_id {
            query.push_str(&format!(" AND plugin_id = '{}'", plugin_id));
        }

        let row: (i64,) = sqlx::query_as(&query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| TrafficError::Database(format!("Failed to count vulnerabilities: {}", e)))?;

        Ok(row.0)
    }

    /// 根据 ID 获取单个漏洞详情
    pub async fn get_vulnerability_by_id(&self, vuln_id: &str) -> Result<Option<VulnerabilityRecord>> {
        let record = sqlx::query_as::<_, VulnerabilityRecord>(
            r#"
            SELECT id, plugin_id, vuln_type, severity, confidence, title, description,
                   cwe, owasp, remediation, status, signature, first_seen_at, last_seen_at,
                   hit_count, session_id, created_at, updated_at
            FROM traffic_vulnerabilities
            WHERE id = ?
            "#,
        )
        .bind(vuln_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to fetch vulnerability: {}", e)))?;

        Ok(record)
    }

    /// 根据漏洞 ID 获取所有相关证据
    pub async fn get_evidence_by_vuln_id(&self, vuln_id: &str) -> Result<Vec<EvidenceRecord>> {
        let records = sqlx::query_as::<_, EvidenceRecord>(
            r#"
            SELECT id, vuln_id, url, method, location, evidence_snippet,
                   request_headers, request_body, response_status, response_headers,
                   response_body, timestamp
            FROM traffic_evidence
            WHERE vuln_id = ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(vuln_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to fetch evidence: {}", e)))?;

        Ok(records)
    }

    /// 更新漏洞状态
    pub async fn update_vulnerability_status(&self, vuln_id: &str, status: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE traffic_vulnerabilities 
            SET status = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(Utc::now())
        .bind(vuln_id)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to update vulnerability status: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(TrafficError::Database(format!("Vulnerability not found: {}", vuln_id)));
        }

        info!("Vulnerability status updated: {} -> {}", vuln_id, status);
        Ok(())
    }

    /// 删除单个漏洞及其关联的证据和去重索引
    pub async fn delete_vulnerability(&self, vuln_id: &str) -> Result<()> {
        // SQLite 会自动通过 FOREIGN KEY ON DELETE CASCADE 删除关联的证据和去重索引
        let result = sqlx::query(
            r#"
            DELETE FROM traffic_vulnerabilities WHERE id = ?
            "#,
        )
        .bind(vuln_id)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to delete vulnerability: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(TrafficError::Database(format!("Vulnerability not found: {}", vuln_id)));
        }

        info!("Vulnerability deleted: {}", vuln_id);
        Ok(())
    }

    /// 删除所有漏洞
    pub async fn delete_all_vulnerabilities(&self) -> Result<()> {
        // 删除所有漏洞（级联删除会自动处理证据和去重索引）
        sqlx::query("DELETE FROM traffic_vulnerabilities")
            .execute(&self.pool)
            .await
            .map_err(|e| TrafficError::Database(format!("Failed to delete all vulnerabilities: {}", e)))?;

        info!("All vulnerabilities deleted");
        Ok(())
    }

    // ============================================================
    // 证据操作
    // ============================================================

    /// 插入证据
    pub async fn insert_evidence(&self, evidence: &EvidenceRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO traffic_evidence (
                id, vuln_id, url, method, location, evidence_snippet,
                request_headers, request_body, response_status, response_headers,
                response_body, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&evidence.id)
        .bind(&evidence.vuln_id)
        .bind(&evidence.url)
        .bind(&evidence.method)
        .bind(&evidence.location)
        .bind(&evidence.evidence_snippet)
        .bind(&evidence.request_headers)
        .bind(&evidence.request_body)
        .bind(&evidence.response_status)
        .bind(&evidence.response_headers)
        .bind(&evidence.response_body)
        .bind(&evidence.timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to insert evidence: {}", e)))?;

        Ok(())
    }

    // ============================================================
    // 插件注册表操作
    // ============================================================

    /// 注册插件（数据库存储代码方式）
    pub async fn register_plugin_with_code(
        &self, 
        plugin: &PluginMetadata, 
        plugin_code: &str
    ) -> Result<()> {
        // 手动创建的插件默认自动批准，不需要审核
        self.register_plugin_with_code_and_quality(plugin, plugin_code, None, Some("Approved")).await
    }

    /// Register plugin with code and optional quality score
    pub async fn register_plugin_with_code_and_quality(
        &self, 
        plugin: &PluginMetadata, 
        plugin_code: &str,
        quality_score: Option<f64>,
        validation_status: Option<&str>
    ) -> Result<()> {
        let tags_json = serde_json::to_string(&plugin.tags).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO plugin_registry (
                id, name, version, author, main_category, category, description, default_severity,
                tags, enabled, plugin_code, quality_score, validation_status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)
            "#,
        )
        .bind(&plugin.id)
        .bind(&plugin.name)
        .bind(&plugin.version)
        .bind(&plugin.author)
        .bind(&plugin.main_category)
        .bind(&plugin.category)
        .bind(&plugin.description)
        .bind(format!("{}", plugin.default_severity))
        .bind(&tags_json)
        .bind(plugin_code)
        .bind(quality_score)
        .bind(validation_status)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to register plugin with code: {}", e)))?;

        Ok(())
    }

    /// 全量更新插件（元数据 + 代码）
    pub async fn update_plugin(&self, plugin: &PluginMetadata, plugin_code: &str) -> Result<()> {
        let tags_json = serde_json::to_string(&plugin.tags).unwrap_or_default();
        
        sqlx::query(
            r#"
            UPDATE plugin_registry 
            SET name = ?, version = ?, author = ?, main_category = ?, category = ?,
                description = ?, default_severity = ?, tags = ?, plugin_code = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&plugin.name)
        .bind(&plugin.version)
        .bind(&plugin.author)
        .bind(&plugin.main_category)
        .bind(&plugin.category)
        .bind(&plugin.description)
        .bind(plugin.default_severity.to_string())
        .bind(&tags_json)
        .bind(plugin_code)
        .bind(Utc::now())
        .bind(&plugin.id)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to update plugin: {}", e)))?;

        Ok(())
    }

    /// 获取插件代码
    pub async fn get_plugin_code(&self, plugin_id: &str) -> Result<Option<String>> {
        let result = sqlx::query_scalar::<_, Option<String>>(
            r#"
            SELECT plugin_code
            FROM plugin_registry 
            WHERE id = ?
            "#,
        )
        .bind(plugin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to get plugin code: {}", e)))?;

        Ok(result.flatten())
    }

    /// 获取完整插件信息（包含代码）
    pub async fn get_plugin_by_id(&self, plugin_id: &str) -> Result<Option<serde_json::Value>> {
        let row: Option<(
            String, String, String, Option<String>, String, String, Option<String>, 
            String, Option<String>, bool, Option<String>
        )> = sqlx::query_as(
            r#"
            SELECT id, name, version, author, main_category, category, description, 
                   default_severity, tags, enabled, plugin_code
            FROM plugin_registry 
            WHERE id = ?
            "#,
        )
        .bind(plugin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to get plugin: {}", e)))?;

        if let Some((id, name, version, author, main_category, category, description, 
                     default_severity, tags, enabled, plugin_code)) = row {
            let tags_array: Vec<String> = tags
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();
            
            Ok(Some(serde_json::json!({
                "id": id,
                "name": name,
                "version": version,
                "author": author,
                "main_category": main_category,
                "category": category,
                "description": description,
                "default_severity": default_severity,
                "tags": tags_array,
                "enabled": enabled,
                "plugin_code": plugin_code
            })))
        } else {
            Ok(None)
        }
    }

    /// 查询指定类别的高质量插件（用于复用检查）
    pub async fn find_reusable_plugins_by_category(
        &self,
        category: &str,
        min_quality_score: f64,
    ) -> Result<Vec<serde_json::Value>> {
        let rows: Vec<(
            String, String, String, Option<String>, String, String, Option<String>,
            String, Option<String>, bool, Option<String>, Option<f64>, Option<String>
        )> = sqlx::query_as(
            r#"
            SELECT id, name, version, author, main_category, category, description,
                   default_severity, tags, enabled, plugin_code, quality_score, validation_status
            FROM plugin_registry
            WHERE category = ? 
              AND (quality_score IS NULL OR quality_score >= ?)
              AND validation_status IN ('Approved', 'Passed')
              AND main_category = 'traffic'
            ORDER BY quality_score DESC NULLS LAST, updated_at DESC
            LIMIT 5
            "#,
        )
        .bind(category)
        .bind(min_quality_score)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to query reusable plugins: {}", e)))?;

        let mut plugins = Vec::new();
        for (id, name, version, author, main_category, category, description,
             default_severity, tags, enabled, plugin_code, quality_score, validation_status) in rows
        {
            let tags_array: Vec<String> = tags
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();

            plugins.push(serde_json::json!({
                "id": id,
                "name": name,
                "version": version,
                "author": author,
                "main_category": main_category,
                "category": category,
                "description": description,
                "default_severity": default_severity,
                "tags": tags_array,
                "enabled": enabled,
                "plugin_code": plugin_code,
                "quality_score": quality_score,
                "validation_status": validation_status
            }));
        }

        Ok(plugins)
    }

    /// 更新插件启用状态
    pub async fn update_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE plugin_registry 
            SET enabled = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(enabled)
        .bind(Utc::now())
        .bind(plugin_id)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to update plugin status: {}", e)))?;

        Ok(())
    }

    /// 删除插件
    pub async fn delete_plugin(&self, plugin_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM plugin_registry 
            WHERE id = ?
            "#,
        )
        .bind(plugin_id)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to delete plugin: {}", e)))?;

        Ok(())
    }

    // ============================================================
    // 代理请求历史操作
    // ============================================================

    /// 插入代理请求记录
    pub async fn insert_proxy_request(&self, request: &ProxyRequestRecord) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO proxy_requests (
                url, host, protocol, method, status_code,
                request_headers, request_body, response_headers, response_body,
                response_size, response_time, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&request.url)
        .bind(&request.host)
        .bind(&request.protocol)
        .bind(&request.method)
        .bind(request.status_code)
        .bind(&request.request_headers)
        .bind(&request.request_body)
        .bind(&request.response_headers)
        .bind(&request.response_body)
        .bind(request.response_size)
        .bind(request.response_time)
        .bind(&request.timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to insert proxy request: {}", e)))?;

        Ok(result.last_insert_rowid())
    }

    /// 列出代理请求（带分页和筛选）
    pub async fn list_proxy_requests(
        &self,
        filters: ProxyRequestFilters,
    ) -> Result<Vec<ProxyRequestRecord>> {
        let mut query = String::from(
            r#"
            SELECT id, url, host, protocol, method, status_code,
                   request_headers, request_body, response_headers, response_body,
                   response_size, response_time, timestamp
            FROM proxy_requests
            WHERE 1=1
            "#,
        );

        // 动态拼接筛选条件
        if let Some(ref protocol) = filters.protocol {
            query.push_str(&format!(" AND protocol = '{}'", protocol));
        }
        if let Some(ref method) = filters.method {
            query.push_str(&format!(" AND method = '{}'", method));
        }
        if let Some(ref host) = filters.host {
            query.push_str(&format!(" AND host LIKE '%{}%'", host));
        }
        if let Some(status_min) = filters.status_code_min {
            query.push_str(&format!(" AND status_code >= {}", status_min));
        }
        if let Some(status_max) = filters.status_code_max {
            query.push_str(&format!(" AND status_code <= {}", status_max));
        }

        query.push_str(" ORDER BY timestamp DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let records = sqlx::query_as::<_, ProxyRequestRecord>(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| TrafficError::Database(format!("Failed to list proxy requests: {}", e)))?;

        Ok(records)
    }

    /// 按主机名列出代理请求（便捷方法）
    pub async fn list_proxy_requests_by_host(
        &self,
        host: &str,
        limit: i64,
    ) -> Result<Vec<ProxyRequestRecord>> {
        self.list_proxy_requests(ProxyRequestFilters {
            host: Some(host.to_string()),
            limit: Some(limit),
            ..Default::default()
        }).await
    }

    /// 统计代理请求数量
    pub async fn count_proxy_requests(&self, filters: ProxyRequestFilters) -> Result<i64> {
        let mut query = String::from(
            r#"
            SELECT COUNT(*) as count
            FROM proxy_requests
            WHERE 1=1
            "#,
        );

        // 动态拼接筛选条件
        if let Some(ref protocol) = filters.protocol {
            query.push_str(&format!(" AND protocol = '{}'", protocol));
        }
        if let Some(ref method) = filters.method {
            query.push_str(&format!(" AND method = '{}'", method));
        }
        if let Some(ref host) = filters.host {
            query.push_str(&format!(" AND host LIKE '%{}%'", host));
        }
        if let Some(status_min) = filters.status_code_min {
            query.push_str(&format!(" AND status_code >= {}", status_min));
        }
        if let Some(status_max) = filters.status_code_max {
            query.push_str(&format!(" AND status_code <= {}", status_max));
        }

        let row: (i64,) = sqlx::query_as(&query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| TrafficError::Database(format!("Failed to count proxy requests: {}", e)))?;

        Ok(row.0)
    }

    /// 根据 ID 获取单个代理请求详情
    pub async fn get_proxy_request_by_id(&self, id: i64) -> Result<Option<ProxyRequestRecord>> {
        let record = sqlx::query_as::<_, ProxyRequestRecord>(
            r#"
            SELECT id, url, host, protocol, method, status_code,
                   request_headers, request_body, response_headers, response_body,
                   response_size, response_time, timestamp
            FROM proxy_requests
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to fetch proxy request: {}", e)))?;

        Ok(record)
    }

    /// 清空所有代理请求历史
    pub async fn clear_proxy_requests(&self) -> Result<u64> {
        let mut tx = self.pool.begin().await
            .map_err(|e| TrafficError::Database(format!("Failed to begin transaction: {}", e)))?;

        let result = sqlx::query("DELETE FROM proxy_requests")
            .execute(&mut *tx)
            .await
            .map_err(|e| TrafficError::Database(format!("Failed to clear proxy requests: {}", e)))?;

        // 重置自增 ID
        sqlx::query("DELETE FROM sqlite_sequence WHERE name='proxy_requests'")
            .execute(&mut *tx)
            .await
            .map_err(|e| TrafficError::Database(format!("Failed to reset auto increment: {}", e)))?;

        tx.commit().await
            .map_err(|e| TrafficError::Database(format!("Failed to commit transaction: {}", e)))?;

        info!("Cleared {} proxy request records and reset ID", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// 删除指定时间之前的代理请求记录
    pub async fn delete_proxy_requests_before(&self, before: DateTime<Utc>) -> Result<u64> {
        let result = sqlx::query("DELETE FROM proxy_requests WHERE timestamp < ?")
            .bind(before)
            .execute(&self.pool)
            .await
            .map_err(|e| TrafficError::Database(format!("Failed to delete old proxy requests: {}", e)))?;

        info!("Deleted {} old proxy request records", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// 保存配置项
    pub async fn save_config(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO proxy_config (key, value, updated_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to save config: {}", e)))?;

        info!("Saved config: {} = {}", key, value);
        Ok(())
    }

    /// 加载配置项
    pub async fn load_config(&self, key: &str) -> Result<Option<String>> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM proxy_config WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to load config: {}", e)))?;

        Ok(result.map(|(value,)| value))
    }

    /// 删除配置项
    pub async fn delete_config(&self, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM proxy_config WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| TrafficError::Database(format!("Failed to delete config: {}", e)))?;

        info!("Deleted config: {}", key);
        Ok(())
    }
}

// ============================================================
// 数据结构
// ============================================================

/// 漏洞筛选条件
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VulnerabilityFilters {
    pub vuln_type: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub plugin_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 漏洞记录（数据库）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VulnerabilityRecord {
    pub id: String,
    pub plugin_id: String,
    pub vuln_type: String,
    pub severity: String,
    pub confidence: String,
    pub title: String,
    pub description: String,
    pub cwe: Option<String>,
    pub owasp: Option<String>,
    pub remediation: Option<String>,
    pub status: String,
    pub signature: String,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub hit_count: i64,
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 证据记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EvidenceRecord {
    pub id: String,
    pub vuln_id: String,
    pub url: String,
    pub method: String,
    pub location: String,
    pub evidence_snippet: String,
    pub request_headers: Option<String>,
    pub request_body: Option<String>,
    pub response_status: Option<i32>,
    pub response_headers: Option<String>,
    pub response_body: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// 漏洞记录（带证据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityWithEvidence {
    #[serde(flatten)]
    pub vulnerability: VulnerabilityRecord,
    pub evidence: Vec<EvidenceRecord>,
    // 从第一条证据中提取的 URL 和方法（用于显示）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

/// 代理请求记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProxyRequestRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub url: String,
    pub host: String,
    pub protocol: String,
    pub method: String,
    pub status_code: i32,
    pub request_headers: Option<String>,
    pub request_body: Option<String>,
    pub response_headers: Option<String>,
    pub response_body: Option<String>,
    pub response_size: i64,
    pub response_time: i64,
    pub timestamp: DateTime<Utc>,
}

/// 代理请求筛选条件
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyRequestFilters {
    pub protocol: Option<String>,
    pub method: Option<String>,
    pub host: Option<String>,
    pub status_code_min: Option<i32>,
    pub status_code_max: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

