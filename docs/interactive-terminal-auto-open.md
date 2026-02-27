# Interactive Terminal 自动打开功能

## 功能说明

当 Agent 调用 `interactive_shell` 工具时，终端面板会自动打开，无需手动操作。

## 实现原理

### 1. 事件监听

在 `useAgentEvents` composable 中监听工具调用事件：

```typescript
// 监听 agent:tool_call_complete 事件
const unlistenToolCallComplete = await listen<AgentToolCallCompleteEvent>('agent:tool_call_complete', (event) => {
  const payload = event.payload
  
  // 检测 interactive_shell 工具调用
  if (payload.tool_name === 'interactive_shell') {
    // 动态导入 useTerminal 以避免循环依赖
    import('@/composables/useTerminal').then(({ useTerminal }) => {
      const terminal = useTerminal()
      terminal.openTerminal()
      console.log('[Agent] Detected interactive_shell call, opening terminal panel')
    })
  }
})
```

### 2. 自动启动服务器

`InteractiveTerminal.vue` 在挂载时会自动：
1. 检查终端服务器状态
2. 如果未运行，自动启动服务器
3. 建立 WebSocket 连接
4. 初始化 xterm.js 终端

```typescript
onMounted(async () => {
  initTerminal()
  await connect()  // 自动连接
})

const connect = async () => {
  // 检查并启动服务器
  const status = await TerminalAPI.getStatus()
  if (!status.running) {
    await TerminalAPI.startServer()
    await new Promise(resolve => setTimeout(resolve, 1000))
  }
  
  // 建立 WebSocket 连接
  const wsUrl = await TerminalAPI.getWebSocketUrl()
  ws.value = new WebSocket(wsUrl)
  // ...
}
```

## 使用流程

### Agent 调用示例

```json
{
  "tool": "interactive_shell",
  "arguments": {
    "use_docker": true,
    "docker_image": "kalilinux/kali-rolling",
    "initial_command": "msfconsole"
  }
}
```

### 自动化流程

1. **Agent 决策**: Agent 分析任务，决定需要使用交互式工具
2. **调用工具**: Agent 调用 `interactive_shell` 工具
3. **事件触发**: 后端发送 `agent:tool_call_complete` 事件
4. **自动打开**: 前端检测到事件，自动打开终端面板
5. **服务器启动**: 终端组件检查服务器状态，必要时启动
6. **建立连接**: 创建 WebSocket 连接
7. **用户交互**: 用户在终端中进行操作

## 状态管理

### useTerminal Composable

```typescript
// 全局状态
const terminalState = ref<TerminalState>({
  isActive: false,      // 面板是否打开
  sessionId: null,      // 当前会话 ID
  hasHistory: false,    // 是否有历史记录
})

// 打开终端
function openTerminal(sessionId?: string) {
  terminalState.value.isActive = true
  if (sessionId) {
    terminalState.value.sessionId = sessionId
  }
  terminalState.value.hasHistory = true
}
```

### AgentView 集成

```vue
<template>
  <div class="agent-view">
    <!-- 主内容区 -->
    <MessageFlow />
    
    <!-- 侧边面板 (Vision/Todo/Terminal 互斥) -->
    <div v-if="isTerminalActive" class="sidebar">
      <InteractiveTerminal @close="handleCloseTerminal" />
    </div>
  </div>
</template>

<script setup>
import { useTerminal } from '@/composables/useTerminal'

const terminalComposable = useTerminal()
const isTerminalActive = computed(() => terminalComposable.isTerminalActive.value)

function handleCloseTerminal() {
  terminalComposable.closeTerminal()
}
</script>
```

## 调试

### 检查事件触发

在浏览器控制台查看日志：

```javascript
// 应该看到以下日志
[Agent] Detected interactive_shell call, opening terminal panel
```

### 检查服务器状态

```typescript
import TerminalAPI from '@/api/terminal'

// 检查服务器是否运行
const status = await TerminalAPI.getStatus()
console.log('Server running:', status.running)
console.log('Active sessions:', status.session_count)

// 列出会话
const sessions = await TerminalAPI.listSessions()
console.log('Sessions:', sessions)
```

