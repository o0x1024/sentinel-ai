#[async_trait::async_trait]
pub trait PluginExecutor: Send + Sync {
    async fn execute_plugin(
        &self,
        plugin_id: &str,
        config: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value>;
}

// And we'll add run_workflow method to WorkflowOrchestrator
