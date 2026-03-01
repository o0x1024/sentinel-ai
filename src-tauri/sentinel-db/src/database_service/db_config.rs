use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    SQLite,
    PostgreSQL,
    MySQL,
}

impl DatabaseType {
    pub fn as_str(&self) -> &str {
        match self {
            DatabaseType::SQLite => "sqlite",
            DatabaseType::PostgreSQL => "postgresql",
            DatabaseType::MySQL => "mysql",
        }
    }
}

impl From<String> for DatabaseType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "postgres" | "postgresql" => DatabaseType::PostgreSQL,
            "mysql" => DatabaseType::MySQL,
            _ => DatabaseType::SQLite,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub db_type: DatabaseType,

    // SQLite config
    pub path: Option<String>,
    pub enable_wal: bool,

    // PostgreSQL/MySQL config
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub enable_ssl: bool,

    // Connection pool settings
    pub max_connections: u32,
    pub query_timeout: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_type: DatabaseType::SQLite,
            path: None,
            enable_wal: true,
            host: Some("localhost".to_string()),
            port: None,
            database: Some("sentinel_ai".to_string()),
            username: None,
            password: None,
            enable_ssl: false,
            max_connections: 10,
            query_timeout: 30,
        }
    }
}

impl DatabaseConfig {
    pub fn build_connection_string(&self) -> String {
        match self.db_type {
            DatabaseType::SQLite => {
                let path = self
                    .path
                    .as_ref()
                    .map(|p| p.clone())
                    .unwrap_or_else(default_sqlite_db_path);
                format!("sqlite:{}?mode=rwc", path)
            }
            DatabaseType::PostgreSQL => {
                let localhost = "localhost".to_string();
                let default_db = "sentinel_ai".to_string();
                let host = self.host.as_ref().unwrap_or(&localhost);
                let port = self.port.unwrap_or(5432);
                let database = self.database.as_ref().unwrap_or(&default_db);
                let ssl_mode = if self.enable_ssl { "require" } else { "prefer" };

                if let (Some(user), Some(pass)) = (&self.username, &self.password) {
                    format!(
                        "postgresql://{}:{}@{}:{}/{}?sslmode={}",
                        user, pass, host, port, database, ssl_mode
                    )
                } else {
                    format!(
                        "postgresql://{}:{}/{}?sslmode={}",
                        host, port, database, ssl_mode
                    )
                }
            }
            DatabaseType::MySQL => {
                let localhost = "localhost".to_string();
                let default_db = "sentinel_ai".to_string();
                let host = self.host.as_ref().unwrap_or(&localhost);
                let port = self.port.unwrap_or(3306);
                let database = self.database.as_ref().unwrap_or(&default_db);
                let ssl_mode = if self.enable_ssl {
                    "REQUIRED"
                } else {
                    "PREFERRED"
                };

                if let (Some(user), Some(pass)) = (&self.username, &self.password) {
                    format!(
                        "mysql://{}:{}@{}:{}/{}?ssl-mode={}",
                        user, pass, host, port, database, ssl_mode
                    )
                } else {
                    format!(
                        "mysql://{}:{}/{}?ssl-mode={}",
                        host, port, database, ssl_mode
                    )
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DbConfigTomlFile {
    #[serde(default = "default_active_db")]
    active_db: String,
    #[serde(default)]
    sqlite: Option<SqliteConfigSection>,
    #[serde(default)]
    postgresql: Option<NetworkConfigSection>,
    #[serde(default)]
    mysql: Option<NetworkConfigSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SqliteConfigSection {
    path: Option<String>,
    #[serde(default = "default_true")]
    enable_wal: bool,
    max_connections: Option<u32>,
    query_timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkConfigSection {
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    username: Option<String>,
    password: Option<String>,
    #[serde(default)]
    enable_ssl: bool,
    max_connections: Option<u32>,
    query_timeout: Option<u64>,
}

fn default_active_db() -> String {
    "sqlite".to_string()
}

fn default_true() -> bool {
    true
}

fn default_config_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sentinel-ai")
}

pub fn default_sqlite_db_path() -> String {
    default_config_dir()
        .join("database.db")
        .to_string_lossy()
        .to_string()
}

pub fn db_config_toml_path() -> PathBuf {
    default_config_dir().join("db_config.toml")
}

pub fn db_config_legacy_json_path() -> PathBuf {
    default_config_dir().join("db_config.json")
}

fn to_toml_file(config: &DatabaseConfig, existing: Option<DbConfigTomlFile>) -> DbConfigTomlFile {
    let mut file = existing.unwrap_or_default();
    file.active_db = config.db_type.as_str().to_string();

    match config.db_type {
        DatabaseType::SQLite => {
            file.sqlite = Some(SqliteConfigSection {
                path: Some(default_sqlite_db_path()),
                enable_wal: config.enable_wal,
                max_connections: Some(config.max_connections),
                query_timeout: Some(config.query_timeout),
            });
        }
        DatabaseType::PostgreSQL => {
            file.postgresql = Some(NetworkConfigSection {
                host: config.host.clone(),
                port: config.port,
                database: config.database.clone(),
                username: config.username.clone(),
                password: config.password.clone(),
                enable_ssl: config.enable_ssl,
                max_connections: Some(config.max_connections),
                query_timeout: Some(config.query_timeout),
            });
        }
        DatabaseType::MySQL => {
            file.mysql = Some(NetworkConfigSection {
                host: config.host.clone(),
                port: config.port,
                database: config.database.clone(),
                username: config.username.clone(),
                password: config.password.clone(),
                enable_ssl: config.enable_ssl,
                max_connections: Some(config.max_connections),
                query_timeout: Some(config.query_timeout),
            });
        }
    }

    file
}

fn from_toml_file(file: DbConfigTomlFile) -> DatabaseConfig {
    let db_type = DatabaseType::from(file.active_db);
    let mut config = DatabaseConfig::default();

    match db_type {
        DatabaseType::SQLite => {
            let sqlite = file.sqlite.unwrap_or(SqliteConfigSection {
                path: None,
                enable_wal: true,
                max_connections: None,
                query_timeout: None,
            });
            config.db_type = DatabaseType::SQLite;
            config.path = Some(sqlite.path.unwrap_or_else(default_sqlite_db_path));
            config.enable_wal = sqlite.enable_wal;
            config.host = None;
            config.port = None;
            config.database = None;
            config.username = None;
            config.password = None;
            config.enable_ssl = false;
            config.max_connections = sqlite.max_connections.unwrap_or(config.max_connections);
            config.query_timeout = sqlite.query_timeout.unwrap_or(config.query_timeout);
        }
        DatabaseType::PostgreSQL => {
            let pg = file.postgresql.unwrap_or(NetworkConfigSection {
                host: Some("localhost".to_string()),
                port: Some(5432),
                database: Some("sentinel_ai".to_string()),
                username: Some("postgres".to_string()),
                password: Some("postgres".to_string()),
                enable_ssl: false,
                max_connections: None,
                query_timeout: None,
            });
            config.db_type = DatabaseType::PostgreSQL;
            config.path = None;
            config.enable_wal = false;
            config.host = pg.host;
            config.port = pg.port.or(Some(5432));
            config.database = pg.database;
            config.username = pg.username;
            config.password = pg.password;
            config.enable_ssl = pg.enable_ssl;
            config.max_connections = pg.max_connections.unwrap_or(config.max_connections);
            config.query_timeout = pg.query_timeout.unwrap_or(config.query_timeout);
        }
        DatabaseType::MySQL => {
            let mysql = file.mysql.unwrap_or(NetworkConfigSection {
                host: Some("localhost".to_string()),
                port: Some(3306),
                database: Some("sentinel_ai".to_string()),
                username: None,
                password: None,
                enable_ssl: false,
                max_connections: None,
                query_timeout: None,
            });
            config.db_type = DatabaseType::MySQL;
            config.path = None;
            config.enable_wal = false;
            config.host = mysql.host;
            config.port = mysql.port.or(Some(3306));
            config.database = mysql.database;
            config.username = mysql.username;
            config.password = mysql.password;
            config.enable_ssl = mysql.enable_ssl;
            config.max_connections = mysql.max_connections.unwrap_or(config.max_connections);
            config.query_timeout = mysql.query_timeout.unwrap_or(config.query_timeout);
        }
    }

    config
}

fn normalize_loaded_config(mut cfg: DatabaseConfig) -> DatabaseConfig {
    if matches!(cfg.db_type, DatabaseType::SQLite) {
        cfg.path = Some(default_sqlite_db_path());
        cfg.host = None;
        cfg.port = None;
        cfg.database = None;
        cfg.username = None;
        cfg.password = None;
        cfg.enable_ssl = false;
    }
    cfg
}

pub fn load_db_config_from_disk() -> anyhow::Result<Option<DatabaseConfig>> {
    let toml_path = db_config_toml_path();
    if toml_path.exists() {
        let content = fs::read_to_string(&toml_path)?;
        let file: DbConfigTomlFile = toml::from_str(&content)?;
        return Ok(Some(normalize_loaded_config(from_toml_file(file))));
    }

    let json_path = db_config_legacy_json_path();
    if json_path.exists() {
        let content = fs::read_to_string(&json_path)?;
        let cfg: DatabaseConfig = serde_json::from_str(&content)?;
        return Ok(Some(normalize_loaded_config(cfg)));
    }

    Ok(None)
}

pub fn save_db_config_to_disk(config: &DatabaseConfig) -> anyhow::Result<PathBuf> {
    let toml_path = db_config_toml_path();
    if let Some(parent) = toml_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing = if toml_path.exists() {
        match fs::read_to_string(&toml_path) {
            Ok(content) => toml::from_str::<DbConfigTomlFile>(&content).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let file = to_toml_file(config, existing);
    let content = toml::to_string_pretty(&file)?;
    fs::write(&toml_path, content)?;

    Ok(toml_path)
}
