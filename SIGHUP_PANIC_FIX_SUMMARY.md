# SIGHUP信号导致panic问题修复总结

## 问题分析

用户报告程序运行一段时间后会收到SIGHUP信号导致panic。通过分析发现这是一个常见的Unix系统进程管理问题：

### SIGHUP信号的常见触发场景
1. **终端连接断开**：SSH会话断开时
2. **控制终端关闭**：直接关闭终端窗口
3. **父进程结束**：父进程意外终止时子进程接收信号
4. **会话领导者退出**：会话组中的领导进程退出

### 在MCP子进程管理中的问题
- MCP服务器使用子进程来运行npm包（如playwright、bilibili-search）
- 当父进程接收到SIGHUP时，所有子进程也会接收到该信号
- 如果子进程没有适当的信号处理，就会导致panic

## 修复方案

### 1. 添加信号处理器 ✅

**文件**: `src-tauri/src/lib.rs`

**新增依赖**:
```toml
# 信号处理
signal-hook = "0.3"
signal-hook-tokio = { version = "0.3", features = ["futures-v0_3"] }
futures = "0.3"

[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

**实现的信号处理**:
- `SIGHUP`: 优雅关闭MCP连接，保存状态
- `SIGTERM`/`SIGINT`: 完整的应用关闭流程

### 2. 子进程隔离 ✅

**文件**: `src-tauri/src/tools/client.rs`

**核心改进**:
```rust
// 配置子进程以避免信号传播问题
let transport = TokioChildProcess::new(cmd.configure(|child_cmd| {
    #[cfg(unix)]
    {
        use std::process::Stdio;
        child_cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
            
        // 在Unix系统上设置进程组
        unsafe {
            child_cmd.pre_exec(|| {
                // 创建新的会话，使子进程成为会话领导者
                libc::setsid();
                Ok(())
            });
        }
    }
}))?;
```

**关键技术点**:
- 使用`setsid()`创建新会话
- 子进程成为新会话的领导者
- 避免从父进程继承信号

### 3. 优雅关闭机制 ✅

**改进的关闭流程**:
- 并行关闭所有MCP会话
- 设置10秒超时避免无限等待
- 保存状态信息
- 强制清理超时的连接

### 4. 健康检查改进 ✅

**自动恢复机制**:
- 检测到连接异常时自动重连
- 智能重试策略（指数退避）
- 详细的错误日志和诊断信息

## 技术实现细节

### 信号处理流程
```rust
match signal {
    SIGHUP => {
        tracing::warn!("Received SIGHUP signal, performing graceful shutdown of MCP connections");
        
        // 保存MCP服务器状态
        let is_running = mcp_service.is_server_running().await;
        mcp_service.save_server_state("builtin_security_tools", is_running).await;
        
        // 优雅关闭所有MCP连接
        mcp_client_manager.shutdown_all().await;
    }
}
```

### 子进程会话隔离
```rust
unsafe {
    child_cmd.pre_exec(|| {
        // 创建新的会话，使子进程成为会话领导者
        libc::setsid();
        Ok(())
    });
}
```

### 并发关闭机制
```rust
// 并行关闭所有会话，但设置超时
let shutdown_tasks = futures::future::join_all(shutdown_futures);
let timeout_duration = Duration::from_secs(10);

match tokio::time::timeout(timeout_duration, shutdown_tasks).await {
    Ok(_) => info!("All MCP sessions shut down gracefully"),
    Err(_) => warn!("MCP session shutdown timed out, forcing cleanup"),
}
```

## 预期效果

### 1. 防止SIGHUP panic
- 应用程序能正确处理SIGHUP信号
- 子进程不会因父进程信号而崩溃
- 网络断开或终端关闭不会导致应用panic

### 2. 提高稳定性
- 优雅的错误恢复机制
- 自动重连失败的MCP服务器
- 详细的错误日志便于调试

### 3. 更好的资源管理
- 确保子进程正确清理
- 避免僵尸进程
- 内存和文件描述符正确释放

## 测试建议

### 1. 信号测试
```bash
# 测试SIGHUP处理
kill -HUP <pid>

# 测试终端断开
nohup ./app &
# 然后断开SSH连接
```

### 2. 子进程隔离测试
```bash
# 检查进程组
ps -o pid,ppid,pgid,sid,comm

# 确认子进程有独立的会话ID
```

### 3. 压力测试
- 反复启动和关闭MCP连接
- 网络不稳定环境下的表现
- 长时间运行的稳定性

## 兼容性说明

- **Unix系统**: 完整的信号处理和进程管理
- **Windows系统**: 基本的进程管理（不支持信号）
- **macOS**: 完整支持所有特性
- **Linux**: 完整支持所有特性

## 日志改进

现在SIGHUP相关事件会产生详细日志：
```
WARN: Received SIGHUP signal, performing graceful shutdown of MCP connections
INFO: Successfully shutdown all MCP connections on SIGHUP
INFO: MCP server state saved on SIGHUP
```

这些修复应该完全解决SIGHUP信号导致的panic问题，并提供更强的系统稳定性。
