# 交互式终端过早打开问题修复

## 🐛 问题描述

**症状**：
- LLM 调用 `interactive_shell` 工具
- 终端面板立即打开并显示 "Connected"
- 但没有显示命令执行结果
- 创建了错误的会话

**日志证据**：
```javascript
// 前端日志
[Agent] Detected interactive_shell call, opening terminal panel
[Terminal] Initial connection attempt, session ID: null  // ← 关键！
[Terminal] No session ID, creating new session with default config
[Terminal] ✓ Session established: 666c98ae  // ← 错误的会话
```

---

## 🔍 根本原因

### 问题时序

```
T1: LLM 决定调用 interactive_shell 工具
    ↓
T2: 发送 agent:tool_call 事件
    ↓
T3: useAgentEvents 监听到 tool_call 事件
    ├─ 检测到 tool_name === 'interactive_shell'
    └─ ❌ 立即调用 terminal.openTerminal() (没有参数)
    ↓
T4: InteractiveTerminal.vue 挂载
    ├─ currentSessionId.value = null
    └─ connect() → 创建新会话 "666c98ae"
    ↓
T5: 后端 ToolServer 执行 interactive_shell
    ├─ 检查现有会话 → 找到 "666c98ae"
    ├─ 复用该会话
    ├─ 执行命令 "whoami"
    ├─ 收集输出
    └─ 返回 { session_id: "666c98ae", output: "sandbox\n" }
    ↓
T6: 发送 agent:tool_result 事件
    ↓
T7: useAgentEvents 监听到 tool_result
    ├─ 解析 session_id: "666c98ae"
    └─ terminal.openTerminal("666c98ae")  // ← 但终端已经打开了！
```

**核心问题**：
1. **在 `tool_call` 事件中过早打开终端**，此时还没有 `session_id`
2. **前端创建了自己的会话**，然后后端工具复用了这个会话
3. **前端已经连接到该会话**，但没有订阅到命令执行时的输出
4. **历史输出没有被回放**，因为前端订阅在命令执行之前

---

## 🔧 修复方案

### 修复原则

**正确的时序应该是**：
```
1. LLM 调用 interactive_shell 工具
2. 后端创建会话并执行命令
3. 后端返回结果（包含 session_id）
4. 前端收到 tool_result 事件
5. 前端解析 session_id 并打开终端
6. 前端连接到现有会话
7. 后端回放历史输出
```

---

### 修复 1: 移除过早的终端打开

**位置**: `src/composables/useAgentEvents.ts`

**修改 1 - agent:tool_call 事件**:
```typescript
// ❌ 之前：立即打开终端
if (payload.tool_name === 'interactive_shell') {
  const terminal = useTerminal()
  terminal.openTerminal()  // ← 没有 session_id！
}

// ✅ 现在：只记录日志，等待结果
if (payload.tool_name === 'interactive_shell') {
  console.log('[Agent] Detected interactive_shell call, will open terminal when result arrives')
}
```

**修改 2 - agent:tool_call_complete 事件**:
```typescript
// ❌ 之前：立即打开终端
if (payload.tool_name === 'interactive_shell') {
  const terminal = useTerminal()
  terminal.openTerminal()  // ← 没有 session_id！
}

// ✅ 现在：只记录日志，等待结果
if (payload.tool_name === 'interactive_shell') {
  console.log('[Agent] Detected interactive_shell call (complete), will open terminal when result arrives')
}
```

---

### 修复 2: 增强 tool_result 处理

**位置**: `src/composables/useAgentEvents.ts` - `agent:tool_result` 事件

```typescript
// 如果是 interactive_shell 工具，自动打开终端面板
if (callInfo.tool_name === 'interactive_shell') {
  import('@/composables/useTerminal').then(({ useTerminal }) => {
    const terminal = useTerminal()
    try {
      const parsed = JSON.parse(resultContent)
      console.log('[Agent] interactive_shell result parsed, session_id:', parsed.session_id)
      
      if (parsed.session_id) {
        terminal.openTerminal(parsed.session_id)  // ✅ 带上 session_id
        console.log('[Agent] ✅ Terminal opened with session_id:', parsed.session_id)
      } else {
        console.warn('[Agent] ⚠️ No session_id in interactive_shell result')
        terminal.openTerminal()
      }
    } catch (e) {
      console.error('[Agent] ❌ Failed to parse interactive_shell result:', e)
      terminal.openTerminal()
    }
  })
}
```

---

## 📊 修复后的正确流程

### 时序图

