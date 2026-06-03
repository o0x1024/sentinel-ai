# 交互式终端输出捕获修复

## 问题描述

在之前的实现中，`interactive_shell` 工具存在以下问题：

1. **命令执行结果无法返回给 LLM**：工具调用后只返回配置信息，不执行实际命令
2. **`whoami` 返回 `root` 而不是 `sandbox`**：Docker 容器未正确设置非特权用户
3. **`initial_command` 未执行**：虽然配置中有此字段，但没有实际执行逻辑

## 修复内容

### 1. 实现命令执行和输出捕获

**文件**: `src-tauri/sentinel-tools/src/tool_server.rs`

修改了 `interactive_shell` 工具的 executor，使其：

```rust
// 创建临时会话
let session_id = format!("temp_{}", uuid::Uuid::new_v4());
let (output_tx, mut output_rx) = mpsc::unbounded_channel();

let mut session = TerminalSession::new(session_id.clone(), config).await?;

// 启动会话
session.start(output_tx).await?;

// 等待会话就绪
tokio::time::sleep(Duration::from_millis(1000)).await;

// 执行命令
let cmd_with_newline = format!("{}\n", cmd);
session.write(cmd_with_newline.into_bytes()).await?;

// 收集输出（最多10秒）
let mut output = Vec::new();
while start.elapsed() < collect_timeout {
    match timeout(Duration::from_millis(500), output_rx.recv()).await {
        Ok(Some(data)) => output.extend_from_slice(&data),
        Ok(None) => break,
        Err(_) => if !output.is_empty() { break }
    }
}

// 停止会话
session.stop().await;

// 返回输出给 LLM
Ok(serde_json::json!({
    "success": true,
    "command": cmd,
    "output": String::from_utf8_lossy(&output).to_string(),
    ...
}))
```

**关键改进**：
- ✅ 实际创建并启动终端会话
- ✅ 执行 `initial_command`
- ✅ 收集命令输出（带超时保护）
- ✅ 将输出返回给 LLM
- ✅ 自动清理临时会话

### 2. 修复 Docker 用户权限问题

**文件**: `src-tauri/sentinel-tools/src/terminal/session.rs`

#### 2.1 容器创建时设置 sandbox 用户

```rust
async fn create_container(&self) -> Result<String, String> {
    // 检测是否使用 Kali 镜像
    let use_sandbox_user = self.config.docker_image.contains("kali");
    
    // 创建容器...
    
    // 为 Kali 镜像创建 sandbox 用户
    if use_sandbox_user {
        let setup_commands = vec![
            "id -u sandbox &>/dev/null || useradd -m -s /bin/bash sandbox",
            "mkdir -p /workspace",
            "chown -R sandbox:sandbox /workspace 2>/dev/null || true",
        ];
        
        for cmd in setup_commands {
            Command::new("docker")
                .args(&["exec", &container_id, "bash", "-c", cmd])
                .output()
                .await;
        }
    }
    
    Ok(container_id)
}
```

#### 2.2 docker exec 时指定用户

```rust
async fn start_docker_session(&mut self, output_tx: ...) -> Result<(), String> {
    let mut cmd = Command::new("docker");
    cmd.arg("exec").arg("-i");
    
    // 为 Kali 镜像使用 sandbox 用户
    if self.config.docker_image.contains("kali") {
        cmd.arg("-u").arg("sandbox");
    }
    
    // 添加工作目录
    if let Some(ref wd) = self.config.working_dir {
        cmd.arg("-w").arg(wd);
    }
    
    cmd.arg(&container_id).arg(&self.config.shell);
    // ...
}
```

**效果**：
- ✅ `whoami` 现在返回 `sandbox` 而不是 `root`
- ✅ 提高安全性，避免以 root 权限执行命令
- ✅ 符合最小权限原则

### 3. 实现 initial_command 自动执行

**文件**: `src-tauri/sentinel-tools/src/terminal/session.rs`

#### 3.1 添加配置字段

```rust
pub struct TerminalSessionConfig {
    // ... 其他字段
    /// Optional command to execute immediately after session starts
    pub initial_command: Option<String>,
}
```

#### 3.2 会话启动后自动执行

```rust
async fn start_docker_session(&mut self, output_tx: ...) -> Result<(), String> {
    // ... 启动会话逻辑
    
    self.process = Some(child);
    *self.state.write().await = SessionState::Running;
    
    // 执行 initial_command（如果提供）
    if let Some(ref initial_cmd) = self.config.initial_command {
        if !initial_cmd.is_empty() {
            info!("Executing initial command: {}", initial_cmd);
            // 等待 shell 就绪
            tokio::time::sleep(Duration::from_millis(500)).await;
            
            // 发送命令
            let cmd_with_newline = format!("{}\n", initial_cmd);
            if let Err(e) = stdin_tx.send(cmd_with_newline.into_bytes()) {
                error!("Failed to send initial command: {}", e);
            }
        }
    }
    
    Ok(())
}
```

