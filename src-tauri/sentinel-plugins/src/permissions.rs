//! Capability-based permission system for plugins.
//!
//! Each plugin declares required capabilities. At load time, the engine
//! checks permissions and rejects calls to forbidden host functions
//! with a `PermissionDenied` error rather than silently not registering them.

use serde::{Deserialize, Serialize};

/// Network access level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPermission {
    None,
    FetchOnly,
    FullScan,
}

/// Filesystem access level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsPermission {
    None,
    ReadOnly,
    ReadWrite,
}

/// Plugin permissions (capability set).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermissions {
    pub network: NetworkPermission,
    pub filesystem: FsPermission,
    pub dictionary: bool,
    pub monitor_events: bool,
    pub ast_parse: bool,
    pub tls_inspect: bool,
}

impl Default for PluginPermissions {
    fn default() -> Self {
        Self::full_access()
    }
}

impl PluginPermissions {
    /// Fully sandboxed plugin: only fetch, no fs, no network scan.
    pub fn sandboxed() -> Self {
        Self {
            network: NetworkPermission::FetchOnly,
            filesystem: FsPermission::None,
            dictionary: false,
            monitor_events: false,
            ast_parse: false,
            tls_inspect: false,
        }
    }

    /// Full access (for trusted internal plugins).
    pub fn full_access() -> Self {
        Self {
            network: NetworkPermission::FullScan,
            filesystem: FsPermission::ReadWrite,
            dictionary: true,
            monitor_events: true,
            ast_parse: true,
            tls_inspect: true,
        }
    }

    pub fn can_fetch(&self) -> bool {
        matches!(
            self.network,
            NetworkPermission::FetchOnly | NetworkPermission::FullScan
        )
    }

    pub fn can_scan_network(&self) -> bool {
        self.network == NetworkPermission::FullScan
    }

    pub fn can_read_fs(&self) -> bool {
        matches!(
            self.filesystem,
            FsPermission::ReadOnly | FsPermission::ReadWrite
        )
    }

    pub fn can_write_fs(&self) -> bool {
        self.filesystem == FsPermission::ReadWrite
    }
}

/// Resource limits for plugin execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_fuel: Option<u64>,
    pub max_call_depth: Option<usize>,
    pub max_execution_ms: Option<u64>,
    pub max_memory_bytes: Option<usize>,
    pub max_output_bytes: Option<usize>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_fuel: Some(10_000_000),
            max_call_depth: Some(256),
            max_execution_ms: Some(30_000),
            max_memory_bytes: Some(50 * 1024 * 1024),
            max_output_bytes: Some(10 * 1024 * 1024),
        }
    }
}

impl ResourceLimits {
    pub fn unlimited() -> Self {
        Self {
            max_fuel: None,
            max_call_depth: None,
            max_execution_ms: None,
            max_memory_bytes: None,
            max_output_bytes: None,
        }
    }
}
