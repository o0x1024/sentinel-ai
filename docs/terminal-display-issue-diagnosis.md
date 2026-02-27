# 交互式终端显示问题诊断与修复

## 🐛 问题描述

**症状**：
- LLM 执行 `interactive_shell { initial_command: "whoami" }` 成功
- 终端面板自动打开并显示 "Connected"
- **但没有显示命令 "whoami" 和其输出 "sandbox"**

**用户截图显示**：
```
左侧消息区:
  > interactive_shell
  ✓ 已完成
  执行结果为：sandbox

右侧终端面板:
  Sentinel AI Interactive Terminal
  Connecting to terminal server...
  ✓ Connected!
  [空白，没有任何命令输出]
```

---

## 🔍 问题分析

### 可能原因 1: 时序问题

**问题场景**：
```
时间线:
T1: LLM 调用 interactive_shell
T2: 后端创建会话 (session: 3c5d2f61)
T3: 后端执行 "whoami\n"
T4: 输出 "sandbox\n" 被添加到 output_history
T5: 工具返回 { session_id: "3c5d2f61", output: "sandbox\n" }
T6: 前端收到 tool_result 事件
T7: useAgentEvents 调用 terminal.openTerminal("3c5d2f61")
T8: InteractiveTerminal.vue onMounted
T9: connect() 被调用
T10: WebSocket 连接建立
T11: 发送 "session:3c5d2f61" (重连)
T12: 后端 add_subscriber() 应该发送 output_history
T13: ❌ 但前端没有收到历史数据
```

**潜在问题**：
1. **`onMounted` 和 `watch` 冲突**
   - `onMounted` 调用 `connect()`
   - `watch(currentSessionId)` 也可能触发 `connect()`
   - 可能导致双重连接或连接被中断

2. **历史数据在订阅者添加前丢失**
   - 工具执行时的订阅者接收了输出
   - 但该订阅者在工具返回后被销毁
   - WebSocket 订阅者在历史数据已清空后才加入

---

### 可能原因 2: 输出历史未正确保存

**代码检查**：

`session.rs`:
```rust
async fn broadcast_output(
    output_txs: Arc<RwLock<Vec<mpsc::UnboundedSender<Vec<u8>>>>>, 
    output_history: Arc<RwLock<Vec<Vec<u8>>>>,
    data: Vec<u8>
) {
    // ✅ 添加到历史 (保留最近 1000 个块)
    {
        let mut history = output_history.write().await;
        history.push(data.clone());
        if history.len() > 1000 {
            history.remove(0);
        }
    }

    // ✅ 广播到所有订阅者
    let mut txs = output_txs.write().await;
    txs.retain(|tx| {
        tx.send(data.clone()).is_ok()
    });
}
```

**问题可能性**：
- 输出是按行读取 (`read_until(b'\n')`)
- 如果 `whoami` 输出是 `"sandbox\n"`，应该被保存为一个块
- 但如果进程还输出了 prompt (如 `"$ "`)，可能被拆分

---

### 可能原因 3: WebSocket 连接时机问题

**当前流程**：
```javascript
// InteractiveTerminal.vue
onMounted(async () => {
  initTerminal()
  
  // 立即连接
  await connect()  // ← 可能此时 currentSessionId 还是 null
  
  // 监听 sessionId 变化
  watch(currentSessionId, async (newId, oldId) => {
    if (newId && newId !== oldId && !isConnected.value) {
      await connect()  // ← 又连接一次？
    }
  })
})
```

**问题**：
- 如果 `onMounted` 时 `currentSessionId` 已经设置（通过 `openTerminal(sessionId)`），第一次 `connect()` 应该会重连
- 但如果 `useAgentEvents` 设置 `sessionId` 的时机晚于 `onMounted`，就会先创建新会话，然后 `watch` 再触发重连
- 这可能导致连接到了错误的会话

---

## 🔧 修复方案

### 修复 1: 添加详细日志

**目的**: 确定数据流向和时序

**修改位置 1**: `session.rs` - `add_subscriber`
```rust
pub async fn add_subscriber(&self, tx: mpsc::UnboundedSender<Vec<u8>>) {
    let history = self.output_history.read().await;
    info!("[Terminal Session {}] Adding subscriber, history chunks: {}", 
        self.id, history.len());  // ← 记录历史块数量
    
    for (i, data) in history.iter().enumerate() {
        info!("[Terminal Session {}] Sending history chunk {}: {} bytes", 
            self.id, i, data.len());  // ← 记录每个块的大小
        if let Err(e) = tx.send(data.clone()) {
            error!("[Terminal Session {}] Failed to send history chunk {}: {}", 
                self.id, i, e);
        }
    }
    
    self.output_txs.write().await.push(tx);
    info!("[Terminal Session {}] Subscriber added, total subscribers: {}", 
        self.id, self.output_txs.read().await.len());
}
```

