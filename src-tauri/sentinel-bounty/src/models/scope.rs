//! Scope management for bug bounty programs

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Scope inclusion type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScopeType {
    /// In-scope targets
    #[default]
    InScope,
    /// Out-of-scope targets
    OutOfScope,
}

/// Target type for scope
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TargetType {
    /// Domain (e.g., example.com)
    Domain,
    /// Wildcard domain (e.g., *.example.com)
    WildcardDomain,
    /// Specific URL
    Url,
    /// IP address or range (CIDR)
    IpRange,
    /// Mobile app (iOS/Android)
    MobileApp,
    /// API endpoint
    ApiEndpoint,
    /// Source code repository
    SourceCode,
    /// Other target type
    Other,
}

/// Allowed test types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TestType {
    /// Web application testing
    WebApp,
    /// API testing
    Api,
    /// Network/infrastructure testing
    Network,
    /// Mobile application testing
    Mobile,
    /// Social engineering
    SocialEngineering,
    /// Physical security
    Physical,
    /// Denial of Service
    Dos,
    /// Authentication testing
    Authentication,
    /// Authorization testing
    Authorization,
}

/// Scope item within a program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramScope {
    /// Unique identifier
    pub id: String,
    /// Associated program ID
    pub program_id: String,
    /// Scope type (in/out)
    pub scope_type: ScopeType,
    /// Target type
    pub target_type: TargetType,
    /// Target value (domain, URL, IP, etc.)
    pub target: String,
    /// Regex pattern for matching (compiled from target)
    #[serde(skip)]
    pub pattern: Option<Regex>,
    /// Description or notes
    pub description: Option<String>,
    /// Allowed test types for this scope
    pub allowed_tests: Vec<TestType>,
    /// Special instructions or restrictions
    pub instructions: Vec<String>,
    /// Whether authentication is required
    pub requires_auth: bool,
    /// Test account info (if provided)
    pub test_accounts: Vec<TestAccount>,
    /// Asset count within this scope
    pub asset_count: i32,
    /// Finding count from this scope
    pub finding_count: i32,
    /// Priority score
    pub priority: f64,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl ProgramScope {
    pub fn new(program_id: String, target: String, target_type: TargetType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            program_id,
            scope_type: ScopeType::default(),
            target_type,
            target,
            pattern: None,
            description: None,
            allowed_tests: Vec::new(),
            instructions: Vec::new(),
            requires_auth: false,
            test_accounts: Vec::new(),
            asset_count: 0,
            finding_count: 0,
            priority: 0.0,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if a given target matches this scope
    pub fn matches(&self, target: &str) -> bool {
        // Simple matching logic - can be enhanced with regex patterns
        match self.target_type {
            TargetType::Domain => target == self.target || target.ends_with(&format!(".{}", self.target)),
            TargetType::WildcardDomain => {
                let base = self.target.trim_start_matches("*.");
                target.ends_with(base)
            }
            TargetType::Url => target.starts_with(&self.target),
            _ => target == self.target,
        }
    }

    /// Compile regex pattern from target
    pub fn compile_pattern(&mut self) -> Result<(), String> {
        let pattern = match self.target_type {
            TargetType::WildcardDomain => {
                let base = self.target.trim_start_matches("*.");
                format!(r"^([a-zA-Z0-9-]+\.)*{}$", regex::escape(base))
            }
            TargetType::Domain => {
                format!(r"^([a-zA-Z0-9-]+\.)*{}$", regex::escape(&self.target))
            }
            _ => regex::escape(&self.target),
        };

        match Regex::new(&pattern) {
            Ok(re) => {
                self.pattern = Some(re);
                Ok(())
            }
            Err(e) => Err(format!("Failed to compile pattern: {}", e)),
        }
    }
}

/// Test account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAccount {
    /// Account type (admin, user, etc.)
    pub account_type: String,
    /// Username
    pub username: String,
    /// Password (should be encrypted in production)
    pub password: String,
    /// Additional notes
    pub notes: Option<String>,
}

/// Create scope request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScopeRequest {
    pub program_id: String,
    pub scope_type: ScopeType,
    pub target_type: TargetType,
    pub target: String,
    pub description: Option<String>,
    pub allowed_tests: Option<Vec<TestType>>,
    pub instructions: Option<Vec<String>>,
    pub requires_auth: Option<bool>,
    pub test_accounts: Option<Vec<TestAccount>>,
    pub priority: Option<f64>,
}

/// Update scope request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScopeRequest {
    pub scope_type: Option<ScopeType>,
    pub target_type: Option<TargetType>,
    pub target: Option<String>,
    pub description: Option<String>,
    pub allowed_tests: Option<Vec<TestType>>,
    pub instructions: Option<Vec<String>>,
    pub requires_auth: Option<bool>,
    pub test_accounts: Option<Vec<TestAccount>>,
    pub priority: Option<f64>,
}

/// Scope filter for queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScopeFilter {
    pub program_ids: Option<Vec<String>>,
    pub scope_types: Option<Vec<ScopeType>>,
    pub target_types: Option<Vec<TargetType>>,
    pub requires_auth: Option<bool>,
    pub search: Option<String>,
}

/// Scope validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeValidation {
    /// Whether the target is in scope
    pub in_scope: bool,
    /// Matched scope item (if any)
    pub matched_scope: Option<ProgramScope>,
    /// Reason for out-of-scope (if applicable)
    pub reason: Option<String>,
    /// Warnings or notes
    pub warnings: Vec<String>,
}
