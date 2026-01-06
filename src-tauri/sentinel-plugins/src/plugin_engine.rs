//! Deno Core 插件引擎
//!
//! 提供 JS/TS 插件执行环境，支持：
//! - ESM/TypeScript 模块加载
//! - 插件加载与热重载
//! - 全权限沙箱（--allow-all）
//! - 原生 Web API 支持（via deno_web/deno_webidl）
//! - 文件系统操作（via custom ops）
//!
//! 基于 deno_core 0.373.0 + deno_web 0.254.0

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
use deno_core::v8;
use deno_features::FeatureChecker;
#[cfg(not(target_os = "windows"))]
use deno_permissions::{PermissionsContainer, RuntimePermissionDescriptorParser};

use deno_error::JsErrorBox;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use tracing::debug;

/// 插件模块加载器
///
/// 支持：
/// - TypeScript 转译为 JavaScript
/// - ESM 导入语句解析
/// - 内存中的插件代码管理
struct PluginModuleLoader {
    /// 插件代码缓存 (specifier -> source code)
    modules: std::cell::RefCell<std::collections::HashMap<String, String>>,
    /// HTTP 客户端（用于远程模块加载）
    http_client: reqwest::Client,
    /// 远程模块缓存目录
    cache_dir: PathBuf,
}

impl PluginModuleLoader {
    fn new() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("sentinel-ai")
            .join("plugin-modules");
        let _ = std::fs::create_dir_all(&cache_dir);

        // Create default HTTP client (proxy will be applied when actually used)
        let http_client = reqwest::Client::new();
        
        Self {
            modules: std::cell::RefCell::new(std::collections::HashMap::new()),
            http_client,
            cache_dir,
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
        transpile_module(specifier, source)
    }
}