**修改位置 2**: `server.rs` - WebSocket 输出转发
```rust
let output_task = tokio::spawn(async move {
    info!("[WS Session {}] Output forwarding task started", session_id_clone);
    let mut chunk_count = 0;
    while let Some(data) = output_rx.recv().await {
        chunk_count += 1;
        info!("[WS Session {}] Forwarding chunk #{}: {} bytes", 
            session_id_clone, chunk_count, data.len());  // ← 记录每个转发的块
        if let Err(e) = ws_sender.send(Message::Binary(data)).await {
            error!("[WS Session {}] Failed to send output: {}", session_id_clone, e);
            break;
        }
    }
    info!("[WS Session {}] Output task ended, total chunks sent: {}", 
        session_id_clone, chunk_count);
});
```

**修改位置 3**: `InteractiveTerminal.vue` - WebSocket 消息接收
```javascript
ws.value.onmessage = (event) => {
  if (typeof event.data === 'string') {
    if (event.data.startsWith('session:')) {
      // ... session ID 处理
      console.log('[Terminal] ✓ Session established and synced to global state:', newSessionId)
    } else {
      // 普通输出
      console.log('[Terminal] Received output, length:', event.data.length)  // ← 新增日志
      terminal.value?.write(event.data)
    }
  } else if (event.data instanceof Blob) {
    event.data.arrayBuffer().then((buffer) => {
      const text = new TextDecoder().decode(buffer)
      console.log('[Terminal] Received binary output, length:', text.length)  // ← 新增日志
      terminal.value?.write(text)
    })
  }
}
```

---

### 修复 2: 确保前端接收 Binary 消息

**问题**: 后端发送的是 `Message::Binary(data)`，前端需要正确处理

**当前代码检查**:
```javascript
// InteractiveTerminal.vue
} else if (event.data instanceof Blob) {
  // Binary data
  event.data.arrayBuffer().then((buffer) => {
    const text = new TextDecoder().decode(buffer)
    terminal.value?.write(text)
  })
} else if (event.data instanceof ArrayBuffer) {
  const text = new TextDecoder().decode(event.data)
  terminal.value?.write(text)
}
```

**状态**: ✅ 已正确处理 `Blob` 和 `ArrayBuffer`

---

### 修复 3: 优化前端连接时序

**问题**: `onMounted` 和 `watch` 可能冲突

**建议修改** (`InteractiveTerminal.vue`):
```javascript
onMounted(async () => {
  // 1. 初始化 UI
  initTerminal()
  
  // 2. 注册写入回调
  unregisterWriteCallback = terminalComposable.onTerminalWrite((content) => {
    if (terminal.value) {
      terminal.value.write(content)
    }
  })

  // 3. 延迟一下，等待 useAgentEvents 设置 sessionId
  await new Promise(resolve => setTimeout(resolve, 100))
  
  // 4. 检查是否有 sessionId
  if (terminalComposable.currentSessionId.value) {
    console.log('[Terminal] Session ID available on mount:', terminalComposable.currentSessionId.value)
    await connect()
  } else {
    console.log('[Terminal] No session ID on mount, creating new session')
    await connect()
  }
  
  // 5. 监听 sessionId 变化（避免重复连接）
  watch(
    () => terminalComposable.currentSessionId.value,
    async (newSessionId, oldSessionId) => {
      console.log('[Terminal] Session ID changed:', oldSessionId, '→', newSessionId)
      
      // 只有在未连接 且 sessionId 有效 且 与旧值不同时才重连
      if (newSessionId && newSessionId !== oldSessionId && !isConnected.value) {
        console.log('[Terminal] Reconnecting due to session ID change')
        await disconnect()  // 先断开旧连接
        await connect()
      }
    }
  )
})
```

**关键改进**:
1. **添加 100ms 延迟**：给 `useAgentEvents` 时间设置 `sessionId`
2. **检查连接状态**：避免在已连接时重复连接
3. **先断开再连接**：确保清理旧连接

---

### 修复 4: 工具执行时确保输出被捕获

**问题**: 工具执行时的订阅者可能在输出到达前就被销毁

**当前代码** (`tool_server.rs`):
```rust
// 创建订阅者
let (tx, rx) = mpsc::unbounded_channel();
session.add_subscriber(tx).await;

// 执行命令
TERMINAL_MANAGER.write_to_session(&session_id, cmd_with_newline.into_bytes()).await?;

// 收集输出
let mut output = Vec::new();
let collect_timeout = Duration::from_secs(10);
let start = tokio::time::Instant::now();

while start.elapsed() < collect_timeout {
    match timeout(Duration::from_millis(500), output_rx.recv()).await {
        Ok(Some(data)) => {
            output.extend_from_slice(&data);
        }
        Ok(None) => break,
        Err(_) => {
            if !output.is_empty() {
                break;  // ← 有输出就停止
            }
        }
    }
}
```

**潜在问题**: 
- 如果命令输出非常快（< 500ms），第一次 `timeout` 成功，第二次超时就会 `break`
- 这是正常的，但需要确保输出已经被 `broadcast_output` 添加到 `output_history`

