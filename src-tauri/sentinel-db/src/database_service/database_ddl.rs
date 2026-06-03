use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;

impl DatabaseService {
    pub(crate) async fn execute_runtime_ddl(
        &self,
        runtime: &DatabasePool,
        sql: &str,
    ) -> Result<()> {
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(sql).execute(pool).await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(sql).execute(pool).await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(sql).execute(pool).await?;
            }
        }
        Ok(())
    }

    pub(crate) async fn execute_sql_script(
        &self,
        runtime: &DatabasePool,
        sql_script: &str,
    ) -> Result<()> {
        for statement in split_sql_statements(sql_script) {
            if statement.trim().is_empty() {
                continue;
            }
            self.execute_runtime_ddl(runtime, &statement).await?;
        }
        Ok(())
    }
}

fn split_sql_statements(script: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = script.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(ch);
            }
            ';' if !in_single && !in_double => {
                let stmt = current.trim();
                if !stmt.is_empty() && !stmt.starts_with("--") {
                    out.push(stmt.to_string());
                }
                current.clear();
            }
            '-' if !in_single && !in_double && chars.peek() == Some(&'-') => {
                let _ = chars.next();
                for next in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
            }
            _ => current.push(ch),
        }
    }

    let tail = current.trim();
    if !tail.is_empty() && !tail.starts_with("--") {
        out.push(tail.to_string());
    }
    out
}
