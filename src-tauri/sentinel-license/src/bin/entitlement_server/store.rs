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
pub struct StoredLicenseCard {
    pub id: i64,
    pub batch_id: Option<i64>,
    pub username: Option<String>,
    pub activation_key: Option<String>,
    pub tier: String,
    pub feature_ids: Vec<String>,
    pub status: String,
    pub device_limit: usize,
    pub expires_at: Option<i64>,
    pub first_activated_at: Option<i64>,
    pub last_activated_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct StoredLicenseCardWithUsage {
    pub card: StoredLicenseCard,
    pub device_count: i64,
}

#[derive(Debug, Clone)]
pub struct NewLicenseCard {
    pub batch_id: Option<i64>,
    pub username: String,
    pub activation_key: String,
    pub card_key_hash: String,
    pub tier: String,
    pub feature_ids: Vec<String>,
    pub device_limit: usize,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct StoredLicenseDevice {
    pub id: i64,
    pub card_id: i64,
    pub username: String,
    pub machine_id: String,
    pub device_name: Option<String>,
    pub revoked: bool,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone)]
pub struct LicenseActivationRecord {
    pub card: StoredLicenseCard,
    pub device: StoredLicenseDevice,
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

    pub async fn license_card_count(&self) -> Result<i64> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM license_cards")
            .fetch_one(&self.pool)
            .await
            .context("failed to count license cards")
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

    pub async fn create_card_batch(&self, name: Option<&str>) -> Result<i64> {
        let now = current_unix_timestamp();
        let result = sqlx::query(
            r#"
            INSERT INTO license_card_batches (name, created_at)
            VALUES (?, ?)
            "#,
        )
        .bind(name.map(str::trim).filter(|value| !value.is_empty()))
        .bind(now)
        .execute(&self.pool)
        .await
        .context("failed to create license card batch")?;
        Ok(result.last_insert_rowid())
    }

    pub async fn insert_license_card(&self, card: NewLicenseCard) -> Result<StoredLicenseCard> {
        let now = current_unix_timestamp();
        let feature_ids_json = serde_json::to_string(&card.feature_ids)
            .context("failed to encode card feature_ids")?;
        let result = sqlx::query(
            r#"
            INSERT INTO license_cards (
                batch_id, card_key_hash, username, activation_key, tier, feature_ids_json, status,
                device_limit, expires_at, first_activated_at, last_activated_at,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, 'unused', ?, ?, NULL, NULL, ?, ?)
            "#,
        )
        .bind(card.batch_id)
        .bind(card.card_key_hash)
        .bind(card.username)
        .bind(card.activation_key)
        .bind(card.tier)
        .bind(feature_ids_json)
        .bind(card.device_limit as i64)
        .bind(card.expires_at)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("failed to insert license card")?;

        self.get_license_card_by_id(result.last_insert_rowid())
            .await?
            .ok_or_else(|| anyhow!("license card missing after insert"))
    }

    pub async fn list_license_cards(&self) -> Result<Vec<StoredLicenseCardWithUsage>> {
        let rows = sqlx::query(
            r#"
            SELECT c.id, c.batch_id, c.card_key_hash, c.username, c.activation_key, c.tier,
                   c.feature_ids_json, c.status, c.device_limit, c.expires_at,
                   c.first_activated_at, c.last_activated_at, c.created_at, c.updated_at,
                   COUNT(d.id) AS device_count
            FROM license_cards c
            LEFT JOIN license_devices d ON d.card_id = c.id AND d.revoked = 0
            GROUP BY c.id
            ORDER BY c.created_at DESC, c.id DESC
            LIMIT 500
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list license cards")?;

        rows.into_iter()
            .map(decode_license_card_usage_row)
            .collect()
    }

    pub async fn get_license_card_by_id(&self, id: i64) -> Result<Option<StoredLicenseCard>> {
        let row = sqlx::query(
            r#"
            SELECT id, batch_id, card_key_hash, username, activation_key, tier, feature_ids_json, status,
                   device_limit, expires_at, first_activated_at, last_activated_at,
                   created_at, updated_at
            FROM license_cards
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("failed to load license card {}", id))?;

        row.map(decode_license_card_row).transpose()
    }

    pub async fn get_license_card_by_key_hash(
        &self,
        card_key_hash: &str,
    ) -> Result<Option<StoredLicenseCard>> {
        let row = sqlx::query(
            r#"
            SELECT id, batch_id, card_key_hash, username, activation_key, tier, feature_ids_json, status,
                   device_limit, expires_at, first_activated_at, last_activated_at,
                   created_at, updated_at
            FROM license_cards
            WHERE card_key_hash = ?
            "#,
        )
        .bind(card_key_hash)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load license card by key hash")?;

        row.map(decode_license_card_row).transpose()
    }

    pub async fn activate_license_card(
        &self,
        card_id: i64,
        username: &str,
        machine_id_full: &str,
        device_name: Option<&str>,
        refresh_token_hash: &str,
    ) -> Result<LicenseActivationRecord> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start license card activation transaction")?;
        let now = current_unix_timestamp();
        let card = sqlx::query(
            r#"
            SELECT id, batch_id, card_key_hash, username, activation_key, tier, feature_ids_json, status,
                   device_limit, expires_at, first_activated_at, last_activated_at,
                   created_at, updated_at
            FROM license_cards
            WHERE id = ?
            "#,
        )
        .bind(card_id)
        .fetch_optional(&mut *tx)
        .await
        .context("failed to lock license card for activation")?
        .map(decode_license_card_row)
        .transpose()?
        .ok_or_else(|| anyhow!("license card does not exist"))?;

        let device_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM license_devices WHERE card_id = ? AND revoked = 0",
        )
        .bind(card_id)
        .fetch_one(&mut *tx)
        .await
        .context("failed to count license card devices")?;
        let existing_device = sqlx::query(
            r#"
            SELECT id, card_id, username, machine_id, device_name, refresh_token_hash,
                   revoked, first_seen_at, last_seen_at
            FROM license_devices
            WHERE card_id = ? AND machine_id = ?
            "#,
        )
        .bind(card_id)
        .bind(machine_id_full)
        .fetch_optional(&mut *tx)
        .await
        .context("failed to load existing license device")?
        .map(decode_license_device_row)
        .transpose()?;

