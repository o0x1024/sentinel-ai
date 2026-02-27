# 交互式终端会话管理修复

## 问题描述

执行一次 `interactive_shell` 工具调用会启动**两个 Docker 容器**，导致资源浪费和会话混乱。

## 根本原因

**时序竞争条件**：

1. **前端主动创建会话**：用户打开 AgentView 时，`InteractiveTerminal.vue` 组件挂载并立即调用 `connect()`，创建新的 WebSocket 连接和会话（**容器1**）。
2. **后端工具创建会话**：LLM 调用 `interactive_shell` 工具，工具检查 `TERMINAL_MANAGER.list_sessions()`，此时前端会话可能还未完全注册，导致工具认为没有现有会话，于是创建新会话（**容器2**）。
3. **会话隔离**：两个会话使用不同的容器，互不相通，导致 LLM 执行的命令看不到，终端面板也是空的。

## 解决方案

### 1. 前端改为被动连接模式

**修改文件**: `src/components/Tools/InteractiveTerminal.vue`

**关键改动**:
- **不再主动创建会话**：`onMounted` 时不立即 `connect()`，而是检查是否已有 `currentSessionId`。
- **监听会话创建**：使用 `watch` 监听 `currentSessionId` 的变化，当后端创建会话并设置 ID 后，前端才连接。
- **防止误创建**：`connect()` 中，如果没有 `session_id`，不再发送配置创建新会话，而是报错。

```typescript
onMounted(async () => {
  initTerminal()
  
  unregisterWriteCallback = terminalComposable.onTerminalWrite((content: string) => {
    if (terminal.value) {
      terminal.value.write(content)
    }
  })

  // Check if there's already a session ID
  if (terminalComposable.currentSessionId.value) {
    console.log('[Terminal] Connecting to existing session')
    await connect()
  } else {
    console.log('[Terminal] Waiting for session to be created by backend...')
    // Watch for session ID changes
    const stopWatch = watch(
      () => terminalComposable.currentSessionId.value,
      async (newSessionId) => {
        if (newSessionId && !isConnected.value) {
          console.log('[Terminal] Session created, connecting:', newSessionId)
          await connect()
          stopWatch()
        }
      }
    )
  }
})
```

### 2. 后端优先使用全局会话

**修改文件**: `src-tauri/sentinel-tools/src/tool_server.rs`

**已实现逻辑**:
```rust
// 1. Try to find an existing active session
let sessions = TERMINAL_MANAGER.list_sessions().await;
let active_session = if !sessions.is_empty() {
    TERMINAL_MANAGER.get_session(&sessions[0].id).await
} else {
    None
};

if let Some(session_lock) = active_session {
    // Reuse existing session
    let id = { session.id.clone() };
    // Add new subscriber for LLM output
    session.add_subscriber(tx).await;
} else {
    // Create new persistent session
    TERMINAL_MANAGER.create_session(config).await?
}
```

### 3. 会话持久化

**修改文件**: `src-tauri/sentinel-tools/src/terminal/server.rs`

**移除了 WebSocket 断开时停止会话的逻辑**:
```rust
// Cleanup
output_task.abort();
// Do not stop session here, let the manager handle it (persistent sessions)
info!("WebSocket connection ended for session: {}", session_id);
```

会话现在由 `TerminalSessionManager` 的清理任务管理，只有在空闲超过 30 分钟后才会被清理。

### 4. 历史记录回放

**修改文件**: `src-tauri/sentinel-tools/src/terminal/session.rs`

**添加输出历史缓存**:
```rust
pub struct TerminalSession {
    // ...
    output_history: Arc<RwLock<Vec<Vec<u8>>>>,
}

pub async fn add_subscriber(&self, tx: mpsc::UnboundedSender<Vec<u8>>) {
    // Send history to new subscriber
    let history = self.output_history.read().await;
    for data in history.iter() {
        let _ = tx.send(data.clone());
    }
    
    self.output_txs.write().await.push(tx);
}
```

当前端连接到现有会话时，会立即接收最近 1000 条输出记录，确保能看到之前的命令和结果。

### 5. 移除错误的命令提示

**修改文件**: `src-tauri/sentinel-tools/src/tool_server.rs`

**移除了手动写入 `$ command` 的代码**，因为：
1. 在非 TTY 模式下，`$` 会被当作命令执行，导致 `bash: line 1: $: command not found`
2. 历史记录机制已经能够保留所有输出，无需手动添加

## 修复后的流程

### 场景 1: 用户打开终端面板，然后 LLM 执行命令

