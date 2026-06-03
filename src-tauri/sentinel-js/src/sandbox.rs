//! Sandbox configuration for the plugin runtime.

/// Controls what the sandboxed plugin code can and cannot do.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Allow `eval()` and `new Function()` inside plugin code.
    /// Default: false (disabled for security).
    pub allow_eval: bool,
    /// Allow file system access (via Node compat fs module).
    /// Default: false.
    pub allow_fs: bool,
    /// Allowed file system paths (only if allow_fs is true).
    pub allowed_paths: Vec<String>,
    /// Allow network access (fetch).
    /// Default: true (plugins need HTTP).
    pub allow_network: bool,
    /// Allowed network hosts (empty = all allowed).
    pub allowed_hosts: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            allow_eval: false,
            allow_fs: false,
            allowed_paths: Vec::new(),
            allow_network: true,
            allowed_hosts: Vec::new(),
        }
    }
}
