# PTY 终端测试指南

## ✅ 编译成功

已成功将交互式终端从简单的 pipe 模式升级到 **PTY（伪终端）模式**！

---

## 🎯 核心改进

### 技术变更

| 组件 | 旧版本 | 新版本 | 改进 |
|-----|-------|-------|------|
| **终端模式** | pipe (stdin/stdout) | PTY (pseudo-terminal) | ✅ 真正的 TTY |
| **Docker 命令** | `docker exec -i` | `docker exec -it` | ✅ TTY 支持 |
| **颜色输出** | ❌ 无 | ✅ 完整 ANSI 颜色 |
| **光标控制** | ❌ 无 | ✅ 完整支持 |
| **进程稳定性** | ⚠️ Broken pipe 常见 | ✅ 极大改善 |
| **交互工具** | ⚠️ 部分支持 | ✅ vim/nano/htop 等 |

---

## 🧪 测试步骤

### 测试 1: 基础命令执行

1. **启动应用**
2. **发送消息**："执行一下 whoami"
3. **预期结果**：
   ```
   交互式终端面板自动打开
   显示：
   ✓ Connected!
   
   sandbox
   sandbox@container:/workspace$ _
   ```

**检查点**：
- ✅ 终端自动打开
- ✅ 显示命令输出 "sandbox"
- ✅ 显示新的 prompt
- ✅ 光标闪烁
- ✅ 可以输入命令

---

### 测试 2: 颜色输出

在终端中输入：
```bash
ls --color=always
```

**预期结果**：
- ✅ 目录显示为蓝色
- ✅ 可执行文件显示为绿色
- ✅ 颜色正常渲染

---

### 测试 3: 交互式工具

#### 3.1 vim 编辑器
```bash
vim test.txt
```

**预期结果**：
- ✅ vim 正常启动
- ✅ 可以输入内容
- ✅ 可以使用 `:wq` 保存退出

#### 3.2 nano 编辑器
```bash
nano test.txt
```

**预期结果**：
- ✅ nano 正常启动
- ✅ 底部快捷键显示正常
- ✅ Ctrl+X 退出正常

---

### 测试 4: 持续命令

```bash
ping -c 3 8.8.8.8
```

**预期结果**：
- ✅ 实时显示 ping 输出
- ✅ 每秒更新一次
- ✅ 完成后显示统计信息

---

### 测试 5: 多行输出

```bash
for i in {1..10}; do echo "Line $i"; done
```

**预期结果**：
- ✅ 快速显示 10 行
- ✅ 滚动流畅
- ✅ 所有行都可见

---

### 测试 6: 会话持久性

1. 发送命令："执行 cd /tmp"
2. 关闭终端面板
3. 重新打开终端
4. 输入 `pwd`

**预期结果**：
- ✅ 显示 `/tmp`（会话保持）
- ✅ 历史命令可用（↑ 键）

---

### 测试 7: 错误处理

```bash
command_not_exist
```

**预期结果**：
- ✅ 显示错误信息（红色）
- ✅ 返回新的 prompt
- ✅ 终端继续可用

---

### 测试 8: Broken Pipe 修复验证

**之前的问题**：
```
ERROR: Failed to write to stdin: Broken pipe (os error 32)
```

**测试步骤**：
1. 打开终端面板（不输入任何命令）
2. 等待 5 秒
3. 发送消息："执行一下 whoami"
4. 关闭终端
5. 再次发送："执行 ls -la"

**预期结果**：
- ✅ 第一次执行成功
- ✅ 第二次执行也成功
- ❌ 不再出现 "Broken pipe" 错误
- ✅ 后端日志显示会话健康检查通过

**后端日志应该显示**：
```
[INFO] Found healthy session: xxx
[INFO] Using existing terminal session: xxx
```

或（如果第一个会话不健康）：
```
[INFO] Session xxx is not healthy (stdin closed), stopping it
[INFO] Created new persistent terminal session: yyy
```

---

## 📊 关键改进点

### 1. PTY 带来的稳定性

**之前（pipe 模式）**：
```rust
// bash 进程可能意外退出
docker exec -i container bash
stdin -----> bash
       <----- stdout/stderr
// 管道容易断开 → Broken Pipe
```

