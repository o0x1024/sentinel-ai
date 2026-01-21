//! Data models for Bug Bounty module

pub mod program;
pub mod scope;
pub mod finding;
pub mod submission;
pub mod evidence;
pub mod change_event;

pub use program::*;
pub use scope::*;
pub use finding::*;
pub use submission::*;
pub use evidence::*;
pub use change_event::*;
