# 宿主机/Docker 工具执行环境分析与解决方案

## 执行时间
2026-01-19

## 问题概述

当前系统在宿主机和 Docker 沙箱中运行工具时存在多个环境不一致和路径混淆问题，可能导致工具执行失败、数据泄露和用户体验下降。

---

## 已识别的问题与缺陷

### 1. 工具执行环境不一致导致路径混淆

**问题描述**：
- 系统提示告诉 LLM 使用容器路径（如 `/workspace/context`）
- 但工具可能在宿主机执行，导致路径不存在

**影响范围**：
- Shell 工具在宿主机执行时访问容器路径失败
- HTTP 工具输出存储到容器，但 shell 在宿主机无法读取
- 跨工具协作失败（HTTP → Shell）

**严重程度**：高

**解决状态**：✅ 已修复
- 在 `builder.rs` 中实现环境感知提示
- 根据实际执行环境（Host/Docker）生成对应路径
- 系统提示中明确标注当前环境和 OS

---

### 2. 工具输出存储位置与提示不匹配

**问题描述**：
- `store_output_unified` 自动选择容器/宿主机存储
- 但系统提示中的路径是静态的，不会根据实际存储位置更新

**影响范围**：
- 工具输出存到宿主机，但提示说在容器中
- LLM 尝试用容器路径访问，读取失败

**严重程度**：高

**解决状态**：✅ 已修复
- Shell 工具在 Docker 模式下使用 `store_output_in_container`
- Host 模式下使用 `store_output_on_host`
- 返回的 summary 中明确标注实际存储位置

---

### 3. 历史文件跨会话泄露风险

**问题描述**：
- 所有会话共享 `history.txt`
- 不同执行的历史数据混在一起

**影响范围**：
- 会话 A 可以读取会话 B 的历史
- 敏感信息跨会话泄露
- 多用户环境下的隐私问题

**严重程度**：严重

**解决状态**：✅ 已修复
- 实现按 execution_id 隔离的历史文件：`history_{execution_id}.txt`
- 每个执行有独立的历史文件
- 清理时只删除对应执行的历史

---

### 4. 跨环境文件访问缺少映射说明

**问题描述**：
- 用户在宿主机上传文件（如 `C:\Users\...`）
- 但工具在容器中执行，找不到文件
- 没有路径映射机制或说明

**影响范围**：
- 文档附件在容器中找不到
- 用户指定的宿主机路径在容器中无效
- 需要手动复制文件到容器

**严重程度**：中

**解决状态**：⚠️ 部分修复
- 文档附件处理时会复制到容器（`container_path`）
- 但缺少宿主机 → 容器路径映射的用户提示
- **待改进**：在系统提示中说明路径映射规则

---

### 5. Shell 工具执行模式与系统提示脱节

**问题描述**：
- Shell 工具可以动态选择 Host/Docker 模式
- 但系统提示在构建时就固定了环境信息
- 工具调用时可能切换环境

**影响范围**：
- LLM 认为在容器中，但 shell 实际在宿主机执行
- 命令语法不匹配（Linux vs Windows）
- Fallback 机制不透明

**严重程度**：中

**解决状态**：✅ 已修复
- Shell 工具返回时在输出中标注实际执行环境
- Docker 失败时自动 fallback 到 Host，并记录日志
- 存储路径根据实际执行环境选择

---

### 6. 清理逻辑未区分环境

**问题描述**：
- `cleanup_container_context` 只清理容器
- 如果工具在宿主机执行，宿主机文件不会被清理

**影响范围**：
- 宿主机上下文目录积累大量历史文件
- 可能泄露跨会话数据
- 磁盘空间浪费

**严重程度**：中

**解决状态**：✅ 已修复
- 新增 `cleanup_host_context_with_id` 函数
- 支持按 execution_id 清理宿主机文件
- **待集成**：在执行完成时调用清理函数

---

### 7. Windows 路径与 Linux 路径混用

**问题描述**：
- 提示中的示例命令可能不适用当前 OS
- 路径分隔符不一致（`/` vs `\`）
- 命令语法差异（`cat` vs `Get-Content`）

**影响范围**：
- Windows 上执行 `cat /workspace/context/history.txt` 失败
- 路径拼接错误
- 用户体验差

**严重程度**：中

**解决状态**：✅ 已修复
- 在 `build_context_storage_examples` 中按 OS 生成命令
- Windows 使用 PowerShell 命令（`Get-Content`）
- Linux/macOS 使用 Unix 命令（`cat`, `grep`）
- 路径格式化根据 OS 自动调整

---

### 8. 工具间协作缺少环境一致性保证

**问题描述**：
- HTTP 工具在容器存储输出
- Shell 工具在宿主机执行
- 无法读取彼此的输出

**影响范围**：
- 工具链断裂
- 需要手动复制文件
- 自动化流程失败

**严重程度**：高

**解决状态**：⚠️ 部分修复
- 当前 Shell 工具会尝试 Docker 优先，fallback 到 Host
- 但 HTTP 工具的存储位置不可控
- **待改进**：统一工具执行环境策略

---

## 已实施的修复

### 1. 历史文件隔离（`output_storage.rs`）
```rust
// 新增按 execution_id 隔离的历史存储
pub async fn store_history_in_container_with_id(
    sandbox: &DockerSandbox,
    history_content: &str,
    execution_id: Option<&str>,
) -> anyhow::Result<String>

