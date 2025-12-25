use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use crate::database_service::service::DatabaseService;

/// 代理服务器记录
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProxifierProxyRecord {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub proxy_type: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub enabled: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 代理规则记录
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProxifierRuleRecord {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub applications: String,
    pub target_hosts: String,
    pub target_ports: String,
    pub action: String,
    pub proxy_id: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl DatabaseService {
    // ============================================================================
    // 代理服务器操作
    // ============================================================================

    /// 获取所有代理服务器
    pub async fn get_all_proxies_internal(&self) -> Result<Vec<ProxifierProxyRecord>> {
        let pool = self.get_pool()?;
        let proxies = sqlx::query_as::<_, ProxifierProxyRecord>(
            "SELECT * FROM proxifier_proxies ORDER BY sort_order, created_at"
        )
        .fetch_all(pool)
        .await?;
        Ok(proxies)
    }

    /// 获取单个代理服务器
    pub async fn get_proxy_by_id_internal(&self, id: &str) -> Result<Option<ProxifierProxyRecord>> {
        let pool = self.get_pool()?;
        let proxy = sqlx::query_as::<_, ProxifierProxyRecord>(
            "SELECT * FROM proxifier_proxies WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(proxy)
    }

    /// 创建代理服务器
    pub async fn create_proxy_internal(
        &self,
        id: &str,
        name: &str,
        host: &str,
        port: u16,
        proxy_type: &str,
        username: Option<&str>,
        password: Option<&str>,
        enabled: bool,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"
            INSERT INTO proxifier_proxies (id, name, host, port, proxy_type, username, password, enabled, sort_order)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM proxifier_proxies))
            "#
        )
        .bind(id)
        .bind(name)
        .bind(host)
        .bind(port as i64)
        .bind(proxy_type)
        .bind(username)
        .bind(password)
        .bind(enabled)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// 更新代理服务器
    pub async fn update_proxy_internal(
        &self,
        id: &str,
        name: &str,
        host: &str,
        port: u16,
        proxy_type: &str,
        username: Option<&str>,
        password: Option<&str>,
        enabled: bool,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"
            UPDATE proxifier_proxies 
            SET name = ?, host = ?, port = ?, proxy_type = ?, username = ?, password = ?, enabled = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#
        )
        .bind(name)
        .bind(host)
        .bind(port as i64)
        .bind(proxy_type)
        .bind(username)
        .bind(password)
        .bind(enabled)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// 删除代理服务器
    pub async fn delete_proxy_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM proxifier_proxies WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// 批量保存代理服务器（先删除所有再插入）
    pub async fn save_all_proxies_internal(&self, proxies: &[ProxifierProxyRecord]) -> Result<()> {
        let pool = self.get_pool()?;
        let mut tx = pool.begin().await?;
        
        // 删除所有现有代理
        sqlx::query("DELETE FROM proxifier_proxies")
            .execute(&mut *tx)
            .await?;
        
        // 插入新代理
        for (idx, proxy) in proxies.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO proxifier_proxies (id, name, host, port, proxy_type, username, password, enabled, sort_order)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&proxy.id)
            .bind(&proxy.name)
            .bind(&proxy.host)
            .bind(proxy.port)
            .bind(&proxy.proxy_type)
            .bind(&proxy.username)
            .bind(&proxy.password)
            .bind(proxy.enabled)
            .bind(idx as i64)
            .execute(&mut *tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }

    // ============================================================================
    // 代理规则操作
    // ============================================================================

    /// 获取所有代理规则
    pub async fn get_all_rules_internal(&self) -> Result<Vec<ProxifierRuleRecord>> {
        let pool = self.get_pool()?;
        let rules = sqlx::query_as::<_, ProxifierRuleRecord>(
            "SELECT * FROM proxifier_rules ORDER BY sort_order, created_at"
        )
        .fetch_all(pool)
        .await?;
        Ok(rules)
    }

    /// 获取单个代理规则
    pub async fn get_rule_by_id_internal(&self, id: &str) -> Result<Option<ProxifierRuleRecord>> {
        let pool = self.get_pool()?;
        let rule = sqlx::query_as::<_, ProxifierRuleRecord>(
            "SELECT * FROM proxifier_rules WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(rule)
    }

    /// 创建代理规则
    pub async fn create_rule_internal(
        &self,
        id: &str,
        name: &str,
        enabled: bool,
        applications: &str,
        target_hosts: &str,
        target_ports: &str,
        action: &str,
        proxy_id: Option<&str>,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"
            INSERT INTO proxifier_rules (id, name, enabled, applications, target_hosts, target_ports, action, proxy_id, sort_order)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM proxifier_rules))
            "#
        )
        .bind(id)
        .bind(name)
        .bind(enabled)
        .bind(applications)
        .bind(target_hosts)
        .bind(target_ports)
        .bind(action)
        .bind(proxy_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// 更新代理规则
    pub async fn update_rule_internal(
        &self,
        id: &str,
        name: &str,
        enabled: bool,
        applications: &str,
        target_hosts: &str,
        target_ports: &str,
        action: &str,
        proxy_id: Option<&str>,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"
            UPDATE proxifier_rules 
            SET name = ?, enabled = ?, applications = ?, target_hosts = ?, target_ports = ?, action = ?, proxy_id = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#
        )
        .bind(name)
        .bind(enabled)
        .bind(applications)
        .bind(target_hosts)
        .bind(target_ports)
        .bind(action)
        .bind(proxy_id)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// 删除代理规则
    pub async fn delete_rule_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM proxifier_rules WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// 批量保存代理规则（先删除所有再插入）
    pub async fn save_all_rules_internal(&self, rules: &[ProxifierRuleRecord]) -> Result<()> {
        let pool = self.get_pool()?;
        let mut tx = pool.begin().await?;
        
        // 删除所有现有规则
        sqlx::query("DELETE FROM proxifier_rules")
            .execute(&mut *tx)
            .await?;
        
        // 插入新规则
        for (idx, rule) in rules.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO proxifier_rules (id, name, enabled, applications, target_hosts, target_ports, action, proxy_id, sort_order)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&rule.id)
            .bind(&rule.name)
            .bind(rule.enabled)
            .bind(&rule.applications)
            .bind(&rule.target_hosts)
            .bind(&rule.target_ports)
            .bind(&rule.action)
            .bind(&rule.proxy_id)
            .bind(idx as i64)
            .execute(&mut *tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }
}

