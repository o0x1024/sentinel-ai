//! Bug Bounty Program service

use crate::error::{BountyError, Result};
use crate::models::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use sentinel_db::{BountyProgramRow, DatabaseService};
use chrono::Utc;
use uuid::Uuid;

#[async_trait]
pub trait ProgramServiceTrait: Send + Sync {
    async fn create_program(&self, request: CreateProgramRequest) -> Result<BountyProgram>;
    async fn get_program(&self, id: &str) -> Result<Option<BountyProgram>>;
    async fn update_program(&self, id: &str, request: UpdateProgramRequest) -> Result<bool>;
    async fn delete_program(&self, id: &str) -> Result<bool>;
    async fn list_programs(&self, filter: Option<ProgramFilter>) -> Result<Vec<BountyProgram>>;
    async fn get_program_stats(&self) -> Result<ProgramStats>;
    
    async fn create_scope(&self, request: CreateScopeRequest) -> Result<ProgramScope>;
    async fn get_scope(&self, id: &str) -> Result<Option<ProgramScope>>;
    async fn update_scope(&self, id: &str, request: UpdateScopeRequest) -> Result<bool>;
    async fn delete_scope(&self, id: &str) -> Result<bool>;
    async fn list_scopes(&self, filter: Option<ScopeFilter>) -> Result<Vec<ProgramScope>>;
    async fn validate_scope(&self, program_id: &str, target: &str) -> Result<ScopeValidation>;
}

pub struct ProgramService {
    // In-memory storage for now - will be replaced with database
    programs: Arc<RwLock<Vec<BountyProgram>>>,
    scopes: Arc<RwLock<Vec<ProgramScope>>>,
}

