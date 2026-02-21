    /// Batch delete bounty findings
    pub async fn batch_delete_bounty_findings(&self, ids: &[String]) -> Result<u64> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        if ids.is_empty() { return Ok(0); }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let pg_placeholders = ids.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect::<Vec<_>>().join(",");
        match runtime {
            DatabasePool::SQLite(pool) => {
                let query_str = format!("DELETE FROM bounty_findings WHERE id IN ({})", placeholders);
                let mut query = sqlx::query(&query_str);
                for id in ids { query = query.bind(id); }
                let result = query.execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::MySQL(pool) => {
                let query_str = format!("DELETE FROM bounty_findings WHERE id IN ({})", placeholders);
                let mut query = sqlx::query(&query_str);
                for id in ids { query = query.bind(id); }
                let result = query.execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::PostgreSQL(_) => {
                let query_str = format!("DELETE FROM bounty_findings WHERE id IN ({})", pg_placeholders);
                let mut query = sqlx::query(&query_str);
                for id in ids { query = query.bind(id); }
                let result = query.execute(self.get_pool()?).await?;
                Ok(result.rows_affected())
            }
        }
    }

    /// Batch update bounty finding status
    pub async fn batch_update_bounty_finding_status(&self, ids: &[String], status: &str) -> Result<u64> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        if ids.is_empty() { return Ok(0); }
        let now = Utc::now().to_rfc3339();
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let pg_placeholders = ids.iter().enumerate().map(|(i, _)| format!("${}", i + 3)).collect::<Vec<_>>().join(",");
        match runtime {
            DatabasePool::SQLite(pool) => {
                let query_str = format!("UPDATE bounty_findings SET status = ?, updated_at = ? WHERE id IN ({})", placeholders);
                let mut query = sqlx::query(&query_str).bind(status).bind(&now);
                for id in ids { query = query.bind(id); }
                let result = query.execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::MySQL(pool) => {
                let query_str = format!("UPDATE bounty_findings SET status = ?, updated_at = ? WHERE id IN ({})", placeholders);
                let mut query = sqlx::query(&query_str).bind(status).bind(&now);
                for id in ids { query = query.bind(id); }
                let result = query.execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::PostgreSQL(_) => {
                let query_str = format!("UPDATE bounty_findings SET status = $1, updated_at = $2 WHERE id IN ({})", pg_placeholders);
                let mut query = sqlx::query(&query_str).bind(status).bind(&now);
                for id in ids { query = query.bind(id); }
                let result = query.execute(self.get_pool()?).await?;
                Ok(result.rows_affected())
            }
        }
    }
