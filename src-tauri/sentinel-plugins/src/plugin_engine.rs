//! Deno Core 插件引擎
//!
//! 提供 JS/TS 插件执行环境，支持：
//! - ESM/TypeScript 模块加载
//! - 插件加载与热重载
//! - 全权限沙箱（--allow-all）
//! - 插件 API 注入（scan_request, scan_response, emit_finding）

use crate::error::{PluginError, Result};
use crate::plugin_ops::{sentinel_plugin_ext, PluginContext};
use crate::types::{Finding, PluginMetadata, RequestContext, ResponseContext};
use deno_ast::{
    EmitOptions, MediaType, ParseParams, SourceMapOption, TranspileModuleOptions, TranspileOptions,
};
use deno_core::{
    url::Url, FastString, JsRuntime, ModuleCodeString, ModuleLoadOptions, ModuleLoadResponse,
    ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType, ResolutionKind,
    RuntimeOptions,
};
use deno_error::JsErrorBox;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tracing::{debug, info};

/// 插件模块加载器
///
/// 支持：
/// - TypeScript 转译为 JavaScript
/// - ESM 导入语句解析
/// - 内存中的插件代码管理
struct PluginModuleLoader {
    /// 插件代码缓存 (specifier -> source code)
    modules: std::cell::RefCell<std::collections::HashMap<String, String>>,
}

impl PluginModuleLoader {
    fn new() -> Self {
        Self {
            modules: std::cell::RefCell::new(std::collections::HashMap::new()),
        }
    }

    /// 注册插件模块代码
    fn register_module(&self, specifier: &str, code: String) {
        self.modules
            .borrow_mut()
            .insert(specifier.to_string(), code);
    }

    /// 转译 TypeScript/JSX 为 JavaScript
    fn transpile(&self, specifier: &str, source: &str) -> std::result::Result<String, String> {
        // 根据文件扩展名确定 MediaType
        let media_type = if specifier.ends_with(".ts") {
            MediaType::TypeScript
        } else if specifier.ends_with(".tsx") {
            MediaType::Tsx
        } else if specifier.ends_with(".jsx") {
            MediaType::Jsx
        } else if specifier.ends_with(".mjs") {
            MediaType::Mjs
        } else {
            MediaType::JavaScript
        };

        // 如果是纯 JS，直接返回
        if media_type == MediaType::JavaScript || media_type == MediaType::Mjs {
            return Ok(source.to_string());
        }

        // 解析并转译
        let parsed = deno_ast::parse_module(ParseParams {
            specifier: ModuleSpecifier::parse(specifier).unwrap(),
            text: source.into(),
            media_type,
            capture_tokens: false,
            scope_analysis: false,
            maybe_syntax: None,
        })
        .map_err(|e| format!("Failed to parse {}: {}", specifier, e))?;

        let transpiled = parsed
            .transpile(
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions {
                    source_map: SourceMapOption::None,
                    ..Default::default()
                },
            )
            .map_err(|e| format!("Failed to transpile {}: {}", specifier, e))?;

        Ok(transpiled.into_source().text)
    }
}

impl ModuleLoader for PluginModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: ResolutionKind,
    ) -> std::result::Result<ModuleSpecifier, JsErrorBox> {
        // 简单解析：如果是相对路径，基于 referrer 解析；否则直接作为 URL
        if specifier.starts_with("./") || specifier.starts_with("../") {
            let base = Url::parse(referrer)
                .map_err(|e| JsErrorBox::generic(format!("Failed to parse referrer: {}", e)))?;
            base.join(specifier)
                .map_err(|e| JsErrorBox::generic(format!("Failed to join specifier: {}", e)))
        } else if specifier.starts_with("file://") || specifier.starts_with("sentinel://") {
            Url::parse(specifier)
                .map_err(|e| JsErrorBox::generic(format!("Failed to parse URL: {}", e)))
        } else {
            // 默认作为 sentinel:// 协议的模块
            Url::parse(&format!("sentinel://{}", specifier))
                .map_err(|e| JsErrorBox::generic(format!("Failed to parse module URL: {}", e)))
        }
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&deno_core::ModuleLoadReferrer>,
        _load_options: ModuleLoadOptions,
    ) -> ModuleLoadResponse {
        let specifier = module_specifier.as_str();

        // 从缓存中获取模块代码
        let modules = self.modules.borrow();
        let source = match modules.get(specifier) {
            Some(code) => code.clone(),
            None => {
                return ModuleLoadResponse::Sync(Err(JsErrorBox::generic(format!(
                    "Module not found: {}",
                    specifier
                ))));
            }
        };
        drop(modules);

        // 转译代码（如果是 TS）
        let transpiled = match self.transpile(specifier, &source) {
            Ok(code) => code,
            Err(e) => {
                return ModuleLoadResponse::Sync(Err(JsErrorBox::generic(format!(
                    "Transpile error for {}: {}",
                    specifier, e
                ))));
            }
        };

        // 返回模块源码
        let module_source = ModuleSource::new(
            ModuleType::JavaScript,
            ModuleSourceCode::String(ModuleCodeString::from(transpiled)),
            module_specifier,
            None,
        );

        ModuleLoadResponse::Sync(Ok(module_source))
    }
}

