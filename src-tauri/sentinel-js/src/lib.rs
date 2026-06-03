//! # sentinel-js
//!
//! Embedded JavaScript plugin runtime for Sentinel, built on QuickJS-NG.
//!
//! Provides a sandboxed JS execution environment with:
//! - Web APIs (fetch, URL, crypto.subtle, setTimeout, TextEncoder)
//! - Node.js compatibility (require, Buffer, process, path, crypto)
//! - Plugin host API (Sentinel.*, network, TLS, dictionary)

pub mod builtins;
pub mod convert;
pub mod error;
pub mod host;
#[cfg(feature = "node-compat")]
pub mod node_compat;
pub mod runtime;
pub mod sandbox;

pub use error::{JsError, Result};
pub use host::{HostBindings, HostBindingsExt, HostFn, HostFn2};
pub use runtime::{PluginRuntime, RuntimeConfig, FetchRequest, FetchResponse, LogLevel};

/// Re-export rquickjs types that consumers may need.
pub mod qjs {
    pub use rquickjs::*;
}
