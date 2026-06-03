//! Node.js compatibility layer for the plugin runtime.
//!
//! Provides: require(), Buffer, process, path, crypto (Node-style)

pub mod buffer;
pub mod crypto;
pub mod path;
pub mod process;
pub mod require;

use rquickjs::Ctx;
use crate::error::Result;

/// Install all Node.js compatibility APIs.
pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    require::install(ctx)?;
    buffer::install(ctx)?;
    process::install(ctx)?;
    Ok(())
}