/// 插件引擎
pub struct PluginEngine {
    /// Deno Runtime
    runtime: JsRuntime,
    /// 模块加载器
    loader: Rc<PluginModuleLoader>,
    /// 已加载的插件元数据
    metadata: Option<PluginMetadata>,
    /// 插件路径（用于标识）
    plugin_path: Option<PathBuf>,
    /// 主模块 ID
    main_module_id: Option<deno_core::ModuleId>,
}

impl PluginEngine {
    /// 创建新的插件引擎实例
    pub fn new() -> Result<Self> {
        let loader = Rc::new(PluginModuleLoader::new());

        // 创建 Deno Runtime，注入自定义 extension 和模块加载器
        let runtime = JsRuntime::new(RuntimeOptions {
            module_loader: Some(loader.clone()),
            extensions: vec![sentinel_plugin_ext::init()],
            ..Default::default()
        });

        info!("PluginEngine initialized with ESM/TS support via ModuleLoader");

        Ok(Self {
            runtime,
            loader,
            metadata: None,
            plugin_path: None,
            main_module_id: None,
        })
    }

    /// 加载插件代码（从文件路径 - 已弃用）
    ///
    /// **注意**: 此方法已弃用，仅用于向后兼容。
    /// 在DB-only模式下，请使用 `load_plugin_from_code()` 方法。
    #[deprecated(note = "Use load_plugin_from_code() for DB-only mode")]
    pub async fn load_plugin(&mut self, plugin_path: &Path) -> Result<PluginMetadata> {
        let code = tokio::fs::read_to_string(plugin_path)
            .await
            .map_err(|e| PluginError::Load(format!("Failed to read plugin: {}", e)))?;

        self.load_plugin_from_code(&code, &plugin_path.to_string_lossy())
            .await
    }

    /// 从代码字符串加载插件（DB-only 模式推荐方法）
    ///
    /// # 参数
    /// - `code`: 插件 JavaScript/TypeScript 代码
    /// - `plugin_id`: 插件唯一标识符（用于日志和错误消息）
    ///
    /// # 返回
    /// 返回插件元数据。如果插件代码中没有 `get_metadata()` 函数，
    /// 将使用传入的 `metadata` 或返回默认元数据。
    pub async fn load_plugin_from_code(
        &mut self,
        code: &str,
        plugin_id: &str,
    ) -> Result<PluginMetadata> {
        // 执行插件代码（初始化）
        self.runtime
            .execute_script(
                format!("plugin_{}.js", plugin_id),
                deno_core::FastString::from(code.to_string()),
            )
            .map_err(|e| {
                PluginError::Load(format!("Failed to execute plugin {}: {}", plugin_id, e))
            })?;

        // 调用 get_metadata() 获取插件元数据
        // 使用简化版本：直接执行 JavaScript 代码获取元数据
        let metadata_script = r#"
            (function() {
                if (typeof get_metadata === 'function') {
                    return JSON.stringify(get_metadata());
                } else {
                    throw new Error('get_metadata function not found');
                }
            })();
        "#
        .to_string();

        let _metadata_value = self
            .runtime
            .execute_script("get_metadata", deno_core::FastString::from(metadata_script))
            .map_err(|e| {
                PluginError::Load(format!(
                    "Failed to call get_metadata for {}: {}",
                    plugin_id, e
                ))
            })?;

        // 解析元数据（通过 JavaScript 的 JSON.stringify）
        // TODO: 实现从 v8::Global 到 String 的正确转换
        // 临时使用直接解析的方式
        let metadata: PluginMetadata = serde_json::from_str(
            r#"{"id":"unknown","name":"Unknown","version":"1.0.0","category":"unknown","default_severity":"medium","tags":[]}"#
        ).unwrap();

        info!(
            "Loaded plugin from code: {} v{} (metadata parsing pending)",
            metadata.id, metadata.version
        );
        self.metadata = Some(metadata.clone());
        self.plugin_path = Some(PathBuf::from(format!("db://{}", plugin_id)));

        Ok(metadata)
    }

