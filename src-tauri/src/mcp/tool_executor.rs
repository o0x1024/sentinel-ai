use tokio::process::Command as TokioCommand;

/// 工具执行器，负责执行真实的命令行工具
pub struct ToolExecutor;

impl ToolExecutor {
    /// 检查工具是否已安装
    pub async fn check_tool_installed(tool_name: &str) -> bool {
        let result = if cfg!(target_os = "windows") {
            TokioCommand::new("where").arg(tool_name).output().await
        } else {
            TokioCommand::new("which").arg(tool_name).output().await
        };

        match result {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}
