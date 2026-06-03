//! btoa / atob (Base64 encoding/decoding)

use rquickjs::{Ctx, Function};
use crate::error::{JsError, Result};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    globals.set("btoa", Function::new(ctx.clone(), btoa_impl)
        .map_err(|e| JsError::Internal(e.to_string()))?)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    globals.set("atob", Function::new(ctx.clone(), atob_impl)
        .map_err(|e| JsError::Internal(e.to_string()))?)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    Ok(())
}

fn btoa_impl(input: String) -> rquickjs::Result<String> {
    use base64::Engine as _;
    Ok(base64::engine::general_purpose::STANDARD.encode(input.as_bytes()))
}

fn atob_impl(input: String) -> rquickjs::Result<String> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input.as_bytes())
        .map_err(|_| rquickjs::Error::new_from_js("string", "atob: invalid base64"))?;
    String::from_utf8(bytes)
        .map_err(|_| rquickjs::Error::new_from_js("string", "atob: invalid UTF-8"))
}
