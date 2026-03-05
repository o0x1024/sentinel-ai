use anyhow::Result;
use std::collections::HashMap;
use uuid::Uuid;

use sentinel_core::models::dictionary::{
    Dictionary, DictionaryExport, DictionaryFilter, DictionaryImportOptions, DictionarySet,
    DictionarySetRelation, DictionaryStats, DictionaryType, DictionaryWord, MergeMode, ServiceType,
};
use sentinel_db::DatabasePool;

macro_rules! db_execute {
    ($svc:expr, $sql:expr, |$q:ident| $binds:expr) => {{
        let sql = $svc.sql($sql);
        match &$svc.pool {
            DatabasePool::PostgreSQL(pool) => {
                let $q = sqlx::query(&sql);
                $binds.execute(pool).await?.rows_affected()
            }
            DatabasePool::SQLite(pool) => {
                let $q = sqlx::query(&sql);
                $binds.execute(pool).await?.rows_affected()
            }
            DatabasePool::MySQL(pool) => {
                let $q = sqlx::query(&sql);
                $binds.execute(pool).await?.rows_affected()
            }
        }
    }};
}

macro_rules! db_fetch_optional_as {
    ($svc:expr, $ty:ty, $sql:expr, |$q:ident| $binds:expr) => {{
        let sql = $svc.sql($sql);
        match &$svc.pool {
            DatabasePool::PostgreSQL(pool) => {
                let $q = sqlx::query_as::<_, $ty>(&sql);
                $binds.fetch_optional(pool).await?
            }
            DatabasePool::SQLite(pool) => {
                let $q = sqlx::query_as::<_, $ty>(&sql);
                $binds.fetch_optional(pool).await?
            }
            DatabasePool::MySQL(pool) => {
                let $q = sqlx::query_as::<_, $ty>(&sql);
                $binds.fetch_optional(pool).await?
            }
        }
    }};
}

macro_rules! db_fetch_all_as {
    ($svc:expr, $ty:ty, $sql:expr, |$q:ident| $binds:expr) => {{
        let sql = $svc.sql($sql);
        match &$svc.pool {
            DatabasePool::PostgreSQL(pool) => {
                let $q = sqlx::query_as::<_, $ty>(&sql);
                $binds.fetch_all(pool).await?
            }
            DatabasePool::SQLite(pool) => {
                let $q = sqlx::query_as::<_, $ty>(&sql);
                $binds.fetch_all(pool).await?
            }
            DatabasePool::MySQL(pool) => {
                let $q = sqlx::query_as::<_, $ty>(&sql);
                $binds.fetch_all(pool).await?
            }
        }
    }};
}

macro_rules! db_fetch_scalar {
    ($svc:expr, $ty:ty, $sql:expr, |$q:ident| $binds:expr) => {{
        let sql = $svc.sql($sql);
        match &$svc.pool {
            DatabasePool::PostgreSQL(pool) => {
                let $q = sqlx::query_scalar::<_, $ty>(&sql);
                $binds.fetch_one(pool).await?
            }
            DatabasePool::SQLite(pool) => {
                let $q = sqlx::query_scalar::<_, $ty>(&sql);
                $binds.fetch_one(pool).await?
            }
            DatabasePool::MySQL(pool) => {
                let $q = sqlx::query_scalar::<_, $ty>(&sql);
                $binds.fetch_one(pool).await?
            }
        }
    }};
}

#[derive(Debug, Clone)]
pub struct DictionaryService {
    pool: DatabasePool,
}

impl DictionaryService {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    fn sql(&self, sql: &str) -> String {
        if matches!(self.pool, DatabasePool::PostgreSQL(_)) {
            return sql.to_string();
        }

        let mut out = String::with_capacity(sql.len());
        let bytes = sql.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == b'$' {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > i + 1 {
                    out.push('?');
                    i = j;
                    continue;
                }
            }

            out.push(bytes[i] as char);
            i += 1;
        }

