//! Sentinel AI 被动扫描插件系统
//!
//! 此 crate 提供：
//! - **插件引擎**: 基于 Deno Core 的 JS/TS 插件运行时
//! - **插件管理器**: 插件加载、启用/禁用、注册表管理
//! - **内置插件**: SQL 注入、XSS、敏感信息检测等
//! - **类型定义**: TypeScript 类型定义和插件模板
//!
//! ## 模块结构
//!
//! - `plugin_engine`: Deno Core 插件引擎
//! - `plugin`: 插件管理器（PluginManager）
//! - `types`: 核心类型（Finding, RequestContext, ResponseContext 等）
//! - `error`: 错误类型
//!
//! ## 内置插件
//!
//! 所有插件源码位于 `plugins/` 目录：
//! - `plugins/builtin/` - 内置插件（SQL 注入、XSS、敏感信息）
//! - `plugins/template.ts` - 插件模板
//! - `plugins/plugin-types.d.ts` - TypeScript 类型定义
//! - `plugins/README.md` - 开发指南

pub mod error;
pub mod types;
pub mod plugin_ops;
pub mod plugin_engine;
pub mod plugin;

pub use plugin_engine::PluginEngine;
pub use plugin::{PluginManager, PluginStatus, PluginRecord};
pub use plugin_ops::{PluginContext, sentinel_plugin_ext};
pub use types::*;
pub use error::{PluginError, Result};

/// 获取内置插件目录路径
pub fn get_builtin_plugins_dir() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/plugins/builtin")
}

/// 获取插件模板路径
pub fn get_plugin_template_path() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/plugins/template.ts")
}

/// 获取类型定义路径
pub fn get_types_definition_path() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/plugins/plugin-types.d.ts")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths() {
        let builtin_dir = get_builtin_plugins_dir();
        let template = get_plugin_template_path();
        let types = get_types_definition_path();

        assert!(builtin_dir.contains("sentinel-plugins/plugins/builtin"));
        assert!(template.contains("sentinel-plugins/plugins/template.ts"));
        assert!(types.contains("sentinel-plugins/plugins/plugin-types.d.ts"));
    }
}
