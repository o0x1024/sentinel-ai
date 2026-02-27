# Interactive Terminal API Documentation

## 概述

交互式终端功能已完全暴露给前端，包括 WebSocket 服务器管理、会话管理和工具集成。

## 后端 API

### Tauri Commands

所有命令已在 `src-tauri/src/lib.rs` 中注册并可供前端调用。

#### 1. 启动终端服务器

```rust
#[tauri::command]
pub async fn start_terminal_server(
    config: Option<TerminalServerConfig>
) -> Result<String, String>
```

**参数**:
- `config` (可选): 服务器配置
  - `host`: 主机地址 (默认: "127.0.0.1")
  - `port`: 端口号 (默认: 8765)

**返回**: 成功消息

**示例**:
```typescript
await invoke('start_terminal_server', {
  config: { host: '127.0.0.1', port: 8765 }
})
```

#### 2. 停止终端服务器

```rust
#[tauri::command]
pub async fn stop_terminal_server() -> Result<String, String>
```

**返回**: 成功消息

#### 3. 获取服务器状态

```rust
#[tauri::command]
pub async fn get_terminal_server_status() -> Result<TerminalServerStatus, String>
```

**返回**:
```typescript
{
  running: boolean
  session_count: number
}
```

#### 4. 列出所有会话

```rust
#[tauri::command]
pub async fn list_terminal_sessions() -> Result<Vec<SessionInfo>, String>
```

**返回**:
```typescript
Array<{
  id: string
  state: 'Starting' | 'Running' | 'Stopped' | 'Error'
  last_activity: number  // seconds since last activity
  use_docker: boolean
}>
```

#### 5. 停止会话

```rust
#[tauri::command]
pub async fn stop_terminal_session(session_id: String) -> Result<String, String>
```

**参数**:
- `session_id`: 会话 ID

**返回**: 成功消息

#### 6. 获取 WebSocket URL

```rust
#[tauri::command]
pub async fn get_terminal_websocket_url() -> Result<String, String>
```

**返回**: WebSocket URL (例如: `ws://127.0.0.1:8765`)

## 前端 API

### TypeScript 接口

位置: `src/api/terminal.ts`

#### 导入

```typescript
import TerminalAPI from '@/api/terminal'
// 或
import { startTerminalServer, getTerminalServerStatus, ... } from '@/api/terminal'
```

#### API 方法

##### 1. 启动服务器

```typescript
await TerminalAPI.startServer(config?: TerminalServerConfig): Promise<string>
```

##### 2. 停止服务器

```typescript
await TerminalAPI.stopServer(): Promise<string>
```

##### 3. 获取状态

```typescript
const status = await TerminalAPI.getStatus(): Promise<TerminalServerStatus>
```

##### 4. 列出会话

```typescript
const sessions = await TerminalAPI.listSessions(): Promise<SessionInfo[]>
```

##### 5. 停止会话

```typescript
await TerminalAPI.stopSession(sessionId: string): Promise<string>
```

##### 6. 获取 WebSocket URL

```typescript
const wsUrl = await TerminalAPI.getWebSocketUrl(): Promise<string>
```

### 使用示例

#### 在组件中使用

```vue
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import TerminalAPI from '@/api/terminal'

const serverRunning = ref(false)
const sessions = ref<SessionInfo[]>([])

onMounted(async () => {
  // 检查服务器状态
  const status = await TerminalAPI.getStatus()
  serverRunning.value = status.running
  
  // 如果未运行，启动服务器
  if (!status.running) {
    await TerminalAPI.startServer()
    serverRunning.value = true
  }
  
  // 获取会话列表
  sessions.value = await TerminalAPI.listSessions()
})

async function stopSession(sessionId: string) {
  await TerminalAPI.stopSession(sessionId)
  sessions.value = await TerminalAPI.listSessions()
}
</script>
```

## Agent 工具集成

### interactive_shell 工具

`interactive_shell` 工具已添加到内置工具列表，Agent 可以使用它来创建交互式终端会话。

#### 工具定义

