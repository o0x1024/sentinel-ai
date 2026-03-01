#[cfg(feature = "db-postgres")]
pub type PgPool = sqlx::postgres::PgPool;
#[cfg(not(feature = "db-postgres"))]
pub type PgPool = sqlx::SqlitePool;

#[cfg(feature = "db-postgres")]
pub type PgPoolOptions = sqlx::postgres::PgPoolOptions;
#[cfg(not(feature = "db-postgres"))]
pub type PgPoolOptions = sqlx::sqlite::SqlitePoolOptions;

#[cfg(feature = "db-postgres")]
pub type PgRow = sqlx::postgres::PgRow;
#[cfg(not(feature = "db-postgres"))]
pub type PgRow = sqlx::sqlite::SqliteRow;

#[cfg(feature = "db-postgres")]
pub type Postgres = sqlx::Postgres;
#[cfg(not(feature = "db-postgres"))]
pub type Postgres = sqlx::Sqlite;

#[cfg(feature = "db-mysql")]
pub type MySqlPool = sqlx::mysql::MySqlPool;
#[cfg(not(feature = "db-mysql"))]
pub type MySqlPool = sqlx::SqlitePool;

#[cfg(feature = "db-mysql")]
pub type MySqlPoolOptions = sqlx::mysql::MySqlPoolOptions;
#[cfg(not(feature = "db-mysql"))]
pub type MySqlPoolOptions = sqlx::sqlite::SqlitePoolOptions;

#[cfg(feature = "db-mysql")]
pub type MySqlRow = sqlx::mysql::MySqlRow;
#[cfg(not(feature = "db-mysql"))]
pub type MySqlRow = sqlx::sqlite::SqliteRow;

#[cfg(feature = "db-mysql")]
pub type MySql = sqlx::MySql;
#[cfg(not(feature = "db-mysql"))]
pub type MySql = sqlx::Sqlite;