        if existing_device.is_none() && device_count >= card.device_limit as i64 {
            return Err(anyhow!(
                "device limit exceeded for license card {} (limit {})",
                card_id,
                card.device_limit
            ));
        }

        let resolved_username = card
            .username
            .as_deref()
            .map(str::to_string)
            .unwrap_or_else(|| username.to_string());
        sqlx::query(
            r#"
            UPDATE license_cards
            SET username = ?, status = 'active',
                first_activated_at = COALESCE(first_activated_at, ?),
                last_activated_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&resolved_username)
        .bind(now)
        .bind(now)
        .bind(now)
        .bind(card_id)
        .execute(&mut *tx)
        .await
        .context("failed to update activated license card")?;

        let device = if let Some(existing_device) = existing_device {
            sqlx::query(
                r#"
                UPDATE license_devices
                SET username = ?, device_name = ?, refresh_token_hash = ?,
                    revoked = 0, last_seen_at = ?
                WHERE id = ?
                "#,
            )
            .bind(&resolved_username)
            .bind(device_name)
            .bind(refresh_token_hash)
            .bind(now)
            .bind(existing_device.id)
            .execute(&mut *tx)
            .await
            .context("failed to update license device")?;
            existing_device.id
        } else {
            sqlx::query(
                r#"
                INSERT INTO license_devices (
                    card_id, username, machine_id, device_name, refresh_token_hash,
                    revoked, first_seen_at, last_seen_at
                ) VALUES (?, ?, ?, ?, ?, 0, ?, ?)
                "#,
            )
            .bind(card_id)
            .bind(&resolved_username)
            .bind(machine_id_full)
            .bind(device_name)
            .bind(refresh_token_hash)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .context("failed to insert license device")?
            .last_insert_rowid()
        };

        tx.commit()
            .await
            .context("failed to commit license card activation")?;

        let card = self
            .get_license_card_by_id(card_id)
            .await?
            .ok_or_else(|| anyhow!("license card missing after activation"))?;
        let device = self
            .get_license_device_by_id(device)
            .await?
            .ok_or_else(|| anyhow!("license device missing after activation"))?;
        Ok(LicenseActivationRecord { card, device })
    }

    pub async fn get_license_device_by_id(&self, id: i64) -> Result<Option<StoredLicenseDevice>> {
        let row = sqlx::query(
            r#"
            SELECT id, card_id, username, machine_id, device_name, refresh_token_hash,
                   revoked, first_seen_at, last_seen_at
            FROM license_devices
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("failed to load license device {}", id))?;

        row.map(decode_license_device_row).transpose()
    }

    pub async fn get_license_device_by_refresh_hash(
        &self,
        refresh_token_hash: &str,
    ) -> Result<Option<StoredLicenseDevice>> {
        let row = sqlx::query(
            r#"
            SELECT id, card_id, username, machine_id, device_name, refresh_token_hash,
                   revoked, first_seen_at, last_seen_at
            FROM license_devices
            WHERE refresh_token_hash = ?
            "#,
        )
        .bind(refresh_token_hash)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load license device by refresh token hash")?;

        row.map(decode_license_device_row).transpose()
    }