```
用户: "执行一下 whoami"
    ↓
LLM 决策: 使用 interactive_shell { initial_command: "whoami" }
    ↓
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
后端工具执行
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ↓
1. 检查现有会话: list_sessions() → []
    ↓
2. 创建新会话:
   ├─ session_id: "abc123"
   ├─ 启动 Docker 容器
   └─ 启动 bash 进程
    ↓
3. 创建订阅者用于捕获输出:
   ├─ (tx, rx) = mpsc::unbounded_channel()
   └─ session.add_subscriber(tx)
    ↓
4. 执行命令:
   ├─ write_to_session("whoami\n")
   └─ 等待输出 (timeout 10s)
    ↓
5. 收集输出:
   ├─ output_rx.recv() → "sandbox\n"
   └─ output_history += "sandbox\n"  // ✅ 保存到历史
    ↓
6. 返回结果:
   {
     "success": true,
     "session_id": "abc123",
     "output": "sandbox\n",
     "note": "Output is visible in the terminal panel."
   }
    ↓
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
前端处理
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ↓
7. useAgentEvents 收到 tool_result:
   ├─ 解析 JSON → session_id: "abc123"
   └─ terminal.openTerminal("abc123")
    ↓
8. useTerminal 更新状态:
   ├─ isTerminalActive = true
   └─ currentSessionId = "abc123"
    ↓
9. InteractiveTerminal.vue 挂载:
   ├─ initTerminal() → 初始化 xterm.js
   └─ connect()
    ↓
10. WebSocket 连接:
    ├─ ws.onopen → 检查 currentSessionId
    ├─ currentSessionId.value = "abc123"  // ✅ 有值
    └─ 发送 "session:abc123" (重连)
    ↓
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
后端 WebSocket 处理
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ↓
11. TerminalServer 收到 "session:abc123":
    ├─ 找到现有会话: TERMINAL_MANAGER.get_session("abc123")
    ├─ 创建新订阅者: (tx, rx) = unbounded_channel()
    └─ session.add_subscriber(tx)
    ↓
12. add_subscriber() 回放历史:
    ├─ output_history.len() = 1
    ├─ 发送历史块 #0: "sandbox\n" (8 bytes)
    └─ 添加到订阅者列表
    ↓
13. WebSocket 转发任务:
    ├─ output_rx.recv() → "sandbox\n"
    └─ ws_sender.send(Message::Binary(b"sandbox\n"))
    ↓
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
前端显示
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ↓
14. InteractiveTerminal.vue 收到数据:
    ├─ ws.onmessage → event.data instanceof Blob
    ├─ 解码: "sandbox\n"
    └─ terminal.write("sandbox\n")
    ↓
15. xterm.js 显示:
    ✅ sandbox
       _
```

---

## ✅ 验证步骤

### 1. 编译前端

```bash
cd /Users/a1024/code/ai/sentinel-ai
npm run dev
```

### 2. 测试

1. 清除旧会话（如果有）
2. 发送消息："执行一下 whoami"
3. 观察日志

### 3. 期望的日志输出

**前端控制台**：
```javascript
[Agent] Detected interactive_shell call, will open terminal when result arrives  // ← 新日志
[Agent] interactive_shell result parsed, session_id: abc123  // ← 新日志
[Agent] ✅ Terminal opened with session_id: abc123  // ← 新日志
[Terminal] Initial connection attempt, session ID: abc123  // ← 关键！有 session_id
[Terminal] WebSocket connected
[Terminal] Connecting to existing session: abc123  // ← 重连而非创建
[Terminal] ✓ Session established and synced to global state: abc123
[Terminal] Received output, length: 8  // ← 收到历史输出
```

**后端日志**：
```
[INFO] [Terminal Session abc123] Created
[INFO] [Terminal Session abc123] Adding subscriber (LLM)
[INFO] [Terminal Session abc123] Executing: whoami
[INFO] [Terminal Session abc123] broadcast_output: 8 bytes
[INFO] [WS Session abc123] Reconnecting to existing session
[INFO] [Terminal Session abc123] Adding subscriber, history chunks: 1  // ← 关键！
[INFO] [Terminal Session abc123] Sending history chunk 0: 8 bytes
[INFO] [WS Session abc123] Forwarding chunk #1: 8 bytes  // ← 关键！
```

### 4. 期望的终端显示

```
Sentinel AI Interactive Terminal
Connecting to terminal server...
✓ Connected!

sandbox
sandbox@abc123:/workspace$ _
```

---

## 📝 关键改进

1. **延迟终端打开**：只在收到 `tool_result` 且有 `session_id` 时才打开
2. **避免创建错误会话**：前端不会在没有 `session_id` 时创建会话
3. **正确回放历史**：前端重连时会收到命令执行的历史输出
4. **单一会话**：整个流程只有一个会话 `abc123`

---

## 🎯 预期效果

**修复前**：
```
后端创建会话 A (没人连接)
前端创建会话 B (用户看到)
命令在会话 A 执行
用户在会话 B 看不到输出 ❌
```

**修复后**：
```
后端创建会话 A
命令在会话 A 执行
前端连接到会话 A
前端看到历史输出 ✅
```

---

## 📚 相关文档

- [交互式终端工作流程](./interactive-terminal-workflow.md)
- [终端显示问题诊断](./terminal-display-issue-diagnosis.md)
- [会话管理修复](./terminal-session-management-fix.md)