```json
{
  "name": "interactive_shell",
  "description": "Create an interactive terminal session for persistent command execution (e.g., msfconsole, sqlmap, database clients). Returns a session ID for continuous interaction.",
  "input_schema": {
    "type": "object",
    "properties": {
      "use_docker": {
        "type": "boolean",
        "description": "Whether to run in Docker container (recommended for security)",
        "default": true
      },
      "docker_image": {
        "type": "string",
        "description": "Docker image to use (default: kalilinux/kali-rolling)",
        "default": "kalilinux/kali-rolling"
      },
      "initial_command": {
        "type": "string",
        "description": "Optional initial command to run (e.g., 'msfconsole', 'sqlmap')"
      }
    }
  }
}
```

#### Agent 使用示例

当 Agent 需要使用交互式工具时：

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

工具返回：

```json
{
  "success": true,
  "message": "Interactive terminal session created",
  "instructions": "Use the Terminal panel in AgentView to interact with this session",
  "config": {
    "use_docker": true,
    "docker_image": "kalilinux/kali-rolling",
    "initial_command": "msfconsole"
  },
  "note": "The terminal server must be started first. The frontend will automatically open the terminal panel."
}
```

### 工具路由器集成

`interactive_shell` 已添加到 `ToolRouter` 的默认工具列表中：

```rust
ToolMetadata {
    id: "interactive_shell".to_string(),
    name: "interactive_shell".to_string(),
    description: "Create an interactive terminal session for persistent command execution...",
    category: ToolCategory::System,
    tags: vec![
        "terminal", "interactive", "session", "persistent",
        "msfconsole", "sqlmap", "shell"
    ],
    cost_estimate: ToolCost::Low,
    always_available: false,
}
```

## WebSocket 协议

### 连接流程

1. **建立连接**: 客户端连接到 `ws://127.0.0.1:8765`
2. **发送配置**: 发送 JSON 格式的会话配置
   ```json
   {
     "use_docker": true,
     "docker_image": "kalilinux/kali-rolling",
     "working_dir": "/root",
     "env_vars": {},
     "shell": "bash"
   }
   ```
3. **接收会话 ID**: 服务器返回会话 ID
4. **交互**: 发送命令，接收输出

### 消息格式

#### 客户端 -> 服务器

- **文本消息**: 发送到终端的命令
- **二进制消息**: 原始字节数据

#### 服务器 -> 客户端

- **文本消息**: 终端输出（UTF-8 编码）
- **二进制消息**: 原始终端输出

### 示例代码

```typescript
// 连接到终端服务器
const ws = new WebSocket('ws://127.0.0.1:8765')

// 发送会话配置
ws.onopen = () => {
  ws.send(JSON.stringify({
    use_docker: true,
    docker_image: 'kalilinux/kali-rolling',
    working_dir: '/root',
    env_vars: {},
    shell: 'bash'
  }))
}

// 接收输出
ws.onmessage = (event) => {
  if (typeof event.data === 'string') {
    console.log('Output:', event.data)
  }
}

// 发送命令
ws.send('ls -la\n')
```

## 架构图

```
┌─────────────────────────────────────────────────────────┐
│                       Frontend                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │  InteractiveTerminal.vue                         │  │
│  │  - xterm.js (Terminal UI)                        │  │
│  │  - WebSocket Client                              │  │
│  │  - TerminalAPI (TypeScript)                      │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
                          │ Tauri Commands + WebSocket
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    Backend (Rust)                       │
│  ┌──────────────────────────────────────────────────┐  │
│  │  terminal_commands.rs (Tauri Commands)           │  │
│  │  - start_terminal_server                         │  │
│  │  - stop_terminal_server                          │  │
│  │  - get_terminal_server_status                    │  │
│  │  - list_terminal_sessions                        │  │
│  │  - stop_terminal_session                         │  │
│  │  - get_terminal_websocket_url                    │  │
│  └──────────────────────────────────────────────────┘  │
│                          │                              │
│                          ▼                              │
│  ┌──────────────────────────────────────────────────┐  │
│  │  TerminalServer (WebSocket Server)               │  │
│  │  - Accept connections                            │  │
│  │  - Handle WebSocket messages                     │  │
│  │  - Manage sessions                               │  │
│  └──────────────────────────────────────────────────┘  │
│                          │                              │
│                          ▼                              │
│  ┌──────────────────────────────────────────────────┐  │
│  │  TerminalSessionManager                          │  │
│  │  - Create sessions                               │  │
│  │  - Track active sessions                         │  │
│  │  - Cleanup idle sessions                         │  │
│  └──────────────────────────────────────────────────┘  │
│                          │                              │
│                          ▼                              │
│  ┌──────────────────────────────────────────────────┐  │
│  │  TerminalSession                                 │  │
│  │  - PTY process                                   │  │
│  │  - Docker container (optional)                   │  │
│  │  - Input/Output handling                         │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│              Docker Container (Optional)                │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Kali Linux / Custom Image                       │  │
│  │  - bash / zsh / sh                               │  │
│  │  - Security tools (nmap, sqlmap, etc.)           │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## 工具集成

### ToolServer 注册

`interactive_shell` 工具已在 `ToolServer` 中注册：

```rust
// sentinel-tools/src/tool_server.rs
let interactive_shell_def = DynamicToolBuilder::new("interactive_shell")
    .description("Create an interactive terminal session...")
    .input_schema(...)
    .source(ToolSource::Builtin)
    .executor(|args| async move {
        // 创建会话配置
        // 返回指令和配置
    })
    .build()
    .expect("Failed to build interactive_shell tool");

