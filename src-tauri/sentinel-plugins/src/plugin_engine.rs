//! Deno Core 插件引擎
//!
//! 提供 JS/TS 插件执行环境，支持：
//! - ESM/TypeScript 模块加载
//! - 插件加载与热重载
//! - 全权限沙箱（--allow-all）
//! - 原生 Web API 支持（via deno_web/deno_webidl/deno_fetch）
//!
//! 基于 deno_core 0.373.0 + deno_web 0.254.0 + deno_fetch 0.247.0

use crate::error::{PluginError, Result};
use crate::plugin_ops::{sentinel_plugin_ext, PluginContext};
use crate::types::{Finding, PluginMetadata};
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
use std::sync::Arc;
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
        } else if specifier.starts_with("file://") || specifier.starts_with("sentinel://") || specifier.starts_with("ext:") || specifier.starts_with("internal:") {
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

        // Create extensions for native Web API support
        // Order matters: deno_webidl -> deno_web -> sentinel_plugin_ext
        let extensions = vec![
            // deno_webidl: WebIDL bindings (required by deno_web)
            deno_webidl::deno_webidl::init(),
            // deno_web: TextEncoder, TextDecoder, URL, URLSearchParams, console, timers, Headers, etc.
            deno_web::deno_web::init(
                Arc::new(deno_web::BlobStore::default()),
                None, // maybe_location
                deno_web::InMemoryBroadcastChannel::default(),
            ),
            // sentinel_plugin_ext: custom ops for plugin system (emitFinding, log, fetch)
            sentinel_plugin_ext::init(),
        ];

        // Register extension ESM modules in our loader so they can be imported
        for ext in &extensions {
            for file in ext.esm_files.iter() {
                let code = match &file.code {
                    deno_core::ExtensionFileSourceCode::IncludedInBinary(s) => s.as_str().to_string(),
                    deno_core::ExtensionFileSourceCode::LoadedFromMemoryDuringSnapshot(s) => s.as_str().to_string(),
                    deno_core::ExtensionFileSourceCode::Computed(s) => s.to_string(),
                    _ => continue,
                };
                loader.register_module(file.specifier, code);
            }
        }

        // Create Deno Runtime with extensions and module loader
        let runtime = JsRuntime::new(RuntimeOptions {
            module_loader: Some(loader.clone()),
            extensions,
            ..Default::default()
        });

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

        // 对 plugin_id 进行 URL 安全化处理（替换特殊字符）
        let safe_id: String = plugin_id
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        // 确定模块 URL（使用 sentinel:// 协议）
        let module_specifier = format!("sentinel://plugin_{}.ts", safe_id);

        // 自动在 ESM 模块末尾追加代码，将导出的函数绑定到 globalThis
        // 这解决了 ESM 导出函数无法被 call_plugin_function 正确调用的问题
        let binding_code = r#"
// Auto-generated: Bind ESM exports to globalThis for plugin engine compatibility
if (typeof scan_transaction === 'function') {
    globalThis.scan_transaction = scan_transaction;
}
if (typeof get_metadata === 'function') {
    globalThis.get_metadata = get_metadata;
}
if (typeof analyze === 'function') {
    globalThis.analyze = analyze;
}
if (typeof run === 'function') {
    globalThis.run = run;
}
if (typeof execute === 'function') {
    globalThis.execute = execute;
}
"#;
        let augmented_code = format!("{}\n{}", code, binding_code);

        // 注册模块到加载器
        self.loader
            .register_module(&module_specifier, augmented_code);

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

        // Inject Sentinel plugin API and fetch polyfill
        // Note: We need to make sure deno_web/deno_webidl modules are evaluated
        let init_script = r#"
            // Internal bootstrap to ensure extension modules are evaluated
            // These are side-effect imports that set up globals like TextEncoder, URL, etc.
            import "ext:deno_webidl/00_webidl.js";
            import "ext:deno_web/00_infra.js";
            import "ext:deno_web/01_dom_exception.js";
            import "ext:deno_web/01_mimesniff.js";
            import "ext:deno_web/02_event.js";
            import "ext:deno_web/02_structured_clone.js";
            import "ext:deno_web/02_timers.js";
            import "ext:deno_web/03_abort_signal.js";
            import "ext:deno_web/04_global_interfaces.js";
            import "ext:deno_web/05_base64.js";
            import "ext:deno_web/06_streams.js";
            import "ext:deno_web/08_text_encoding.js";
            import "ext:deno_web/09_file.js";
            import "ext:deno_web/10_filereader.js";
            import "ext:deno_web/12_location.js";
            import "ext:deno_web/13_message_port.js";
            import "ext:deno_web/14_compression.js";
            import "ext:deno_web/15_performance.js";
            import "ext:deno_web/16_image_data.js";
            import "ext:deno_web/01_urlpattern.js";
            import "ext:deno_web/01_broadcast_channel.js";
            import "ext:deno_web/01_console.js";
            import "ext:deno_web/00_url.js";

            // Sentinel plugin API for vulnerability reporting
            globalThis.Sentinel = {
                emitFinding: function(finding) {
                    Deno.core.ops.op_emit_finding(finding);
                },
                log: function(level, message) {
                    Deno.core.ops.op_plugin_log(level, message);
                }
            };

            // Fetch API polyfill using custom op
            globalThis.fetch = async function(input, init = {}) {
                const url = typeof input === 'string' ? input : input.url;
                const method = init.method || (input.method || 'GET');
                const headers = {};
                
                if (init.headers) {
                    if (init.headers instanceof Headers) {
                        init.headers.forEach((v, k) => headers[k] = v);
                    } else if (Array.isArray(init.headers)) {
                        init.headers.forEach(([k, v]) => headers[k] = v);
                    } else {
                        Object.assign(headers, init.headers);
                    }
                }
                
                const body = init.body || null;
                const timeout = init.timeout || 30000; // default 30s
                const result = await Deno.core.ops.op_fetch(url, { method, headers, body, timeout });
                
                if (!result.success) {
                    throw new Error(result.error || 'Fetch failed');
                }
                
                return {
                    ok: result.ok,
                    status: result.status,
                    statusText: result.ok ? 'OK' : 'Error',
                    headers: new Headers(Object.entries(result.headers)),
                    text: async () => result.body,
                    json: async () => JSON.parse(result.body),
                    arrayBuffer: async () => new TextEncoder().encode(result.body).buffer,
                };
            };
        "#
        .to_string();

        // Use a module instead of a script to support top-level imports
        let bootstrap_specifier = deno_core::ModuleSpecifier::parse("internal:bootstrap").unwrap();
        self.loader.register_module("internal:bootstrap", init_script);
        
        let mod_id = self.runtime.load_side_es_module(&bootstrap_specifier).await
            .map_err(|e| {
                PluginError::Load(format!(
                    "Failed to load Sentinel bootstrap for {}: {}",
                    plugin_id, e
                ))
            })?;
            
        let _ = self.runtime.mod_evaluate(mod_id);
        
        self.runtime.run_event_loop(Default::default()).await
            .map_err(|e| {
                PluginError::Load(format!(
                    "Failed to initialize Sentinel API for {}: {}",
                    plugin_id, e
                ))
            })?;

        debug!(
            "Loaded ESM/TS plugin: {} v{}",
            metadata.name, metadata.version
        );
        self.metadata = Some(metadata);
        self.plugin_path = Some(PathBuf::from(format!("db://{}", plugin_id)));
        self.main_module_id = Some(module_id);

        Ok(())
    }

    /// 扫描完整 HTTP 事务
    ///
    /// 仅调用插件的 `scan_transaction`。
    pub async fn scan_transaction(
        &mut self,
        transaction: &crate::types::HttpTransaction,
    ) -> Result<Vec<Finding>> {
        let combined = serde_json::to_value(transaction).map_err(|e| {
            PluginError::Execution(format!("Failed to serialize transaction: {}", e))
        })?;

        // 仅调用 scan_transaction
        let result = self
            .call_plugin_function("scan_transaction", &combined)
            .await;

        if let Err(e) = result {
            debug!("Plugin execution failed or function not found: {}", e);
        }

        let findings = {
            let op_state = self.runtime.op_state();
            let op_state_borrow = op_state.borrow();
            let plugin_ctx = op_state_borrow.borrow::<PluginContext>();
            plugin_ctx.take_findings()
        };

        Ok(findings)
    }

    /// 执行Agent工具（调用插件的 analyze/run/execute）
    pub async fn execute_agent(
        &mut self,
        input: &serde_json::Value,
    ) -> Result<(Vec<Finding>, Option<serde_json::Value>)> {
        // 依次尝试常见的Agent入口函数名称: analyze -> run -> execute
        if let Err(e1) = self.call_plugin_function("analyze", input).await {
            if let Err(e2) = self.call_plugin_function("run", input).await {
                self
                    .call_plugin_function("execute", input)
                    .await
                    .map_err(|e3| PluginError::Execution(format!(
                        "Failed to call agent entrypoint (analyze/run/execute): analyze_err={:?}, run_err={:?}, execute_err={:?}",
                        e1, e2, e3
                    )))?;
            }
        }

        // 收集插件通过 Sentinel.emitFinding() 发送的漏洞
        let (findings, last_result) = {
            let op_state = self.runtime.op_state();
            let op_state_borrow = op_state.borrow();
            let plugin_ctx = op_state_borrow.borrow::<crate::plugin_ops::PluginContext>();
            (plugin_ctx.take_findings(), plugin_ctx.take_last_result())
        };

        Ok((findings, last_result))
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
                    const resolved = await result;
                    try {{ Deno.core.ops.op_plugin_return(resolved); }} catch (_e) {{}}
                }} else {{
                    try {{ Deno.core.ops.op_plugin_return(result); }} catch (_e) {{}}
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