**验证**: 输出应该同时被发送给：
1. LLM 订阅者 (`rx`)
2. 添加到 `output_history`（通过 `broadcast_output`）

**代码检查**:
```rust
// session.rs - stdout 读取任务
tokio::spawn(async move {
    let mut reader = BufReader::new(stdout);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer).await {
            Ok(0) => break,
            Ok(_) => {
                Self::broadcast_output(
                    output_txs_clone.clone(), 
                    output_history_clone.clone(),  // ✅ 会添加到历史
                    buffer.clone()
                ).await;
            }
            Err(e) => {
                error!("Failed to read stdout: {}", e);
                break;
            }
        }
    }
});
```

**状态**: ✅ 输出应该被正确保存到历史

---

## 📊 调试步骤

### 步骤 1: 检查后端日志

**编译并运行**:
```bash
cd /Users/a1024/code/ai/sentinel-ai/src-tauri
cargo build
```

**查看日志**:
```bash
# 执行 interactive_shell 后，检查日志
grep -E "Terminal Session|WS Session|Adding subscriber|Forwarding chunk|history chunks" \
  ~/Library/Logs/sentinel-ai/*.log | tail -50
```

**期望输出**:
```
[INFO] [Terminal Session 3c5d2f61] Created
[INFO] [Terminal Session 3c5d2f61] Executing: whoami
[INFO] [Terminal Session 3c5d2f61] broadcast_output: 8 bytes  # "sandbox\n"
[INFO] [WS Session 3c5d2f61] Reconnecting to existing session
[INFO] [Terminal Session 3c5d2f61] Adding subscriber, history chunks: 1  # ← 关键！
[INFO] [Terminal Session 3c5d2f61] Sending history chunk 0: 8 bytes
[INFO] [WS Session 3c5d2f61] Output forwarding task started
[INFO] [WS Session 3c5d2f61] Forwarding chunk #1: 8 bytes  # ← 关键！
```

**如果看到 `history chunks: 0`**:
- 说明输出没有被保存到历史
- 检查 `broadcast_output` 是否被调用
- 检查 stdout 读取任务是否正常

**如果看到 `history chunks: 1` 但没有 `Forwarding chunk`**:
- 说明历史数据没有被发送到 WebSocket
- 检查 `add_subscriber` 中的 `tx.send()` 是否成功
- 检查 `output_rx.recv()` 是否接收到数据

---

### 步骤 2: 检查前端日志

**打开浏览器控制台** (DevTools):
```javascript
// 应该看到:
[Terminal] Initial connection attempt, session ID: 3c5d2f61
[Terminal] WebSocket connected
[Terminal] Connecting to existing session: 3c5d2f61
[Terminal] ✓ Session established and synced to global state: 3c5d2f61
[Terminal] Received binary output, length: 8  // ← 关键！如果没有这行说明没收到数据
```

**如果没有 "Received binary output"**:
- WebSocket 连接正常，但没收到数据
- 检查后端是否发送了数据

---

### 步骤 3: 检查 Docker 容器

```bash
# 查看运行中的容器
docker ps | grep sentinel-sandbox

# 应该只有一个容器（如果看到两个说明还有重复创建问题）

# 连接到容器检查
docker exec -it <CONTAINER_ID> bash
# 在容器内执行 whoami，确认用户是 sandbox
```

---

## ✅ 预期结果

**修复后，终端应该显示**:
```
Sentinel AI Interactive Terminal
Connecting to terminal server...
✓ Connected!

sandbox@3c5d2f61:/workspace$ whoami
sandbox
sandbox@3c5d2f61:/workspace$ _
```

**日志应该显示**:
```
后端:
  [INFO] [Terminal Session 3c5d2f61] Adding subscriber, history chunks: 2
  [INFO] [Terminal Session 3c5d2f61] Sending history chunk 0: 35 bytes  # prompt
  [INFO] [Terminal Session 3c5d2f61] Sending history chunk 1: 8 bytes   # whoami output
  [INFO] [WS Session 3c5d2f61] Forwarding chunk #1: 35 bytes
  [INFO] [WS Session 3c5d2f61] Forwarding chunk #2: 8 bytes

前端:
  [Terminal] Received binary output, length: 35
  [Terminal] Received binary output, length: 8
```

---

## 🎯 下一步行动

1. **立即测试**: 重新编译并运行，执行 `interactive_shell` 工具
2. **查看日志**: 收集上述的所有日志输出
3. **报告结果**: 如果问题仍存在，提供日志以进一步诊断

**如果修复成功**:
- 关闭此 issue
- 更新文档说明已解决

**如果问题仍存在**:
- 根据日志确定问题在哪个环节
- 考虑是否需要调整数据流架构

---

## 📝 相关文档

- [交互式终端工作流程](./interactive-terminal-workflow.md)
- [Shell 工具 vs 交互式终端](./shell-vs-interactive-terminal.md)
- [会话管理修复文档](./terminal-session-management-fix.md)
