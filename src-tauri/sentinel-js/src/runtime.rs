//! Plugin runtime — the primary interface for executing JavaScript plugins.

use rquickjs::{Context, Function, Module, Runtime, Value};
use std::sync::Arc;

use crate::error::{JsError, Result};
use crate::sandbox::SandboxConfig;

/// Configuration for the plugin runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum memory in bytes (default: 64MB).
    pub max_memory: usize,
    /// Maximum execution time in milliseconds (default: 30000).
    pub max_execution_ms: u64,
    /// Maximum call stack depth (default: 128).
    pub max_stack_depth: usize,
    /// Enable Node.js compatibility APIs.
    pub node_compat: bool,
    /// Enable Web APIs (fetch, URL, crypto.subtle, etc.).
    pub web_apis: bool,
    /// Sandbox configuration.
    pub sandbox: SandboxConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024,
            max_execution_ms: 30_000,
            max_stack_depth: 128,
            node_compat: true,
            web_apis: true,
            sandbox: SandboxConfig::default(),
        }
    }
}

/// Callback type for host functions that handle fetch requests.
pub type FetchHandler = Arc<
    dyn Fn(FetchRequest) -> std::result::Result<FetchResponse, String> + Send + Sync + 'static,
>;

/// A fetch request from plugin code.
#[derive(Debug, Clone)]
pub struct FetchRequest {
    pub url: String,
    pub method: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub redirect: String,
    pub max_redirects: Option<u32>,
}

/// A fetch response returned to plugin code.
#[derive(Debug, Clone)]
pub struct FetchResponse {
    pub status: u16,
    pub ok: bool,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub url: String,
    pub redirected: bool,
}

/// Log level for plugin console output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Callback for console.log/warn/error output.
pub type LogHandler = Arc<dyn Fn(LogLevel, &str) + Send + Sync + 'static>;

/// Callback for Sentinel.resolve() — plugin returning its result.
pub type ResolveHandler = Arc<dyn Fn(serde_json::Value) + Send + Sync + 'static>;

/// The plugin runtime — manages a QuickJS-NG context for plugin execution.
pub struct PluginRuntime {
    runtime: Runtime,
    context: Context,
    config: RuntimeConfig,
    fetch_handler: Option<FetchHandler>,
    log_handler: Option<LogHandler>,
}

impl PluginRuntime {
    /// Create a new plugin runtime with the given configuration.
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        let runtime = Runtime::new().map_err(|e| JsError::Internal(e.to_string()))?;

        runtime.set_memory_limit(config.max_memory);
        runtime.set_max_stack_size(config.max_stack_depth * 1024);

        let context = Context::full(&runtime).map_err(|e| JsError::Internal(e.to_string()))?;

        let mut rt = Self {
            runtime,
            context,
            config,
            fetch_handler: None,
            log_handler: None,
        };

        rt.install_builtins()?;