impl ProgramService {
    pub fn new() -> Self {
        Self {
            programs: Arc::new(RwLock::new(Vec::new())),
            scopes: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for ProgramService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProgramServiceTrait for ProgramService {
    async fn create_program(&self, request: CreateProgramRequest) -> Result<BountyProgram> {
        let mut program = BountyProgram::new(request.name, request.organization);
        
        if let Some(platform) = request.platform {
            program.platform = platform;
        }
        program.platform_handle = request.platform_handle;
        program.url = request.url;
        if let Some(program_type) = request.program_type {
            program.program_type = program_type;
        }
        program.description = request.description;
        if let Some(rewards) = request.rewards {
            program.rewards = rewards;
        }
        if let Some(rules) = request.rules {
            program.rules = rules;
        }
        if let Some(tags) = request.tags {
            program.tags = tags;
        }

        let mut programs = self.programs.write().await;
        programs.push(program.clone());
        
        Ok(program)
    }

    async fn get_program(&self, id: &str) -> Result<Option<BountyProgram>> {
        let programs = self.programs.read().await;
        Ok(programs.iter().find(|p| p.id == id).cloned())
    }

    async fn update_program(&self, id: &str, request: UpdateProgramRequest) -> Result<bool> {
        let mut programs = self.programs.write().await;
        
        if let Some(program) = programs.iter_mut().find(|p| p.id == id) {
            if let Some(name) = request.name {
                program.name = name;
            }
            if let Some(organization) = request.organization {
                program.organization = organization;
            }
            if let Some(platform) = request.platform {
                program.platform = platform;
            }
            program.platform_handle = request.platform_handle.or(program.platform_handle.clone());
            program.url = request.url.or(program.url.clone());
            if let Some(program_type) = request.program_type {
                program.program_type = program_type;
            }
            if let Some(status) = request.status {
                program.status = status;
            }
            program.description = request.description.or(program.description.clone());
            if let Some(rewards) = request.rewards {
                program.rewards = rewards;
            }
            program.response_sla_days = request.response_sla_days.or(program.response_sla_days);
            program.resolution_sla_days = request.resolution_sla_days.or(program.resolution_sla_days);
            if let Some(rules) = request.rules {
                program.rules = rules;
            }
            if let Some(tags) = request.tags {
                program.tags = tags;
            }
            if let Some(priority_score) = request.priority_score {
                program.priority_score = priority_score;
            }
            
            program.updated_at = chrono::Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn delete_program(&self, id: &str) -> Result<bool> {
        let mut programs = self.programs.write().await;
        let initial_len = programs.len();
        programs.retain(|p| p.id != id);
        Ok(programs.len() < initial_len)
    }

    async fn list_programs(&self, filter: Option<ProgramFilter>) -> Result<Vec<BountyProgram>> {
        let programs = self.programs.read().await;
        let mut result: Vec<BountyProgram> = programs.clone();

        if let Some(f) = filter {
            if let Some(platforms) = f.platforms {
                result.retain(|p| platforms.contains(&p.platform));
            }
            if let Some(statuses) = f.statuses {
                result.retain(|p| statuses.contains(&p.status));
            }
            if let Some(program_types) = f.program_types {
                result.retain(|p| program_types.contains(&p.program_type));
            }
            if let Some(tags) = f.tags {
                result.retain(|p| p.tags.iter().any(|t| tags.contains(t)));
            }
            if let Some(search) = f.search {
                let search_lower = search.to_lowercase();
                result.retain(|p| {
                    p.name.to_lowercase().contains(&search_lower)
                        || p.organization.to_lowercase().contains(&search_lower)
                });
            }
            if let Some(min_priority) = f.min_priority {
                result.retain(|p| p.priority_score >= min_priority);
            }
        }

        Ok(result)
    }

    async fn get_program_stats(&self) -> Result<ProgramStats> {
        let programs = self.programs.read().await;
        
        let total_programs = programs.len() as i32;
        let active_programs = programs.iter().filter(|p| p.status == ProgramStatus::Active).count() as i32;
        
        let mut by_platform = std::collections::HashMap::new();
        let mut by_type = std::collections::HashMap::new();
        let mut total_submissions = 0;
        let mut total_accepted = 0;
        let mut total_earnings = 0.0;

        for program in programs.iter() {
            *by_platform.entry(format!("{:?}", program.platform)).or_insert(0) += 1;
            *by_type.entry(format!("{:?}", program.program_type)).or_insert(0) += 1;
            total_submissions += program.total_submissions;
            total_accepted += program.accepted_submissions;
            total_earnings += program.total_earnings;
        }

        Ok(ProgramStats {
            total_programs,
            active_programs,
            by_platform,
            by_type,
            total_submissions,
            total_accepted,
            total_earnings,
        })
    }

    async fn create_scope(&self, request: CreateScopeRequest) -> Result<ProgramScope> {
        let mut scope = ProgramScope::new(
            request.program_id,
            request.target,
            request.target_type,
        );
        
        scope.scope_type = request.scope_type;
        scope.description = request.description;
        if let Some(allowed_tests) = request.allowed_tests {
            scope.allowed_tests = allowed_tests;
        }
        if let Some(instructions) = request.instructions {
            scope.instructions = instructions;
        }
        if let Some(requires_auth) = request.requires_auth {
            scope.requires_auth = requires_auth;
        }
        if let Some(test_accounts) = request.test_accounts {
            scope.test_accounts = test_accounts;
        }
        if let Some(priority) = request.priority {
            scope.priority = priority;
        }

        // Compile pattern
        scope.compile_pattern().map_err(|e| BountyError::InvalidScopePattern(e))?;

        let mut scopes = self.scopes.write().await;
        scopes.push(scope.clone());
        
        Ok(scope)
    }

    async fn get_scope(&self, id: &str) -> Result<Option<ProgramScope>> {
        let scopes = self.scopes.read().await;
        Ok(scopes.iter().find(|s| s.id == id).cloned())
    }

    async fn update_scope(&self, id: &str, request: UpdateScopeRequest) -> Result<bool> {
        let mut scopes = self.scopes.write().await;
        
        if let Some(scope) = scopes.iter_mut().find(|s| s.id == id) {
            if let Some(scope_type) = request.scope_type {
                scope.scope_type = scope_type;
            }
            if let Some(target_type) = request.target_type {
                scope.target_type = target_type;
            }
            if let Some(target) = request.target {
                scope.target = target;
                scope.compile_pattern().map_err(|e| BountyError::InvalidScopePattern(e))?;
            }
            scope.description = request.description.or(scope.description.clone());
            if let Some(allowed_tests) = request.allowed_tests {
                scope.allowed_tests = allowed_tests;
            }
            if let Some(instructions) = request.instructions {
                scope.instructions = instructions;
            }
            if let Some(requires_auth) = request.requires_auth {
                scope.requires_auth = requires_auth;
            }
            if let Some(test_accounts) = request.test_accounts {
                scope.test_accounts = test_accounts;
            }
            if let Some(priority) = request.priority {
                scope.priority = priority;
            }
            
            scope.updated_at = chrono::Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn delete_scope(&self, id: &str) -> Result<bool> {
        let mut scopes = self.scopes.write().await;
        let initial_len = scopes.len();
        scopes.retain(|s| s.id != id);
        Ok(scopes.len() < initial_len)
    }

    async fn list_scopes(&self, filter: Option<ScopeFilter>) -> Result<Vec<ProgramScope>> {
        let scopes = self.scopes.read().await;
        let mut result: Vec<ProgramScope> = scopes.clone();

        if let Some(f) = filter {
            if let Some(program_ids) = f.program_ids {
                result.retain(|s| program_ids.contains(&s.program_id));
            }
            if let Some(scope_types) = f.scope_types {
                result.retain(|s| scope_types.contains(&s.scope_type));
            }
            if let Some(target_types) = f.target_types {
                result.retain(|s| target_types.contains(&s.target_type));
            }
            if let Some(requires_auth) = f.requires_auth {
                result.retain(|s| s.requires_auth == requires_auth);
            }
            if let Some(search) = f.search {
                let search_lower = search.to_lowercase();
                result.retain(|s| {
                    s.target.to_lowercase().contains(&search_lower)
                        || s.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&search_lower))
                });
            }
        }

        Ok(result)
    }

    async fn validate_scope(&self, program_id: &str, target: &str) -> Result<ScopeValidation> {
        let scopes = self.scopes.read().await;
        
        // Check in-scope items
        let in_scope_items: Vec<&ProgramScope> = scopes
            .iter()
            .filter(|s| s.program_id == program_id && s.scope_type == ScopeType::InScope)
            .collect();
        
        let mut matched_in_scope: Option<&ProgramScope> = None;
        for scope in &in_scope_items {
            if scope.matches(target) {
                matched_in_scope = Some(scope);
                break;
            }
        }

        // Check out-of-scope items
        let out_of_scope_items: Vec<&ProgramScope> = scopes
            .iter()
            .filter(|s| s.program_id == program_id && s.scope_type == ScopeType::OutOfScope)
            .collect();
        
        for scope in &out_of_scope_items {
            if scope.matches(target) {
                return Ok(ScopeValidation {
                    in_scope: false,
                    matched_scope: None,
                    reason: Some(format!("Target matches out-of-scope rule: {}", scope.target)),
                    warnings: vec![],
                });
            }
        }

        // Return result
        if let Some(matched) = matched_in_scope {
            Ok(ScopeValidation {
                in_scope: true,
                matched_scope: Some(matched.clone()),
                reason: None,
                warnings: vec![],
            })
        } else {
            Ok(ScopeValidation {
                in_scope: false,
                matched_scope: None,
                reason: Some("Target does not match any in-scope rule".to_string()),
                warnings: vec![],
            })
        }
    }
}

// ============================================================================
// Database-backed service (used by commands)
// ============================================================================

#[derive(Debug, Clone)]
pub struct CreateProgramInput {
    pub name: String,
    pub organization: String,
    pub platform: Option<String>,
    pub platform_handle: Option<String>,
    pub url: Option<String>,
    pub program_type: Option<String>,
    pub description: Option<String>,
    pub rewards: Option<serde_json::Value>,
    pub rules: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct UpdateProgramInput {
    pub name: Option<String>,
    pub organization: Option<String>,
    pub platform: Option<String>,
    pub platform_handle: Option<String>,
    pub url: Option<String>,
    pub program_type: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub rewards: Option<serde_json::Value>,
    pub response_sla_days: Option<i32>,
    pub resolution_sla_days: Option<i32>,
    pub rules: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub priority_score: Option<f64>,
}

pub struct ProgramDbService;

impl ProgramDbService {
    pub async fn create_program(
        db: &DatabaseService,
        input: CreateProgramInput,
    ) -> Result<BountyProgramRow, String> {
        validate_required(&input.name, "name")?;
        validate_required(&input.organization, "organization")?;

        let now = Utc::now();
        let program = BountyProgramRow {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            organization: input.organization,
            platform: input.platform.unwrap_or_else(|| "private".to_string()),
            platform_handle: input.platform_handle,
            url: input.url,
            program_type: input.program_type.unwrap_or_else(|| "public".to_string()),
            status: "active".to_string(),
            description: input.description,
            rewards_json: input.rewards.map(|r| serde_json::to_string(&r).unwrap_or_default()),
            response_sla_days: None,
            resolution_sla_days: None,
            rules_json: input.rules.map(|r| serde_json::to_string(&r).unwrap_or_default()),
            tags_json: input.tags.map(|t| serde_json::to_string(&t).unwrap_or_default()),
            metadata_json: None,
            priority_score: 0.0,
            total_submissions: 0,
            accepted_submissions: 0,
            total_earnings: 0.0,
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
            last_activity_at: None,
        };

        db.create_bounty_program(&program)
            .await
            .map_err(|e| e.to_string())?;
        Ok(program)
    }

    pub async fn update_program(
        db: &DatabaseService,
        id: &str,
        input: UpdateProgramInput,
    ) -> Result<bool, String> {
        let existing = db
            .get_bounty_program(id)
            .await
            .map_err(|e| e.to_string())?;

        let Some(mut program) = existing else {
            return Ok(false);
        };

        if let Some(name) = input.name {
            validate_required(&name, "name")?;
            program.name = name;
        }
        if let Some(organization) = input.organization {
            validate_required(&organization, "organization")?;
            program.organization = organization;
        }
        if let Some(platform) = input.platform {
            program.platform = platform;
        }
        if input.platform_handle.is_some() {
            program.platform_handle = input.platform_handle;
        }
        if input.url.is_some() {
            program.url = input.url;
        }
        if let Some(program_type) = input.program_type {
            program.program_type = program_type;
        }
        if let Some(status) = input.status {
            program.status = status;
        }
        if input.description.is_some() {
            program.description = input.description;
        }
        if let Some(rewards) = input.rewards {
            program.rewards_json = Some(serde_json::to_string(&rewards).unwrap_or_default());
        }
        if input.response_sla_days.is_some() {
            program.response_sla_days = input.response_sla_days;
        }
        if input.resolution_sla_days.is_some() {
            program.resolution_sla_days = input.resolution_sla_days;
        }
        if let Some(rules) = input.rules {
            program.rules_json = Some(serde_json::to_string(&rules).unwrap_or_default());
        }
        if let Some(tags) = input.tags {
            program.tags_json = Some(serde_json::to_string(&tags).unwrap_or_default());
        }
        if let Some(priority_score) = input.priority_score {
            program.priority_score = priority_score;
        }

        program.updated_at = Utc::now().to_rfc3339();

        db.update_bounty_program(&program)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn delete_program(db: &DatabaseService, id: &str) -> Result<bool, String> {
        db.delete_bounty_program(id)
            .await
            .map_err(|e| e.to_string())
    }
}

fn validate_required(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{} is required", field));
    }
    Ok(())
}
