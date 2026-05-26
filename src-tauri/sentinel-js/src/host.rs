//! Host function registration infrastructure.
//!
//! Uses JSON string transport to bridge between JS and Rust host functions,
//! avoiding rquickjs lifetime issues with closures that capture state.

use rquickjs::{Ctx, Function};
use std::sync::Arc;

use crate::error::{JsError, Result};

/// A callback that takes a JSON string argument and returns a JSON string.
pub type HostFn = Arc<dyn Fn(&str) -> String + Send + Sync + 'static>;

/// A two-argument callback (json_str1, json_str2) -> json_str_result.
pub type HostFn2 = Arc<dyn Fn(&str, &str) -> String + Send + Sync + 'static>;

/// Builder for registering host functions into the runtime.
pub struct HostBindingsExt {
    pub(crate) bindings: Vec<HostBinding>,
}

pub(crate) enum HostBinding {
    SingleArg { name: String, handler: HostFn },
    TwoArg { name: String, handler: HostFn2 },
    NoArg { name: String, handler: Arc<dyn Fn() -> String + Send + Sync + 'static> },
    Void { name: String, handler: Arc<dyn Fn(&str) + Send + Sync + 'static> },
}

impl HostBindingsExt {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// Register fn(json_str) -> json_str
    pub fn register_fn1(
        &mut self,
        name: impl Into<String>,
        handler: impl Fn(&str) -> String + Send + Sync + 'static,
    ) -> &mut Self {
        self.bindings.push(HostBinding::SingleArg {
            name: name.into(),
            handler: Arc::new(handler),
        });
        self
    }

    /// Register fn(json_str, json_str) -> json_str (e.g. fetch(url, opts))
    pub fn register_fn2(
        &mut self,
        name: impl Into<String>,
        handler: impl Fn(&str, &str) -> String + Send + Sync + 'static,
    ) -> &mut Self {
        self.bindings.push(HostBinding::TwoArg {
            name: name.into(),
            handler: Arc::new(handler),
        });
        self
    }

    /// Register fn() -> json_str
    pub fn register_fn0(
        &mut self,
        name: impl Into<String>,
        handler: impl Fn() -> String + Send + Sync + 'static,
    ) -> &mut Self {
        self.bindings.push(HostBinding::NoArg {
            name: name.into(),
            handler: Arc::new(handler),
        });
        self
    }

    /// Register fn(json_str) -> void
    pub fn register_void(
        &mut self,
        name: impl Into<String>,
        handler: impl Fn(&str) + Send + Sync + 'static,
    ) -> &mut Self {
        self.bindings.push(HostBinding::Void {
            name: name.into(),
            handler: Arc::new(handler),
        });
        self
    }

    /// Install all registered functions into the JS context.
    ///
    /// Each host function uses JSON string serialization as the transport layer:
    /// - JS side: JSON.stringify(args) before calling Rust
    /// - Rust side: receives/returns plain strings
    /// - JS side: JSON.parse(result) after Rust returns
    pub(crate) fn install(&self, ctx: &Ctx<'_>) -> Result<()> {
        let globals = ctx.globals();

        for binding in &self.bindings {
            match binding {
                HostBinding::SingleArg { name, handler } => {
                    let handler = handler.clone();
                    // Rust closure accepts String, returns String
                    let func = Function::new(ctx.clone(), move |_ctx: Ctx<'_>, arg: String| -> rquickjs::Result<String> {
                        Ok(handler(&arg))
                    }).map_err(|e| JsError::Internal(format!("register {name}: {e}")))?;

                    let raw_name = format!("__raw_{name}");
                    globals.set(raw_name.as_str(), func)
                        .map_err(|e| JsError::Internal(format!("set {name}: {e}")))?;

                    // JS wrapper: stringify input, parse output
                    let js_wrapper = format!(
                        "globalThis[\"{name}\"] = function() {{ \
                            var arg = (arguments.length > 0) ? JSON.stringify(arguments[0]) : 'null'; \
                            var result = globalThis[\"__raw_{name}\"](arg); \
                            try {{ return JSON.parse(result); }} catch(e) {{ return result; }} \
                        }}"
                    );
                    ctx.eval::<(), _>(js_wrapper.as_str())
                        .map_err(|e| JsError::Internal(format!("wrapper {name}: {e}")))?;
                }
                HostBinding::TwoArg { name, handler } => {
                    let handler = handler.clone();
                    let func = Function::new(ctx.clone(), move |_ctx: Ctx<'_>, arg1: String, arg2: String| -> rquickjs::Result<String> {
                        Ok(handler(&arg1, &arg2))
                    }).map_err(|e| JsError::Internal(format!("register {name}: {e}")))?;

                    let raw_name = format!("__raw_{name}");
                    globals.set(raw_name.as_str(), func)
                        .map_err(|e| JsError::Internal(format!("set {name}: {e}")))?;

                    let js_wrapper = format!(
                        "globalThis[\"{name}\"] = function() {{ \
                            var a1 = (arguments.length > 0) ? JSON.stringify(arguments[0]) : 'null'; \
                            var a2 = (arguments.length > 1) ? JSON.stringify(arguments[1]) : 'null'; \
                            var result = globalThis[\"__raw_{name}\"](a1, a2); \
                            try {{ return JSON.parse(result); }} catch(e) {{ return result; }} \
                        }}"
                    );
                    ctx.eval::<(), _>(js_wrapper.as_str())
                        .map_err(|e| JsError::Internal(format!("wrapper {name}: {e}")))?;
                }
                HostBinding::NoArg { name, handler } => {
                    let handler = handler.clone();
                    let func = Function::new(ctx.clone(), move |_ctx: Ctx<'_>| -> rquickjs::Result<String> {
                        Ok(handler())
                    }).map_err(|e| JsError::Internal(format!("register {name}: {e}")))?;

                    let raw_name = format!("__raw_{name}");
                    globals.set(raw_name.as_str(), func)
                        .map_err(|e| JsError::Internal(format!("set {name}: {e}")))?;

                    let js_wrapper = format!(
                        "globalThis[\"{name}\"] = function() {{ \
                            var result = globalThis[\"__raw_{name}\"](); \
                            try {{ return JSON.parse(result); }} catch(e) {{ return result; }} \
                        }}"
                    );
                    ctx.eval::<(), _>(js_wrapper.as_str())
                        .map_err(|e| JsError::Internal(format!("wrapper {name}: {e}")))?;
                }
                HostBinding::Void { name, handler } => {
                    let handler = handler.clone();
                    let func = Function::new(ctx.clone(), move |_ctx: Ctx<'_>, arg: String| -> rquickjs::Result<()> {
                        handler(&arg);
                        Ok(())
                    }).map_err(|e| JsError::Internal(format!("register {name}: {e}")))?;

                    // For void, wrap to stringify input but don't parse output
                    let raw_name = format!("__raw_{name}");
                    globals.set(raw_name.as_str(), func)
                        .map_err(|e| JsError::Internal(format!("set {name}: {e}")))?;

                    let js_wrapper = format!(
                        "globalThis[\"{name}\"] = function() {{ \
                            var arg = (arguments.length > 0) ? JSON.stringify(arguments[0]) : 'null'; \
                            globalThis[\"__raw_{name}\"](arg); \
                        }}"
                    );
                    ctx.eval::<(), _>(js_wrapper.as_str())
                        .map_err(|e| JsError::Internal(format!("wrapper {name}: {e}")))?;
                }
            }
        }

        Ok(())
    }
}

impl Default for HostBindingsExt {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for backward compatibility.
pub type HostBindings = HostBindingsExt;
