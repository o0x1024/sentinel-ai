//! fetch() implementation with standard Response/Headers/Request classes.
//!
//! The actual HTTP request is delegated to a host-provided FetchHandler.
//! In the plugin runtime context, this is typically the Rust-side reqwest client
//! integrated with the request scheduler.

use rquickjs::Ctx;
use crate::error::{JsError, Result};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    // Install Headers, Request, Response classes and fetch function.
    // fetch delegates to __host_fetch which is registered by the plugin engine.
    let fetch_code = include_str!("fetch_polyfill.js");
    ctx.eval::<(), _>(fetch_code)
        .map_err(|e| JsError::Internal(format!("fetch polyfill install failed: {e}")))?;
    Ok(())
}