        Ok(rt)
    }

    /// Create a new runtime with host bindings pre-registered.
    pub fn with_host_bindings(
        config: RuntimeConfig,
        bindings: &crate::host::HostBindingsExt,
    ) -> Result<Self> {
        let runtime = Runtime::new().map_err(|e| JsError::Internal(e.to_string()))?;

        runtime.set_memory_limit(config.max_memory);
        runtime.set_max_stack_size(config.max_stack_depth * 1024);

        let context = Context::full(&runtime).map_err(|e| JsError::Internal(e.to_string()))?;

        let mut rt = Self {
            runtime,
            context,
            config,
            fetch_handler: None,
            log_handler: None,
        };

        // Install host bindings before builtins (fetch polyfill references __host_fetch)
        rt.context.with(|ctx| {
            bindings.install(&ctx)
        })?;

        rt.install_builtins()?;

        Ok(rt)
    }

    /// Set the fetch handler for network requests from plugin code.
    pub fn set_fetch_handler(&mut self, handler: FetchHandler) {
        self.fetch_handler = Some(handler);
    }

    /// Set the log handler for console output.
    pub fn set_log_handler(&mut self, handler: LogHandler) {
        self.log_handler = Some(handler);
    }

    /// Install all builtin APIs based on config.
    fn install_builtins(&mut self) -> Result<()> {
        self.context.with(|ctx| {
            crate::builtins::console::install(&ctx)?;

            if self.config.web_apis {
                crate::builtins::timers::install(&ctx)?;
                crate::builtins::encoding::install(&ctx)?;
                crate::builtins::url::install(&ctx)?;
                crate::builtins::fetch::install(&ctx)?;
                crate::builtins::crypto::install(&ctx)?;
                crate::builtins::performance::install(&ctx)?;
                crate::builtins::base64::install(&ctx)?;
            }

            #[cfg(feature = "node-compat")]
            if self.config.node_compat {
                crate::node_compat::install(&ctx)?;
            }

            // Security: disable eval() and Function constructor if sandbox config says so
            if !self.config.sandbox.allow_eval {
                ctx.eval::<(), _>(r#"
                    Object.defineProperty(globalThis, 'eval', {
                        value: function() { throw new Error("eval() is disabled in plugin sandbox"); },
                        writable: false, configurable: false
                    });
                "#).map_err(|e| JsError::Internal(format!("sandbox eval disable: {e}")))?;
            }

            Ok::<(), JsError>(())
        })?;

        Ok(())
    }

    /// Drain all pending microtasks/promises. Must be called OUTSIDE context.with().
    fn drain_jobs(&self) {
        loop {
            match self.runtime.execute_pending_job() {
                Ok(true) => continue,
                _ => break,
            }
        }
    }

    /// Load and evaluate plugin source code (TypeScript should be pre-stripped).
    pub fn load_plugin(&self, code: &str, is_module: bool) -> Result<()> {
        self.context.with(|ctx| {
            let result = if is_module {
                Module::evaluate(ctx.clone(), "plugin", code)
                    .map(|_| ())
                    .map_err(|e| e)
            } else {
                ctx.eval::<(), _>(code)
            };

            if let Err(e) = result {
                let err_msg = if let rquickjs::Error::Exception = &e {
                    let caught = ctx.catch();
                    if let Some(exc) = caught.as_exception() {
                        format!(
                            "{}: {}",
                            exc.message().unwrap_or_default(),
                            exc.stack().unwrap_or_default()
                        )
                    } else {
                        format!("{caught:?}")
                    }
                } else {
                    e.to_string()
                };

                return Err(if is_module {
                    JsError::ModuleLoad(err_msg)
                } else {
                    JsError::Execution(err_msg)
                });
            }

            Ok::<(), JsError>(())
        })?;
        self.drain_jobs();
        Ok(())
    }

    /// Call a named function in the global scope with a JSON input argument.
    pub fn call_function(
        &self,
        fn_name: &str,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.context.with(|ctx| {
            let globals = ctx.globals();

            let func: Function = globals
                .get(fn_name)
                .map_err(|_| JsError::Execution(format!("Function '{}' not found", fn_name)))?;

            let input_val = crate::convert::json_to_js(&ctx, input)?;

            let result: Value = func
                .call((input_val,))
                .map_err(|e| JsError::Execution(e.to_string()))?;

            crate::convert::js_to_json(&ctx, &result)
        })
        .map(|result| {
            self.drain_jobs();
            result
        })
    }

    /// Set a global JSON value accessible to plugin code.
    pub fn set_global(&self, name: &str, value: &serde_json::Value) -> Result<()> {
        self.context.with(|ctx| {
            let js_val = crate::convert::json_to_js(&ctx, value)?;
            ctx.globals()
                .set(name, js_val)
                .map_err(|e| JsError::Execution(e.to_string()))?;
            Ok::<(), JsError>(())
        })?;
        Ok(())
    }

    /// Evaluate raw JavaScript code and return the result as JSON.
    pub fn eval(&self, code: &str) -> Result<serde_json::Value> {
        let result = self.context.with(|ctx| {
            let val: Value = ctx
                .eval(code)
                .map_err(|e| JsError::Execution(e.to_string()))?;
            crate::convert::js_to_json(&ctx, &val)
        })?;
        self.drain_jobs();
        Ok(result)
    }

    /// Evaluate JavaScript code without expecting a return value.
    pub fn eval_void(&self, code: &str) -> Result<()> {
        self.context.with(|ctx| {
            ctx.eval::<(), _>(code)
                .map_err(|e| {
                    if let rquickjs::Error::Exception = e {
                        let caught = ctx.catch();
                        if let Some(exc) = caught.as_exception() {
                            return JsError::Execution(format!(
                                "{}: {}",
                                exc.message().unwrap_or_default(),
                                exc.stack().unwrap_or_default()
                            ));
                        }
                    }
                    JsError::Execution(e.to_string())
                })?;
            Ok::<(), JsError>(())
        })?;
        self.drain_jobs();
        Ok(())
    }

    /// Execute all pending jobs (microtasks/promises) in the event loop.
    pub fn execute_pending_jobs(&self) -> Result<()> {
        loop {
            match self.runtime.execute_pending_job() {
                Ok(false) => break,
                Ok(true) => continue,
                Err(e) => {
                    return Err(JsError::Execution(format!("Pending job failed: {e}")));
                }
            }
        }
        Ok(())
    }

    /// Install additional host bindings after construction.
    pub fn install_host_bindings(&self, bindings: &crate::host::HostBindingsExt) -> Result<()> {
        self.context.with(|ctx| {
            bindings.install(&ctx)
        })
    }

    /// Get access to the underlying rquickjs Context (for advanced usage).
    pub fn with_context<F, R>(&self, f: F) -> R
    where
        F: FnOnce(rquickjs::Ctx<'_>) -> R,
    {
        self.context.with(f)
    }
}
