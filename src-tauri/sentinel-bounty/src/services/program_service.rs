//! Bug Bounty Program service

use crate::error::{BountyError, Result};
use crate::models::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

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
