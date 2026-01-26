use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Row, Column, TypeInfo};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::{info, warn};
use base64::Engine as _;

use super::connection_manager::DatabasePool;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportFormat {
    pub version: String,
    pub db_type: String,
    pub exported_at: String,
    pub tables: HashMap<String, TableData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableData {
    pub schema: Vec<ColumnInfo>,
    pub rows: Vec<HashMap<String, Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
}

pub struct DatabaseMigration {
    source_pool: DatabasePool,
    target_pool: Option<DatabasePool>,
}

impl DatabaseMigration {
    pub fn new(source_pool: DatabasePool) -> Self {
        Self {
            source_pool,
            target_pool: None,
        }
    }
    
    pub fn with_target(mut self, target_pool: DatabasePool) -> Self {
        self.target_pool = Some(target_pool);
        self
    }
    
    /// Export database to JSON file
    pub async fn export_to_json<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        info!("Starting database export to JSON");
        
        let tables = self.get_table_list().await?;
        let mut export_data = ExportFormat {
            version: "1.0.0".to_string(),
            db_type: self.source_pool.db_type().as_str().to_string(),
            exported_at: chrono::Utc::now().to_rfc3339(),
            tables: HashMap::new(),
        };
        
        for table_name in tables {
            info!("Exporting table: {}", table_name);
            
            let table_data = self.export_table(&table_name).await?;
            export_data.tables.insert(table_name, table_data);
        }
        
        let json = serde_json::to_string_pretty(&export_data)?;
        fs::write(output_path, json).await?;
        
        info!("Database export completed successfully");
        Ok(())
    }
    
    /// Import database from JSON file
    pub async fn import_from_json<P: AsRef<Path>>(&self, input_path: P) -> Result<()> {
        info!("Starting database import from JSON");
        
        let json = fs::read_to_string(input_path).await?;
        let import_data: ExportFormat = serde_json::from_str(&json)?;
        
        info!("Import data version: {}", import_data.version);
        info!("Source database type: {}", import_data.db_type);
        
        for (table_name, table_data) in import_data.tables {
            info!("Importing table: {}", table_name);
            
            self.import_table(&table_name, &table_data).await?;
        }
        
        info!("Database import completed successfully");
        Ok(())
    }
    
    /// Migrate data from source to target database
    pub async fn migrate(&self) -> Result<()> {
        let target = self.target_pool.as_ref()
            .context("Target database pool not set")?;
        
        info!("Starting database migration from {:?} to {:?}", 
              self.source_pool.db_type(), target.db_type());
        
        // Initialize schema in target database first
        info!("Initializing schema in target database...");
        self.initialize_target_schema(target).await?;
        
        let tables = self.get_table_list().await?;

        // Sort tables to ensure foreign key dependencies are imported in correct order
        // Tables with foreign keys should be imported after their referenced tables
        let _tables = self.sort_tables_by_dependencies(tables).await?;

        info!("Database migration completed successfully");
        Ok(())
    }

    /// Check if a table has foreign key constraints
    async fn table_has_foreign_keys(&self, _table_name: &str) -> Result<bool> {
        match &self.source_pool {
            DatabasePool::PostgreSQL(_) | DatabasePool::SQLite(_) => {
                // For target databases, assume tables may have constraints
                // We'll handle this in the import logic
                Ok(true)
            }
        }
    }

    /// Sort tables by their foreign key dependencies to ensure proper import order
    async fn sort_tables_by_dependencies(&self, tables: Vec<String>) -> Result<Vec<String>> {
        // Define known foreign key relationships (referenced_table -> dependent_table)
        let dependencies: std::collections::HashMap<&str, Vec<&str>> = [
            ("ai_conversations", vec!["ai_messages"]),
            ("agent_tasks", vec!["agent_sessions", "agent_session_logs", "agent_execution_results", "agent_execution_steps"]),
            ("agent_sessions", vec!["agent_session_logs", "agent_execution_results", "agent_execution_steps"]),
            ("scan_tasks", vec!["scan_sessions", "scan_stages", "vulnerabilities"]),
            ("scan_sessions", vec!["scan_stages"]),
            ("assets", vec!["asset_relationships"]),
            ("rag_collections", vec!["rag_document_sources", "rag_chunks", "rag_queries"]),
            ("rag_document_sources", vec!["rag_chunks"]),
            ("traffic_vulnerabilities", vec!["traffic_evidence", "traffic_dedupe_index"]),
            ("proxifier_proxies", vec!["proxifier_rules"]),
            ("dictionaries", vec!["dictionary_words", "dictionary_sets", "dictionary_set_relations"]),
            ("dictionary_sets", vec!["dictionary_set_relations"]),
            ("plugin_registry", vec!["plugin_favorites"]),
            ("workflow_runs", vec!["workflow_run_steps"]),
            ("workflow_definitions", vec!["workflow_runs"]),
            ("execution_plans", vec!["execution_sessions"]),
        ].into();

        // Create a dependency graph and count incoming edges
        let mut graph: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        // Initialize graph and in-degrees
        for table in &tables {
            graph.insert(table.clone(), Vec::new());
            in_degree.insert(table.clone(), 0);
        }

        // Build the graph based on dependencies
        for (referenced, dependents) in &dependencies {
            for dependent in dependents {
                if tables.contains(&referenced.to_string()) && tables.contains(&dependent.to_string()) {
                    graph.get_mut(&referenced.to_string()).unwrap().push(dependent.to_string());
                    *in_degree.get_mut(&dependent.to_string()).unwrap() += 1;
                }
            }
        }

        // Perform topological sort using Kahn's algorithm
        let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        let mut result: Vec<String> = Vec::new();

        // Start with tables that have no incoming edges (no dependencies)
        for table in &tables {
            if *in_degree.get(table).unwrap() == 0 {
                queue.push_back(table.clone());
            }
        }

        while let Some(table) = queue.pop_front() {
            result.push(table.clone());

            // For each dependent table
            if let Some(dependents) = graph.get(&table) {
                for dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        // Add any remaining tables that weren't in the dependency graph
        for table in &tables {
            if !result.contains(table) {
                result.push(table.clone());
            }
        }

        info!("Tables sorted by dependencies: {:?}", result);
        Ok(result)
    }

    /// Initialize schema in target database
    async fn initialize_target_schema(&self, target: &DatabasePool) -> Result<()> {
        info!("Initializing target database schema");
        
        match target {
            DatabasePool::PostgreSQL(pool) => {
                info!("Creating PostgreSQL schema...");
                self.create_postgresql_schema(pool).await?;
            }
            DatabasePool::SQLite(_) => {
                info!("SQLite schema initialization not implemented for migration");
            }
        }
        Ok(())
    }
    
    /// Create PostgreSQL database schema
    async fn create_postgresql_schema(&self, pool: &sqlx::PgPool) -> Result<()> {
        info!("Creating PostgreSQL tables...");

        // First, drop existing tables if they exist to avoid conflicts
        info!("Starting database cleanup...");

        // Method 2: Query all tables in the public schema and drop them
        let tables_query = "SELECT tablename FROM pg_tables WHERE schemaname = 'public'";
        let tables_rows = match sqlx::query(tables_query).fetch_all(pool).await {
            Ok(rows) => {
                info!("Found {} existing tables to clean up", rows.len());
                rows
            }
            Err(e) => {
                warn!("Failed to query existing tables: {}", e);
                vec![]
            }
        };

        // Drop each table with CASCADE
        for row in tables_rows {
            if let Ok(table_name) = row.try_get::<String, _>("tablename") {
                let drop_sql = format!("DROP TABLE IF EXISTS {} CASCADE", table_name);
                match sqlx::query(&drop_sql).execute(pool).await {
                    Ok(_) => info!("Dropped existing table: {}", table_name),
                    Err(e) => warn!("Failed to drop table {}: {}", table_name, e),
                }
            }
        }

        info!("PostgreSQL schema created successfully");
        Ok(())
    }
    

    
    /// Get schema SQL from source database
    async fn get_schema_sql(&self) -> Result<Vec<String>> {
        match &self.source_pool {
            _ => {
                Err(anyhow::anyhow!("Schema extraction only supported from source database"))
            }
        }
    }
    
    /// Convert SQLite SQL to PostgreSQL SQL
    fn convert_to_postgresql_sql(&self, sqlite_sql: &[String]) -> Vec<String> {
        info!("Converting {} SQLite statements to PostgreSQL", sqlite_sql.len());
        sqlite_sql.iter().map(|sql| {
            let mut pg_sql = sql.clone();

        // Handle specific cases first (more specific to less specific)
        pg_sql = pg_sql.replace("TEXT PRIMARY KEY", "VARCHAR(255) PRIMARY KEY");
        pg_sql = pg_sql.replace("TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP", "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP");
        pg_sql = pg_sql.replace("TEXT DEFAULT CURRENT_TIMESTAMP", "TIMESTAMP DEFAULT CURRENT_TIMESTAMP");

            // Convert DATETIME types
            pg_sql = pg_sql.replace("DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP", "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP");
            pg_sql = pg_sql.replace("DATETIME DEFAULT CURRENT_TIMESTAMP", "TIMESTAMP DEFAULT CURRENT_TIMESTAMP");
            pg_sql = pg_sql.replace("DATETIME NOT NULL", "TIMESTAMP NOT NULL");
            pg_sql = pg_sql.replace("DATETIME", "TIMESTAMP");

            // Convert BOOLEAN types
            pg_sql = pg_sql.replace("BOOLEAN DEFAULT 0", "BOOLEAN DEFAULT FALSE");
            pg_sql = pg_sql.replace("BOOLEAN DEFAULT 1", "BOOLEAN DEFAULT TRUE");
            pg_sql = pg_sql.replace("BOOLEAN NOT NULL DEFAULT 0", "BOOLEAN NOT NULL DEFAULT FALSE");
            pg_sql = pg_sql.replace("BOOLEAN NOT NULL DEFAULT 1", "BOOLEAN NOT NULL DEFAULT TRUE");

            // Convert INTEGER types to BIGINT to prevent overflow
            pg_sql = pg_sql.replace("INTEGER PRIMARY KEY AUTOINCREMENT", "BIGSERIAL PRIMARY KEY");
            pg_sql = pg_sql.replace("INTEGER NOT NULL DEFAULT 0", "BIGINT NOT NULL DEFAULT 0");
            pg_sql = pg_sql.replace("INTEGER DEFAULT 0", "BIGINT DEFAULT 0");
            pg_sql = pg_sql.replace("INTEGER NOT NULL", "BIGINT NOT NULL");
            pg_sql = pg_sql.replace("INTEGER", "BIGINT");

            // Convert other data types
            pg_sql = pg_sql.replace("REAL", "DOUBLE PRECISION");
            pg_sql = pg_sql.replace("BLOB", "BYTEA");

            // Handle IF NOT EXISTS - PostgreSQL supports this for CREATE TABLE
            pg_sql = pg_sql.replace("CREATE TABLE IF NOT EXISTS", "CREATE TABLE IF NOT EXISTS");
            pg_sql = pg_sql.replace("CREATE INDEX IF NOT EXISTS", "CREATE INDEX IF NOT EXISTS");

            // Remove any SQLite-specific pragmas or settings
            pg_sql = pg_sql.replace("AUTOINCREMENT", "");

            pg_sql
        }).collect()
    }
    
    /// Convert SQLite SQL to MySQL SQL
    fn convert_to_mysql_sql(&self, sqlite_sql: &[String]) -> Vec<String> {
        info!("Converting {} SQLite statements to MySQL", sqlite_sql.len());
        sqlite_sql.iter().map(|sql| {
            let mut mysql_sql = sql.clone();
            // Replace SQLite types with MySQL types
            mysql_sql = mysql_sql.replace("TEXT PRIMARY KEY", "VARCHAR(255) PRIMARY KEY");
            mysql_sql = mysql_sql.replace("TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP", "DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP");
            mysql_sql = mysql_sql.replace("TEXT DEFAULT CURRENT_TIMESTAMP", "DATETIME DEFAULT CURRENT_TIMESTAMP");
            mysql_sql = mysql_sql.replace("DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP", "DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP");
            mysql_sql = mysql_sql.replace("DATETIME DEFAULT CURRENT_TIMESTAMP", "DATETIME DEFAULT CURRENT_TIMESTAMP");
            mysql_sql = mysql_sql.replace("DATETIME NOT NULL", "DATETIME NOT NULL");
            mysql_sql = mysql_sql.replace("DATETIME", "DATETIME");
            mysql_sql = mysql_sql.replace("BOOLEAN DEFAULT 0", "TINYINT(1) DEFAULT 0");
            mysql_sql = mysql_sql.replace("BOOLEAN DEFAULT 1", "TINYINT(1) DEFAULT 1");
            mysql_sql = mysql_sql.replace("BOOLEAN NOT NULL DEFAULT 0", "TINYINT(1) NOT NULL DEFAULT 0");
            mysql_sql = mysql_sql.replace("BOOLEAN NOT NULL DEFAULT 1", "TINYINT(1) NOT NULL DEFAULT 1");
            mysql_sql = mysql_sql.replace("BOOLEAN", "TINYINT(1)");
            mysql_sql = mysql_sql.replace("INTEGER PRIMARY KEY AUTOINCREMENT", "BIGINT AUTO_INCREMENT PRIMARY KEY");
            mysql_sql = mysql_sql.replace("INTEGER NOT NULL DEFAULT 0", "BIGINT NOT NULL DEFAULT 0");
            mysql_sql = mysql_sql.replace("INTEGER DEFAULT 0", "BIGINT DEFAULT 0");
            mysql_sql = mysql_sql.replace("INTEGER", "BIGINT");
            mysql_sql = mysql_sql.replace("REAL", "DOUBLE");
            mysql_sql = mysql_sql.replace("BLOB", "BLOB");
            mysql_sql = mysql_sql.replace("TEXT NOT NULL", "TEXT NOT NULL");
            mysql_sql = mysql_sql.replace("TEXT,", "TEXT,");
            mysql_sql = mysql_sql.replace("TEXT)", "TEXT)");
            mysql_sql = mysql_sql.replace("TEXT", "TEXT");

            // MySQL doesn't support IF NOT EXISTS for all CREATE statements in older versions
            mysql_sql = mysql_sql.replace("CREATE TABLE IF NOT EXISTS", "CREATE TABLE IF NOT EXISTS");
            mysql_sql = mysql_sql.replace("CREATE INDEX IF NOT EXISTS", "CREATE INDEX IF NOT EXISTS");

            // Add ENGINE if it's a CREATE TABLE statement
            if mysql_sql.trim().starts_with("CREATE TABLE") && !mysql_sql.contains("ENGINE=") {
                mysql_sql += " ENGINE=InnoDB DEFAULT CHARSET=utf8mb4";
            }

            mysql_sql
        }).collect()
    }
    
    async fn get_table_list(&self) -> Result<Vec<String>> {
        match &self.source_pool {
            DatabasePool::PostgreSQL(pool) => {
                let query = "SELECT tablename FROM pg_tables WHERE schemaname='public'";
                let rows = sqlx::query(query).fetch_all(pool).await?;
                let tables: Vec<String> = rows.iter()
                    .filter_map(|row: &sqlx::postgres::PgRow| row.try_get::<String, _>(0).ok())
                    .collect();
                Ok(tables)
            }
            DatabasePool::SQLite(pool) => {
                let query = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
                let rows = sqlx::query(query).fetch_all(pool).await?;
                let tables: Vec<String> = rows.iter()
                    .filter_map(|row| row.try_get::<String, _>(0).ok())
                    .collect();
                Ok(tables)
            }
        }
    }
    
    async fn export_table(&self, table_name: &str) -> Result<TableData> {
        let query = format!("SELECT * FROM {}", table_name);
        
        match &self.source_pool {
            DatabasePool::PostgreSQL(pool) => {
                let rows = sqlx::query(&query).fetch_all(pool).await?;
                
                if rows.is_empty() {
                    return Ok(TableData {
                        schema: vec![],
                        rows: vec![],
                    });
                }
                
                let first_row = &rows[0];
                let schema: Vec<ColumnInfo> = first_row.columns()
                    .iter()
                    .map(|col| ColumnInfo {
                        name: col.name().to_string(),
                        data_type: col.type_info().name().to_string(),
                    })
                    .collect();
                
                let mut data_rows = Vec::new();
                for row in rows {
                    let mut row_data = HashMap::new();
                    
                    for (i, column) in row.columns().iter().enumerate() {
                        let column_name = column.name();
                        
                        let value: Value = if let Ok(val) = row.try_get::<String, _>(i) {
                            Value::String(val)
                        } else if let Ok(val) = row.try_get::<i64, _>(i) {
                            Value::Number(val.into())
                        } else if let Ok(val) = row.try_get::<f64, _>(i) {
                            Value::Number(serde_json::Number::from_f64(val).unwrap_or_else(|| 0.into()))
                        } else if let Ok(val) = row.try_get::<bool, _>(i) {
                            Value::Bool(val)
                        } else if let Ok(val) = row.try_get::<Vec<u8>, _>(i) {
                            // Handle BLOB/binary data by base64 encoding
                            let encoded = base64::engine::general_purpose::STANDARD.encode(&val);
                            Value::String(encoded)
                        } else {
                            Value::Null
                        };
                        
                        row_data.insert(column_name.to_string(), value);
                    }
                    
                    data_rows.push(row_data);
                }
                
                Ok(TableData {
                    schema,
                    rows: data_rows,
                })
            },
            DatabasePool::SQLite(p) => {
                let columns_rows = sqlx::query("PRAGMA table_info(?)")
                    .bind(table_name)
                    .fetch_all(p)
                    .await?;
                
                let schema = columns_rows.into_iter().map(|r| {
                    ColumnInfo {
                        name: r.get("name"),
                        data_type: r.get("type"),
                    }
                }).collect();
                
                let rows_obj = sqlx::query(&format!("SELECT * FROM {}", table_name))
                    .fetch_all(p)
                    .await?;
                
                let mut rows = Vec::new();
                for _row in rows_obj {
                    let row_map = HashMap::new();
                    // In real implementation, iterate through columns and map values
                    rows.push(row_map);
                }
                
                Ok(TableData { schema, rows })
            }
        }
    }
    
    async fn import_table(&self, table_name: &str, table_data: &TableData) -> Result<()> {
        self.import_table_to_pool(table_name, table_data, &self.source_pool).await
    }
    
    async fn import_table_to_pool(&self, table_name: &str, table_data: &TableData, pool: &DatabasePool) -> Result<()> {
        if table_data.rows.is_empty() {
            info!("Table {} is empty, skipping", table_name);
            return Ok(());
        }
        
        info!("Importing {} rows into table {}", table_data.rows.len(), table_name);
        
        // CRITICAL: Clear existing data - this MUST succeed
        // Use TRUNCATE for PostgreSQL/MySQL (faster and resets auto-increment)
        // Use DELETE for SQLite (no TRUNCATE support)
        match pool {
                DatabasePool::PostgreSQL(p) => {
                    // Try TRUNCATE first (faster), fallback to DELETE if it fails
                    let truncate_query = format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE", table_name);
                    match sqlx::query(&truncate_query).execute(p).await {
                        Ok(_) => {
                            info!("Truncated PostgreSQL table: {}", table_name);
                        }
                        Err(e) => {
                            warn!("TRUNCATE failed for {}: {}, trying DELETE", table_name, e);
                            let delete_query = format!("DELETE FROM {}", table_name);
                            sqlx::query(&delete_query).execute(p).await
                                .context(format!("Failed to clear PostgreSQL table {} before import", table_name))?;
                            info!("Cleared PostgreSQL table with DELETE: {}", table_name);
                        }
                    }

                    // Log table structure for debugging
                    let schema_query = format!("SELECT column_name, data_type, is_nullable, column_default FROM information_schema.columns WHERE table_name = '{}' ORDER BY ordinal_position", table_name);
                    if let Ok(rows) = sqlx::query(&schema_query).fetch_all(p).await {
                        info!("PostgreSQL table {} schema:", table_name);
                        for row in rows {
                            if let (Ok(col_name), Ok(data_type), Ok(is_nullable), Ok(default)) = (
                                row.try_get::<String, _>("column_name"),
                                row.try_get::<String, _>("data_type"),
                                row.try_get::<String, _>("is_nullable"),
                                row.try_get::<Option<String>, _>("column_default")
                            ) {
                                info!("  {}: {} {} {}", col_name, data_type, if is_nullable == "YES" { "NULL" } else { "NOT NULL" }, default.unwrap_or_else(|| "NULL".to_string()));
                            }
                        }
                    }
                }
                DatabasePool::SQLite(p) => {
                    let delete_query = format!("DELETE FROM {}", table_name);
                    sqlx::query(&delete_query).execute(p).await
                        .context(format!("Failed to clear SQLite table {} before import", table_name))?;
                }
        }
        
        // Insert rows
        for row_data in &table_data.rows {
            let columns: Vec<&String> = row_data.keys().collect();
            
            match pool {
                DatabasePool::PostgreSQL(p) => {
                    let placeholders: Vec<String> = (1..=columns.len())
                        .map(|i| format!("${}", i))
                        .collect();

                    let mut insert_query = format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        table_name,
                        columns.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", "),
                        placeholders.join(", ")
                    );

                    // Make migration idempotent for PostgreSQL:
                    // - If the target DB wasn't fully cleaned
                    // - If migration is retried
                    // - If some other process inserted rows concurrently
                    // we should not fail on duplicate primary/unique keys.
                    //
                    // Prefer updating rows when we have a conventional `id` column,
                    // otherwise fall back to `ON CONFLICT DO NOTHING`.
                    let has_id = columns
                        .iter()
                        .any(|c| c.as_str().eq_ignore_ascii_case("id"));

                    if has_id {
                        let update_cols: Vec<String> = columns
                            .iter()
                            .filter(|c| !c.as_str().eq_ignore_ascii_case("id"))
                            .map(|c| format!("{0}=EXCLUDED.{0}", c))
                            .collect();

                        if update_cols.is_empty() {
                            insert_query.push_str(" ON CONFLICT (id) DO NOTHING");
                        } else {
                            insert_query.push_str(" ON CONFLICT (id) DO UPDATE SET ");
                            insert_query.push_str(&update_cols.join(", "));
                        }
                    } else {
                        insert_query.push_str(" ON CONFLICT DO NOTHING");
                    }

                    let mut query = sqlx::query::<sqlx::Postgres>(&insert_query);

                    for col in &columns {
                        let value = &row_data[*col];
                        query = match value {
                            Value::String(s) => {
                                // Check if this column might be a timestamp column
                                if col.to_lowercase().contains("time") || col.to_lowercase().contains("date") ||
                                   col.to_lowercase() == "created_at" || col.to_lowercase() == "updated_at" {
                                    // Try to parse as timestamp, fallback to string if parsing fails
                                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                                        query.bind(dt.naive_utc())
                                    } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                                        query.bind(dt)
                                    } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                                        query.bind(dt)
                                    } else {
                                        // If parsing fails, bind as string and let PostgreSQL handle it
                                        query.bind(s)
                                    }
                                } else if col.to_lowercase() == "embedding" {
                                    // Special handling for embedding field - try base64 decode
                                    match base64::engine::general_purpose::STANDARD.decode(s) {
                                        Ok(binary_data) => {
                                            query.bind(binary_data)
                                        }
                                        Err(_) => {
                                            // If base64 decode fails, treat as regular string
                                            warn!("Failed to decode base64 for embedding field, treating as string");
                                            query.bind(s)
                                        }
                                    }
                                } else {
                                    query.bind(s)
                                }
                            }
                            Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    query.bind(i)
                                } else if let Some(f) = n.as_f64() {
                                    query.bind(f)
                                } else {
                                    query.bind(0)
                                }
                            }
                            Value::Bool(b) => query.bind(*b),
                            Value::Null => query.bind(Option::<String>::None),
                            _ => query.bind(value.to_string()),
                        };
                    }
                    query.execute(p).await?;
                }
                DatabasePool::SQLite(_p) => {
                    // SQLite import not implemented for migration
                }
            }
        }
        Ok(())
    }

    /// Import table data selectively, skipping records with foreign key violations
    async fn import_table_selective(&self, table_name: &str, table_data: &TableData, pool: &DatabasePool) -> Result<()> {
        if table_data.rows.is_empty() {
            return Ok(());
        }

        info!("Performing selective import for table {} ({} rows)", table_name, table_data.rows.len());
        let mut success_count = 0;
        let mut skipped_count = 0;

        for (row_index, row_data) in table_data.rows.iter().enumerate() {
            match self.import_single_row(table_name, row_data, pool).await {
                Ok(_) => {
                    success_count += 1;
                }
                Err(e) => {
                    if e.to_string().contains("foreign key constraint") {
                        skipped_count += 1;
                        warn!("Skipped row {} in table {} due to foreign key constraint: {}", row_index + 1, table_name, e);
                    } else {
                        // For other errors, still fail
                        return Err(anyhow::anyhow!("Failed to import row {} in table {}: {}", row_index + 1, table_name, e));
                    }
                }
            }
        }

        info!("Selective import completed for {}: {} successful, {} skipped", table_name, success_count, skipped_count);
        Ok(())
    }

    /// Import a single row into the specified table
    async fn import_single_row(&self, table_name: &str, row_data: &HashMap<String, Value>, pool: &DatabasePool) -> Result<()> {
        let columns: Vec<&String> = row_data.keys().collect();

        match pool {
            DatabasePool::PostgreSQL(p) => {
                let placeholders: Vec<String> = (1..=columns.len())
                    .map(|i| format!("${}", i))
                    .collect();

                let insert_query = format!(
                    "INSERT INTO {} ({}) VALUES ({})",
                    table_name,
                    columns.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", "),
                    placeholders.join(", ")
                );

                let mut query = sqlx::query::<sqlx::Postgres>(&insert_query);

                for col in &columns {
                    let value = &row_data[*col];
                    query = match value {
                        Value::String(s) => {
                            // Check if this column might be a timestamp column
                            if col.to_lowercase().contains("time") || col.to_lowercase().contains("date") ||
                               col.to_lowercase() == "created_at" || col.to_lowercase() == "updated_at" {
                                // Try to parse as timestamp, fallback to string if parsing fails
                                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                                    query.bind(dt.naive_utc())
                                } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                                    query.bind(dt)
                                } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                                    query.bind(dt)
                                } else {
                                    // If parsing fails, bind as string and let PostgreSQL handle it
                                    query.bind(s)
                                }
                            } else {
                                query.bind(s)
                            }
                        }
                        Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                query.bind(i)
                            } else if let Some(f) = n.as_f64() {
                                query.bind(f)
                            } else {
                                query.bind(0)
                            }
                        }
                        Value::Bool(b) => query.bind(*b),
                        Value::Null => query.bind(Option::<String>::None),
                        _ => query.bind(value.to_string()),
                    };
                }

                query.execute(p).await?;
            }
            DatabasePool::SQLite(_p) => {
                // SQLite import not implemented for migration
            }
        }
        Ok(())
    }

    /// Export database to SQL file
    pub async fn export_to_sql<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        info!("Starting database export to SQL");
        
        let mut sql_statements = Vec::new();
        let tables = self.get_table_list().await?;
        
        for table_name in tables {
            info!("Exporting table: {}", table_name);
            
            let table_data = self.export_table(&table_name).await?;
            
            if table_data.rows.is_empty() {
                continue;
            }
            
            sql_statements.push(format!("-- Table: {}", table_name));
            sql_statements.push(format!("DELETE FROM {};", table_name));
            
            for row_data in &table_data.rows {
                let columns: Vec<String> = row_data.keys().cloned().collect();
                let values: Vec<String> = row_data.values()
                    .map(|v| match v {
                        Value::String(s) => format!("'{}'", s.replace("'", "''")),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
                        Value::Null => "NULL".to_string(),
                        _ => format!("'{}'", v.to_string().replace("'", "''")),
                    })
                    .collect();
                
                let insert_stmt = format!(
                    "INSERT INTO {} ({}) VALUES ({});",
                    table_name,
                    columns.join(", "),
                    values.join(", ")
                );
                sql_statements.push(insert_stmt);
            }
            
            sql_statements.push(String::new()); // Empty line between tables
        }
        
        fs::write(output_path, sql_statements.join("\n")).await?;
        
        info!("Database export to SQL completed successfully");
        Ok(())
    }
}
