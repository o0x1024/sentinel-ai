//! performance.now() and performance.timeOrigin

use rquickjs::{Ctx, Function, Object};
use std::time::Instant;
use std::sync::OnceLock;

use crate::error::{JsError, Result};

static TIME_ORIGIN: OnceLock<Instant> = OnceLock::new();

fn get_origin() -> &'static Instant {
    TIME_ORIGIN.get_or_init(Instant::now)
}

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    // Initialize time origin
    let _ = get_origin();

    let globals = ctx.globals();
    let perf = Object::new(ctx.clone())
        .map_err(|e| JsError::Internal(e.to_string()))?;

    perf.set("now", Function::new(ctx.clone(), performance_now)
        .map_err(|e| JsError::Internal(e.to_string()))?)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    perf.set("timeOrigin", 0.0f64)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    globals.set("performance", perf)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    Ok(())
}

fn performance_now() -> f64 {
    get_origin().elapsed().as_secs_f64() * 1000.0
}
