use serde::{Deserialize, Serialize};

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
                let path = self.path.as_ref()
                    .map(|p| p.clone())
                    .unwrap_or_else(|| {
                        dirs::data_dir()
                            .unwrap_or_else(|| std::path::PathBuf::from("."))
                            .join("sentinel-ai")
                            .join("database.db")
                            .to_string_lossy()
                            .to_string()
                    });
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
                let ssl_mode = if self.enable_ssl { "REQUIRED" } else { "PREFERRED" };
                
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
