use deno_error::JsErrorBox;
use serde::Serialize;
use std::sync::OnceLock;

#[cfg(feature = "db-postgres")]
pub type DictionaryPool = sqlx::PgPool;
#[cfg(all(not(feature = "db-postgres"), feature = "db-mysql"))]
pub type DictionaryPool = sqlx::MySqlPool;
#[cfg(all(not(feature = "db-postgres"), not(feature = "db-mysql")))]
pub type DictionaryPool = sqlx::SqlitePool;

static DICTIONARY_POOL: OnceLock<DictionaryPool> = OnceLock::new();

fn dictionary_backend_name() -> &'static str {
    #[cfg(feature = "db-postgres")]
    {
        "postgres"
    }
    #[cfg(all(not(feature = "db-postgres"), feature = "db-mysql"))]
    {
        "mysql"
    }
    #[cfg(all(not(feature = "db-postgres"), not(feature = "db-mysql")))]
    {
        "sqlite"
    }
}

#[derive(Serialize)]
pub struct JsDictionary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub dict_type: String,
    pub service_type: Option<String>,
    pub category: Option<String>,
    pub word_count: i64,
    pub tags: Option<String>,
}

#[derive(Serialize)]
pub struct JsDictionaryEntry {
    pub word: String,
    pub weight: f64,
    pub category: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

pub fn init_dictionary_pool(pool: DictionaryPool) {
    if DICTIONARY_POOL.set(pool).is_ok() {
        tracing::info!(
            "Dictionary pool initialized in sentinel-plugins runtime (backend={})",
            dictionary_backend_name()
        );
    } else {
        tracing::warn!(
            "Dictionary pool initialization skipped because it was already initialized (backend={})",
            dictionary_backend_name()
        );
    }
}

fn get_dictionary_pool() -> Result<&'static DictionaryPool, JsErrorBox> {
    match DICTIONARY_POOL.get() {
        Some(pool) => Ok(pool),
        None => {
            tracing::error!(
                "Dictionary pool requested before initialization in sentinel-plugins runtime (backend={})",
                dictionary_backend_name()
            );
            Err(JsErrorBox::generic("Dictionary database not initialized"))
        }
    }
}

#[cfg(feature = "db-postgres")]
async fn resolve_dictionary_id(pool: &DictionaryPool, id_or_name: &str) -> Result<Option<String>, JsErrorBox> {
    sqlx::query_scalar("SELECT id FROM dictionaries WHERE id = $1 OR name = $2")
        .bind(id_or_name)
        .bind(id_or_name)
        .fetch_optional(pool)
        .await
        .map_err(|e| JsErrorBox::generic(format!("Query error: {}", e)))
}

#[cfg(not(feature = "db-postgres"))]
async fn resolve_dictionary_id(pool: &DictionaryPool, id_or_name: &str) -> Result<Option<String>, JsErrorBox> {
    sqlx::query_scalar("SELECT id FROM dictionaries WHERE id = ? OR name = ?")
        .bind(id_or_name)
        .bind(id_or_name)
        .fetch_optional(pool)
        .await
        .map_err(|e| JsErrorBox::generic(format!("Query error: {}", e)))
}

#[cfg(feature = "db-postgres")]
pub async fn get_dictionary(id_or_name: String) -> Result<Option<JsDictionary>, JsErrorBox> {
    let pool = get_dictionary_pool()?;
    let row: Option<(
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        i64,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT id, name, description, dict_type, service_type, category, word_count, tags FROM dictionaries WHERE id = $1 OR name = $2",
    )
    .bind(&id_or_name)
    .bind(&id_or_name)
    .fetch_optional(pool)
    .await
    .map_err(|e| JsErrorBox::generic(format!("Query error: {}", e)))?;

    Ok(row.map(
        |(id, name, description, dict_type, service_type, category, word_count, tags)| {
            JsDictionary {
                id,
                name,
                description,
                dict_type,
                service_type,
                category,
                word_count,
                tags,
            }
        },
    ))
}

#[cfg(not(feature = "db-postgres"))]
pub async fn get_dictionary(id_or_name: String) -> Result<Option<JsDictionary>, JsErrorBox> {
    let pool = get_dictionary_pool()?;
    let row: Option<(
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        i64,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT id, name, description, dict_type, service_type, category, word_count, tags FROM dictionaries WHERE id = ? OR name = ?",
    )
    .bind(&id_or_name)
    .bind(&id_or_name)
    .fetch_optional(pool)
    .await
    .map_err(|e| JsErrorBox::generic(format!("Query error: {}", e)))?;

    Ok(row.map(
        |(id, name, description, dict_type, service_type, category, word_count, tags)| {
            JsDictionary {
                id,
                name,
                description,
                dict_type,
                service_type,
                category,
                word_count,
                tags,
            }
        },
    ))
}

#[cfg(feature = "db-postgres")]
pub async fn get_default_dictionary_id(dict_type: String) -> Result<String, JsErrorBox> {
    let pool = get_dictionary_pool()?;
    let dict_id: Option<String> = sqlx::query_scalar(
        "SELECT value FROM configurations WHERE category = 'dictionary_default' AND key = $1",
    )
    .bind(&dict_type)
    .fetch_optional(pool)
    .await
    .map_err(|e| JsErrorBox::generic(format!("Query error: {}", e)))?;

    Ok(dict_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_default())
}

