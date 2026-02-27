# Terminal Docker Exec 参数顺序修复

## 问题描述

交互式终端在启动时出现以下错误：

```
bash: -w: invalid option
Usage:  bash [GNU long option] [option] ...
```

随后连接关闭，终端无法使用。

## 根本原因

### 错误的命令顺序

**之前的代码**:
```rust
let mut cmd = Command::new("docker");
cmd.args(&["exec", "-i", &container_id, &self.config.shell]);

if let Some(ref wd) = self.config.working_dir {
    cmd.args(&["-w", wd]);  // ❌ 错误：-w 参数在 shell 之后
}
```

**生成的命令**:
```bash
docker exec -i container_id bash -w /workspace
```

**问题**: `-w /workspace` 被传递给了 `bash`，而不是 `docker exec`。bash 不认识 `-w` 参数，因此报错并退出。

### 正确的命令顺序

`docker exec` 的参数必须在容器 ID 和命令之前：

```bash
docker exec [OPTIONS] CONTAINER COMMAND [ARG...]
```

**正确的命令**:
```bash
docker exec -i -w /workspace container_id bash
```

## 修复方案

### 修改 `session.rs`

```rust
/// Start Docker-based session
async fn start_docker_session(
    &mut self,
    output_tx: mpsc::UnboundedSender<Vec<u8>>,
) -> Result<(), String> {
    // Create container
    let container_id = self.create_container().await?;
    self.container_id = Some(container_id.clone());

    // Start interactive shell in container
    let mut cmd = Command::new("docker");
    cmd.arg("exec").arg("-i");
    
    // ✅ Add working directory BEFORE container_id (docker exec option)
    if let Some(ref wd) = self.config.working_dir {
        cmd.arg("-w").arg(wd);
    }
    
    // ✅ Add container_id and shell AFTER docker options
    cmd.arg(&container_id).arg(&self.config.shell);

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    // ... rest of the code
}
```

### 命令构建顺序

1. `docker` - 命令名
2. `exec` - 子命令
3. `-i` - 交互模式
4. `-w /workspace` - 工作目录（docker exec 参数）
5. `container_id` - 容器 ID
6. `bash` - 要执行的 shell

## 错误日志分析

### 终端输出

```
Sentinel AI Interactive Terminal
Connecting to terminal server...

✓ Connected!

bash: -w: invalid option
Usage:  bash [GNU long option] [option] ...
...
✗ Connection closed
```

### 后端日志

```
ERROR sentinel_tools::terminal::session: 121: Failed to write to stdin: Broken pipe (os error 32)
```

**解释**:
1. bash 因为无效参数 `-w` 立即退出
2. 进程退出导致 stdin 管道关闭
3. 尝试写入已关闭的管道导致 "Broken pipe" 错误
4. WebSocket 连接因进程退出而关闭

## 测试验证

### 手动测试命令

**错误的命令**:
```bash
docker exec -i container_id bash -w /workspace
# 输出: bash: -w: invalid option
```

**正确的命令**:
```bash
docker exec -i -w /workspace container_id bash
# 成功启动 bash，工作目录为 /workspace
```

### 验证工作目录

```bash
# 在容器中执行
docker exec -i -w /workspace container_id bash -c "pwd"
# 输出: /workspace
```

## 其他 Docker Exec 参数

如果将来需要添加更多 `docker exec` 参数，确保它们在容器 ID 之前：

```rust
let mut cmd = Command::new("docker");
cmd.arg("exec");

// 所有 docker exec 的参数都在这里
cmd.arg("-i");  // 交互模式

if let Some(ref wd) = self.config.working_dir {
    cmd.arg("-w").arg(wd);  // 工作目录
}

if let Some(ref user) = self.config.user {
    cmd.arg("-u").arg(user);  // 用户
}

// 环境变量
for (key, value) in &self.config.env_vars {
    cmd.arg("-e").arg(format!("{}={}", key, value));
}

// 最后才是容器 ID 和命令
cmd.arg(&container_id);
cmd.arg(&self.config.shell);
```

## 相关文档

- [Docker exec 文档](https://docs.docker.com/engine/reference/commandline/exec/)
- [Rust std::process::Command](https://doc.rust-lang.org/std/process/struct.Command.html)

## 修复的文件

- `src-tauri/sentinel-tools/src/terminal/session.rs` - 修复 Docker exec 参数顺序

## 影响范围

- ✅ Docker 模式终端会话
- ✅ 工作目录设置
- ⚠️ 主机模式不受影响（使用 `current_dir()` 方法）

## 后续改进建议

1. **添加单元测试**: 测试命令构建逻辑
2. **验证参数顺序**: 在启动前验证命令格式
3. **更好的错误处理**: 捕获并报告 bash 启动失败
4. **日志改进**: 记录实际执行的完整命令

## 测试清单

- [x] 修复参数顺序
- [x] 编译通过
- [ ] 手动测试 Docker 模式
- [ ] 验证工作目录正确
- [ ] 测试环境变量传递
- [ ] 测试不同的 shell（bash, sh, zsh）
- [ ] 测试主机模式（确保未受影响）

## 总结

这是一个典型的**参数顺序错误**：

- **问题**: `-w` 参数被传递给 bash 而不是 docker exec
- **原因**: 参数添加顺序错误
- **修复**: 确保 docker exec 的参数在容器 ID 之前
- **影响**: 所有使用 Docker 模式的终端会话
- **严重性**: 高（导致终端完全无法使用）

修复后，终端应该能够正常启动并在指定的工作目录中运行。
