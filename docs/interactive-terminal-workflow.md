# 交互式终端工作流程详解

## 📋 目录
- [核心架构](#核心架构)
- [工作流程](#工作流程)
- [关键组件](#关键组件)
- [数据流向](#数据流向)
- [会话管理](#会话管理)
- [问题诊断](#问题诊断)

---

## 🏗️ 核心架构

```
┌─────────────────────────────────────────────────────────────┐
│                        前端层                                │
├─────────────────────────────────────────────────────────────┤
│  AgentView.vue                                              │
│    ├─ InteractiveTerminal.vue (xterm.js)                   │
│    └─ useAgentEvents.ts (监听 LLM 工具调用)                 │
├─────────────────────────────────────────────────────────────┤
│  useTerminal.ts (全局状态管理)                              │
│    ├─ isTerminalActive: boolean                             │
│    ├─ currentSessionId: string | null                       │
│    └─ terminalWriteCallbacks: Set<Function>                 │
└─────────────────────────────────────────────────────────────┘
                            ↕ WebSocket
┌─────────────────────────────────────────────────────────────┐
│                        后端层                                │
├─────────────────────────────────────────────────────────────┤
│  TerminalServer (WebSocket Server)                          │
│    ├─ 监听 ws://127.0.0.1:3777                             │
│    ├─ 处理前端连接请求                                      │
│    └─ 转发输入/输出                                         │
├─────────────────────────────────────────────────────────────┤
│  TERMINAL_MANAGER (全局单例)                                │
│    ├─ sessions: HashMap<SessionId, TerminalSession>         │
│    ├─ create_session() → (session_id, output_rx)           │
│    ├─ get_session(id) → Option<Arc<RwLock<Session>>>       │
│    └─ write_to_session(id, data)                           │
├─────────────────────────────────────────────────────────────┤
│  TerminalSession (会话实例)                                 │
│    ├─ id: String                                            │
│    ├─ state: SessionState                                   │
│    ├─ stdin_tx: Vec<UnboundedSender<Vec<u8>>> (多订阅者)   │
│    ├─ output_history: Vec<Vec<u8>> (输出历史)              │
│    └─ Docker/Host 进程                                      │
├─────────────────────────────────────────────────────────────┤
│  interactive_shell Tool (ToolServer)                        │
│    ├─ 检查现有会话                                          │
│    ├─ 创建新会话（如果需要）                                │
│    ├─ 执行 initial_command                                  │
│    └─ 返回 session_id + output                             │
└─────────────────────────────────────────────────────────────┘
                            ↕
                    Docker Container
                (sentinel-sandbox:latest)
```

---

## 🔄 工作流程

### 场景 A: 用户手动打开终端面板

```
1️⃣ 用户点击 "终端" 按钮
   └─ AgentView.vue: terminal.openTerminal()

2️⃣ useTerminal.ts 更新状态
   ├─ isTerminalActive = true
   └─ currentSessionId = null (初始为空)

3️⃣ InteractiveTerminal.vue 挂载并初始化
   ├─ onMounted()
   │   ├─ initTerminal() → 初始化 xterm.js
   │   ├─ 注册 onTerminalWrite 回调
   │   └─ connect()
   │
   └─ connect() 流程:
       ├─ 检查终端服务器状态 (TerminalAPI.getStatus())
       ├─ 如果未运行 → 启动服务器 (TerminalAPI.startServer())
       ├─ 获取 WebSocket URL (ws://127.0.0.1:3777)
       └─ 创建 WebSocket 连接

4️⃣ WebSocket 连接建立
   ├─ ws.onopen 触发
   │   ├─ 检查 terminalComposable.currentSessionId
   │   │   ├─ 如果有 sessionId → 发送 "session:ID" (重连)
   │   │   └─ 如果没有 → 发送默认配置 JSON (创建新会话)
   │   │
   │   └─ 后端处理:
   │       ├─ TerminalServer.handle_connection()
   │       ├─ 收到配置 → TERMINAL_MANAGER.create_session()
   │       │   └─ 启动 Docker 容器 (docker exec -i bash)
   │       └─ 发送 "session:SESSION_ID" 返回前端
   │
   └─ ws.onmessage 收到 "session:SESSION_ID"
       ├─ sessionId.value = SESSION_ID
       ├─ terminalComposable.setSessionId(SESSION_ID) ✅ 同步到全局
       └─ 显示 "✓ Connected!"

5️⃣ 用户在终端输入
   └─ terminal.onData(data) → ws.send(data) → 后端 stdin → Docker 进程
```

---

### 场景 B: LLM 调用 interactive_shell 工具

```
1️⃣ LLM 决定使用 interactive_shell 工具
   └─ Executor 调用工具: { initial_command: "whoami" }

2️⃣ ToolServer 执行 interactive_shell
   ├─ 检查是否有现有会话:
   │   ├─ TERMINAL_MANAGER.list_sessions()
   │   │   ├─ 如果有 → 复用现有会话 ✅
   │   │   └─ 如果没有 → 创建新会话
   │   │
   │   └─ 创建新订阅者 (tx, rx) 用于捕获输出
   │       └─ session.add_subscriber(tx)
   │
   ├─ 执行 initial_command:
   │   ├─ TERMINAL_MANAGER.write_to_session(session_id, "whoami\n")
   │   ├─ 等待输出 (timeout 10s)
   │   └─ 收集输出给 LLM
   │
   └─ 返回结果:
       {
         "success": true,
         "session_id": "xxx",
         "output": "sandbox\n",
         "note": "Output is visible in the terminal panel"
       }

3️⃣ 前端监听到 tool_result 事件
   ├─ useAgentEvents.ts
   │   ├─ agent:tool_result → 检测 tool_name === 'interactive_shell'
   │   ├─ 解析 JSON 结果 → 获取 session_id
   │   └─ terminal.openTerminal(session_id) ✅
   │
   └─ useTerminal.ts 更新状态:
       ├─ isTerminalActive = true
       ├─ currentSessionId = session_id ✅

4️⃣ InteractiveTerminal.vue 连接到现有会话
   ├─ watch(currentSessionId) 触发
   ├─ connect()
   │   └─ ws.onopen → 发送 "session:SESSION_ID" (重连)
   │
   └─ 后端处理:
       ├─ TerminalServer 识别 "session:ID"
       ├─ TERMINAL_MANAGER.get_session(id) → 找到现有会话
       ├─ 创建新订阅者 → session.add_subscriber(tx)
       ├─ 发送历史输出 (output_history)
       └─ 发送 "session:ID" 确认

5️⃣ 前端显示历史输出
   └─ ws.onmessage → terminal.write(output_history) → 显示 "whoami" 输出
```

---

## 🧩 关键组件

### 1. `useTerminal.ts` (全局状态管理)

**职责**:
- 管理终端面板的打开/关闭状态
- 存储当前活跃的 `session_id`
- 提供事件总线用于组件间通信

**关键 API**:
```typescript
// 打开终端并可选设置 session_id
openTerminal(sessionId?: string)

// 设置当前 session_id (同步前后端状态)
setSessionId(sessionId: string)

// 注册写入回调（用于接收消息）
onTerminalWrite(callback: (content: string) => void): () => void
```

---

### 2. `InteractiveTerminal.vue` (终端 UI)

**职责**:
- 渲染 xterm.js 终端界面
- 管理 WebSocket 连接
- 处理用户输入/输出

**关键流程**:
```javascript
onMounted() {
  // 1. 初始化 xterm.js UI
  initTerminal()
  
  // 2. 注册全局写入回调
  terminalComposable.onTerminalWrite((content) => {
    terminal.write(content)
  })
  
  // 3. 尝试连接
  connect()  // 根据是否有 sessionId 决定创建/重连
  
  // 4. 监听 sessionId 变化
  watch(currentSessionId, async (newId, oldId) => {
    if (newId && newId !== oldId && !isConnected) {
      await connect()  // 重连到新会话
    }
  })
}

connect() {
  ws.onopen = () => {
    if (currentSessionId.value) {
      // 重连到现有会话
      ws.send(`session:${currentSessionId.value}`)
    } else {
      // 创建新会话
      ws.send(JSON.stringify(config))
    }
  }
  
  ws.onmessage = (event) => {
    if (event.data.startsWith('session:')) {
      const sessionId = event.data.substring(8)
      sessionId.value = sessionId
      terminalComposable.setSessionId(sessionId)  // ✅ 同步到全局
    } else {
      terminal.write(event.data)  // 显示输出
    }
  }
}
```

---

### 3. `useAgentEvents.ts` (LLM 事件监听)

**职责**:
- 监听后端 Agent 工具调用事件
- 检测 `interactive_shell` 调用
- 自动打开终端面板并传递 `session_id`

**关键代码**:
```typescript
// 监听工具调用开始
listen('agent:tool_call_complete', (event) => {
  if (event.payload.tool_name === 'interactive_shell') {
    terminal.openTerminal()  // 立即打开面板
  }
})

// 监听工具结果
listen('agent:tool_result', (event) => {
  if (callInfo.tool_name === 'interactive_shell') {
    try {
      const parsed = JSON.parse(resultContent)
      terminal.openTerminal(parsed.session_id)  // ✅ 传递 session_id
    } catch (e) {
      terminal.openTerminal()
    }
  }
})
```

---

### 4. `TerminalServer` (后端 WebSocket 服务器)

**职责**:
- 接受前端 WebSocket 连接
- 处理会话创建/重连请求
- 转发输入/输出数据

**关键流程**:
```rust
async fn handle_connection(&self, stream: TcpStream) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // 接收初始消息
    let init_msg = ws_receiver.next().await?;
    
    let (session_id, output_rx) = match init_msg {
        // 场景 1: 重连到现有会话
        Message::Text(text) if text.starts_with("session:") => {
            let id = text.strip_prefix("session:").unwrap();
            let session = self.manager.get_session(id).await?;
            
            // 创建新订阅者
            let (tx, rx) = mpsc::unbounded_channel();
            session.read().await.add_subscriber(tx).await;
            
            (id, rx)
        }
        
        // 场景 2: 创建新会话
        Message::Text(json) => {
            let config: TerminalSessionConfig = serde_json::from_str(&json)?;
            let (id, rx) = self.manager.create_session(config).await?;
            
            // 发送 session_id 给前端
            ws_sender.send(Message::Text(format!("session:{}", id))).await?;
            
            (id, rx)
        }
    };
    
    // 双向转发
    loop {
        select! {
            // 前端输入 → 会话 stdin
            Some(msg) = ws_receiver.next() => {
                self.manager.write_to_session(&session_id, msg).await;
            }
            
            // 会话输出 → 前端
            Some(data) = output_rx.recv() => {
                ws_sender.send(Message::Binary(data)).await;
            }
        }
    }
}
```

---

### 5. `TERMINAL_MANAGER` (全局会话管理器)

**职责**:
- 维护所有活跃的终端会话
- 提供会话创建/查询/写入接口
- 确保会话唯一性和持久性

**关键 API**:
```rust
// 创建新会话
async fn create_session(config: TerminalSessionConfig) 
    -> Result<(String, mpsc::UnboundedReceiver<Vec<u8>>)>

// 获取现有会话
async fn get_session(id: &str) 
    -> Option<Arc<RwLock<TerminalSession>>>

// 写入数据到会话
async fn write_to_session(id: &str, data: Vec<u8>) 
    -> Result<()>

// 列出所有会话
async fn list_sessions() -> Vec<SessionInfo>
```

---

### 6. `TerminalSession` (会话实例)

**职责**:
- 管理单个终端会话
- 维护 Docker 进程或本地进程
- 支持多订阅者（LLM + 前端）
- 缓存输出历史

**关键特性**:
```rust
pub struct TerminalSession {
    pub id: String,
    pub state: SessionState,
    
    // 多订阅者支持 (广播模式)
    stdin_tx: Arc<RwLock<Vec<mpsc::UnboundedSender<Vec<u8>>>>>,
    
    // 输出历史 (用于重连时回放)
    output_history: Arc<RwLock<Vec<Vec<u8>>>>,
    
    // Docker 进程或本地进程
    child_process: Arc<RwLock<Option<tokio::process::Child>>>,
}

// 添加新订阅者
pub async fn add_subscriber(&self, tx: mpsc::UnboundedSender<Vec<u8>>) {
    // 1. 发送历史输出
    for chunk in self.output_history.read().await.iter() {
        let _ = tx.send(chunk.clone());
    }
    
    // 2. 添加到订阅者列表
    self.stdin_tx.write().await.push(tx);
}

// 广播输出到所有订阅者
async fn broadcast_output(&self, data: &[u8]) {
    for tx in self.stdin_tx.read().await.iter() {
        let _ = tx.send(data.to_vec());
    }
}
```

---

### 7. `interactive_shell` Tool (工具执行器)

**职责**:
- 响应 LLM 的工具调用
- 创建或复用终端会话
- 执行 `initial_command` 并捕获输出
- 返回 `session_id` 给 LLM

**执行流程**:
```rust
executor(|args| async move {
    // 1. 解析参数
    let use_docker = args.get("use_docker").unwrap_or(true);
    let initial_command = args.get("initial_command");
    
    // 2. 查找或创建会话
    let sessions = TERMINAL_MANAGER.list_sessions().await;
    let (session_id, mut output_rx) = if !sessions.is_empty() {
        // 复用现有会话
        let id = sessions[0].id.clone();
        let (tx, rx) = mpsc::unbounded_channel();
        session.add_subscriber(tx).await;  // 订阅输出
        (id, rx)
    } else {
        // 创建新会话
        TERMINAL_MANAGER.create_session(config).await?
    };
    
    // 3. 执行命令
    if let Some(cmd) = initial_command {
        TERMINAL_MANAGER.write_to_session(&session_id, format!("{}\n", cmd)).await?;
        
        // 等待输出 (timeout 10s)
        let mut output = Vec::new();
        while timeout(500ms, output_rx.recv()).await {
            output.extend(data);
        }
    }
    
    // 4. 返回结果
    Ok(json!({
        "success": true,
        "session_id": session_id,
        "output": output_str,
        "note": "Output is visible in the terminal panel"
    }))
})
```

---

## 🔀 数据流向

### 用户输入流 (前端 → Docker)

```
用户在 xterm.js 输入 "ls -la" + Enter
    ↓
terminal.onData("ls -la\r")
    ↓
ws.send("ls -la\r")  ← WebSocket
    ↓
TerminalServer 接收
    ↓
TERMINAL_MANAGER.write_to_session(session_id, "ls -la\r")
    ↓
TerminalSession.stdin_tx.send("ls -la\r")
    ↓
Docker 进程 stdin (bash)
```

---

### 输出流 (Docker → 前端 + LLM)

```
Docker 进程 stdout 输出 "total 42\n-rw-r--r-- ..."
    ↓
TerminalSession 读取输出
    ↓
┌─────────────────────────────────────┐
│  broadcast_output(data)             │
│   ├─ 添加到 output_history          │  ← 缓存历史
│   └─ 发送给所有订阅者:              │
│       ├─ WebSocket 订阅者 (前端)    │  → ws.send(data) → xterm.js
│       └─ LLM 订阅者 (如果有)        │  → output_rx.recv() → 工具结果
└─────────────────────────────────────┘
```

---

### 会话重连流 (带历史回放)

```
前端发送: "session:abc123"
    ↓
TerminalServer 识别重连请求
    ↓
TERMINAL_MANAGER.get_session("abc123")
    ↓
session.add_subscriber(new_tx)
    ├─ 1. 发送 output_history (历史)
    └─ 2. 添加到订阅者列表
    ↓
前端 ws.onmessage 依次收到:
    ├─ "session:abc123" (确认)
    ├─ <历史输出 1>
    ├─ <历史输出 2>
    └─ <后续实时输出>
```

---

## 🔧 会话管理

### 会话生命周期

```
┌──────────────┐
│   创建会话    │ ← TERMINAL_MANAGER.create_session()
│  (Running)   │   - 启动 Docker/Host 进程
└──────┬───────┘   - 生成 session_id
       │           - 创建 stdin_tx 通道
       │
       ├─────────► 【活跃状态】
       │           - 接受用户输入
       │           - 转发进程输出
       │           - 支持多订阅者
       │
       ├─────────► 【断开连接】
       │           - WebSocket 关闭
       │           - 订阅者被移除
       │           - ⚠️ 会话保持运行
       │           - ⚠️ 进程继续执行
       │
       ├─────────► 【重新连接】
       │           - 前端发送 "session:ID"
       │           - 回放 output_history
       │           - 恢复实时通信
       │
       └─────────► 【停止会话】
                   - TERMINAL_MANAGER.stop_session()
                   - 终止 Docker/Host 进程
                   - 清理资源
```

---

### 会话复用逻辑

**interactive_shell 工具**:
```rust
// 优先复用现有会话
let sessions = TERMINAL_MANAGER.list_sessions().await;
if !sessions.is_empty() {
    // ✅ 复用第一个会话 (单一持久会话)
    let session = sessions[0];
    session.add_subscriber(tx).await;
} else {
    // 创建新会话
    TERMINAL_MANAGER.create_session(config).await;
}
```

**前端连接**:
```javascript
// 如果有 sessionId，重连到现有会话
if (terminalComposable.currentSessionId.value) {
    ws.send(`session:${terminalComposable.currentSessionId.value}`)
} else {
    // 创建新会话
    ws.send(JSON.stringify(config))
}
```

---

## 🐛 问题诊断

### ❌ 问题 1: 执行命令时启动了两个容器

**原因**:
- 前端 `connect()` 时创建了一个会话
- 后端工具执行时又创建了一个会话
- 前后端的 `sessionId` 不同步

**解决方案**:
1. 前端收到 `session:ID` 时，立即同步到全局状态:
   ```javascript
   terminalComposable.setSessionId(newSessionId)  // ✅
   ```

2. 后端工具优先检查现有会话:
   ```rust
   let sessions = TERMINAL_MANAGER.list_sessions().await;
   if !sessions.is_empty() {
       // 复用现有会话 ✅
   }
   ```

---

### ❌ 问题 2: 终端显示 "Disconnected"，无法输入

**可能原因**:
1. **WebSocket 未连接**
   - 检查终端服务器是否运行: `TerminalAPI.getStatus()`
   - 检查 WebSocket URL: `ws://127.0.0.1:3777`

2. **Session ID 不存在**
   - `terminalComposable.currentSessionId.value` 为 `null`
   - 后端会话已被清理

3. **前后端 session_id 不同步**
   - 前端有 `sessionId`，但全局状态未更新
   - 检查是否调用了 `terminalComposable.setSessionId()`

**调试步骤**:
```javascript
// 前端
console.log('[Terminal] Current session ID:', terminalComposable.currentSessionId.value)
console.log('[Terminal] WebSocket state:', ws.value?.readyState)

// 后端
// 检查会话列表
let sessions = TERMINAL_MANAGER.list_sessions().await;
println!("Active sessions: {:?}", sessions);
```

---

### ❌ 问题 3: LLM 执行命令，但终端没有显示

**可能原因**:
1. **订阅者未正确添加**
   - LLM 调用 `interactive_shell` 时，未调用 `add_subscriber()`

2. **输出历史未缓存**
   - `broadcast_output()` 未调用 `add_history()`

3. **前端重连时机问题**
   - `useAgentEvents` 未正确解析 `session_id`
   - `terminal.openTerminal(session_id)` 未调用

**解决方案**:
1. 确保工具执行时创建订阅者:
   ```rust
   let (tx, rx) = mpsc::unbounded_channel();
   session.add_subscriber(tx).await;  // ✅
   ```

2. 确保输出被缓存:
   ```rust
   async fn broadcast_output(&self, data: &[u8]) {
       self.add_history(data).await;  // ✅ 添加到历史
       for tx in self.stdin_tx.read().await.iter() {
           let _ = tx.send(data.to_vec());
       }
   }
   ```

3. 确保前端解析 `session_id`:
   ```typescript
   const parsed = JSON.parse(resultContent)
   terminal.openTerminal(parsed.session_id)  // ✅
   ```

---

### ✅ 健康检查清单

```bash
# 1. 检查终端服务器状态
curl http://localhost:3777/status

# 2. 检查 Docker 容器
docker ps | grep sentinel-sandbox

# 3. 检查会话数量
# (通过 Tauri 命令或日志)
grep "Active sessions" ~/Library/Logs/sentinel-ai/*.log

# 4. 前端控制台
# 查看 WebSocket 连接状态
# 查看 session_id 同步日志
[Terminal] Session established and synced to global state: xxx
```

---

## 📝 总结

### 关键设计原则

1. **单一会话持久性**
   - 优先复用现有会话，避免创建多个容器
   - 通过全局 `TERMINAL_MANAGER` 确保会话唯一性

2. **前后端状态同步**
   - 前端收到 `session:ID` → 立即同步到 `useTerminal`
   - 后端工具返回 `session_id` → 前端解析并传递给 `openTerminal()`

3. **多订阅者支持**
   - 一个会话可以有多个订阅者 (LLM + 多个 WebSocket)
   - 使用广播模式确保所有订阅者收到输出

4. **输出历史回放**
   - 缓存最近的输出历史
   - 新订阅者加入时自动回放

5. **断线重连**
   - WebSocket 断开不影响会话
   - 重连时通过 `session:ID` 恢复

---

## 📖 相关文档

- [交互式终端集成指南](./interactive-terminal-integration.md)
- [Shell 工具 vs 交互式终端](./shell-vs-interactive-terminal.md)
- [交互式终端 API 文档](./interactive-terminal-api.md)