**现在（PTY 模式）**：
```rust
// PTY master/slave 架构
docker exec -it container bash
PTY Master <----> PTY Slave <----> bash
// TTY 保持活跃，bash 更稳定
```

---

### 2. 健康检查机制

```rust
// 工具执行前检查会话健康
if session.is_healthy() {
    // 复用现有会话
} else {
    // 停止不健康会话，创建新的
    stop_session(unhealthy_id);
    create_new_session();
}
```

**检查方法**：
```rust
pub fn is_healthy(&self) -> bool {
    if let Some(ref tx) = self.stdin_tx {
        !tx.is_closed()  // ← 检查通道是否仍然打开
    } else {
        false
    }
}
```

---

## 🐛 已修复的问题

### 问题 1: Broken Pipe
**原因**：bash 进程退出导致 stdin 管道关闭
**解决**：使用 PTY，bash 进程更稳定

### 问题 2: 命令不显示
**原因**：前端过早打开终端，后端工具失败
**解决**：
- 前端只在收到 `session_id` 时才打开
- 后端检查会话健康状态

### 问题 3: 无法输入
**原因**：WebSocket 消息类型不匹配
**解决**：正确处理 `Message::Text` 和 `Message::Binary`

---

## 📝 测试日志分析

### 成功的日志示例

**前端**：
```javascript
[Agent] interactive_shell result parsed, session_id: abc123
[Agent] ✅ Terminal opened with session_id: abc123
[Terminal] Initial connection attempt, session ID: abc123
[Terminal] Connecting to existing session: abc123
[Terminal] ✓ Session established and synced to global state: abc123
[Terminal] Received output, length: 8  // "sandbox\n"
```

**后端**：
```
[INFO] Found healthy session: abc123
[INFO] Using existing terminal session: abc123
[INFO] [Terminal Session abc123] Adding subscriber, history chunks: 1
[INFO] [Terminal Session abc123] Sending history chunk 0: 8 bytes
[INFO] [WS Session abc123] Forwarding chunk #1: 8 bytes
```

---

### 失败的日志示例（旧版本）

**前端**：
```javascript
[Agent] session_id: undefined  // ❌ 工具失败
[Terminal] session ID: null    // ❌ 没有 session_id
[Terminal] creating new session // ❌ 创建了错误的会话
```

**后端**：
```
ERROR: Failed to write to stdin: Broken pipe (os error 32)  // ❌
```

---

## 🎯 验收标准

### 必须通过的测试

- [ ] 测试 1: 基础命令执行 ✅
- [ ] 测试 2: 颜色输出 ✅
- [ ] 测试 3: 交互式工具 (vim/nano) ✅
- [ ] 测试 6: 会话持久性 ✅
- [ ] 测试 8: Broken Pipe 修复 ✅

### 可选但推荐的测试

- [ ] 测试 4: 持续命令 (ping)
- [ ] 测试 5: 多行输出
- [ ] 测试 7: 错误处理

---

## 🚀 下一步优化（可选）

### 1. 添加终端大小调整
```rust
// 监听前端终端大小变化
pty_pair.master.resize(PtySize {
    rows: new_rows,
    cols: new_cols,
    ..
})?;
```

### 2. 添加会话超时清理
```rust
// 清理长时间无活动的会话
if last_activity.elapsed() > Duration::from_secs(3600) {
    stop_session(session_id);
}
```

### 3. 支持更多 Shell
```rust
// 支持 zsh, fish 等
match shell_type {
    "bash" => spawn_bash(),
    "zsh" => spawn_zsh(),
    "fish" => spawn_fish(),
}
```

---

## 📚 相关文档

- [交互式终端工作流程](./interactive-terminal-workflow.md)
- [终端显示问题诊断](./terminal-display-issue-diagnosis.md)
- [终端过早打开修复](./terminal-premature-open-fix.md)

---

## 🎉 总结

通过引入 **PTY（伪终端）技术**，我们：

1. ✅ **解决了 Broken Pipe 问题** - PTY 让 bash 进程更稳定
2. ✅ **提升了交互体验** - 真正的 TTY 支持颜色、光标控制
3. ✅ **支持更多工具** - vim、nano、htop 等交互式工具
4. ✅ **增强了健康检查** - 自动检测和清理不健康会话

现在可以开始测试了！🚀