### 检查 WebSocket 连接

在浏览器开发者工具的 Network 标签中：
1. 过滤 WS (WebSocket)
2. 查看连接状态
3. 查看消息流

## 常见问题

### 1. 终端面板没有自动打开

**可能原因**:
- `useAgentEvents` 未正确监听事件
- 工具名称不匹配（确保是 `interactive_shell`）
- `useTerminal` 导入失败

**解决方法**:
```typescript
// 检查控制台是否有错误
// 手动测试打开
import { useTerminal } from '@/composables/useTerminal'
const terminal = useTerminal()
terminal.openTerminal()
```

### 2. 终端服务器未启动

**可能原因**:
- 端口被占用
- Docker 未运行
- 权限不足

**解决方法**:
```typescript
// 手动启动服务器
import TerminalAPI from '@/api/terminal'
try {
  await TerminalAPI.startServer({ host: '127.0.0.1', port: 8765 })
} catch (error) {
  console.error('Failed to start server:', error)
}
```

### 3. WebSocket 连接失败

**可能原因**:
- 服务器未启动
- 端口配置错误
- 防火墙阻止

**解决方法**:
```bash
# 检查端口是否监听
netstat -an | grep 8765

# 检查 Docker 是否运行
docker ps

# 查看后端日志
tail -f logs/sentinel-ai.log
```

## 配置选项

### 终端服务器配置

```typescript
// 自定义端口
await TerminalAPI.startServer({
  host: '127.0.0.1',
  port: 9000  // 自定义端口
})
```

### Docker 配置

```json
{
  "tool": "interactive_shell",
  "arguments": {
    "use_docker": true,
    "docker_image": "kalilinux/kali-rolling",  // 自定义镜像
    "initial_command": "bash"  // 初始命令
  }
}
```

## 性能优化

### 1. 服务器预启动

在应用启动时预先启动终端服务器：

```typescript
// src/main.ts 或 App.vue
import TerminalAPI from '@/api/terminal'

onMounted(async () => {
  const status = await TerminalAPI.getStatus()
  if (!status.running) {
    await TerminalAPI.startServer()
  }
})
```

### 2. 会话复用

保持会话 ID，避免重复创建：

```typescript
const terminalComposable = useTerminal()

// 打开时传入会话 ID
terminalComposable.openTerminal(existingSessionId)
```

### 3. 延迟加载

使用动态导入减少初始加载时间：

```typescript
// 仅在需要时导入
const { useTerminal } = await import('@/composables/useTerminal')
```

## 相关文件

- `src/composables/useAgentEvents.ts` - 事件监听和自动打开逻辑
- `src/composables/useTerminal.ts` - 终端状态管理
- `src/components/Tools/InteractiveTerminal.vue` - 终端组件
- `src/components/Agent/AgentView.vue` - Agent 视图集成
- `src/api/terminal.ts` - 终端 API

## 测试

### 手动测试

1. 启动应用
2. 在 Agent 中输入需要交互式工具的任务
3. 观察终端面板是否自动打开
4. 检查 WebSocket 连接是否建立
5. 在终端中输入命令测试

### 自动化测试

```typescript
import { describe, it, expect } from 'vitest'
import { useTerminal } from '@/composables/useTerminal'

describe('Terminal Auto Open', () => {
  it('should open terminal when interactive_shell is called', async () => {
    const terminal = useTerminal()
    
    // 模拟工具调用
    terminal.openTerminal()
    
    expect(terminal.isTerminalActive.value).toBe(true)
  })
})
```

## 总结

自动打开功能通过事件监听实现，当 Agent 调用 `interactive_shell` 工具时：

1. ✅ 自动打开终端面板
2. ✅ 自动启动服务器（如果未运行）
3. ✅ 自动建立 WebSocket 连接
4. ✅ 自动初始化终端 UI

用户无需任何手动操作，即可开始使用交互式终端。
