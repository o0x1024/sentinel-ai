use anyhow::Result;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::dictionary_surface_builtins::{
    builtin_favicon_fingerprint_entries, builtin_service_fingerprint_entries,
};
use sentinel_core::models::dictionary::{
    Dictionary, DictionaryExport, DictionaryFilter, DictionaryImportOptions, DictionarySet,
    DictionarySetRelation, DictionaryStats, DictionaryType, DictionaryWord, DictionaryWordInput,
    MergeMode, ServiceType,
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
        let (mut query, params) = self.build_dictionary_filter_query(filter, None, None);

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

    pub async fn list_dictionaries_paged(
        &self,
        filter: Option<DictionaryFilter>,
        dict_types: Option<Vec<String>>,
        subtype: Option<String>,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Dictionary>> {
        let (mut query, params) =
            self.build_dictionary_filter_query(filter, dict_types, subtype.as_deref());
        query.push_str(&format!(
            " ORDER BY created_at DESC LIMIT {} OFFSET {}",
            limit, offset
        ));
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

    pub async fn count_dictionaries(
        &self,
        filter: Option<DictionaryFilter>,
        dict_types: Option<Vec<String>>,
        subtype: Option<String>,
    ) -> Result<i64> {
        let (query, params) =
            self.build_dictionary_count_query(filter, dict_types, subtype.as_deref());
        let query = self.sql(&query);

        let total = match &self.pool {
            DatabasePool::PostgreSQL(pool) => {
                let mut sql_query = sqlx::query_scalar::<_, i64>(&query);
                for param in params {
                    sql_query = sql_query.bind(param);
                }
                sql_query.fetch_one(pool).await?
            }
            DatabasePool::SQLite(pool) => {
                let mut sql_query = sqlx::query_scalar::<_, i64>(&query);
                for param in params {
                    sql_query = sql_query.bind(param);
                }
                sql_query.fetch_one(pool).await?
            }
            DatabasePool::MySQL(pool) => {
                let mut sql_query = sqlx::query_scalar::<_, i64>(&query);
                for param in params {
                    sql_query = sql_query.bind(param);
                }
                sql_query.fetch_one(pool).await?
            }
        };

        Ok(total)
    }

    fn build_dictionary_count_query(
        &self,
        filter: Option<DictionaryFilter>,
        dict_types: Option<Vec<String>>,
        subtype: Option<&str>,
    ) -> (String, Vec<String>) {
        let (query, params) = self.build_dictionary_filter_query(filter, dict_types, subtype);
        (query.replacen("SELECT *", "SELECT COUNT(*)", 1), params)
    }

    fn build_dictionary_filter_query(
        &self,
        filter: Option<DictionaryFilter>,
        dict_types: Option<Vec<String>>,
        subtype: Option<&str>,
    ) -> (String, Vec<String>) {
        let mut query = "SELECT * FROM dictionaries WHERE 1=1".to_string();
        let mut param_idx = 1;
        let mut params: Vec<String> = Vec::new();
        let mut selected_dict_type: Option<String> = None;

        if let Some(filter) = filter {
            if let Some(dict_type) = filter.dict_type {
                let value = dict_type.to_string();
                query.push_str(&format!(" AND dict_type = ${}", param_idx));
                param_idx += 1;
                params.push(value.clone());
                selected_dict_type = Some(value);
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
                query.push_str(if is_builtin {
                    " AND is_builtin = TRUE"
                } else {
                    " AND is_builtin = FALSE"
                });
            }
            if let Some(is_active) = filter.is_active {
                query.push_str(if is_active {
                    " AND is_active = TRUE"
                } else {
                    " AND is_active = FALSE"
                });
            }
            if let Some(search_term) = filter.search_term {
                query.push_str(&format!(
                    " AND (LOWER(name) LIKE ${0} OR LOWER(COALESCE(description, '')) LIKE ${0})",
                    param_idx
                ));
                params.push(format!("%{}%", search_term.to_lowercase()));
                param_idx += 1;
            }
        }

        // dict_types IN filter (only when dict_type single filter is not set)
        if selected_dict_type.is_none() {
            if let Some(ref types) = dict_types {
                if !types.is_empty() {
                    let placeholders: Vec<String> = types
                        .iter()
                        .map(|t| {
                            let ph = format!("${}", param_idx);
                            param_idx += 1;
                            params.push(t.clone());
                            ph
                        })
                        .collect();
                    query.push_str(&format!(" AND dict_type IN ({})", placeholders.join(", ")));
                }
            }
        }

        if let Some(subtype) = subtype {
            self.append_dictionary_subtype_filter(
                &mut query,
                &mut param_idx,
                &mut params,
                selected_dict_type.as_deref(),
                subtype,
            );
        }

        (query, params)
    }

    fn append_dictionary_subtype_filter(
        &self,
        query: &mut String,
        param_idx: &mut i32,
        params: &mut Vec<String>,
        selected_dict_type: Option<&str>,
        subtype: &str,
    ) {
        if subtype == "service_identification" && selected_dict_type == Some("service_probe_rule") {
            return;
        }

        let metadata_pattern = format!("%{}%", subtype.to_lowercase());
        let push_param = |params: &mut Vec<String>, param_idx: &mut i32, value: String| {
            let placeholder = format!("${}", *param_idx);
            params.push(value);
            *param_idx += 1;
            placeholder
        };
        let marker_clause = |params: &mut Vec<String>,
                             param_idx: &mut i32,
                             markers: &[&str]|
         -> String {
            markers
                    .iter()
                    .map(|marker| {
                        let placeholder =
                            push_param(params, param_idx, format!("%{}%", marker.to_lowercase()));
                        format!(
                            "(LOWER(id) LIKE {0} OR LOWER(name) LIKE {0} OR LOWER(COALESCE(tags, '')) LIKE {0})",
                            placeholder
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" OR ")
        };

        let metadata_placeholder = push_param(params, param_idx, metadata_pattern);

        match subtype {
            "service_identification" => {
                let markers = marker_clause(params, param_idx, &["service", "banner"]);
                query.push_str(&format!(
                    " AND (LOWER(COALESCE(metadata, '')) LIKE {} OR {})",
                    metadata_placeholder, markers
                ));
            }
            "web_fingerprint" => {
                let markers = marker_clause(params, param_idx, &["web", "technology", "tech"]);
                query.push_str(&format!(
                    " AND (LOWER(COALESCE(metadata, '')) LIKE {} OR {})",
                    metadata_placeholder, markers
                ));
            }
            "favicon_fingerprint" => {
                let markers = marker_clause(params, param_idx, &["favicon"]);
                query.push_str(&format!(
                    " AND (LOWER(COALESCE(metadata, '')) LIKE {} OR {})",
                    metadata_placeholder, markers
                ));
            }
            "generic_fingerprint" => {
                let service_markers = marker_clause(params, param_idx, &["service", "banner"]);
                let web_markers = marker_clause(params, param_idx, &["web", "technology", "tech"]);
                let favicon_markers = marker_clause(params, param_idx, &["favicon"]);
                query.push_str(&format!(
                    " AND (LOWER(COALESCE(metadata, '')) LIKE {0} OR NOT (({1}) OR ({2}) OR ({3})))",
                    metadata_placeholder, service_markers, web_markers, favicon_markers
                ));
            }
            "risk_verification" => {
                let verification_markers =
                    marker_clause(params, param_idx, &["verification", "poc"]);
                let category_placeholder = push_param(params, param_idx, "risk".to_string());
                query.push_str(&format!(
                    " AND (LOWER(COALESCE(metadata, '')) LIKE {0} OR LOWER(COALESCE(category, '')) = {1} OR {2})",
                    metadata_placeholder, category_placeholder, verification_markers
                ));
            }
            _ => {
                query.push_str(&format!(
                    " AND LOWER(COALESCE(metadata, '')) LIKE {}",
                    metadata_placeholder
                ));
            }
        }
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
        let entries = words
            .into_iter()
            .map(|word| DictionaryWordInput {
                word,
                weight: None,
                category: None,
                metadata: None,
            })
            .collect();
        self.add_word_entries(dictionary_id, entries).await
    }

    pub async fn add_word_entries(
        &self,
        dictionary_id: &str,
        entries: Vec<DictionaryWordInput>,
    ) -> Result<Vec<DictionaryWord>> {
        let mut added_words = Vec::new();

        for entry in entries {
            let dict_word = entry.into_word(dictionary_id.to_string());

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

    pub async fn sync_word_entries(
        &self,
        dictionary_id: &str,
        entries: Vec<DictionaryWordInput>,
        remove_missing: bool,
    ) -> Result<Vec<DictionaryWord>> {
        let existing_words = self.get_dictionary_words(dictionary_id).await?;
        let mut existing_by_word: HashMap<String, DictionaryWord> = existing_words
            .into_iter()
            .map(|word| (word.word.clone(), word))
            .collect();
        let mut updated_words = Vec::new();
        let mut new_entries = Vec::new();

        for entry in entries {
            let entry_word = entry.word.clone();
            let entry_weight = entry.weight.unwrap_or(1.0);
            let entry_category = entry.category.clone();
            let entry_metadata = entry
                .metadata
                .as_ref()
                .and_then(|value| serde_json::to_string(value).ok());

            if let Some(mut existing) = existing_by_word.remove(&entry_word) {
                let weight_changed = (existing.weight - entry_weight).abs() > f64::EPSILON;
                let category_changed = existing.category != entry_category;
                let metadata_changed = existing.metadata != entry_metadata;

                if weight_changed || category_changed || metadata_changed {
                    existing.weight = entry_weight;
                    existing.category = entry_category;
                    existing.metadata = entry_metadata;
                    updated_words.push(existing);
                }
            } else {
                new_entries.push(entry);
            }
        }

        let mut affected_words = Vec::new();

        if remove_missing {
            let stale_words: Vec<String> = existing_by_word.into_keys().collect();
            if !stale_words.is_empty() {
                self.remove_words(dictionary_id, stale_words).await?;
            }
        }

        if !updated_words.is_empty() {
            affected_words.extend(self.update_words_batch(updated_words).await?);
        }
        if !new_entries.is_empty() {
            affected_words.extend(self.add_word_entries(dictionary_id, new_entries).await?);
        }

        self.update_word_count(dictionary_id).await?;

        Ok(affected_words)
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

    pub async fn update_word(&self, mut word: DictionaryWord) -> Result<DictionaryWord> {
        word.word = word.word.trim().to_string();

        db_execute!(
            self,
            r#"
            UPDATE dictionary_words
            SET word = $1, weight = $2, category = $3, metadata = $4
            WHERE id = $5 AND dictionary_id = $6
        "#,
            |q| {
                q.bind(&word.word)
                    .bind(word.weight)
                    .bind(&word.category)
                    .bind(&word.metadata)
                    .bind(&word.id)
                    .bind(&word.dictionary_id)
            }
        );

        self.update_word_count(&word.dictionary_id).await?;

        Ok(word)
    }

    pub async fn update_words_batch(
        &self,
        words: Vec<DictionaryWord>,
    ) -> Result<Vec<DictionaryWord>> {
        let mut updated_words = Vec::with_capacity(words.len());
        let mut touched_dictionary_ids = HashSet::new();

        for mut word in words {
            word.word = word.word.trim().to_string();

            db_execute!(
                self,
                r#"
                UPDATE dictionary_words
                SET word = $1, weight = $2, category = $3, metadata = $4
                WHERE id = $5 AND dictionary_id = $6
            "#,
                |q| {
                    q.bind(&word.word)
                        .bind(word.weight)
                        .bind(&word.category)
                        .bind(&word.metadata)
                        .bind(&word.id)
                        .bind(&word.dictionary_id)
                }
            );

            touched_dictionary_ids.insert(word.dictionary_id.clone());
            updated_words.push(word);
        }

        for dictionary_id in touched_dictionary_ids {
            self.update_word_count(&dictionary_id).await?;
        }

        Ok(updated_words)
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
        self.ensure_builtin_dictionary(
            Dictionary {
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
            },
            vec![
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
            ]
            .into_iter()
            .map(|word| DictionaryWordInput {
                word: word.to_string(),
                weight: None,
                category: None,
                metadata: None,
            })
            .collect(),
        )
        .await?;

        self.ensure_builtin_dictionary_metadata_only(Dictionary {
            id: "builtin_sensitive_files_web".to_string(),
            name: "Sensitive Web Files".to_string(),
            description: Some(
                "Common sensitive files, API docs, debug outputs, and exposed configuration paths"
                    .to_string(),
            ),
            dict_type: DictionaryType::SensitiveFile.to_string(),
            service_type: Some(ServiceType::Web.to_string()),
            category: Some("risk".to_string()),
            is_builtin: true,
            is_active: true,
            word_count: 0,
            file_size: 0,
            checksum: None,
            version: "1.0.0".to_string(),
            author: Some("Sentinel AI".to_string()),
            source_url: None,
            tags: Some("file,exposure,swagger,config,leak".to_string()),
            metadata: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await?;

        self.ensure_builtin_dictionary(
            Dictionary {
                id: "builtin_web_fingerprint_rules".to_string(),
                name: "Web Fingerprint Rules".to_string(),
                description: Some(
                    "Dictionary-driven web technology fingerprint rules used by Technology Fingerprinter"
                        .to_string(),
                ),
                dict_type: DictionaryType::FingerprintRule.to_string(),
                service_type: Some(ServiceType::Web.to_string()),
                category: Some("fingerprint".to_string()),
                is_builtin: true,
                is_active: true,
                word_count: 0,
                file_size: 0,
                checksum: None,
                version: "1.0.0".to_string(),
                author: Some("Sentinel AI".to_string()),
                source_url: Some("https://github.com/adysec/ARL".to_string()),
                tags: Some("fingerprint,web,arl".to_string()),
                metadata: Some(serde_json::json!({
                    "subtype": "web_fingerprint"
                }).to_string()),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            vec![
                DictionaryWordInput {
                    word: "swagger_ui".to_string(),
                    weight: Some(10.0),
                    category: Some("api".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Swagger UI",
                        "product": "Swagger UI",
                        "vendor": "SmartBear",
                        "asset_category": "api_portal",
                        "asset_family": "developer_portal",
                        "priority": 120,
                        "matchers": [
                            {"part": "body", "type": "contains", "value": "Swagger UI"},
                            {"part": "title", "type": "contains", "value": "Swagger UI"}
                        ],
                        "confidence": 0.95
                    })),
                },
                DictionaryWordInput {
                    word: "jenkins".to_string(),
                    weight: Some(10.0),
                    category: Some("ci".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Jenkins",
                        "product": "Jenkins",
                        "vendor": "Jenkins",
                        "asset_category": "ci_cd",
                        "asset_family": "devops",
                        "priority": 130,
                        "matchers": [
                            {"part": "header", "key": "x-jenkins", "type": "exists"},
                            {"part": "body", "type": "contains", "value": "Jenkins"}
                        ],
                        "confidence": 0.95
                    })),
                },
                DictionaryWordInput {
                    word: "grafana".to_string(),
                    weight: Some(9.0),
                    category: Some("observability".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Grafana",
                        "product": "Grafana",
                        "vendor": "Grafana Labs",
                        "asset_category": "observability",
                        "asset_family": "monitoring",
                        "priority": 120,
                        "matchers": [
                            {"part": "body", "type": "contains", "value": "grafana-app"},
                            {"part": "title", "type": "contains", "value": "Grafana"}
                        ],
                        "confidence": 0.9
                    })),
                },
                DictionaryWordInput {
                    word: "spring_boot".to_string(),
                    weight: Some(8.0),
                    category: Some("java".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Spring Boot",
                        "product": "Spring Boot",
                        "vendor": "VMware",
                        "asset_category": "framework",
                        "asset_family": "java_framework",
                        "priority": 90,
                        "matchers": [
                            {"part": "header", "key": "x-application-context", "type": "exists"},
                            {"part": "body", "type": "contains", "value": "\"_links\""}
                        ],
                        "confidence": 0.75
                    })),
                },
                DictionaryWordInput {
                    word: "nextjs".to_string(),
                    weight: Some(8.0),
                    category: Some("javascript".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Next.js",
                        "product": "Next.js",
                        "vendor": "Vercel",
                        "asset_category": "framework",
                        "asset_family": "javascript_framework",
                        "priority": 100,
                        "operator": "or",
                        "matchers": [
                            {"part": "body", "type": "contains", "value": "__NEXT_DATA__"},
                            {"part": "body", "type": "contains", "value": "_next/static"},
                            {"part": "header", "key": "x-powered-by", "type": "contains", "value": "Next.js"}
                        ],
                        "confidence": 0.9
                    })),
                },
                DictionaryWordInput {
                    word: "nginx".to_string(),
                    weight: Some(7.0),
                    category: Some("web_server".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Nginx",
                        "product": "Nginx",
                        "vendor": "NGINX",
                        "asset_category": "web_server",
                        "asset_family": "http_server",
                        "priority": 80,
                        "matchers": [
                            {"part": "header", "key": "server", "type": "regex", "value": "nginx(?:/([0-9.]+))?"}
                        ],
                        "confidence": 0.95
                    })),
                },
                DictionaryWordInput {
                    word: "apache".to_string(),
                    weight: Some(7.0),
                    category: Some("web_server".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Apache HTTP Server",
                        "product": "Apache",
                        "vendor": "Apache Software Foundation",
                        "asset_category": "web_server",
                        "asset_family": "http_server",
                        "priority": 80,
                        "matchers": [
                            {"part": "header", "key": "server", "type": "regex", "value": "apache(?:/([0-9.]+))?"}
                        ],
                        "confidence": 0.95
                    })),
                },
                DictionaryWordInput {
                    word: "cloudflare".to_string(),
                    weight: Some(7.0),
                    category: Some("cdn".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Cloudflare",
                        "product": "Cloudflare",
                        "vendor": "Cloudflare",
                        "asset_category": "cdn",
                        "asset_family": "edge_network",
                        "priority": 110,
                        "operator": "or",
                        "matchers": [
                            {"part": "header", "key": "server", "type": "contains", "value": "cloudflare"},
                            {"part": "header", "key": "cf-ray", "type": "exists"}
                        ],
                        "confidence": 0.9
                    })),
                },
                DictionaryWordInput {
                    word: "prometheus".to_string(),
                    weight: Some(7.0),
                    category: Some("observability".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Prometheus",
                        "product": "Prometheus",
                        "vendor": "CNCF",
                        "asset_category": "observability",
                        "asset_family": "monitoring",
                        "priority": 120,
                        "operator": "or",
                        "matchers": [
                            {"part": "title", "type": "contains", "value": "Prometheus Time Series Collection"},
                            {"part": "body", "type": "contains", "value": "Prometheus Time Series Collection"}
                        ],
                        "confidence": 0.9
                    })),
                },
                DictionaryWordInput {
                    word: "kibana".to_string(),
                    weight: Some(7.0),
                    category: Some("observability".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Kibana",
                        "product": "Kibana",
                        "vendor": "Elastic",
                        "asset_category": "observability",
                        "asset_family": "logging",
                        "priority": 120,
                        "operator": "or",
                        "matchers": [
                            {"part": "title", "type": "contains", "value": "Kibana"},
                            {"part": "body", "type": "contains", "value": "kibanaWelcomeLogo"}
                        ],
                        "confidence": 0.88
                    })),
                },
                DictionaryWordInput {
                    word: "elasticsearch".to_string(),
                    weight: Some(7.0),
                    category: Some("search".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Elasticsearch",
                        "product": "Elasticsearch",
                        "vendor": "Elastic",
                        "asset_category": "search",
                        "asset_family": "data_platform",
                        "priority": 120,
                        "operator": "or",
                        "matchers": [
                            {"part": "body", "type": "contains", "value": "\"cluster_name\""},
                            {"part": "body", "type": "contains", "value": "\"tagline\":\"You Know, for Search\""}
                        ],
                        "confidence": 0.85
                    })),
                },
                DictionaryWordInput {
                    word: "traefik".to_string(),
                    weight: Some(7.0),
                    category: Some("gateway".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Traefik",
                        "product": "Traefik",
                        "vendor": "Traefik Labs",
                        "asset_category": "gateway",
                        "asset_family": "reverse_proxy",
                        "priority": 100,
                        "operator": "or",
                        "matchers": [
                            {"part": "header", "key": "server", "type": "contains", "value": "Traefik"},
                            {"part": "body", "type": "contains", "value": "Traefik"}
                        ],
                        "confidence": 0.85
                    })),
                },
                DictionaryWordInput {
                    word: "gitlab".to_string(),
                    weight: Some(8.0),
                    category: Some("devops".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "GitLab",
                        "product": "GitLab",
                        "vendor": "GitLab",
                        "asset_category": "devops",
                        "asset_family": "code_platform",
                        "priority": 125,
                        "operator": "or",
                        "matchers": [
                            {"part": "body", "type": "contains", "value": "GitLab"},
                            {"part": "header", "key": "x-gitlab-meta", "type": "exists"}
                        ],
                        "confidence": 0.88
                    })),
                },
                DictionaryWordInput {
                    word: "harbor".to_string(),
                    weight: Some(7.0),
                    category: Some("registry".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Harbor",
                        "product": "Harbor",
                        "vendor": "VMware",
                        "asset_category": "registry",
                        "asset_family": "artifact_registry",
                        "priority": 115,
                        "operator": "or",
                        "matchers": [
                            {"part": "title", "type": "contains", "value": "Harbor"},
                            {"part": "body", "type": "contains", "value": "Harbor"}
                        ],
                        "confidence": 0.84
                    })),
                },
                DictionaryWordInput {
                    word: "rabbitmq".to_string(),
                    weight: Some(7.0),
                    category: Some("messaging".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "RabbitMQ Management",
                        "product": "RabbitMQ",
                        "vendor": "VMware",
                        "asset_category": "messaging",
                        "asset_family": "message_broker",
                        "priority": 120,
                        "operator": "or",
                        "matchers": [
                            {"part": "title", "type": "contains", "value": "RabbitMQ Management"},
                            {"part": "body", "type": "contains", "value": "rabbitmq"}
                        ],
                        "confidence": 0.86
                    })),
                },
                DictionaryWordInput {
                    word: "consul".to_string(),
                    weight: Some(7.0),
                    category: Some("service_discovery".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Consul",
                        "product": "Consul",
                        "vendor": "HashiCorp",
                        "asset_category": "service_discovery",
                        "asset_family": "infrastructure",
                        "priority": 120,
                        "operator": "or",
                        "matchers": [
                            {"part": "title", "type": "contains", "value": "Consul by HashiCorp"},
                            {"part": "body", "type": "contains", "value": "Consul by HashiCorp"}
                        ],
                        "confidence": 0.87
                    })),
                },
                DictionaryWordInput {
                    word: "vault".to_string(),
                    weight: Some(7.0),
                    category: Some("secrets".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "HashiCorp Vault",
                        "product": "Vault",
                        "vendor": "HashiCorp",
                        "asset_category": "secrets_management",
                        "asset_family": "infrastructure",
                        "priority": 125,
                        "operator": "or",
                        "matchers": [
                            {"part": "title", "type": "contains", "value": "Vault"},
                            {"part": "body", "type": "contains", "value": "Vault UI"}
                        ],
                        "confidence": 0.84
                    })),
                },
            ],
        )
        .await?;

        self.ensure_builtin_dictionary(
            Dictionary {
                id: "builtin_service_fingerprint_rules".to_string(),
                name: "Service Fingerprint Rules".to_string(),
                description: Some(
                    "Dictionary-driven service banner and protocol fingerprint rules used by Service Probe and Service Monitor"
                        .to_string(),
                ),
                dict_type: DictionaryType::ServiceProbeRule.to_string(),
                service_type: Some(ServiceType::General.to_string()),
                category: Some("fingerprint".to_string()),
                is_builtin: true,
                is_active: true,
                word_count: 0,
                file_size: 0,
                checksum: None,
                version: "1.0.0".to_string(),
                author: Some("Sentinel AI".to_string()),
                source_url: None,
                tags: Some("fingerprint,service,banner".to_string()),
                metadata: Some(serde_json::json!({
                    "subtype": "service_identification"
                }).to_string()),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            builtin_service_fingerprint_entries(),
        )
        .await?;

        self.ensure_builtin_dictionary(
            Dictionary {
                id: "builtin_favicon_fingerprint_rules".to_string(),
                name: "Favicon Fingerprint Rules".to_string(),
                description: Some(
                    "Dictionary-driven favicon URL and hash rules used by Favicon Fingerprinter"
                        .to_string(),
                ),
                dict_type: DictionaryType::FingerprintRule.to_string(),
                service_type: Some(ServiceType::Web.to_string()),
                category: Some("fingerprint".to_string()),
                is_builtin: true,
                is_active: true,
                word_count: 0,
                file_size: 0,
                checksum: None,
                version: "1.0.0".to_string(),
                author: Some("Sentinel AI".to_string()),
                source_url: None,
                tags: Some("fingerprint,favicon,web".to_string()),
                metadata: Some(
                    serde_json::json!({
                        "subtype": "favicon_fingerprint"
                    })
                    .to_string(),
                ),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            builtin_favicon_fingerprint_entries(),
        )
        .await?;

        self.ensure_builtin_dictionary_seed_once(
            Dictionary {
                id: "builtin_safe_poc_rules".to_string(),
                name: "Safe Risk Verification Rules".to_string(),
                description: Some(
                    "Safe HTTP validation rules for exposure and weak-access verification".to_string(),
                ),
                dict_type: DictionaryType::PocRule.to_string(),
                service_type: Some(ServiceType::Web.to_string()),
                category: Some("risk".to_string()),
                is_builtin: true,
                is_active: true,
                word_count: 0,
                file_size: 0,
                checksum: None,
                version: "1.0.0".to_string(),
                author: Some("Sentinel AI".to_string()),
                source_url: None,
                tags: Some("poc,risk,verification,safe".to_string()),
                metadata: Some(serde_json::json!({
                    "subtype": "risk_verification"
                }).to_string()),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            vec![
                DictionaryWordInput {
                    word: "swagger_ui_exposure".to_string(),
                    weight: Some(8.0),
                    category: Some("exposure".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Swagger UI Exposure",
                        "finding_type": "api_exposure",
                        "severity": "medium",
                        "target_types": ["web"],
                        "match_scope": { "fingerprints": ["swagger_ui"] },
                        "request": { "method": "GET", "path": "/swagger-ui.html" },
                        "matchers": [
                            { "part": "status", "type": "in", "value": [200] },
                            { "part": "body", "type": "contains", "value": "Swagger UI" }
                        ],
                        "impact": "Exposed interactive API documentation can disclose endpoints and schemas",
                        "remediation": "Restrict Swagger UI access or disable it in production",
                        "safe_mode": true
                    })),
                },
                DictionaryWordInput {
                    word: "spring_actuator_env".to_string(),
                    weight: Some(9.0),
                    category: Some("exposure".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Spring Actuator Env Exposure",
                        "finding_type": "config_exposure",
                        "severity": "high",
                        "target_types": ["web"],
                        "match_scope": { "fingerprints": ["spring_boot"] },
                        "request": { "method": "GET", "path": "/actuator/env" },
                        "matchers": [
                            { "part": "status", "type": "in", "value": [200] },
                            { "part": "body", "type": "contains", "value": "propertySources" }
                        ],
                        "impact": "Environment and configuration values may be exposed to unauthenticated users",
                        "remediation": "Disable or restrict sensitive actuator endpoints",
                        "safe_mode": true
                    })),
                },
                DictionaryWordInput {
                    word: "jenkins_anonymous_read".to_string(),
                    weight: Some(8.0),
                    category: Some("access".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Jenkins Anonymous Read",
                        "finding_type": "anonymous_access",
                        "severity": "medium",
                        "target_types": ["web"],
                        "match_scope": { "fingerprints": ["jenkins"] },
                        "request": { "method": "GET", "path": "/api/json" },
                        "matchers": [
                            { "part": "status", "type": "in", "value": [200] },
                            { "part": "body", "type": "contains", "value": "\"jobs\"" }
                        ],
                        "impact": "Anonymous users can enumerate Jenkins data",
                        "remediation": "Disable anonymous read or restrict unauthenticated API access",
                        "safe_mode": true
                    })),
                },
                DictionaryWordInput {
                    word: "prometheus_metrics_exposure".to_string(),
                    weight: Some(8.0),
                    category: Some("exposure".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Prometheus Metrics Exposure",
                        "finding_type": "metrics_exposure",
                        "severity": "medium",
                        "target_types": ["web"],
                        "match_scope": { "fingerprints": ["prometheus"] },
                        "request": { "method": "GET", "path": "/metrics" },
                        "matchers": [
                            { "part": "status", "type": "in", "value": [200] },
                            { "part": "body", "type": "contains", "value": "# HELP" }
                        ],
                        "impact": "Operational metrics are exposed to unauthenticated users",
                        "remediation": "Require authentication for metrics endpoints or restrict network access",
                        "safe_mode": true
                    })),
                },
                DictionaryWordInput {
                    word: "kibana_status_exposure".to_string(),
                    weight: Some(7.0),
                    category: Some("exposure".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Kibana Status Exposure",
                        "finding_type": "status_exposure",
                        "severity": "medium",
                        "target_types": ["web"],
                        "match_scope": { "fingerprints": ["kibana"] },
                        "request": { "method": "GET", "path": "/api/status" },
                        "matchers": [
                            { "part": "status", "type": "in", "value": [200] },
                            { "part": "body", "type": "contains", "value": "\"name\":\"kibana\"" }
                        ],
                        "impact": "Kibana status endpoint discloses stack and deployment details",
                        "remediation": "Restrict or disable the status endpoint for unauthenticated users",
                        "safe_mode": true
                    })),
                },
                DictionaryWordInput {
                    word: "elasticsearch_cluster_health_exposure".to_string(),
                    weight: Some(8.0),
                    category: Some("exposure".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Elasticsearch Cluster Health Exposure",
                        "finding_type": "cluster_exposure",
                        "severity": "high",
                        "target_types": ["web"],
                        "match_scope": { "fingerprints": ["elasticsearch"] },
                        "request": { "method": "GET", "path": "/_cluster/health" },
                        "matchers": [
                            { "part": "status", "type": "in", "value": [200] },
                            { "part": "body", "type": "contains", "value": "\"cluster_name\"" }
                        ],
                        "impact": "Cluster health information is accessible without authentication",
                        "remediation": "Enable authentication and restrict Elasticsearch administrative endpoints",
                        "safe_mode": true
                    })),
                },
                DictionaryWordInput {
                    word: "phpinfo_exposure".to_string(),
                    weight: Some(7.0),
                    category: Some("exposure".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "phpinfo Exposure",
                        "finding_type": "debug_exposure",
                        "severity": "medium",
                        "target_types": ["web"],
                        "request": { "method": "GET", "path": "/phpinfo.php" },
                        "matchers": [
                            { "part": "status", "type": "in", "value": [200] },
                            { "part": "body", "type": "contains", "value": "PHP Version" }
                        ],
                        "impact": "phpinfo output reveals environment, modules, and path details",
                        "remediation": "Remove phpinfo pages from production environments",
                        "safe_mode": true
                    })),
                },
                DictionaryWordInput {
                    word: "grafana_health_exposure".to_string(),
                    weight: Some(7.0),
                    category: Some("exposure".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Grafana Health Exposure",
                        "finding_type": "status_exposure",
                        "severity": "medium",
                        "target_types": ["web"],
                        "match_scope": { "fingerprints": ["grafana"] },
                        "request": { "method": "GET", "path": "/api/health" },
                        "matchers": [
                            { "part": "status", "type": "in", "value": [200] },
                            { "part": "body", "type": "contains", "value": "\"database\"" }
                        ],
                        "impact": "Grafana health endpoint discloses service status information",
                        "remediation": "Restrict unauthenticated access to Grafana API endpoints",
                        "safe_mode": true
                    })),
                },
                DictionaryWordInput {
                    word: "rabbitmq_management_overview".to_string(),
                    weight: Some(8.0),
                    category: Some("exposure".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "RabbitMQ Management Overview Exposure",
                        "finding_type": "management_exposure",
                        "severity": "high",
                        "target_types": ["web"],
                        "match_scope": { "fingerprints": ["rabbitmq"] },
                        "request": { "method": "GET", "path": "/api/overview" },
                        "matchers": [
                            { "part": "status", "type": "in", "value": [200] },
                            { "part": "body", "type": "contains", "value": "\"rabbitmq_version\"" }
                        ],
                        "impact": "RabbitMQ management API is exposed without authentication",
                        "remediation": "Disable public exposure of management endpoints or require authentication",
                        "safe_mode": true
                    })),
                },
                DictionaryWordInput {
                    word: "vault_health_exposure".to_string(),
                    weight: Some(8.0),
                    category: Some("exposure".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Vault Health Exposure",
                        "finding_type": "status_exposure",
                        "severity": "medium",
                        "target_types": ["web"],
                        "match_scope": { "fingerprints": ["vault"] },
                        "request": { "method": "GET", "path": "/v1/sys/health" },
                        "matchers": [
                            { "part": "status", "type": "in", "value": [200, 429, 472, 473, 501, 503] },
                            { "part": "body", "type": "contains", "value": "\"initialized\"" }
                        ],
                        "impact": "Vault health endpoint discloses service state and initialization details",
                        "remediation": "Restrict or proxy health endpoints behind authenticated access",
                        "safe_mode": true
                    })),
                },
                DictionaryWordInput {
                    word: "consul_ui_exposure".to_string(),
                    weight: Some(7.0),
                    category: Some("exposure".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Consul UI Exposure",
                        "finding_type": "ui_exposure",
                        "severity": "medium",
                        "target_types": ["web"],
                        "match_scope": { "fingerprints": ["consul"] },
                        "request": { "method": "GET", "path": "/ui/" },
                        "matchers": [
                            { "part": "status", "type": "in", "value": [200] },
                            { "part": "body", "type": "contains", "value": "Consul" }
                        ],
                        "impact": "Consul UI may expose service topology and cluster metadata",
                        "remediation": "Disable or restrict UI access to trusted networks",
                        "safe_mode": true
                    })),
                },
                DictionaryWordInput {
                    word: "spring_actuator_heapdump_exposure".to_string(),
                    weight: Some(9.0),
                    category: Some("exposure".to_string()),
                    metadata: Some(serde_json::json!({
                        "name": "Spring Actuator Heapdump Exposure",
                        "finding_type": "heapdump_exposure",
                        "severity": "critical",
                        "target_types": ["web"],
                        "match_scope": { "fingerprints": ["spring_boot"] },
                        "requests": [
                            {
                                "id": "health",
                                "method": "GET",
                                "path": "/actuator/health",
                                "matchers": [
                                    { "part": "status", "type": "in", "value": [200] }
                                ]
                            },
                            {
                                "id": "heapdump",
                                "method": "GET",
                                "path": "/actuator/heapdump",
                                "matchers": [
                                    { "part": "status", "type": "in", "value": [200] },
                                    { "part": "header", "key": "content-type", "type": "contains", "value": "application/octet-stream" }
                                ]
                            }
                        ],
                        "impact": "Heapdump files can expose credentials, tokens, and in-memory secrets",
                        "remediation": "Disable heapdump endpoints or restrict them behind authentication and trusted networks",
                        "safe_mode": true
                    })),
                },
            ],
        )
        .await?;

        tracing::info!("Builtin dictionaries initialized");
        Ok(())
    }

    async fn ensure_builtin_dictionary(
        &self,
        dictionary: Dictionary,
        entries: Vec<DictionaryWordInput>,
    ) -> Result<()> {
        if let Some(existing) = self.get_dictionary(&dictionary.id).await? {
            let mut updated = existing;
            updated.name = dictionary.name;
            updated.description = dictionary.description;
            updated.dict_type = dictionary.dict_type;
            updated.service_type = dictionary.service_type;
            updated.category = dictionary.category;
            updated.is_builtin = dictionary.is_builtin;
            updated.is_active = dictionary.is_active;
            updated.file_size = dictionary.file_size;
            updated.checksum = dictionary.checksum;
            updated.version = dictionary.version;
            updated.author = dictionary.author;
            updated.source_url = dictionary.source_url;
            updated.tags = dictionary.tags;
            updated.metadata = dictionary.metadata;

            self.update_dictionary(updated).await?;
            self.sync_builtin_dictionary_words(&dictionary.id, entries)
                .await?;
        } else {
            self.create_dictionary(dictionary.clone()).await?;
            self.add_word_entries(&dictionary.id, entries).await?;
        }
        Ok(())
    }

    /// Seed entries on first creation; on subsequent runs only update metadata,
    /// preserving any user modifications to the dictionary entries.
    async fn ensure_builtin_dictionary_seed_once(
        &self,
        dictionary: Dictionary,
        entries: Vec<DictionaryWordInput>,
    ) -> Result<()> {
        if let Some(existing) = self.get_dictionary(&dictionary.id).await? {
            let mut updated = existing;
            updated.name = dictionary.name;
            updated.description = dictionary.description;
            updated.dict_type = dictionary.dict_type;
            updated.service_type = dictionary.service_type;
            updated.category = dictionary.category;
            updated.is_builtin = dictionary.is_builtin;
            updated.is_active = dictionary.is_active;
            updated.file_size = dictionary.file_size;
            updated.checksum = dictionary.checksum;
            updated.version = dictionary.version;
            updated.author = dictionary.author;
            updated.source_url = dictionary.source_url;
            updated.tags = dictionary.tags;
            updated.metadata = dictionary.metadata;

            self.update_dictionary(updated).await?;
        } else {
            self.create_dictionary(dictionary.clone()).await?;
            self.add_word_entries(&dictionary.id, entries).await?;
        }
        Ok(())
    }

    async fn ensure_builtin_dictionary_metadata_only(&self, dictionary: Dictionary) -> Result<()> {
        if let Some(existing) = self.get_dictionary(&dictionary.id).await? {
            let mut updated = existing;
            updated.name = dictionary.name;
            updated.description = dictionary.description;
            updated.dict_type = dictionary.dict_type;
            updated.service_type = dictionary.service_type;
            updated.category = dictionary.category;
            updated.is_builtin = dictionary.is_builtin;
            updated.is_active = dictionary.is_active;
            updated.file_size = dictionary.file_size;
            updated.checksum = dictionary.checksum;
            updated.version = dictionary.version;
            updated.author = dictionary.author;
            updated.source_url = dictionary.source_url;
            updated.tags = dictionary.tags;
            updated.metadata = dictionary.metadata;

            self.update_dictionary(updated).await?;
        } else {
            self.create_dictionary(dictionary).await?;
        }
        Ok(())
    }

    async fn sync_builtin_dictionary_words(
        &self,
        dictionary_id: &str,
        entries: Vec<DictionaryWordInput>,
    ) -> Result<()> {
        self.sync_word_entries(dictionary_id, entries, true).await?;
        Ok(())
    }
}
