use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct EntitlementStore {
    pool: SqlitePool,
}

#[derive(Debug, Clone)]
pub struct StoredCustomer {
    pub customer_id: String,
    pub license_id: String,
    pub tier: String,
    pub feature_ids: Vec<String>,
    pub device_limit: usize,
    pub revoked: bool,
    pub allowed_machine_ids: Vec<String>,
    pub ttl_seconds: Option<i64>,
    pub refresh_api_key_hash: Option<String>,
    pub refresh_api_key_last_used_at: Option<i64>,
    pub last_entitlement_issued_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct StoredDeviceBinding {
    pub customer_id: String,
    pub machine_id: String,
    pub bound_at: i64,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone)]
pub struct StoredAuditEvent {
    pub id: i64,
    pub event_type: String,
    pub actor: String,
    pub customer_id: Option<String>,
    pub machine_id: Option<String>,
    pub details: Option<String>,
    pub metadata: Option<Value>,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct StoredAdminUser {
    pub admin_id: String,
    pub role: String,
    pub active: bool,
    pub api_key_hash: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct NewAuditEvent {
    pub event_type: String,
    pub actor: String,
    pub customer_id: Option<String>,
    pub machine_id: Option<String>,
    pub details: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct EntitlementPolicy {
    #[serde(default = "default_feature_ids")]
    pub default_feature_ids: Vec<String>,
    #[serde(default)]
    pub customers: Vec<CustomerPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomerPolicy {
    pub customer_id: String,
    pub license_id: String,
    #[serde(default = "default_tier")]
    pub tier: String,
    #[serde(default)]
    pub feature_ids: Vec<String>,
    #[serde(default = "default_device_limit")]
    pub device_limit: usize,
    #[serde(default)]
    pub revoked: bool,
    #[serde(default)]
    pub allowed_machine_ids: Vec<String>,
    #[serde(default)]
    pub ttl_seconds: Option<i64>,
    #[serde(default)]
    pub refresh_api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub enum CustomerRefreshApiKeyUpdate {
    Preserve,
    Set(String),
    Clear,
}

impl EntitlementStore {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .with_context(|| format!("failed to connect entitlement database {}", database_url))?;
        let store = Self { pool };
        store.init_schema().await?;
        Ok(store)
    }

    pub async fn bootstrap_from_policy_file(&self, path: &Path) -> Result<()> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read policy file {}", path.display()))?;
        let policy: EntitlementPolicy = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse policy file {}", path.display()))?;

        for customer in policy.customers {
            let customer_id = customer.customer_id.clone();
            let feature_ids = if customer.feature_ids.is_empty() {
                policy.default_feature_ids.clone()
            } else {
                customer.feature_ids.clone()
            };
            self.upsert_customer(&StoredCustomer {
                customer_id: customer_id.clone(),
                license_id: customer.license_id,
                tier: customer.tier,
                feature_ids,
                device_limit: customer.device_limit,
                revoked: customer.revoked,
                allowed_machine_ids: customer.allowed_machine_ids,
                ttl_seconds: customer.ttl_seconds,
                refresh_api_key_hash: None,
                refresh_api_key_last_used_at: None,
                last_entitlement_issued_at: None,
            })
            .await?;
            if let Some(refresh_api_key) = customer.refresh_api_key.as_deref() {
                self.upsert_customer_refresh_api_key(
                    &customer_id,
                    CustomerRefreshApiKeyUpdate::Set(Self::hash_api_key(refresh_api_key)),
                )
                .await?;
            }
        }

        Ok(())
    }

    pub async fn customer_count(&self) -> Result<i64> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM entitlement_customers")
            .fetch_one(&self.pool)
            .await
            .context("failed to count entitlement customers")
    }

    pub async fn device_binding_count(&self) -> Result<i64> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM entitlement_device_bindings")
            .fetch_one(&self.pool)
            .await
            .context("failed to count entitlement device bindings")
    }

    pub async fn audit_event_count(&self) -> Result<i64> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM entitlement_audit_events")
            .fetch_one(&self.pool)
            .await
            .context("failed to count entitlement audit events")
    }