    pub async fn touch_license_device(&self, device_id: i64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE license_devices
            SET last_seen_at = ?
            WHERE id = ?
            "#,
        )
        .bind(current_unix_timestamp())
        .bind(device_id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("failed to touch license device {}", device_id))?;
        Ok(())
    }

    pub async fn list_license_devices(&self, card_id: i64) -> Result<Vec<StoredLicenseDevice>> {
        let rows = sqlx::query(
            r#"
            SELECT id, card_id, username, machine_id, device_name, refresh_token_hash,
                   revoked, first_seen_at, last_seen_at
            FROM license_devices
            WHERE card_id = ?
            ORDER BY last_seen_at DESC, id DESC
            "#,
        )
        .bind(card_id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("failed to list license devices for card {}", card_id))?;

        rows.into_iter().map(decode_license_device_row).collect()
    }

    pub async fn delete_license_card(&self, card_id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM license_cards WHERE id = ?")
            .bind(card_id)
            .execute(&self.pool)
            .await
            .with_context(|| format!("failed to delete license card {}", card_id))?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_license_cards(&self, card_ids: &[i64]) -> Result<Vec<i64>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start license card bulk delete transaction")?;
        let mut deleted_ids = Vec::new();
        for card_id in card_ids {
            let result = sqlx::query("DELETE FROM license_cards WHERE id = ?")
                .bind(card_id)
                .execute(&mut *tx)
                .await
                .with_context(|| format!("failed to delete license card {}", card_id))?;
            if result.rows_affected() > 0 {
                deleted_ids.push(*card_id);
            }
        }
        tx.commit()
            .await
            .context("failed to commit license card bulk delete")?;
        Ok(deleted_ids)
    }

    pub async fn license_card_username_exists(&self, username: &str) -> Result<bool> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM license_cards WHERE username = ?")
                .bind(username)
                .fetch_one(&self.pool)
                .await
                .with_context(|| format!("failed to check license card username {}", username))?;
        Ok(count > 0)
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

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS license_card_batches (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NULL,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create license_card_batches table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS license_cards (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                batch_id INTEGER NULL,
                card_key_hash TEXT NOT NULL UNIQUE,
                username TEXT NULL,
                activation_key TEXT NULL,
                tier TEXT NOT NULL,
                feature_ids_json TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'unused',
                device_limit INTEGER NOT NULL DEFAULT 1,
                expires_at INTEGER NULL,
                first_activated_at INTEGER NULL,
                last_activated_at INTEGER NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (batch_id) REFERENCES license_card_batches(id) ON DELETE SET NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create license_cards table")?;
        add_column_if_missing(
            &self.pool,
            "ALTER TABLE license_cards ADD COLUMN activation_key TEXT NULL",
        )
        .await
        .context("failed to ensure license_cards activation_key column")?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_license_cards_status ON license_cards(status, created_at DESC)",
        )
        .execute(&self.pool)
        .await
        .context("failed to create license_cards status index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS license_devices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                card_id INTEGER NOT NULL,
                username TEXT NOT NULL,
                machine_id TEXT NOT NULL,
                device_name TEXT NULL,
                refresh_token_hash TEXT NOT NULL UNIQUE,
                revoked INTEGER NOT NULL DEFAULT 0,
                first_seen_at INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL,
                UNIQUE(card_id, machine_id),
                FOREIGN KEY (card_id) REFERENCES license_cards(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create license_devices table")?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_license_devices_card_id ON license_devices(card_id, last_seen_at DESC)",
        )
        .execute(&self.pool)
        .await
        .context("failed to create license_devices card index")?;

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

fn decode_license_card_row(row: sqlx::sqlite::SqliteRow) -> Result<StoredLicenseCard> {
    let feature_ids_json: String = row.try_get("feature_ids_json")?;
    Ok(StoredLicenseCard {
        id: row.try_get("id")?,
        batch_id: row.try_get("batch_id")?,
        username: row.try_get("username")?,
        activation_key: row.try_get("activation_key")?,
        tier: row.try_get("tier")?,
        feature_ids: serde_json::from_str(&feature_ids_json)
            .context("failed to decode license card feature_ids_json")?,
        status: row.try_get("status")?,
        device_limit: row.try_get::<i64, _>("device_limit")? as usize,
        expires_at: row.try_get("expires_at")?,
        first_activated_at: row.try_get("first_activated_at")?,
        last_activated_at: row.try_get("last_activated_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn decode_license_card_usage_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<StoredLicenseCardWithUsage> {
    let device_count = row.try_get("device_count")?;
    Ok(StoredLicenseCardWithUsage {
        card: decode_license_card_row(row)?,
        device_count,
    })
}

fn decode_license_device_row(row: sqlx::sqlite::SqliteRow) -> Result<StoredLicenseDevice> {
    Ok(StoredLicenseDevice {
        id: row.try_get("id")?,
        card_id: row.try_get("card_id")?,
        username: row.try_get("username")?,
        machine_id: row.try_get("machine_id")?,
        device_name: row.try_get("device_name")?,
        revoked: row.try_get::<i64, _>("revoked")? != 0,
        first_seen_at: row.try_get("first_seen_at")?,
        last_seen_at: row.try_get("last_seen_at")?,
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
