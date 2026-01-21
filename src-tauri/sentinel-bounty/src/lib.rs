//! Sentinel Bounty - Bug Bounty and SRC vulnerability hunting module
//!
//! Core capabilities:
//! - Program management (bounty programs, scopes, rules)
//! - Finding tracking (discoveries, evidence, deduplication)
//! - Submission management (platform submissions, status tracking)
//! - Asset Surface Management (ASM) integration
//! - Change monitoring and workflow triggers

pub mod models;
pub mod services;
pub mod error;

pub use error::{BountyError, Result};
pub use models::*;
pub use services::*;