    pub async fn admin_user_count(&self) -> Result<i64> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM entitlement_admin_users")
            .fetch_one(&self.pool)
            .await
            .context("failed to count entitlement admin users")
    }

    pub async fn active_admin_count(&self) -> Result<i64> {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM entitlement_admin_users WHERE active = 1 AND role = 'admin'",
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to count active admin users")
    }

    pub async fn get_customer(&self, customer_id: &str) -> Result<Option<StoredCustomer>> {
        let row = sqlx::query(
            r#"
            SELECT customer_id, license_id, tier, feature_ids_json, device_limit,
                   revoked, allowed_machine_ids_json, ttl_seconds, refresh_api_key_hash,
                   refresh_api_key_last_used_at, last_entitlement_issued_at
            FROM entitlement_customers
            WHERE customer_id = ?
            "#,
        )
        .bind(customer_id)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("failed to load customer {}", customer_id))?;

        row.map(decode_customer_row).transpose()
    }

    pub async fn list_customers(&self) -> Result<Vec<StoredCustomer>> {
        let rows = sqlx::query(
            r#"
            SELECT customer_id, license_id, tier, feature_ids_json, device_limit,
                   revoked, allowed_machine_ids_json, ttl_seconds, refresh_api_key_hash,
                   refresh_api_key_last_used_at, last_entitlement_issued_at
            FROM entitlement_customers
            ORDER BY customer_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list entitlement customers")?;

        rows.into_iter().map(decode_customer_row).collect()
    }

    pub async fn ensure_machine_binding(
        &self,
        customer_id: &str,
        machine_id_full: &str,
        device_limit: usize,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start entitlement binding transaction")?;
        let now = current_unix_timestamp();

        let existing = sqlx::query_scalar::<_, String>(
            r#"
            SELECT machine_id
            FROM entitlement_device_bindings
            WHERE customer_id = ? AND machine_id = ?
            "#,
        )
        .bind(customer_id)
        .bind(machine_id_full)
        .fetch_optional(&mut *tx)
        .await
        .context("failed to query existing device binding")?;

        if existing.is_some() {
            sqlx::query(
                r#"
                UPDATE entitlement_device_bindings
                SET last_seen_at = ?
                WHERE customer_id = ? AND machine_id = ?
                "#,
            )
            .bind(now)
            .bind(customer_id)
            .bind(machine_id_full)
            .execute(&mut *tx)
            .await
            .context("failed to update device binding last_seen_at")?;
            tx.commit()
                .await
                .context("failed to commit existing binding update")?;
            return Ok(());
        }

        let device_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM entitlement_device_bindings WHERE customer_id = ?",
        )
        .bind(customer_id)
        .fetch_one(&mut *tx)
        .await
        .context("failed to count bound devices")?;
        if device_count >= device_limit as i64 {
            return Err(anyhow!(
                "device limit exceeded for customer {} (limit {})",
                customer_id,
                device_limit
            ));
        }

        sqlx::query(
            r#"
            INSERT INTO entitlement_device_bindings (
                customer_id, machine_id, bound_at, last_seen_at
            ) VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(customer_id)
        .bind(machine_id_full)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .context("failed to insert entitlement device binding")?;
        tx.commit()
            .await
            .context("failed to commit entitlement device binding")?;
        Ok(())
    }

    pub async fn list_device_bindings(
        &self,
        customer_id: &str,
    ) -> Result<Vec<StoredDeviceBinding>> {
        let rows = sqlx::query(
            r#"
            SELECT customer_id, machine_id, bound_at, last_seen_at
            FROM entitlement_device_bindings
            WHERE customer_id = ?
            ORDER BY last_seen_at DESC, bound_at DESC
            "#,
        )
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("failed to list device bindings for {}", customer_id))?;

        rows.into_iter().map(decode_device_binding_row).collect()
    }

    pub async fn delete_device_binding(
        &self,
        customer_id: &str,
        machine_id_full: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM entitlement_device_bindings
            WHERE customer_id = ? AND machine_id = ?
            "#,
        )
        .bind(customer_id)
        .bind(machine_id_full)
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "failed to delete device binding {} for {}",
                machine_id_full, customer_id
            )
        })?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn insert_audit_event(&self, event: NewAuditEvent) -> Result<()> {
        let metadata_json = event
            .metadata
            .map(|value| serde_json::to_string(&value))
            .transpose()
            .context("failed to encode audit metadata")?;

        sqlx::query(
            r#"
            INSERT INTO entitlement_audit_events (
                event_type, actor, customer_id, machine_id, details, metadata_json, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event.event_type)
        .bind(event.actor)
        .bind(event.customer_id)
        .bind(event.machine_id)
        .bind(event.details)
        .bind(metadata_json)
        .bind(current_unix_timestamp())
        .execute(&self.pool)
        .await
        .context("failed to insert entitlement audit event")?;

        Ok(())
    }

    pub async fn list_audit_events(
        &self,
        customer_id: Option<&str>,
        event_type: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<StoredAuditEvent>> {
        let capped_limit = limit.clamp(1, 200) as i64;
        let capped_offset = offset.min(10_000) as i64;
        let rows = if let (Some(customer_id), Some(event_type)) = (customer_id, event_type) {
            sqlx::query(
                r#"
                SELECT id, event_type, actor, customer_id, machine_id, details, metadata_json, created_at
                FROM entitlement_audit_events
                WHERE customer_id = ? AND event_type = ?
                ORDER BY created_at DESC, id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(customer_id)
            .bind(event_type)
            .bind(capped_limit)
            .bind(capped_offset)
            .fetch_all(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "failed to list audit events for {} with event_type {}",
                    customer_id, event_type
                )
            })?
        } else if let Some(customer_id) = customer_id {
            sqlx::query(
                r#"
                SELECT id, event_type, actor, customer_id, machine_id, details, metadata_json, created_at
                FROM entitlement_audit_events
                WHERE customer_id = ?
                ORDER BY created_at DESC, id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(customer_id)
            .bind(capped_limit)
            .bind(capped_offset)
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!("failed to list audit events for {}", customer_id))?
        } else if let Some(event_type) = event_type {
            sqlx::query(
                r#"
                SELECT id, event_type, actor, customer_id, machine_id, details, metadata_json, created_at
                FROM entitlement_audit_events
                WHERE event_type = ?
                ORDER BY created_at DESC, id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(event_type)
            .bind(capped_limit)
            .bind(capped_offset)
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!("failed to list audit events for event_type {}", event_type))?
        } else {
            sqlx::query(
                r#"
                SELECT id, event_type, actor, customer_id, machine_id, details, metadata_json, created_at
                FROM entitlement_audit_events
                ORDER BY created_at DESC, id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(capped_limit)
            .bind(capped_offset)
            .fetch_all(&self.pool)
            .await
            .context("failed to list entitlement audit events")?
        };

        rows.into_iter().map(decode_audit_event_row).collect()
    }

    pub async fn list_admin_users(&self) -> Result<Vec<StoredAdminUser>> {
        let rows = sqlx::query(
            r#"
            SELECT admin_id, role, active, api_key_hash, created_at, updated_at, last_used_at
            FROM entitlement_admin_users
            ORDER BY admin_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list entitlement admin users")?;

        rows.into_iter().map(decode_admin_user_row).collect()
    }

    pub async fn get_admin_user(&self, admin_id: &str) -> Result<Option<StoredAdminUser>> {
        let row = sqlx::query(
            r#"
            SELECT admin_id, role, active, api_key_hash, created_at, updated_at, last_used_at
            FROM entitlement_admin_users
            WHERE admin_id = ?
            "#,
        )
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("failed to load admin user {}", admin_id))?;

        row.map(decode_admin_user_row).transpose()
    }

    pub async fn find_admin_by_api_key_hash(
        &self,
        api_key_hash: &str,
    ) -> Result<Option<StoredAdminUser>> {
        let row = sqlx::query(
            r#"
            SELECT admin_id, role, active, api_key_hash, created_at, updated_at, last_used_at
            FROM entitlement_admin_users
            WHERE api_key_hash = ?
            "#,
        )
        .bind(api_key_hash)
        .fetch_optional(&self.pool)
        .await
        .context("failed to find admin by api_key_hash")?;

        row.map(decode_admin_user_row).transpose()
    }

    pub async fn upsert_admin_user(
        &self,
        admin_id: &str,
        role: &str,
        active: bool,
        api_key_hash: Option<&str>,
    ) -> Result<()> {
        let now = current_unix_timestamp();
        let existing = self.get_admin_user(admin_id).await?;
        let resolved_api_key_hash = match (api_key_hash, existing.as_ref()) {
            (Some(hash), _) => hash.to_string(),
            (None, Some(existing)) => existing.api_key_hash.clone(),
            (None, None) => {
                return Err(anyhow!(
                    "api_key is required when creating a new admin user"
                ))
            }
        };

        sqlx::query(
            r#"
            INSERT INTO entitlement_admin_users (
                admin_id, role, active, api_key_hash, created_at, updated_at, last_used_at
            ) VALUES (?, ?, ?, ?, ?, ?, NULL)
            ON CONFLICT(admin_id) DO UPDATE SET
                role = excluded.role,
                active = excluded.active,
                api_key_hash = excluded.api_key_hash,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(admin_id)
        .bind(role)
        .bind(if active { 1_i64 } else { 0_i64 })
        .bind(resolved_api_key_hash)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .with_context(|| format!("failed to upsert admin user {}", admin_id))?;

        Ok(())
    }

    pub async fn touch_admin_last_used(&self, admin_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE entitlement_admin_users
            SET last_used_at = ?, updated_at = ?
            WHERE admin_id = ?
            "#,
        )
        .bind(current_unix_timestamp())
        .bind(current_unix_timestamp())
        .bind(admin_id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("failed to touch admin user {}", admin_id))?;

        Ok(())
    }

    pub async fn ensure_bootstrap_admin(
        &self,
        admin_id: &str,
        role: &str,
        api_key: &str,
    ) -> Result<()> {
        let hash = Self::hash_api_key(api_key);
        self.upsert_admin_user(admin_id, role, true, Some(&hash))
            .await
    }

    pub fn hash_api_key(api_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(api_key.trim().as_bytes());
        hex::encode(hasher.finalize())
    }

    async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS entitlement_customers (
                customer_id TEXT PRIMARY KEY,
                license_id TEXT NOT NULL,
                tier TEXT NOT NULL,
                feature_ids_json TEXT NOT NULL DEFAULT '[]',
                device_limit INTEGER NOT NULL DEFAULT 3,
                revoked INTEGER NOT NULL DEFAULT 0,
                allowed_machine_ids_json TEXT NOT NULL DEFAULT '[]',
                ttl_seconds INTEGER NULL,
                refresh_api_key_hash TEXT NULL,
                refresh_api_key_last_used_at INTEGER NULL,
                last_entitlement_issued_at INTEGER NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create entitlement_customers table")?;
        add_column_if_missing(
            &self.pool,
            "ALTER TABLE entitlement_customers ADD COLUMN refresh_api_key_hash TEXT NULL",
        )
        .await
        .context("failed to ensure refresh_api_key_hash column")?;
        add_column_if_missing(
            &self.pool,
            "ALTER TABLE entitlement_customers ADD COLUMN refresh_api_key_last_used_at INTEGER NULL",
        )
        .await
        .context("failed to ensure refresh_api_key_last_used_at column")?;
        add_column_if_missing(
            &self.pool,
            "ALTER TABLE entitlement_customers ADD COLUMN last_entitlement_issued_at INTEGER NULL",
        )
        .await
        .context("failed to ensure last_entitlement_issued_at column")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS entitlement_device_bindings (
                customer_id TEXT NOT NULL,
                machine_id TEXT NOT NULL,
                bound_at INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL,
                PRIMARY KEY (customer_id, machine_id),
                FOREIGN KEY (customer_id) REFERENCES entitlement_customers(customer_id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create entitlement_device_bindings table")?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_entitlement_device_bindings_customer_id ON entitlement_device_bindings(customer_id)",
        )
        .execute(&self.pool)
        .await
        .context("failed to create entitlement_device_bindings index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS entitlement_audit_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                actor TEXT NOT NULL,
                customer_id TEXT NULL,
                machine_id TEXT NULL,
                details TEXT NULL,
                metadata_json TEXT NULL,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create entitlement_audit_events table")?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_entitlement_audit_events_customer_id ON entitlement_audit_events(customer_id, created_at DESC)",
        )
        .execute(&self.pool)
        .await
        .context("failed to create entitlement_audit_events customer index")?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_entitlement_audit_events_created_at ON entitlement_audit_events(created_at DESC)",
        )
        .execute(&self.pool)
        .await
        .context("failed to create entitlement_audit_events created_at index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS entitlement_admin_users (
                admin_id TEXT PRIMARY KEY,
                role TEXT NOT NULL,
                active INTEGER NOT NULL DEFAULT 1,
                api_key_hash TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                last_used_at INTEGER NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create entitlement_admin_users table")?;

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_entitlement_admin_users_api_key_hash ON entitlement_admin_users(api_key_hash)",
        )
        .execute(&self.pool)
        .await
        .context("failed to create entitlement_admin_users api_key_hash index")?;

        Ok(())
    }

    pub async fn upsert_customer(&self, customer: &StoredCustomer) -> Result<()> {
        let now = current_unix_timestamp();
        let feature_ids_json =
            serde_json::to_string(&customer.feature_ids).context("failed to encode feature_ids")?;
        let allowed_machine_ids_json = serde_json::to_string(&customer.allowed_machine_ids)
            .context("failed to encode allowed_machine_ids")?;
        let existing_customer = self.get_customer(&customer.customer_id).await?;
        let existing_refresh_api_key_hash = existing_customer
            .as_ref()
            .and_then(|item| item.refresh_api_key_hash.clone());
        let existing_refresh_api_key_last_used_at = existing_customer
            .as_ref()
            .and_then(|item| item.refresh_api_key_last_used_at);
        let existing_last_entitlement_issued_at = existing_customer
            .as_ref()
            .and_then(|item| item.last_entitlement_issued_at);
        let refresh_api_key_hash = customer
            .refresh_api_key_hash
            .clone()
            .or(existing_refresh_api_key_hash);

        sqlx::query(
            r#"
            INSERT INTO entitlement_customers (
                customer_id, license_id, tier, feature_ids_json, device_limit,
                revoked, allowed_machine_ids_json, ttl_seconds, refresh_api_key_hash,
                refresh_api_key_last_used_at, last_entitlement_issued_at, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(customer_id) DO UPDATE SET
                license_id = excluded.license_id,
                tier = excluded.tier,
                feature_ids_json = excluded.feature_ids_json,
                device_limit = excluded.device_limit,
                revoked = excluded.revoked,
                allowed_machine_ids_json = excluded.allowed_machine_ids_json,
                ttl_seconds = excluded.ttl_seconds,
                refresh_api_key_hash = excluded.refresh_api_key_hash,
                refresh_api_key_last_used_at = excluded.refresh_api_key_last_used_at,
                last_entitlement_issued_at = excluded.last_entitlement_issued_at,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&customer.customer_id)
        .bind(&customer.license_id)
        .bind(&customer.tier)
        .bind(feature_ids_json)
        .bind(customer.device_limit as i64)
        .bind(if customer.revoked { 1_i64 } else { 0_i64 })
        .bind(allowed_machine_ids_json)
        .bind(customer.ttl_seconds)
        .bind(refresh_api_key_hash)
        .bind(
            customer
                .refresh_api_key_last_used_at
                .or(existing_refresh_api_key_last_used_at),
        )
        .bind(
            customer
                .last_entitlement_issued_at
                .or(existing_last_entitlement_issued_at),
        )
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .with_context(|| format!("failed to upsert customer {}", customer.customer_id))?;

        Ok(())
    }

    pub async fn upsert_customer_refresh_api_key(
        &self,
        customer_id: &str,
        update: CustomerRefreshApiKeyUpdate,
    ) -> Result<()> {
        let resolved_hash = match update {
            CustomerRefreshApiKeyUpdate::Preserve => self
                .get_customer(customer_id)
                .await?
                .and_then(|item| item.refresh_api_key_hash),
            CustomerRefreshApiKeyUpdate::Set(hash) => Some(hash),
            CustomerRefreshApiKeyUpdate::Clear => None,
        };
        sqlx::query(
            r#"
            UPDATE entitlement_customers
            SET refresh_api_key_hash = ?, updated_at = ?
            WHERE customer_id = ?
            "#,
        )
        .bind(resolved_hash)
        .bind(current_unix_timestamp())
        .bind(customer_id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("failed to update customer refresh api key {}", customer_id))?;
        Ok(())
    }

    pub async fn touch_customer_refresh_api_key_last_used(&self, customer_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE entitlement_customers
            SET refresh_api_key_last_used_at = ?, updated_at = ?
            WHERE customer_id = ?
            "#,
        )
        .bind(current_unix_timestamp())
        .bind(current_unix_timestamp())
        .bind(customer_id)
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "failed to update refresh_api_key_last_used_at for {}",
                customer_id
            )
        })?;
        Ok(())
    }

    pub async fn touch_customer_entitlement_issued(&self, customer_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE entitlement_customers
            SET last_entitlement_issued_at = ?, updated_at = ?
            WHERE customer_id = ?
            "#,
        )
        .bind(current_unix_timestamp())
        .bind(current_unix_timestamp())
        .bind(customer_id)
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "failed to update last_entitlement_issued_at for {}",
                customer_id
            )
        })?;
        Ok(())
    }
}

