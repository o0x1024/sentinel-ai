use anyhow::Result;
use crate::core::models::database::{Configuration, NotificationRule, McpServerConfig};
use crate::core::models::rag_config::RagConfig;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    pub async fn get_rag_config_internal(&self) -> Result<Option<RagConfig>> {
        let value = self.get_config_internal("rag", "config").await?;
        if let Some(v) = value {
            Ok(Some(serde_json::from_str(&v)?))
        } else {
            Ok(None)
        }
    }

    pub async fn save_rag_config_internal(&self, config: &RagConfig) -> Result<()> {
        let value = serde_json::to_string(config)?;
        self.set_config_internal("rag", "config", &value, Some("RAG配置")).await?;
        Ok(())
    }

    pub async fn get_config_internal(&self, category: &str, key: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;

        let value: Option<String> =
            sqlx::query_scalar("SELECT value FROM configurations WHERE category = $1 AND key = $2")
                .bind(category)
                .bind(key)
                .fetch_optional(pool)
                .await?;

        Ok(value)
    }

    pub async fn set_config_internal(
        &self,
        category: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        // Generate ID from category and key for consistency
        let id = format!("{}:{}", category, key);
        sqlx::query(
            "INSERT INTO configurations (id, category, key, value, description) VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT(category, key) DO UPDATE SET value = excluded.value, description = excluded.description, updated_at = CURRENT_TIMESTAMP"
        )
        .bind(&id)
        .bind(category)
        .bind(key)
        .bind(value)
        .bind(description)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_configs_by_category_internal(&self, category: &str) -> Result<Vec<Configuration>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query_as::<_, Configuration>(
            "SELECT * FROM configurations WHERE category = $1 ORDER BY key",
        )
        .bind(category)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn create_notification_rule_internal(&self, rule: &NotificationRule) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"INSERT INTO notification_rules (id, name, description, channel, config, is_encrypted, enabled, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(&rule.channel)
        .bind(&rule.config)
        .bind(rule.is_encrypted)
        .bind(rule.enabled)
        .bind(rule.created_at)
        .bind(rule.updated_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_notification_rules_internal(&self) -> Result<Vec<NotificationRule>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query_as::<_, NotificationRule>(
            "SELECT * FROM notification_rules ORDER BY updated_at DESC"
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_notification_rule_internal(&self, id: &str) -> Result<Option<NotificationRule>> {
        let pool = self.get_pool()?;
        let row = sqlx::query_as::<_, NotificationRule>(
            "SELECT * FROM notification_rules WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    pub async fn update_notification_rule_internal(&self, rule: &NotificationRule) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"UPDATE notification_rules
               SET name = $1, description = $2, channel = $3, config = $4, is_encrypted = $5, enabled = $6, updated_at = $7
               WHERE id = $8"#
        )
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(&rule.channel)
        .bind(&rule.config)
        .bind(rule.is_encrypted)
        .bind(rule.enabled)
        .bind(chrono::Utc::now())
        .bind(&rule.id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete_notification_rule_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM notification_rules WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn create_mcp_server_config_internal(
        &self,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
    ) -> Result<String> {
        let args_json = serde_json::to_string(args)?;
        let pool = self.get_pool()?;
        let id = uuid::Uuid::new_v4().to_string();

        let url = "http://localhost:8080".to_string();
        let connection_type = "stdio";

        sqlx::query(
            r#"
            INSERT INTO mcp_server_configs (id, name, description, url, connection_type, command, args)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(&url)
        .bind(connection_type)
        .bind(command)
        .bind(args_json)
        .execute(pool)
        .await?;
        Ok(id)
    }

    pub async fn get_all_mcp_server_configs_internal(&self) -> Result<Vec<McpServerConfig>> {
        let pool = self.get_pool()?;
        let configs = sqlx::query_as::<_, McpServerConfig>(
            "SELECT id, name, description, url, connection_type, command, args, is_enabled as enabled, COALESCE(auto_connect, FALSE) as auto_connect, created_at, updated_at FROM mcp_server_configs",
        )
        .fetch_all(pool)
        .await?;
        Ok(configs)
    }
    
    pub async fn get_auto_connect_mcp_servers_internal(&self) -> Result<Vec<McpServerConfig>> {
        let pool = self.get_pool()?;
        let configs = sqlx::query_as::<_, McpServerConfig>(
            "SELECT id, name, description, url, connection_type, command, args, is_enabled as enabled, COALESCE(auto_connect, FALSE) as auto_connect, created_at, updated_at FROM mcp_server_configs WHERE auto_connect = TRUE"
        )
        .fetch_all(pool)
        .await?;
        Ok(configs)
    }
    
    pub async fn update_mcp_server_auto_connect_internal(&self, id: &str, auto_connect: bool) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE mcp_server_configs SET auto_connect = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
            .bind(auto_connect)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn update_mcp_server_config_enabled_internal(&self, id: &str, enabled: bool) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE mcp_server_configs SET is_enabled = $1 WHERE id = $2")
            .bind(enabled)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete_mcp_server_config_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM mcp_server_configs WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_mcp_server_config_by_name_internal(
        &self,
        name: &str,
    ) -> Result<Option<McpServerConfig>> {
        let pool = self.get_pool()?;
        let config = sqlx::query_as::<_, McpServerConfig>(
            "SELECT id, name, description, url, connection_type, command, args, is_enabled as enabled, COALESCE(auto_connect, FALSE) as auto_connect, created_at, updated_at FROM mcp_server_configs WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;
        Ok(config)
    }

    pub async fn update_mcp_server_config_internal(
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

        let existing = self.get_mcp_server_config_by_name_internal(name).await?;

        let url = existing
            .as_ref()
            .map(|c| c.url.clone())
            .unwrap_or_else(|| "http://localhost:8080".to_string());
        let connection_type = existing
            .as_ref()
            .map(|c| c.connection_type.clone())
            .unwrap_or_else(|| "stdio".to_string());

        sqlx::query(
            "UPDATE mcp_server_configs SET name = $1, description = $2, url = $3, connection_type = $4, command = $5, args = $6, is_enabled = $7, updated_at = CURRENT_TIMESTAMP WHERE id = $8",
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

    pub async fn get_subdomain_dictionary_internal(&self) -> Result<Vec<String>> {
        let pool = self.get_pool()?;

        if let Some(default_dict_id) = self
            .get_config_internal("dictionary_default", "subdomain")
            .await?
            .filter(|s| !s.is_empty())
        {
            let words: Vec<String> = sqlx::query_scalar(
                r#"SELECT word FROM dictionary_words 
                   WHERE dictionary_id = $1 
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

        if let Some(candidate_id) = sqlx::query_scalar::<_, String>(
            r#"SELECT id FROM dictionaries 
               WHERE dict_type = 'subdomain' AND is_active = TRUE 
               ORDER BY is_builtin DESC, updated_at DESC 
               LIMIT 1"#,
        )
        .fetch_optional(pool)
        .await?
        {
            let words: Vec<String> = sqlx::query_scalar(
                r#"SELECT word FROM dictionary_words 
                   WHERE dictionary_id = $1 
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

        Ok(self.get_default_subdomain_dictionary())
    }

    pub async fn set_subdomain_dictionary_internal(&self, dictionary: &[String]) -> Result<()> {
        let dictionary_json = serde_json::to_string(dictionary)?;
        self.set_config_internal(
            "subdomain_scanner",
            "dictionary",
            &dictionary_json,
            Some("子域名扫描字典"),
        )
        .await?;
        Ok(())
    }

    pub async fn add_subdomain_words_internal(&self, words: &[String]) -> Result<()> {
        let mut current_dict = self.get_subdomain_dictionary_internal().await?;

        for word in words {
            if !current_dict.contains(word) {
                current_dict.push(word.clone());
            }
        }

        current_dict.sort();
        self.set_subdomain_dictionary_internal(&current_dict).await?;
        Ok(())
    }

    pub async fn remove_subdomain_words_internal(&self, words: &[String]) -> Result<()> {
        let mut current_dict = self.get_subdomain_dictionary_internal().await?;

        current_dict.retain(|word| !words.contains(word));

        self.set_subdomain_dictionary_internal(&current_dict).await?;
        Ok(())
    }

    fn get_default_subdomain_dictionary(&self) -> Vec<String> {
        vec![
            "www".to_string(), "mail".to_string(), "ftp".to_string(), "localhost".to_string(),
            "webmail".to_string(), "smtp".to_string(), "pop".to_string(), "ns1".to_string(),
            "webdisk".to_string(), "ns2".to_string(), "cpanel".to_string(), "whm".to_string(),
            "autodiscover".to_string(), "autoconfig".to_string(), "m".to_string(), "imap".to_string(),
            "test".to_string(), "ns".to_string(), "blog".to_string(), "pop3".to_string(),
            "dev".to_string(), "www2".to_string(), "admin".to_string(), "forum".to_string(),
            "news".to_string(), "vpn".to_string(), "ns3".to_string(), "mail2".to_string(),
            "new".to_string(), "mysql".to_string(), "old".to_string(), "lists".to_string(),
            "support".to_string(), "mobile".to_string(), "static".to_string(), "docs".to_string(),
            "beta".to_string(), "shop".to_string(), "sql".to_string(), "secure".to_string(),
            "demo".to_string(), "cp".to_string(), "calendar".to_string(), "wiki".to_string(),
            "web".to_string(), "media".to_string(), "email".to_string(), "images".to_string(),
            "img".to_string(), "www1".to_string(), "intranet".to_string(), "portal".to_string(),
            "video".to_string(), "sip".to_string(), "dns2".to_string(), "api".to_string(),
            "cdn".to_string(), "stats".to_string(), "dns1".to_string(), "ns4".to_string(),
            "www3".to_string(), "dns".to_string(), "search".to_string(), "staging".to_string(),
            "server".to_string(), "mx".to_string(), "chat".to_string(), "en".to_string(),
            "wap".to_string(), "redmine".to_string(), "ftp2".to_string(), "db".to_string(),
            "erp".to_string(), "explore".to_string(), "download".to_string(), "ww1".to_string(),
            "catalog".to_string(), "ssh".to_string(), "management".to_string(), "www4".to_string(),
        ]
    }
}