## 使用示例

### 示例 1: 执行 ping 命令

**LLM 调用**:
```json
{
  "tool": "interactive_shell",
  "arguments": {
    "initial_command": "ping -c 4 baidu.com"
  }
}
```

**返回结果**:
```json
{
  "success": true,
  "message": "Executed command in Docker container",
  "command": "ping -c 4 baidu.com",
  "output": "PING baidu.com (110.242.68.66) 56(84) bytes of data.\n64 bytes from 110.242.68.66: icmp_seq=1 ttl=50 time=30.2 ms\n...",
  "session_id": "temp_xxx",
  "note": "The terminal panel has been opened for further interaction if needed."
}
```

**LLM 可以看到**：
- ✅ 命令执行成功
- ✅ 完整的 ping 输出
- ✅ 可以基于输出做出决策

### 示例 2: 启动 Metasploit（交互式）

**LLM 调用**:
```json
{
  "tool": "interactive_shell",
  "arguments": {
    "initial_command": "msfconsole"
  }
}
```

**返回结果**:
```json
{
  "success": true,
  "message": "Executed command in Docker container",
  "command": "msfconsole",
  "output": "...[msf banner]...\nmsf6 > ",
  "session_id": "temp_xxx",
  "note": "The terminal panel has been opened for further interaction if needed."
}
```

**用户可以**：
- ✅ 在终端面板中继续交互
- ✅ 手动输入 Metasploit 命令
- ✅ LLM 知道 Metasploit 已启动

### 示例 3: 仅打开终端（无初始命令）

**LLM 调用**:
```json
{
  "tool": "interactive_shell",
  "arguments": {}
}
```

**返回结果**:
```json
{
  "success": true,
  "message": "Interactive terminal session will be created",
  "instructions": "Use the Terminal panel in AgentView to interact with the session",
  "note": "No initial command specified. The terminal will open for manual interaction."
}
```

## 技术细节

### 输出收集策略

1. **超时机制**: 最多等待 10 秒收集输出
2. **动态停止**: 如果 500ms 内没有新数据且已有输出，则停止收集
3. **完整性**: 使用 `mpsc::unbounded_channel` 确保不丢失数据

### 用户权限

| 镜像类型 | 默认用户 | 工作目录 | 权限 |
|---------|---------|---------|-----|
| `kalilinux/kali-rolling` | `sandbox` | `/workspace` | 非特权 |
| 其他镜像 | 镜像默认 | `/workspace` | 镜像默认 |

### 会话生命周期

```
创建配置 → 创建会话 → 启动会话 → 执行命令 → 收集输出 → 停止会话 → 返回结果
   ↓          ↓          ↓          ↓          ↓          ↓          ↓
 Config   TerminalSession  Docker   stdin    stdout    cleanup   JSON
```

## 测试验证

### 测试 1: 验证用户权限

```bash
# LLM 调用
{"tool": "interactive_shell", "arguments": {"initial_command": "whoami"}}

# 预期输出
{"output": "sandbox\n", ...}
```

### 测试 2: 验证命令执行

```bash
# LLM 调用
{"tool": "interactive_shell", "arguments": {"initial_command": "echo 'Hello World'"}}

# 预期输出
{"output": "Hello World\n", ...}
```

### 测试 3: 验证网络访问

```bash
# LLM 调用
{"tool": "interactive_shell", "arguments": {"initial_command": "ping -c 2 8.8.8.8"}}

# 预期输出
{"output": "PING 8.8.8.8 ...\n2 packets transmitted, 2 received...", ...}
```

## 注意事项

1. **超时时间**: 当前设置为 10 秒，对于长时间运行的命令可能不够
2. **输出大小**: 没有限制输出大小，大量输出可能影响性能
3. **会话清理**: 临时会话会自动清理，但如果进程崩溃可能留下容器
4. **并发限制**: 没有限制并发会话数量

## 后续改进建议

1. **可配置超时**: 允许工具调用时指定超时时间
2. **输出截断**: 对超大输出进行截断或分页
3. **持久会话**: 支持创建持久会话供多次命令使用
4. **会话管理**: 添加会话列表、重连等功能
5. **资源限制**: 限制并发会话数和容器资源使用

## 相关文件

- `src-tauri/sentinel-tools/src/tool_server.rs` - 工具注册和执行器
- `src-tauri/sentinel-tools/src/terminal/session.rs` - 会话管理
- `src-tauri/sentinel-tools/src/terminal/server.rs` - WebSocket 服务器
- `src-tauri/sentinel-tools/src/terminal/manager.rs` - 会话管理器
- `src/components/Tools/InteractiveTerminal.vue` - 前端终端组件
