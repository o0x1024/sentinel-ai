//! console.log/warn/error/debug implementation.

use rquickjs::{Ctx, Function, Object, Value, function::Rest};
use crate::error::{JsError, Result};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let console = Object::new(ctx.clone())
        .map_err(|e| JsError::Internal(e.to_string()))?;

    console.set("log", Function::new(ctx.clone(), log_info)
        .map_err(|e| JsError::Internal(e.to_string()))?)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    console.set("info", Function::new(ctx.clone(), log_info)
        .map_err(|e| JsError::Internal(e.to_string()))?)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    console.set("warn", Function::new(ctx.clone(), log_warn)
        .map_err(|e| JsError::Internal(e.to_string()))?)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    console.set("error", Function::new(ctx.clone(), log_error)
        .map_err(|e| JsError::Internal(e.to_string()))?)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    console.set("debug", Function::new(ctx.clone(), log_debug)
        .map_err(|e| JsError::Internal(e.to_string()))?)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    globals.set("console", console)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    Ok(())
}

fn format_args(args: Rest<Value<'_>>) -> String {
    args.0
        .iter()
        .map(|v| {
            if let Some(s) = v.as_string() {
                s.to_string().unwrap_or_default()
            } else if v.is_null() {
                "null".to_string()
            } else if v.is_undefined() {
                "undefined".to_string()
            } else {
                format!("{:?}", v)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn log_info(args: Rest<Value<'_>>) {
    let msg = format_args(args);
    tracing::info!(target: "plugin", "{}", msg);
}

fn log_warn(args: Rest<Value<'_>>) {
    let msg = format_args(args);
    tracing::warn!(target: "plugin", "{}", msg);
}

fn log_error(args: Rest<Value<'_>>) {
    let msg = format_args(args);
    tracing::error!(target: "plugin", "{}", msg);
}

fn log_debug(args: Rest<Value<'_>>) {
    let msg = format_args(args);
    tracing::debug!(target: "plugin", "{}", msg);
}