        out
    }

    pub async fn create_dictionary(&self, mut dictionary: Dictionary) -> Result<Dictionary> {
        if dictionary.id.is_empty() {
            dictionary.id = Uuid::new_v4().to_string();
        }

        let now = chrono::Utc::now();
        dictionary.created_at = now;
        dictionary.updated_at = now;

        db_execute!(
            self,
            r#"
            INSERT INTO dictionaries (
                id, name, description, dict_type, service_type, category,
                is_builtin, is_active, word_count, file_size, checksum,
                version, author, source_url, tags, metadata,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        "#,
            |q| {
                q.bind(&dictionary.id)
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
                    .bind(dictionary.created_at)
                    .bind(dictionary.updated_at)
            }
        );

        Ok(dictionary)
    }

    pub async fn get_dictionary(&self, id: &str) -> Result<Option<Dictionary>> {
        let dictionary = db_fetch_optional_as!(
            self,
            Dictionary,
            "SELECT * FROM dictionaries WHERE id = $1",
            |q| q.bind(id)
        );

        Ok(dictionary)
    }

    pub async fn get_dictionary_by_name(&self, name: &str) -> Result<Option<Dictionary>> {
        let dictionary = db_fetch_optional_as!(
            self,
            Dictionary,
            "SELECT * FROM dictionaries WHERE name = $1",
            |q| q.bind(name)
        );

        Ok(dictionary)
    }

    pub async fn list_dictionaries(
        &self,
        filter: Option<DictionaryFilter>,
    ) -> Result<Vec<Dictionary>> {
        let mut query = "SELECT * FROM dictionaries WHERE 1=1".to_string();
        let mut param_idx = 1;
        let mut params: Vec<String> = Vec::new();

        if let Some(filter) = filter {
            if let Some(dict_type) = filter.dict_type {
                query.push_str(&format!(" AND dict_type = ${}", param_idx));
                param_idx += 1;
                params.push(dict_type.to_string());
            }
            if let Some(service_type) = filter.service_type {
                query.push_str(&format!(" AND service_type = ${}", param_idx));
                param_idx += 1;
                params.push(service_type.to_string());
            }
            if let Some(category) = filter.category {
                query.push_str(&format!(" AND category = ${}", param_idx));
                param_idx += 1;
                params.push(category);
            }
            if let Some(is_builtin) = filter.is_builtin {
                query.push_str(&format!(" AND is_builtin = ${}", param_idx));
                param_idx += 1;
                params.push(is_builtin.to_string());
            }
            if let Some(is_active) = filter.is_active {
                query.push_str(&format!(" AND is_active = ${}", param_idx));
                param_idx += 1;
                params.push(is_active.to_string());
            }
            if let Some(search_term) = filter.search_term {
                query.push_str(&format!(
                    " AND (name LIKE ${0} OR description LIKE ${0})",
                    param_idx
                ));
                params.push(format!("%{}%", search_term));
            }
        }

        query.push_str(" ORDER BY created_at DESC");
        let query = self.sql(&query);

        let dictionaries = match &self.pool {
            DatabasePool::PostgreSQL(pool) => {
                let mut sql_query = sqlx::query_as::<_, Dictionary>(&query);
                for param in params {
                    sql_query = sql_query.bind(param);
                }
                sql_query.fetch_all(pool).await?
            }
            DatabasePool::SQLite(pool) => {
                let mut sql_query = sqlx::query_as::<_, Dictionary>(&query);
                for param in params {
                    sql_query = sql_query.bind(param);
                }
                sql_query.fetch_all(pool).await?
            }
            DatabasePool::MySQL(pool) => {
                let mut sql_query = sqlx::query_as::<_, Dictionary>(&query);
                for param in params {
                    sql_query = sql_query.bind(param);
                }
                sql_query.fetch_all(pool).await?
            }
        };

        Ok(dictionaries)
    }

    pub async fn update_dictionary(&self, mut dictionary: Dictionary) -> Result<Dictionary> {
        dictionary.updated_at = chrono::Utc::now();

        db_execute!(
            self,
            r#"
            UPDATE dictionaries SET
                name = $1, description = $2, dict_type = $3, service_type = $4,
                category = $5, is_builtin = $6, is_active = $7, word_count = $8,
                file_size = $9, checksum = $10, version = $11, author = $12,
                source_url = $13, tags = $14, metadata = $15, updated_at = $16
            WHERE id = $17
        "#,
            |q| {
                q.bind(&dictionary.name)
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
                    .bind(dictionary.updated_at)
                    .bind(&dictionary.id)
            }
        );

        Ok(dictionary)
    }

    pub async fn delete_dictionary(&self, id: &str) -> Result<()> {
        db_execute!(
            self,
            "DELETE FROM dictionary_words WHERE dictionary_id = $1",
            |q| q.bind(id)
        );

        db_execute!(
            self,
            "DELETE FROM dictionary_set_relations WHERE dictionary_id = $1",
            |q| q.bind(id)
        );

        db_execute!(self, "DELETE FROM dictionaries WHERE id = $1", |q| q
            .bind(id));

        Ok(())
    }

    pub async fn add_words(
        &self,
        dictionary_id: &str,
        words: Vec<String>,
    ) -> Result<Vec<DictionaryWord>> {
        let mut added_words = Vec::new();

        for word in words {
            let dict_word = DictionaryWord::new(dictionary_id.to_string(), word);

            db_execute!(
                self,
                r#"
                INSERT INTO dictionary_words (id, dictionary_id, word, weight, category, metadata, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
                |q| {
                    q.bind(&dict_word.id)
                        .bind(&dict_word.dictionary_id)
                        .bind(&dict_word.word)
                        .bind(dict_word.weight)
                        .bind(&dict_word.category)
                        .bind(&dict_word.metadata)
                        .bind(dict_word.created_at)
                }
            );

            added_words.push(dict_word);
        }

        self.update_word_count(dictionary_id).await?;

        Ok(added_words)
    }

    pub async fn remove_words(&self, dictionary_id: &str, words: Vec<String>) -> Result<u64> {
        let mut removed_count = 0;

        for word in words {
            let affected = db_execute!(
                self,
                "DELETE FROM dictionary_words WHERE dictionary_id = $1 AND word = $2",
                |q| q.bind(dictionary_id).bind(&word)
            );

            removed_count += affected;
        }

        self.update_word_count(dictionary_id).await?;

        Ok(removed_count)
    }

    pub async fn get_dictionary_words(&self, dictionary_id: &str) -> Result<Vec<DictionaryWord>> {
        let words = db_fetch_all_as!(
            self,
            DictionaryWord,
            "SELECT * FROM dictionary_words WHERE dictionary_id = $1 ORDER BY weight DESC, word ASC",
            |q| q.bind(dictionary_id)
        );

        Ok(words)
    }

    pub async fn get_dictionary_words_paged(
        &self,
        dictionary_id: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<DictionaryWord>> {
        let words = db_fetch_all_as!(
            self,
            DictionaryWord,
            "SELECT * FROM dictionary_words WHERE dictionary_id = $1 ORDER BY weight DESC, word ASC LIMIT $2 OFFSET $3",
            |q| q.bind(dictionary_id).bind(limit as i64).bind(offset as i64)
        );

        Ok(words)
    }

    pub async fn search_words(
        &self,
        dictionary_id: &str,
        pattern: &str,
        limit: Option<u32>,
    ) -> Result<Vec<DictionaryWord>> {
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
        let query = format!(
            "SELECT * FROM dictionary_words WHERE dictionary_id = $1 AND word LIKE $2 ORDER BY weight DESC, word ASC{}",
            limit_clause
        );

        let search_pattern = format!("%{}%", pattern);
        let query = self.sql(&query);

        let words = match &self.pool {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, DictionaryWord>(&query)
                    .bind(dictionary_id)
                    .bind(search_pattern.clone())
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, DictionaryWord>(&query)
                    .bind(dictionary_id)
                    .bind(search_pattern.clone())
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, DictionaryWord>(&query)
                    .bind(dictionary_id)
                    .bind(search_pattern)
                    .fetch_all(pool)
                    .await?
            }
        };

        Ok(words)
    }

    pub async fn search_words_paged(
        &self,
        dictionary_id: &str,
        pattern: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<DictionaryWord>> {
        let search_pattern = format!("%{}%", pattern);
        let words = db_fetch_all_as!(
            self,
            DictionaryWord,
            "SELECT * FROM dictionary_words WHERE dictionary_id = $1 AND word LIKE $2 ORDER BY weight DESC, word ASC LIMIT $3 OFFSET $4",
            |q| q.bind(dictionary_id).bind(search_pattern).bind(limit as i64).bind(offset as i64)
        );

        Ok(words)
    }

    pub async fn clear_dictionary(&self, dictionary_id: &str) -> Result<u64> {
        let affected = db_execute!(
            self,
            "DELETE FROM dictionary_words WHERE dictionary_id = $1",
            |q| q.bind(dictionary_id)
        );

        self.update_word_count(dictionary_id).await?;

        Ok(affected)
    }

    async fn update_word_count(&self, dictionary_id: &str) -> Result<()> {
        let count: i64 = db_fetch_scalar!(
            self,
            i64,
            "SELECT COUNT(*) FROM dictionary_words WHERE dictionary_id = $1",
            |q| q.bind(dictionary_id)
        );

        db_execute!(
            self,
            "UPDATE dictionaries SET word_count = $1, updated_at = $2 WHERE id = $3",
            |q| q.bind(count).bind(chrono::Utc::now()).bind(dictionary_id)
        );

        Ok(())
    }

    pub async fn export_dictionary(&self, dictionary_id: &str) -> Result<DictionaryExport> {
        let dictionary = self
            .get_dictionary(dictionary_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Dictionary not found"))?;

        let words = self.get_dictionary_words(dictionary_id).await?;

        Ok(DictionaryExport::new(dictionary, words))
    }

    pub async fn import_dictionary(
        &self,
        export_data: DictionaryExport,
        options: DictionaryImportOptions,
    ) -> Result<Dictionary> {
        let mut dictionary = export_data.dictionary;

        match options.merge_mode {
            MergeMode::CreateNew => {
                dictionary.id = Uuid::new_v4().to_string();
                dictionary.name = format!("{}_imported", dictionary.name);
                let created_dict = self.create_dictionary(dictionary).await?;

                let words: Vec<String> = export_data.words.into_iter().map(|w| w.word).collect();
                self.add_words(&created_dict.id, words).await?;

                Ok(created_dict)
            }
            MergeMode::Replace => {
                if let Some(existing) = self.get_dictionary_by_name(&dictionary.name).await? {
                    self.clear_dictionary(&existing.id).await?;

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

                    let words: Vec<String> =
                        export_data.words.into_iter().map(|w| w.word).collect();
                    self.add_words(&dictionary.id, words).await?;
                } else {
                    dictionary = self.create_dictionary(dictionary).await?;
                    let words: Vec<String> =
                        export_data.words.into_iter().map(|w| w.word).collect();
                    self.add_words(&dictionary.id, words).await?;
                }

                Ok(dictionary)
            }
            MergeMode::Merge => {
                if let Some(existing) = self.get_dictionary_by_name(&dictionary.name).await? {
                    dictionary = existing;

                    let existing_words = self.get_dictionary_words(&dictionary.id).await?;
                    let existing_word_set: std::collections::HashSet<String> =
                        existing_words.into_iter().map(|w| w.word).collect();

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
                    dictionary = self.create_dictionary(dictionary).await?;
                    let words: Vec<String> =
                        export_data.words.into_iter().map(|w| w.word).collect();
                    self.add_words(&dictionary.id, words).await?;
                }

                Ok(dictionary)
            }
        }
    }

    pub async fn get_stats(&self) -> Result<DictionaryStats> {
        let total_dictionaries: i64 =
            db_fetch_scalar!(self, i64, "SELECT COUNT(*) FROM dictionaries", |q| q);

        let total_words: i64 =
            db_fetch_scalar!(self, i64, "SELECT COUNT(*) FROM dictionary_words", |q| q);

        let builtin_dictionaries: i64 = db_fetch_scalar!(
            self,
            i64,
            "SELECT COUNT(*) FROM dictionaries WHERE is_builtin = TRUE",
            |q| q
        );

        let custom_dictionaries: i64 = db_fetch_scalar!(
            self,
            i64,
            "SELECT COUNT(*) FROM dictionaries WHERE is_builtin = FALSE",
            |q| q
        );

        let active_dictionaries: i64 = db_fetch_scalar!(
            self,
            i64,
            "SELECT COUNT(*) FROM dictionaries WHERE is_active = TRUE",
            |q| q
        );

        let total_sets: i64 =
            db_fetch_scalar!(self, i64, "SELECT COUNT(*) FROM dictionary_sets", |q| q);

        let type_stats: Vec<(String, i64)> = db_fetch_all_as!(
            self,
            (String, i64),
            "SELECT dict_type, COUNT(*) as count FROM dictionaries GROUP BY dict_type",
            |q| q
        );

        let by_type: HashMap<String, f64> = type_stats
            .into_iter()
            .map(|(dict_type, count)| (dict_type, count as f64))
            .collect();

        let service_stats: Vec<(Option<String>, i64)> = db_fetch_all_as!(
            self,
            (Option<String>, i64),
            "SELECT service_type, COUNT(*) as count FROM dictionaries GROUP BY service_type",
            |q| q
        );

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

    pub async fn create_dictionary_set(&self, mut set: DictionarySet) -> Result<DictionarySet> {
        if set.id.is_empty() {
            set.id = Uuid::new_v4().to_string();
        }

        let now = chrono::Utc::now();
        set.created_at = now;
        set.updated_at = now;

        db_execute!(
            self,
            r#"
            INSERT INTO dictionary_sets (id, name, description, service_type, scenario, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
            |q| {
                q.bind(&set.id)
                    .bind(&set.name)
                    .bind(&set.description)
                    .bind(&set.service_type)
                    .bind(&set.scenario)
                    .bind(set.is_active)
                    .bind(set.created_at)
                    .bind(set.updated_at)
            }
        );

        Ok(set)
    }

    pub async fn add_dictionary_to_set(
        &self,
        set_id: &str,
        dictionary_id: &str,
        priority: Option<i32>,
    ) -> Result<DictionarySetRelation> {
        let relation = DictionarySetRelation::new(set_id.to_string(), dictionary_id.to_string())
            .with_priority(priority.unwrap_or(0));

        db_execute!(
            self,
            r#"
            INSERT INTO dictionary_set_relations (id, set_id, dictionary_id, priority, is_enabled, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
        "#,
            |q| {
                q.bind(&relation.id)
                    .bind(&relation.set_id)
                    .bind(&relation.dictionary_id)
                    .bind(relation.priority)
                    .bind(relation.is_enabled)
                    .bind(relation.created_at)
            }
        );

        Ok(relation)
    }

    pub async fn get_set_dictionaries(&self, set_id: &str) -> Result<Vec<Dictionary>> {
        let dictionaries = db_fetch_all_as!(
            self,
            Dictionary,
            r#"
            SELECT d.* FROM dictionaries d
            JOIN dictionary_set_relations r ON d.id = r.dictionary_id
            WHERE r.set_id = $1 AND r.is_enabled = TRUE
            ORDER BY r.priority DESC, d.name ASC
        "#,
            |q| q.bind(set_id)
        );

        Ok(dictionaries)
    }

    pub async fn initialize_builtin_dictionaries(&self) -> Result<()> {
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
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        if self.get_dictionary(&subdomain_dict.id).await?.is_none() {
            self.create_dictionary(subdomain_dict).await?;

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