fn transpile_module(specifier: &str, source: &str) -> std::result::Result<String, String> {
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
        } else if specifier.starts_with("file://")
            || specifier.starts_with("sentinel://")
            || specifier.starts_with("ext:")
            || specifier.starts_with("internal:")
            || specifier.starts_with("http://")
            || specifier.starts_with("https://")
        {
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

        if specifier.starts_with("http://") || specifier.starts_with("https://") {
            let url = specifier.to_string();
            let client = self.http_client.clone();
            let cache_dir = self.cache_dir.clone();
            let module_specifier = module_specifier.clone();

            return ModuleLoadResponse::Async(Box::pin(async move {
                let mut hasher = Sha256::new();
                hasher.update(url.as_bytes());
                let hash = format!("{:x}", hasher.finalize());

                let ext = ModuleSpecifier::parse(&url)
                    .ok()
                    .and_then(|u| {
                        u.path_segments()
                            .and_then(|mut segs| segs.next_back())
                            .and_then(|s| s.rsplit_once('.').map(|(_, e)| format!(".{}", e)))
                    })
                    .unwrap_or_else(|| ".ts".to_string());

                let cache_path = cache_dir.join(format!("{}{}", hash, ext));

                let source = if cache_path.exists() {
                    tokio::fs::read_to_string(&cache_path)
                        .await
                        .map_err(|e| {
                            JsErrorBox::generic(format!("Failed to read cache {}: {}", url, e))
                        })?
                } else {
                    // Create a new client with proxy support for this request
                    let builder = reqwest::Client::builder();
                    let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
                    let client_with_proxy = builder.build().unwrap_or_else(|_| client.clone());
                    
                    let response = client_with_proxy
                        .get(&url)
                        .send()
                        .await
                        .map_err(|e| {
                            JsErrorBox::generic(format!("Failed to fetch {}: {}", url, e))
                        })?;

                    if !response.status().is_success() {
                        return Err(JsErrorBox::generic(format!(
                            "Failed to fetch {}: {}",
                            url,
                            response.status()
                        )));
                    }

                    let text = response.text().await.map_err(|e| {
                        JsErrorBox::generic(format!("Failed to read response {}: {}", url, e))
                    })?;

                    let _ = tokio::fs::create_dir_all(&cache_dir).await;
                    let _ = tokio::fs::write(&cache_path, &text).await;

                    text
                };

                let transpiled = transpile_module(&url, &source).map_err(|e| {
                    JsErrorBox::generic(format!("Transpile error for {}: {}", url, e))
                })?;

                Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(ModuleCodeString::from(transpiled)),
                    &module_specifier,
                    None,
                ))
            }));
        }

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
        // Order matters: deno_webidl -> deno_web -> deno_crypto -> deno_net -> sentinel_plugin_ext
        let extensions = vec![
            // deno_webidl: WebIDL bindings (required by deno_web)
            deno_webidl::deno_webidl::init(),
            // deno_web: TextEncoder, TextDecoder, URL, URLSearchParams, console, timers, Headers, etc.
            deno_web::deno_web::init(
                Arc::new(deno_web::BlobStore::default()),
                None, // maybe_location
                deno_web::InMemoryBroadcastChannel::default(),
            ),
            // deno_crypto: Web Cryptography API (crypto.getRandomValues, subtle, etc.)
            deno_crypto::deno_crypto::init(None),
            // deno_net: TCP/UDP/TLS networking APIs (2 args: root_cert_store_provider, unsafely_ignore_certificate_errors)
            deno_net::deno_net::init(None, None),
            // sentinel_plugin_ext: custom ops for plugin system (emitFinding, log, fetch, file operations)
            sentinel_plugin_ext::init(),
        ];

        // Register extension ESM modules in our loader so they can be imported
        for ext in &extensions {
            for file in ext.esm_files.iter() {
                let code = file.load().unwrap().as_str().to_string();
                loader.register_module(file.specifier, code);
            }
        }

        // Create Deno Runtime with extensions and module loader
        let mut runtime = JsRuntime::new(RuntimeOptions {
            module_loader: Some(loader.clone()),
            extensions,
            ..Default::default()
        });

        {
            let op_state = runtime.op_state();
            let mut state = op_state.borrow_mut();
            
            #[cfg(not(target_os = "windows"))]
            {
                let parser = Arc::new(RuntimePermissionDescriptorParser::new(
                    sys_traits::impls::RealSys,
                ));
                state.put(PermissionsContainer::allow_all(parser));
            }
            
            #[cfg(target_os = "windows")]
            {
                // On Windows, skip permission setup due to cross-compilation issues
                // Plugins will run without permission checks
                tracing::warn!("Running without permission checks on Windows");
            }
            
            let mut features = FeatureChecker::default();
            features.enable_feature(deno_net::UNSTABLE_FEATURE_NAME);
            state.put(Arc::new(features));
        }

        // Patch Deno.build with actual runtime platform info (required by deno_mongo, etc.)
        // This must run after extensions are initialized (bootstrap has set up Deno.build stub)
        let platform_patch = format!(
            r#"
            if (globalThis.Deno && globalThis.Deno.build) {{
                globalThis.Deno.build.os = "{}";
                globalThis.Deno.build.arch = "{}";
            }}
            "#,
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        runtime
            .execute_script("<platform_patch>", platform_patch)
            .map_err(|e| {
                PluginError::Load(format!("Failed to patch Deno.build: {}", e))
            })?;

        Ok(Self {
            runtime,
            loader,
            metadata: None,
            plugin_path: None,
            main_module_id: None,
        })
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
if (typeof analyze === 'function') {
    globalThis.analyze = analyze;
}
if (typeof run === 'function') {
    globalThis.run = run;
}
if (typeof execute === 'function') {
    globalThis.execute = execute;
}
if (typeof get_input_schema === 'function') {
    globalThis.get_input_schema = get_input_schema;
}
if (typeof getInputSchema === 'function') {
    globalThis.getInputSchema = getInputSchema;
}
if (typeof get_metadata === 'function') {
    globalThis.get_metadata = get_metadata;
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

    /// 获取插件的输入参数 Schema（方案2：运行时调用插件的 get_input_schema 函数）
    ///
    /// 插件需要导出一个 `get_input_schema()` 函数，返回 JSON Schema 对象。
    /// 如果插件没有该函数，返回默认的空 schema。
    ///
    /// # 示例插件代码
    /// ```typescript
    /// export function get_input_schema() {
    ///   return {
    ///     type: 'object',
    ///     properties: {
    ///       text: { type: 'string', description: '要处理的文字' },
    ///       mode: { type: 'string', enum: ['encode', 'decode'] }
    ///     },
    ///     required: ['text', 'mode']
    ///   };
    /// }
    /// ```
    pub async fn get_input_schema(&mut self) -> Result<serde_json::Value> {
        // 构造调用脚本：通过 op_plugin_return 存储结果
        let call_script = r#"
            (function() {
                let schema = null;
                
                // 优先尝试 get_input_schema
                if (typeof globalThis.get_input_schema === 'function') {
                    schema = globalThis.get_input_schema();
                }
                // 兼容 getInputSchema (camelCase)
                else if (typeof globalThis.getInputSchema === 'function') {
                    schema = globalThis.getInputSchema();
                }
                
                // 通过 op_plugin_return 存储结果
                if (schema !== null) {
                    try { Deno.core.ops.op_plugin_return(schema); } catch (_e) {}
                }
                
                return schema !== null;
            })();
        "#;

        self.runtime
            .execute_script("get_input_schema", FastString::from(call_script.to_string()))
            .map_err(|e| PluginError::Execution(format!("Failed to call get_input_schema: {}", e)))?;

        // 从 PluginContext 获取结果
        let result = {
            let op_state = self.runtime.op_state();
            let op_state_borrow = op_state.borrow();
            let plugin_ctx = op_state_borrow.borrow::<PluginContext>();
            plugin_ctx.take_last_result()
        };

        // 如果插件返回了 schema，使用它
        if let Some(schema) = result {
            return Ok(schema);
        }

        // 插件未定义 schema，返回默认值
        Ok(serde_json::json!({
            "type": "object",
            "properties": {
                "input": {"type": "string", "description": "Tool input parameter"}
            }
        }))
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

        let exec_result: Result<()> = async {
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
        }.await;

        // If termination was requested, ensure we clear it before next run.
        self.cancel_terminate_execution();

        exec_result
    }

    /// 获取插件元数据
    pub fn get_metadata(&self) -> Option<&PluginMetadata> {
        self.metadata.as_ref()
    }

    /// Get a thread-safe handle for this isolate.
    ///
    /// This handle can be used from other OS threads to request termination of
    /// the currently running JavaScript (best-effort).
    pub fn isolate_handle(&mut self) -> v8::IsolateHandle {
        self.runtime.v8_isolate().thread_safe_handle()
    }

    /// Cancel a previously requested TerminateExecution state (if any).
    ///
    /// This should be called after handling a termination, otherwise the next
    /// JS entry may immediately fail.
    pub fn cancel_terminate_execution(&mut self) {
        self.runtime.v8_isolate().cancel_terminate_execution();
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