    /// 从代码和元数据加载插件（DB-only 模式推荐方法）
    ///
    /// # 参数
    /// - `code`: 插件 JavaScript/TypeScript 代码（支持 ESM）
    /// - `metadata`: 从数据库读取的插件元数据
    ///
    /// # 说明
    /// 此方法直接使用提供的元数据，不调用插件的 `get_metadata()` 函数。
    /// 适用于元数据已经存储在数据库中的场景。
    /// 支持 ES Modules (import/export) 和 TypeScript。
    pub async fn load_plugin_with_metadata(
        &mut self,
        code: &str,
        metadata: PluginMetadata,
    ) -> Result<()> {
        let plugin_id = metadata.id.clone();

        // 确定模块 URL（使用 sentinel:// 协议）
        let module_specifier = format!("sentinel://plugin_{}.ts", plugin_id);

        // 注册模块到加载器
        self.loader
            .register_module(&module_specifier, code.to_string());

        // 加载主模块
        let specifier = ModuleSpecifier::parse(&module_specifier)
            .map_err(|e| PluginError::Load(format!("Invalid module specifier: {}", e)))?;

        let module_id = self
            .runtime
            .load_main_es_module(&specifier)
            .await
            .map_err(|e| {
                PluginError::Load(format!("Failed to load module {}: {}", plugin_id, e))
            })?;

        // 执行模块
        let result = self.runtime.mod_evaluate(module_id);
        self.runtime
            .run_event_loop(deno_core::PollEventLoopOptions::default())
            .await
            .map_err(|e| {
                PluginError::Load(format!("Failed to evaluate module {}: {}", plugin_id, e))
            })?;

        result.await.map_err(|e| {
            PluginError::Load(format!("Module evaluation error {}: {:?}", plugin_id, e))
        })?;

        // 注入 Sentinel 全局对象和必要的运行时 polyfill（如 TextDecoder、URL 等），提供插件 API
        let init_script = r#"
            // Sentinel plugin API
            globalThis.Sentinel = {
                emitFinding: function(finding) {
                    Deno.core.ops.op_emit_finding(finding);
                },
                log: function(level, message) {
                    Deno.core.ops.op_plugin_log(level, message);
                }
            };

            // Polyfill TextDecoder / TextEncoder for plugin environment
            (function () {
                if (typeof TextDecoder === "undefined") {
                    class SimpleTextDecoder {
                        constructor(label = "utf-8", options) {
                            this.encoding = label.toLowerCase();
                        }
                        decode(input) {
                            if (input == null) {
                                return "";
                            }
                            if (input instanceof Uint8Array) {
                                let s = "";
                                for (let i = 0; i < input.length; i++) {
                                    s += String.fromCharCode(input[i]);
                                }
                                try {
                                    // 尝试按 UTF-8 解码
                                    return decodeURIComponent(escape(s));
                                } catch (_) {
                                    return s;
                                }
                            }
                            return String(input);
                        }
                    }
                    globalThis.TextDecoder = SimpleTextDecoder;
                }

                if (typeof TextEncoder === "undefined") {
                    class SimpleTextEncoder {
                        constructor() {}
                        encode(input) {
                            input = input == null ? "" : String(input);
                            const utf8 = unescape(encodeURIComponent(input));
                            const arr = new Uint8Array(utf8.length);
                            for (let i = 0; i < utf8.length; i++) {
                                arr[i] = utf8.charCodeAt(i);
                            }
                            return arr;
                        }
                    }
                    globalThis.TextEncoder = SimpleTextEncoder;
                }

                // Minimal URLSearchParams polyfill
                if (typeof URLSearchParams === "undefined") {
                    class SimpleURLSearchParams {
                        constructor(init) {
                            this._params = [];
                            if (!init) return;
                            let query = typeof init === "string" ? init : "";
                            if (query.startsWith("?")) {
                                query = query.substring(1);
                            }
                            if (query.length === 0) return;
                            const pairs = query.split("&");
                            for (const pair of pairs) {
                                if (!pair) continue;
                                const [k, v = ""] = pair.split("=");
                                const key = decodeURIComponent(k.replace(/\+/g, " "));
                                const value = decodeURIComponent(v.replace(/\+/g, " "));
                                this._params.push([key, value]);
                            }
                        }
                        append(name, value) {
                            this._params.push([String(name), String(value)]);
                        }
                        get(name) {
                            name = String(name);
                            for (const [k, v] of this._params) {
                                if (k === name) return v;
                            }
                            return null;
                        }
                        getAll(name) {
                            name = String(name);
                            const res = [];
                            for (const [k, v] of this._params) {
                                if (k === name) res.push(v);
                            }
                            return res;
                        }
                        has(name) {
                            name = String(name);
                            for (const [k] of this._params) {
                                if (k === name) return true;
                            }
                            return false;
                        }
                        toString() {
                            return this._params
                                .map(([k, v]) => encodeURIComponent(k) + "=" + encodeURIComponent(v))
                                .join("&");
                        }
                        forEach(callback, thisArg) {
                            for (const [k, v] of this._params) {
                                callback.call(thisArg, v, k, this);
                            }
                        }
                    }
                    globalThis.URLSearchParams = SimpleURLSearchParams;
                }

                // Minimal URL polyfill (HTTP/HTTPS only, enough for plugin parsing)
                if (typeof URL === "undefined") {
                    class SimpleURL {
                        constructor(input, base) {
                            let url = String(input);
                            if (base) {
                                // Very small base support: if input is relative, prepend base
                                if (!/^[a-zA-Z][a-zA-Z0-9+\-.]*:/.test(url)) {
                                    const b = String(base);
                                    if (b.endsWith("/") && !url.startsWith("/")) {
                                        url = b + url;
                                    } else {
                                        url = b.replace(/\/+$/, "") + "/" + url.replace(/^\/+/, "");
                                    }
                                }
                            }
                            this.href = url;
                            const m = url.match(/^(https?:)(\/\/([^\/?#]*))?([^?#]*)(\?[^#]*)?(#.*)?$/i);
                            this.protocol = m && m[1] ? m[1].toLowerCase() : "";
                            this.host = m && m[3] ? m[3] : "";
                            const hostParts = this.host.split(":");
                            this.hostname = hostParts[0] || "";
                            this.port = hostParts[1] || "";
                            this.pathname = m && m[4] ? (m[4] || "/") : url;
                            this.search = m && m[5] ? m[5] : "";
                            this.hash = m && m[6] ? m[6] : "";
                            this.origin = this.protocol && this.host ? this.protocol + "//" + this.host : "";
                            const searchWithoutQ = this.search.startsWith("?") ? this.search.substring(1) : this.search;
                            this.searchParams = new globalThis.URLSearchParams(searchWithoutQ);
                        }
                        toString() {
                            return this.href;
                        }
                    }
                    globalThis.URL = SimpleURL;
                }

                // Fetch API polyfill using Deno.core.ops.op_fetch
                if (typeof fetch === "undefined") {
                    globalThis.fetch = async function(url, options = {}) {
                        const fetchOptions = {
                            method: options.method || "GET",
                            headers: options.headers || {},
                            body: options.body || null,
                            timeout: options.timeout || 30000,
                        };
                        
                        const response = await Deno.core.ops.op_fetch(url, fetchOptions);
                        
                        // Check for errors
                        if (!response.success || response.error) {
                            throw new Error(`Fetch failed: ${response.error || 'Unknown error'}`);
                        }
                        
                        // Return a Response-like object
                        return {
                            ok: response.ok,
                            status: response.status,
                            headers: response.headers,
                            text: async () => response.body,
                            json: async () => JSON.parse(response.body),
                            body: response.body,
                        };
                    };
                }
            })();
        "#
        .to_string();

        self.runtime
            .execute_script(
                "init_sentinel_api",
                deno_core::FastString::from(init_script),
            )
            .map_err(|e| {
                PluginError::Load(format!(
                    "Failed to initialize Sentinel API for {}: {}",
                    plugin_id, e
                ))
            })?;

        info!(
            "Loaded ESM/TS plugin: {} v{}",
            metadata.name, metadata.version
        );
        self.metadata = Some(metadata);
        self.plugin_path = Some(PathBuf::from(format!("db://{}", plugin_id)));
        self.main_module_id = Some(module_id);

        Ok(())
    }

    /// 扫描请求（调用插件的 scan_request）
    pub async fn scan_request(&mut self, ctx: &RequestContext) -> Result<Vec<Finding>> {
        let ctx_json = serde_json::to_value(ctx)
            .map_err(|e| PluginError::Execution(format!("Failed to serialize request: {}", e)))?;

        // 调用插件函数
        self.call_plugin_function("scan_request", &ctx_json).await?;

        // 从 PluginContext 中获取插件发送的漏洞
        let findings = {
            let op_state = self.runtime.op_state();
            let op_state_borrow = op_state.borrow();
            let plugin_ctx = op_state_borrow.borrow::<PluginContext>();
            plugin_ctx.take_findings()
        };

        debug!(
            "Plugin {} found {} issues in request",
            self.metadata
                .as_ref()
                .map(|m| m.id.as_str())
                .unwrap_or("unknown"),
            findings.len()
        );

        Ok(findings)
    }

    /// 扫描响应（调用插件的 scan_response）
    pub async fn scan_response(
        &mut self,
        req_ctx: &RequestContext,
        resp_ctx: &ResponseContext,
    ) -> Result<Vec<Finding>> {
        let combined = serde_json::json!({
            "request": req_ctx,
            "response": resp_ctx,
        });

        // 调用插件函数
        self.call_plugin_function("scan_response", &combined)
            .await?;

        // 从 PluginContext 中获取插件发送的漏洞
        let findings = {
            let op_state = self.runtime.op_state();
            let op_state_borrow = op_state.borrow();
            let plugin_ctx = op_state_borrow.borrow::<PluginContext>();
            plugin_ctx.take_findings()
        };

        debug!(
            "Plugin {} found {} issues in response",
            self.metadata
                .as_ref()
                .map(|m| m.id.as_str())
                .unwrap_or("unknown"),
            findings.len()
        );

        Ok(findings)
    }

    /// 调用插件函数（通用）
    ///
    /// 插件通过 Sentinel.emitFinding() 发送漏洞，而不是返回值
    /// 支持调用 ESM 导出的函数
    async fn call_plugin_function(
        &mut self,
        fn_name: &str,
        args: &serde_json::Value,
    ) -> Result<()> {
        // 构造调用脚本：从全局作用域或模块命名空间中获取函数
        // 优先尝试全局函数（兼容旧脚本模式），然后尝试从模块获取导出
        let call_script = format!(
            r#"
            (async function() {{
                const args = {};
                let fn = globalThis.{};

                // 如果全局作用域没有，尝试从模块获取（假设已导出到 globalThis）
                if (typeof fn !== 'function') {{
                    // Debug: 打印 globalThis 上的所有函数
                    const availableFunctions = Object.keys(globalThis).filter(key => typeof globalThis[key] === 'function');
                    console.error('Available functions on globalThis:', availableFunctions);
                    console.error('Looking for function: {}');
                    console.error('Type of globalThis.{}: ', typeof globalThis.{});
                    throw new Error('Function {} not found in global scope or module exports');
                }}

                // 调用函数（支持同步和异步）
                const result = fn(args);
                if (result instanceof Promise) {{
                    await result;
                }}
                return true;
            }})();
            "#,
            serde_json::to_string(args).unwrap(),
            fn_name,
            fn_name,
            fn_name,
            fn_name,
            fn_name,
        );

        let result = self
            .runtime
            .execute_script("call_plugin", FastString::from(call_script))
            .map_err(|e| PluginError::Execution(format!("Failed to call {}: {}", fn_name, e)))?;

        // 如果函数返回 Promise，需要等待其完成
        let promise = self.runtime.resolve(result);
        self.runtime
            .run_event_loop(deno_core::PollEventLoopOptions::default())
            .await
            .map_err(|e| {
                PluginError::Execution(format!("Event loop error during {}: {}", fn_name, e))
            })?;

        promise.await.map_err(|e| {
            PluginError::Execution(format!("Promise rejection in {}: {:?}", fn_name, e))
        })?;

        debug!("Plugin function {} executed successfully", fn_name);

        Ok(())
    }

    /// 获取插件元数据
    pub fn get_metadata(&self) -> Option<&PluginMetadata> {
        self.metadata.as_ref()
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_engine_creation() {
        let engine = PluginEngine::new();
        assert!(engine.is_ok());
    }
}
