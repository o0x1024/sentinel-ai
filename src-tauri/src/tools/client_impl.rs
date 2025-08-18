use super::client::{McpSessionImpl, McpClientManager};

impl std::fmt::Debug for McpSessionImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpSessionImpl")
            .field("name", &"<McpSessionImpl>")
            .finish()
    }
}

impl std::fmt::Debug for McpClientManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClientManager")
            .finish()
    }
}