self.registry.register(interactive_shell_def).await;
```

### ToolRouter 元数据

```rust
// src-tauri/src/agents/tool_router.rs
ToolMetadata {
    id: "interactive_shell".to_string(),
    name: "interactive_shell".to_string(),
    description: "Create an interactive terminal session...",
    category: ToolCategory::System,
    tags: vec!["terminal", "interactive", "session", ...],
    cost_estimate: ToolCost::Low,
    always_available: false,
}
```

## 使用流程

### 1. 前端初始化

```typescript
// 在应用启动时或首次使用时
const status = await TerminalAPI.getStatus()
if (!status.running) {
  await TerminalAPI.startServer()
}
```

### 2. Agent 调用工具

```json
{
  "tool": "interactive_shell",
  "arguments": {
    "use_docker": true,
    "initial_command": "msfconsole"
  }
}
```

### 3. 前端响应

- 检测到 `interactive_shell` 工具调用
- 自动打开终端面板
- 使用 `useTerminal` composable 管理状态

### 4. 用户交互

- 在终端面板中输入命令
- 实时查看输出
- 支持 Ctrl+C 中断
- 支持历史记录

## 安全考虑

1. **Docker 隔离**: 默认在 Docker 容器中执行，隔离主机系统
2. **权限控制**: 可配置允许/禁止的命令
3. **会话超时**: 自动清理闲置会话（默认 30 分钟）
4. **本地连接**: WebSocket 仅监听 127.0.0.1
5. **资源限制**: Docker 容器可配置 CPU/内存限制

## 故障排查

### 服务器无法启动

```typescript
try {
  await TerminalAPI.startServer()
} catch (error) {
  console.error('Failed to start terminal server:', error)
  // 检查端口是否被占用
  // 检查 Docker 是否运行
}
```

### 会话连接失败

```typescript
// 检查服务器状态
const status = await TerminalAPI.getStatus()
if (!status.running) {
  await TerminalAPI.startServer()
}

// 检查会话列表
const sessions = await TerminalAPI.listSessions()
console.log('Active sessions:', sessions)
```

### Docker 容器问题

```bash
# 检查 Docker 是否运行
docker ps

# 检查镜像是否存在
docker images | grep kalilinux

# 构建沙箱镜像
./scripts/build-docker-sandbox.sh kali
```

## 相关文件

### 后端
- `src-tauri/src/commands/terminal_commands.rs` - Tauri 命令
- `src-tauri/sentinel-tools/src/terminal/` - 终端模块
  - `server.rs` - WebSocket 服务器
  - `manager.rs` - 会话管理器
  - `session.rs` - 终端会话
  - `mod.rs` - 模块导出
- `src-tauri/src/agents/tool_router.rs` - 工具路由器
- `src-tauri/sentinel-tools/src/tool_server.rs` - 工具服务器

### 前端
- `src/api/terminal.ts` - TypeScript API
- `src/components/Tools/InteractiveTerminal.vue` - 终端组件
- `src/composables/useTerminal.ts` - 终端状态管理
- `src/components/Agent/AgentView.vue` - Agent 视图集成

### 文档
- `docs/interactive-terminal-integration.md` - 集成文档
- `docs/interactive-terminal-api.md` - API 文档（本文件）