1. 用户打开 AgentView → `InteractiveTerminal` 组件挂载
2. 组件初始化终端 UI，注册回调，但**不创建会话**（因为没有 `sessionId`）
3. 组件显示 "Waiting for session to be created by backend..."
4. LLM 调用 `interactive_shell` 工具
5. 工具检查全局管理器，没有现有会话，创建新会话（**唯一容器**）
6. 工具返回结果，包含 `session_id`
7. `useAgentEvents` 解析结果，调用 `terminal.openTerminal(session_id)` 设置 ID
8. `watch` 检测到 `sessionId` 变化，触发 `connect()`
9. `connect()` 发送 `session:ID` 重连到现有会话
10. 前端接收历史输出，显示之前执行的命令和结果

### 场景 2: LLM 先执行命令，然后用户打开终端面板

1. LLM 调用 `interactive_shell` 工具
2. 工具创建新会话（**唯一容器**），执行命令
3. 工具收集输出并返回给 LLM
4. `useAgentEvents` 设置 `sessionId`，打开终端面板
5. `InteractiveTerminal` 组件挂载
6. `onMounted` 检测到已有 `sessionId`，立即 `connect()`
7. `connect()` 发送 `session:ID` 重连
8. 前端接收历史记录，显示之前的所有命令和输出

### 场景 3: 多次执行命令（共享会话）

1. 第一次 `interactive_shell` 调用：创建会话
2. 第二次 `interactive_shell` 调用：
   - 检查 `TERMINAL_MANAGER.list_sessions()` → 找到现有会话
   - 重用现有会话，添加新的输出订阅者
   - 在同一个容器中执行新命令
3. 前端始终连接到同一个会话，实时看到所有命令和输出

## 优化和注意事项

### 移除 TTY 支持

**原问题**: 使用 `docker exec -it` 时报错 "the input device is not a TTY"

**原因**: 
- `-t` 参数要求分配伪终端，但 Rust 的 `Command::spawn()` 使用管道重定向 I/O
- Docker 检测到不是真正的 TTY，拒绝分配

**解决**: 只使用 `-i` (interactive)，不使用 `-t` (tty)

**影响**: 
- ❌ 失去了彩色输出、回显、PS1 提示符等 TTY 特性
- ✅ 但命令可以正常执行，输出可以正常捕获
- ✅ 通过历史记录机制，前端仍然能看到完整的命令和输出

### 会话清理策略

- **空闲超时**: 30 分钟无活动后自动清理
- **清理间隔**: 每 60 秒检查一次
- **WebSocket 断开**: 不再清理会话，允许重连
- **容器清理**: 会话停止时，Docker 容器会通过 `docker rm -f` 自动删除

### 资源限制

目前没有限制并发会话数量，建议在生产环境中添加：
```rust
const MAX_CONCURRENT_SESSIONS: usize = 10;
```

## 测试验证

### 测试 1: 单会话验证

```bash
# 执行两次 whoami，检查 Docker 容器数量
docker ps | grep sentinel-sandbox | wc -l
# 预期: 1 个容器
```

### 测试 2: 历史记录验证

1. LLM 执行 `whoami`
2. 关闭终端面板
3. 重新打开终端面板
4. 预期: 能看到之前的 `whoami` 命令和 `sandbox` 输出

### 测试 3: 会话持久性验证

1. LLM 执行 `cd /tmp && pwd`
2. LLM 再执行 `pwd`
3. 预期: 第二次 `pwd` 仍然显示 `/tmp`（同一个会话/容器）

## 相关文件

- `src/components/Tools/InteractiveTerminal.vue` - 前端终端组件
- `src/composables/useTerminal.ts` - 终端状态管理
- `src/composables/useAgentEvents.ts` - Agent 事件监听
- `src-tauri/sentinel-tools/src/terminal/session.rs` - 会话管理
- `src-tauri/sentinel-tools/src/terminal/server.rs` - WebSocket 服务器
- `src-tauri/sentinel-tools/src/terminal/manager.rs` - 会话管理器
- `src-tauri/sentinel-tools/src/terminal/mod.rs` - 全局会话管理器
- `src-tauri/sentinel-tools/src/tool_server.rs` - 工具注册和执行

## 未来改进

1. **支持命名会话**: 允许创建多个命名会话，用于不同的任务
2. **会话列表管理**: 前端显示所有活跃会话，允许切换
3. **真正的 TTY 支持**: 使用 `pty` crate 代替 `docker exec`，提供完整的 TTY 体验
4. **会话导出**: 导出会话历史记录为文本文件
5. **会话配置持久化**: 保存会话配置到数据库，重启后恢复
