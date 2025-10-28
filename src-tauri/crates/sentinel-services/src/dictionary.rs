use anyhow::Result;
use sqlx::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::dictionary::{
    Dictionary, DictionaryExport, DictionaryFilter, DictionaryImportOptions, DictionarySet,
    DictionarySetRelation, DictionaryStats, DictionaryType, DictionaryWord, MergeMode, ServiceType,
};

/// 字典服务
#[derive(Debug, Clone)]
pub struct DictionaryService {
    pool: SqlitePool,
}

impl DictionaryService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 创建字典
    pub async fn create_dictionary(&self, mut dictionary: Dictionary) -> Result<Dictionary> {
        if dictionary.id.is_empty() {
            dictionary.id = Uuid::new_v4().to_string();
        }

        let now = chrono::Utc::now().to_rfc3339();
        dictionary.created_at = now.clone();
        dictionary.updated_at = now;

        sqlx::query(
            r#"
            INSERT INTO dictionaries (
                id, name, description, dict_type, service_type, category,
                is_builtin, is_active, word_count, file_size, checksum,
                version, author, source_url, tags, metadata,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&dictionary.id)
        .bind(&dictionary.name)
        .bind(&dictionary.description)
        .bind(&dictionary.dict_type)
        .bind(&dictionary.service_type)
        .bind(&dictionary.category)
        .bind(dictionary.is_builtin)
        .bind(dictionary.is_active)
        .bind(dictionary.word_count)
        .bind(dictionary.file_size)
        .bind(&dictionary.checksum)
        .bind(&dictionary.version)
        .bind(&dictionary.author)
        .bind(&dictionary.source_url)
        .bind(&dictionary.tags)
        .bind(&dictionary.metadata)
        .bind(&dictionary.created_at)
        .bind(&dictionary.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(dictionary)
    }

    /// 获取字典
    pub async fn get_dictionary(&self, id: &str) -> Result<Option<Dictionary>> {
        let dictionary = sqlx::query_as::<_, Dictionary>("SELECT * FROM dictionaries WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(dictionary)
    }

    /// 根据名称获取字典
    pub async fn get_dictionary_by_name(&self, name: &str) -> Result<Option<Dictionary>> {
        let dictionary =
            sqlx::query_as::<_, Dictionary>("SELECT * FROM dictionaries WHERE name = ?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;

        Ok(dictionary)
    }

    /// 获取字典列表
    pub async fn list_dictionaries(
        &self,
        filter: Option<DictionaryFilter>,
    ) -> Result<Vec<Dictionary>> {
        let mut query = "SELECT * FROM dictionaries WHERE 1=1".to_string();
        let mut params: Vec<String> = Vec::new();

        if let Some(filter) = filter {
            if let Some(dict_type) = filter.dict_type {
                query.push_str(" AND dict_type = ?");
                params.push(dict_type.to_string());
            }
            if let Some(service_type) = filter.service_type {
                query.push_str(" AND service_type = ?");
                params.push(service_type.to_string());
            }
            if let Some(category) = filter.category {
                query.push_str(" AND category = ?");
                params.push(category);
            }
            if let Some(is_builtin) = filter.is_builtin {
                query.push_str(" AND is_builtin = ?");
                params.push(is_builtin.to_string());
            }
            if let Some(is_active) = filter.is_active {
                query.push_str(" AND is_active = ?");
                params.push(is_active.to_string());
            }
            if let Some(search_term) = filter.search_term {
                query.push_str(" AND (name LIKE ? OR description LIKE ?)");
                let search_pattern = format!("%{}%", search_term);
                params.push(search_pattern.clone());
                params.push(search_pattern);
            }
        }

        query.push_str(" ORDER BY created_at DESC");

        let mut sql_query = sqlx::query_as::<_, Dictionary>(&query);
        for param in params {
            sql_query = sql_query.bind(param);
        }

        let dictionaries = sql_query.fetch_all(&self.pool).await?;
        Ok(dictionaries)
    }

    /// 更新字典
    pub async fn update_dictionary(&self, mut dictionary: Dictionary) -> Result<Dictionary> {
        dictionary.updated_at = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE dictionaries SET
                name = ?, description = ?, dict_type = ?, service_type = ?,
                category = ?, is_builtin = ?, is_active = ?, word_count = ?,
                file_size = ?, checksum = ?, version = ?, author = ?,
                source_url = ?, tags = ?, metadata = ?, updated_at = ?
            WHERE id = ?
        "#,
        )
        .bind(&dictionary.name)
        .bind(&dictionary.description)
        .bind(&dictionary.dict_type)
        .bind(&dictionary.service_type)
        .bind(&dictionary.category)
        .bind(dictionary.is_builtin)
        .bind(dictionary.is_active)
        .bind(dictionary.word_count)
        .bind(dictionary.file_size)
        .bind(&dictionary.checksum)
        .bind(&dictionary.version)
        .bind(&dictionary.author)
        .bind(&dictionary.source_url)
        .bind(&dictionary.tags)
        .bind(&dictionary.metadata)
        .bind(&dictionary.updated_at)
        .bind(&dictionary.id)
        .execute(&self.pool)
        .await?;

        Ok(dictionary)
    }

    /// 删除字典
    pub async fn delete_dictionary(&self, id: &str) -> Result<()> {
        // 先删除相关的词条
        sqlx::query("DELETE FROM dictionary_words WHERE dictionary_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        // 删除字典集合关系
        sqlx::query("DELETE FROM dictionary_set_relations WHERE dictionary_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        // 删除字典
        sqlx::query("DELETE FROM dictionaries WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// 添加词条到字典
    pub async fn add_words(
        &self,
        dictionary_id: &str,
        words: Vec<String>,
    ) -> Result<Vec<DictionaryWord>> {
        let mut added_words = Vec::new();

        for word in words {
            let dict_word = DictionaryWord::new(dictionary_id.to_string(), word);

            sqlx::query(r#"
                INSERT INTO dictionary_words (id, dictionary_id, word, weight, category, metadata, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            "#)
            .bind(&dict_word.id)
            .bind(&dict_word.dictionary_id)
            .bind(&dict_word.word)
            .bind(dict_word.weight)
            .bind(&dict_word.category)
            .bind(&dict_word.metadata)
            .bind(&dict_word.created_at)
            .execute(&self.pool)
            .await?;

            added_words.push(dict_word);
        }

        // 更新字典的词条数量
        self.update_word_count(dictionary_id).await?;

        Ok(added_words)
    }

    /// 从字典中移除词条
    pub async fn remove_words(&self, dictionary_id: &str, words: Vec<String>) -> Result<u64> {
        let mut removed_count = 0;

        for word in words {
            let result =
                sqlx::query("DELETE FROM dictionary_words WHERE dictionary_id = ? AND word = ?")
                    .bind(dictionary_id)
                    .bind(&word)
                    .execute(&self.pool)
                    .await?;

            removed_count += result.rows_affected();
        }

        // 更新字典的词条数量
        self.update_word_count(dictionary_id).await?;

        Ok(removed_count)
    }

    /// 获取字典的所有词条
    pub async fn get_dictionary_words(&self, dictionary_id: &str) -> Result<Vec<DictionaryWord>> {
        let words = sqlx::query_as::<_, DictionaryWord>(
            "SELECT * FROM dictionary_words WHERE dictionary_id = ? ORDER BY weight DESC, word ASC",
        )
        .bind(dictionary_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(words)
    }

    /// 分页获取字典词条
    pub async fn get_dictionary_words_paged(
        &self,
        dictionary_id: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<DictionaryWord>> {
        let words = sqlx::query_as::<_, DictionaryWord>(
            "SELECT * FROM dictionary_words WHERE dictionary_id = ? ORDER BY weight DESC, word ASC LIMIT ? OFFSET ?",
        )
        .bind(dictionary_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(words)
    }

    /// 搜索词条
    pub async fn search_words(
        &self,
        dictionary_id: &str,
        pattern: &str,
        limit: Option<u32>,
    ) -> Result<Vec<DictionaryWord>> {
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
        let query = format!(
            "SELECT * FROM dictionary_words WHERE dictionary_id = ? AND word LIKE ? ORDER BY weight DESC, word ASC{}",
            limit_clause
        );

        let search_pattern = format!("%{}%", pattern);
        let words = sqlx::query_as::<_, DictionaryWord>(&query)
            .bind(dictionary_id)
            .bind(search_pattern)
            .fetch_all(&self.pool)
            .await?;

        Ok(words)
    }

    /// 搜索词条（分页版，支持 OFFSET）
    pub async fn search_words_paged(
        &self,
        dictionary_id: &str,
        pattern: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<DictionaryWord>> {
        let query =
            "SELECT * FROM dictionary_words WHERE dictionary_id = ? AND word LIKE ? ORDER BY weight DESC, word ASC LIMIT ? OFFSET ?";

        let search_pattern = format!("%{}%", pattern);
        let words = sqlx::query_as::<_, DictionaryWord>(query)
            .bind(dictionary_id)
            .bind(search_pattern)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await?;

        Ok(words)
    }

    /// 清空字典词条
    pub async fn clear_dictionary(&self, dictionary_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM dictionary_words WHERE dictionary_id = ?")
            .bind(dictionary_id)
            .execute(&self.pool)
            .await?;

        // 更新字典的词条数量
        self.update_word_count(dictionary_id).await?;

        Ok(result.rows_affected())
    }

    /// 更新字典的词条数量
    async fn update_word_count(&self, dictionary_id: &str) -> Result<()> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM dictionary_words WHERE dictionary_id = ?")
                .bind(dictionary_id)
                .fetch_one(&self.pool)
                .await?;

        sqlx::query("UPDATE dictionaries SET word_count = ?, updated_at = ? WHERE id = ?")
            .bind(count)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(dictionary_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// 导出字典
    pub async fn export_dictionary(&self, dictionary_id: &str) -> Result<DictionaryExport> {
        let dictionary = self
            .get_dictionary(dictionary_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Dictionary not found"))?;

        let words = self.get_dictionary_words(dictionary_id).await?;

        Ok(DictionaryExport::new(dictionary, words))
    }

    /// 导入字典
    pub async fn import_dictionary(
        &self,
        export_data: DictionaryExport,
        options: DictionaryImportOptions,
    ) -> Result<Dictionary> {
        let mut dictionary = export_data.dictionary;

        match options.merge_mode {
            MergeMode::CreateNew => {
                // 创建新字典
                dictionary.id = Uuid::new_v4().to_string();
                dictionary.name = format!("{}_imported", dictionary.name);
                let created_dict = self.create_dictionary(dictionary).await?;

                // 添加词条
                let words: Vec<String> = export_data.words.into_iter().map(|w| w.word).collect();
                self.add_words(&created_dict.id, words).await?;

                Ok(created_dict)
            }
            MergeMode::Replace => {
                // 查找现有字典
                if let Some(existing) = self.get_dictionary_by_name(&dictionary.name).await? {
                    // 清空现有词条
                    self.clear_dictionary(&existing.id).await?;

                    // 更新字典信息
                    if options.update_metadata {
                        let mut updated_dict = existing;
                        updated_dict.description = dictionary.description;
                        updated_dict.category = dictionary.category;
                        updated_dict.tags = dictionary.tags;
                        updated_dict.metadata = dictionary.metadata;
                        if let Some(author) = dictionary.author {
                            updated_dict.author = Some(author);
                        }
                        if let Some(source_url) = dictionary.source_url {
                            updated_dict.source_url = Some(source_url);
                        }
                        dictionary = self.update_dictionary(updated_dict).await?;
                    } else {
                        dictionary = existing;
                    }

                    // 添加新词条
                    let words: Vec<String> =
                        export_data.words.into_iter().map(|w| w.word).collect();
                    self.add_words(&dictionary.id, words).await?;
                } else {
                    // 字典不存在，创建新的
                    dictionary = self.create_dictionary(dictionary).await?;
                    let words: Vec<String> =
                        export_data.words.into_iter().map(|w| w.word).collect();
                    self.add_words(&dictionary.id, words).await?;
                }

                Ok(dictionary)
            }
            MergeMode::Merge => {
                // 合并到现有字典
                if let Some(existing) = self.get_dictionary_by_name(&dictionary.name).await? {
                    dictionary = existing;

                    // 获取现有词条
                    let existing_words = self.get_dictionary_words(&dictionary.id).await?;
                    let existing_word_set: std::collections::HashSet<String> =
                        existing_words.into_iter().map(|w| w.word).collect();

                    // 过滤重复词条
                    let new_words: Vec<String> = export_data
                        .words
                        .into_iter()
                        .map(|w| w.word)
                        .filter(|word| {
                            !options.skip_duplicates || !existing_word_set.contains(word)
                        })
                        .collect();

                    if !new_words.is_empty() {
                        self.add_words(&dictionary.id, new_words).await?;
                    }
                } else {
                    // 字典不存在，创建新的
                    dictionary = self.create_dictionary(dictionary).await?;
                    let words: Vec<String> =
                        export_data.words.into_iter().map(|w| w.word).collect();
                    self.add_words(&dictionary.id, words).await?;
                }

                Ok(dictionary)
            }
        }
    }

    /// 获取字典统计信息
    pub async fn get_stats(&self) -> Result<DictionaryStats> {
        let total_dictionaries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dictionaries")
            .fetch_one(&self.pool)
            .await?;

        let total_words: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dictionary_words")
            .fetch_one(&self.pool)
            .await?;

        let builtin_dictionaries: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM dictionaries WHERE is_builtin = 1")
                .fetch_one(&self.pool)
                .await?;

        let custom_dictionaries: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM dictionaries WHERE is_builtin = 0")
                .fetch_one(&self.pool)
                .await?;

        let active_dictionaries: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM dictionaries WHERE is_active = 1")
                .fetch_one(&self.pool)
                .await?;

        let total_sets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dictionary_sets")
            .fetch_one(&self.pool)
            .await?;

        // 按类型统计
        let type_stats: Vec<(String, i64)> = sqlx::query_as(
            "SELECT dict_type, COUNT(*) as count FROM dictionaries GROUP BY dict_type",
        )
        .fetch_all(&self.pool)
        .await?;

        let by_type: HashMap<String, f64> = type_stats
            .into_iter()
            .map(|(dict_type, count)| (dict_type, count as f64))
            .collect();

        // 按服务类型统计
        let service_stats: Vec<(Option<String>, i64)> = sqlx::query_as(
            "SELECT service_type, COUNT(*) as count FROM dictionaries GROUP BY service_type",
        )
        .fetch_all(&self.pool)
        .await?;

        let by_service: HashMap<String, f64> = service_stats
            .into_iter()
            .map(|(service_type, count)| {
                let service = service_type.unwrap_or_else(|| "unknown".to_string());
                (service, count as f64)
            })
            .collect();

        Ok(DictionaryStats {
            total_dictionaries: total_dictionaries as f64,
            total_words: total_words as f64,
            builtin_dictionaries: builtin_dictionaries as f64,
            custom_dictionaries: custom_dictionaries as f64,
            active_dictionaries: active_dictionaries as f64,
            total_sets: total_sets as f64,
            by_type,
            by_service,
        })
    }

    /// 创建字典集合
    pub async fn create_dictionary_set(&self, mut set: DictionarySet) -> Result<DictionarySet> {
        if set.id.is_empty() {
            set.id = Uuid::new_v4().to_string();
        }

        let now = chrono::Utc::now().to_rfc3339();
        set.created_at = now.clone();
        set.updated_at = now;

        sqlx::query(r#"
            INSERT INTO dictionary_sets (id, name, description, service_type, scenario, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&set.id)
        .bind(&set.name)
        .bind(&set.description)
        .bind(&set.service_type)
        .bind(&set.scenario)
        .bind(set.is_active)
        .bind(&set.created_at)
        .bind(&set.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(set)
    }

    /// 向字典集合添加字典
    pub async fn add_dictionary_to_set(
        &self,
        set_id: &str,
        dictionary_id: &str,
        priority: Option<i32>,
    ) -> Result<DictionarySetRelation> {
        let relation = DictionarySetRelation::new(set_id.to_string(), dictionary_id.to_string())
            .with_priority(priority.unwrap_or(0));

        sqlx::query(r#"
            INSERT INTO dictionary_set_relations (id, set_id, dictionary_id, priority, is_enabled, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
        "#)
        .bind(&relation.id)
        .bind(&relation.set_id)
        .bind(&relation.dictionary_id)
        .bind(relation.priority)
        .bind(relation.is_enabled)
        .bind(&relation.created_at)
        .execute(&self.pool)
        .await?;

        Ok(relation)
    }

    /// 获取字典集合中的所有字典
    pub async fn get_set_dictionaries(&self, set_id: &str) -> Result<Vec<Dictionary>> {
        let dictionaries = sqlx::query_as::<_, Dictionary>(
            r#"
            SELECT d.* FROM dictionaries d
            JOIN dictionary_set_relations r ON d.id = r.dictionary_id
            WHERE r.set_id = ? AND r.is_enabled = 1
            ORDER BY r.priority DESC, d.name ASC
        "#,
        )
        .bind(set_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(dictionaries)
    }

    /// 初始化内置字典
    pub async fn initialize_builtin_dictionaries(&self) -> Result<()> {
        // 创建默认的子域名字典
        let subdomain_dict = Dictionary {
            id: "builtin_subdomain_common".to_string(),
            name: "Common Subdomains".to_string(),
            description: Some("Common subdomain names for reconnaissance".to_string()),
            dict_type: DictionaryType::Subdomain.to_string(),
            service_type: Some(ServiceType::Web.to_string()),
            category: Some("reconnaissance".to_string()),
            is_builtin: true,
            is_active: true,
            word_count: 0,
            file_size: 0,
            checksum: None,
            version: "1.0.0".to_string(),
            author: Some("Sentinel AI".to_string()),
            source_url: None,
            tags: Some("subdomain,reconnaissance,common".to_string()),
            metadata: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        // 检查是否已存在
        if self.get_dictionary(&subdomain_dict.id).await?.is_none() {
            self.create_dictionary(subdomain_dict).await?;

            // 添加常见子域名
            let common_subdomains = vec![
                "www",
                "mail",
                "ftp",
                "admin",
                "api",
                "dev",
                "test",
                "staging",
                "blog",
                "shop",
                "store",
                "support",
                "help",
                "docs",
                "cdn",
                "static",
                "assets",
                "img",
                "images",
                "media",
                "files",
                "download",
                "upload",
                "secure",
                "ssl",
                "vpn",
                "remote",
                "portal",
                "dashboard",
                "panel",
                "control",
                "manage",
                "login",
                "auth",
                "sso",
                "oauth",
                "app",
                "mobile",
                "m",
                "wap",
                "beta",
                "alpha",
                "demo",
                "sandbox",
                "old",
                "legacy",
                "archive",
                "backup",
                "mirror",
                "proxy",
                "cache",
                "db",
                "database",
                "sql",
                "mysql",
                "postgres",
                "redis",
                "mongo",
                "elastic",
                "search",
                "solr",
                "kibana",
                "grafana",
                "prometheus",
                "jenkins",
                "ci",
                "cd",
                "build",
                "deploy",
                "git",
                "svn",
                "repo",
                "jira",
                "confluence",
                "wiki",
                "forum",
                "chat",
                "slack",
                "teams",
                "monitoring",
                "metrics",
                "logs",
                "analytics",
                "stats",
                "reports",
            ];

            self.add_words(
                "builtin_subdomain_common",
                common_subdomains
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            )
            .await?;
        }

        tracing::info!("Builtin dictionaries initialized");
        Ok(())
    }
}
