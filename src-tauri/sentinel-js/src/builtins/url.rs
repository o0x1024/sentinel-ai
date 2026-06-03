//! URL and URLSearchParams implementation.
//! QuickJS-NG already has basic URL support, but we ensure full WHATWG compliance.

use rquickjs::Ctx;
use crate::error::{JsError, Result};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    // QuickJS-NG doesn't include URL/URLSearchParams natively.
    // Install a spec-compliant JS implementation.
    let url_code = include_str!("url_polyfill.js");
    ctx.eval::<(), _>(url_code)
        .map_err(|e| JsError::Internal(format!("URL polyfill install failed: {e}")))?;
    Ok(())
}