fn decode_customer_row(row: sqlx::sqlite::SqliteRow) -> Result<StoredCustomer> {
    let feature_ids_json: String = row.try_get("feature_ids_json")?;
    let allowed_machine_ids_json: String = row.try_get("allowed_machine_ids_json")?;

    Ok(StoredCustomer {
        customer_id: row.try_get("customer_id")?,
        license_id: row.try_get("license_id")?,
        tier: row.try_get("tier")?,
        feature_ids: serde_json::from_str(&feature_ids_json)
            .context("failed to decode feature_ids_json")?,
        device_limit: row.try_get::<i64, _>("device_limit")? as usize,
        revoked: row.try_get::<i64, _>("revoked")? != 0,
        allowed_machine_ids: serde_json::from_str(&allowed_machine_ids_json)
            .context("failed to decode allowed_machine_ids_json")?,
        ttl_seconds: row.try_get("ttl_seconds")?,
        refresh_api_key_hash: row.try_get("refresh_api_key_hash")?,
        refresh_api_key_last_used_at: row.try_get("refresh_api_key_last_used_at")?,
        last_entitlement_issued_at: row.try_get("last_entitlement_issued_at")?,
    })
}

fn decode_device_binding_row(row: sqlx::sqlite::SqliteRow) -> Result<StoredDeviceBinding> {
    Ok(StoredDeviceBinding {
        customer_id: row.try_get("customer_id")?,
        machine_id: row.try_get("machine_id")?,
        bound_at: row.try_get("bound_at")?,
        last_seen_at: row.try_get("last_seen_at")?,
    })
}