pub async fn store_history_on_host(
    history_content: &str,
    execution_id: Option<&str>,
) -> anyhow::Result<String>

// 历史文件命名：history_{execution_id}.txt
// 避免跨会话泄露
```

### 2. 环境感知系统提示（`builder.rs`）
```rust
// 根据执行环境生成提示
let execution_context = resolve_execution_context().await;

// 提示中包含：
// - Environment: host/docker
// - OS: windows/macos/linux
// - Working Directory: 实际路径
// - Context Storage: 实际路径
// - Execution ID: 当前执行 ID
```

### 3. 按 OS 生成命令示例（`builder.rs`）
```rust
fn build_context_storage_examples(os_name: &str, context_dir: &str) -> String {
    if os_name.eq_ignore_ascii_case("windows") {
        // PowerShell 命令
        format!("Get-Content \"{0}\\history.txt\"", context_dir)
    } else {
        // Unix 命令
        format!("cat \"{0}\"/history.txt", context_dir)
    }
}
```

### 4. 清理函数增强（`output_storage.rs`）
```rust
// 支持按 execution_id 清理
pub async fn cleanup_container_context_with_id(
    sandbox: &DockerSandbox,
    execution_id: Option<&str>,
) -> anyhow::Result<()>

pub fn cleanup_host_context_with_id(
    execution_id: Option<&str>,
) -> anyhow::Result<()>
```

---

## 待改进项

### 1. 工具执行环境统一策略
**优先级**：高

**建议**：
- 在会话开始时确定执行环境（Host/Docker）
- 所有工具使用相同环境
- 避免工具间环境切换

**实施方案**：
```rust
// 在 ToolConfig 中增加 execution_environment 字段
pub struct ToolConfig {
    pub execution_environment: ExecutionEnvironment, // Host or Docker
    // ...
}

// 所有工具从配置读取环境，不再动态选择
```

---

### 2. 路径映射说明
**优先级**：中

**建议**：
- 在系统提示中说明宿主机 ↔ 容器路径映射
- 文档附件处理时显示映射关系

**实施方案**：
```rust
// 在系统提示中增加：
[Path Mapping]
- Host: C:\Users\user\file.txt
- Container: /workspace/uploads/file.txt
- Use container path when executing tools in Docker
```

---

### 3. 执行完成时自动清理
**优先级**：中

**建议**：
- 在 `agent_execute` 完成时调用清理函数
- 根据执行环境选择清理方法

**实施方案**：
```rust
// 在 agent_execute 的 finally 块中
match execution_context.env {
    ExecutionEnvironment::Docker => {
        cleanup_container_context_with_id(&sandbox, Some(&execution_id)).await?;
    }
    ExecutionEnvironment::Host => {
        cleanup_host_context_with_id(Some(&execution_id))?;
    }
}
```

---

### 4. 工具输出环境标注
**优先级**：低

**建议**：
- 在工具返回的 summary 中标注执行环境
- 帮助 LLM 理解实际执行位置

**实施方案**：
```rust
// 在 StorageResult 中增加环境信息
pub enum StorageResult {
    Stored {
        container_path: String,
        summary: String,
        execution_env: ExecutionEnvironment, // 新增
        // ...
    },
}
```

---

## 测试建议

### 1. 环境切换测试
- 测试 Docker 可用 → 不可用的 fallback
- 验证路径提示是否正确切换

### 2. 跨会话隔离测试
- 创建多个会话
- 验证历史文件是否隔离
- 检查清理后是否有残留

### 3. 跨平台测试
- Windows/macOS/Linux 分别测试
- 验证命令示例是否可用
- 检查路径格式是否正确

### 4. 工具协作测试
- HTTP 工具 → Shell 工具读取输出
- 验证在不同环境下是否能正常协作

---

## 总结

通过本次修复，主要解决了以下问题：
1. ✅ 历史文件跨会话泄露（严重）
2. ✅ 环境路径混淆（高）
3. ✅ OS 命令不兼容（中）
4. ✅ 清理逻辑缺失（中）

仍需改进：
1. ⚠️ 工具执行环境统一策略
2. ⚠️ 路径映射用户提示
3. ⚠️ 自动清理集成

**建议优先级**：
1. 集成自动清理逻辑（防止磁盘空间浪费）
2. 实施工具执行环境统一策略（提高稳定性）
3. 增加路径映射说明（改善用户体验）