#[cfg(not(feature = "db-postgres"))]
pub async fn get_default_dictionary_id(dict_type: String) -> Result<String, JsErrorBox> {
    let pool = get_dictionary_pool()?;
    let dict_id: Option<String> = sqlx::query_scalar(
        "SELECT value FROM configurations WHERE category = 'dictionary_default' AND key = ?",
    )
    .bind(&dict_type)
    .fetch_optional(pool)
    .await
    .map_err(|e| JsErrorBox::generic(format!("Query error: {}", e)))?;

    Ok(dict_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_default())
}

pub async fn get_dictionary_words(
    id_or_name: String,
    limit: Option<i32>,
) -> Result<Vec<String>, JsErrorBox> {
    let pool = get_dictionary_pool()?;
    let Some(dict_id) = resolve_dictionary_id(pool, &id_or_name).await? else {
        return Ok(vec![]);
    };
    let limit_val = limit.unwrap_or(10000) as i64;

    #[cfg(feature = "db-postgres")]
    let query = "SELECT word FROM dictionary_words WHERE dictionary_id = $1 ORDER BY weight DESC, word ASC LIMIT $2";
    #[cfg(not(feature = "db-postgres"))]
    let query = "SELECT word FROM dictionary_words WHERE dictionary_id = ? ORDER BY weight DESC, word ASC LIMIT ?";

    sqlx::query_scalar(query)
        .bind(&dict_id)
        .bind(limit_val)
        .fetch_all(pool)
        .await
        .map_err(|e| JsErrorBox::generic(format!("Query error: {}", e)))
}

pub async fn get_dictionary_entries(
    id_or_name: String,
    limit: Option<i32>,
) -> Result<Vec<JsDictionaryEntry>, JsErrorBox> {
    let pool = get_dictionary_pool()?;
    let Some(dict_id) = resolve_dictionary_id(pool, &id_or_name).await? else {
        return Ok(vec![]);
    };
    let limit_val = limit.unwrap_or(10000) as i64;

    #[cfg(feature = "db-postgres")]
    let query = "SELECT word, weight, category, metadata FROM dictionary_words WHERE dictionary_id = $1 ORDER BY weight DESC, word ASC LIMIT $2";
    #[cfg(not(feature = "db-postgres"))]
    let query = "SELECT word, weight, category, metadata FROM dictionary_words WHERE dictionary_id = ? ORDER BY weight DESC, word ASC LIMIT ?";

    let rows: Vec<(String, f64, Option<String>, Option<String>)> = sqlx::query_as(query)
        .bind(&dict_id)
        .bind(limit_val)
        .fetch_all(pool)
        .await
        .map_err(|e| JsErrorBox::generic(format!("Query error: {}", e)))?;

    Ok(rows
        .into_iter()
        .map(|(word, weight, category, metadata)| JsDictionaryEntry {
            word,
            weight,
            category,
            metadata: metadata.and_then(|raw| serde_json::from_str(&raw).ok()),
        })
        .collect())
}

#[cfg(feature = "db-postgres")]
pub async fn list_dictionaries(
    dict_type: Option<String>,
    category: Option<String>,
) -> Result<Vec<JsDictionary>, JsErrorBox> {
    let pool = get_dictionary_pool()?;

    let mut query = "SELECT id, name, description, dict_type, service_type, category, word_count, tags FROM dictionaries WHERE 1=1".to_string();
    let mut params_count = 0;
    if dict_type.is_some() {
        params_count += 1;
        query.push_str(&format!(" AND dict_type = ${}", params_count));
    }
    if category.is_some() {
        params_count += 1;
        query.push_str(&format!(" AND category = ${}", params_count));
    }
    query.push_str(" ORDER BY name ASC");

    let mut sql_query = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
            i64,
            Option<String>,
        ),
    >(&query);

    if let Some(ref dt) = dict_type {
        sql_query = sql_query.bind(dt);
    }
    if let Some(ref cat) = category {
        sql_query = sql_query.bind(cat);
    }

    let rows = sql_query
        .fetch_all(pool)
        .await
        .map_err(|e| JsErrorBox::generic(format!("Query error: {}", e)))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, name, description, dict_type, service_type, category, word_count, tags)| {
                JsDictionary {
                    id,
                    name,
                    description,
                    dict_type,
                    service_type,
                    category,
                    word_count,
                    tags,
                }
            },
        )
        .collect())
}

#[cfg(not(feature = "db-postgres"))]
pub async fn list_dictionaries(
    dict_type: Option<String>,
    category: Option<String>,
) -> Result<Vec<JsDictionary>, JsErrorBox> {
    let pool = get_dictionary_pool()?;

    let mut query = "SELECT id, name, description, dict_type, service_type, category, word_count, tags FROM dictionaries WHERE 1=1".to_string();
    if dict_type.is_some() {
        query.push_str(" AND dict_type = ?");
    }
    if category.is_some() {
        query.push_str(" AND category = ?");
    }
    query.push_str(" ORDER BY name ASC");

    let mut sql_query = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
            i64,
            Option<String>,
        ),
    >(&query);

    if let Some(ref dt) = dict_type {
        sql_query = sql_query.bind(dt);
    }
    if let Some(ref cat) = category {
        sql_query = sql_query.bind(cat);
    }

    let rows = sql_query
        .fetch_all(pool)
        .await
        .map_err(|e| JsErrorBox::generic(format!("Query error: {}", e)))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, name, description, dict_type, service_type, category, word_count, tags)| {
                JsDictionary {
                    id,
                    name,
                    description,
                    dict_type,
                    service_type,
                    category,
                    word_count,
                    tags,
                }
            },
        )
        .collect())
}