fn decode_audit_event_row(row: sqlx::sqlite::SqliteRow) -> Result<StoredAuditEvent> {
    let metadata_json: Option<String> = row.try_get("metadata_json")?;

    Ok(StoredAuditEvent {
        id: row.try_get("id")?,
        event_type: row.try_get("event_type")?,
        actor: row.try_get("actor")?,
        customer_id: row.try_get("customer_id")?,
        machine_id: row.try_get("machine_id")?,
        details: row.try_get("details")?,
        metadata: metadata_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .context("failed to decode audit metadata_json")?,
        created_at: row.try_get("created_at")?,
    })
}

fn decode_admin_user_row(row: sqlx::sqlite::SqliteRow) -> Result<StoredAdminUser> {
    Ok(StoredAdminUser {
        admin_id: row.try_get("admin_id")?,
        role: row.try_get("role")?,
        active: row.try_get::<i64, _>("active")? != 0,
        api_key_hash: row.try_get("api_key_hash")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        last_used_at: row.try_get("last_used_at")?,
    })
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn default_feature_ids() -> Vec<String> {
    vec![
        "ai_runtime".to_string(),
        "bug_bounty".to_string(),
        "plugin_catalog_access".to_string(),
        "plugin_catalog_write".to_string(),
        "plugin_catalog_delete".to_string(),
    ]
}

fn default_tier() -> String {
    "pro".to_string()
}

fn default_device_limit() -> usize {
    3
}

async fn add_column_if_missing(pool: &SqlitePool, sql: &str) -> Result<()> {
    match sqlx::query(sql).execute(pool).await {
        Ok(_) => Ok(()),
        Err(error) => {
            let message = error.to_string().to_ascii_lowercase();
            if message.contains("duplicate column name") {
                Ok(())
            } else {
                Err(error).context("failed to alter sqlite table")
            }
        }
    }
}
