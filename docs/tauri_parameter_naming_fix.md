# Tauri 参数命名约定问题修复

## 问题描述

前端调用 `stop_execution` 命令时报错：

```
invalid args `executionId` for command `stop_execution`: 
command stop_execution missing required key executionId
```

## 根本原因

**命名约定不匹配**：
- **前端**：使用 camelCase `executionId`
- **后端**：使用 snake_case `execution_id`

虽然 Tauri 在某些情况下会自动转换命名约定，但在这个场景下没有正确转换，导致参数匹配失败。

## Tauri 命名约定规则

### Rust 后端（snake_case）
```rust
#[tauri::command]
pub async fn stop_execution(
    execution_id: String,  // ← snake_case
    app: AppHandle,
) -> Result<(), String> {
    // ...
}
```

### 前端调用（应使用 snake_case）
```typescript
// ✅ 正确
await invoke('stop_execution', {
    execution_id: currentConversationId.value,
})

// ❌ 错误
await invoke('stop_execution', {
    executionId: currentConversationId.value,  // camelCase 不匹配
})
```

## 解决方案

### 修改前端参数名为 snake_case

**文件**: `/src/components/AIChat.vue`

**修改前（第1069-1072行）**:
```typescript
try {
  await invoke('stop_execution', {
      executionId: currentConversationId.value,  // ❌ camelCase
    })
```

**修改后**:
```typescript
try {
  await invoke('stop_execution', {
      execution_id: currentConversationId.value,  // ✅ snake_case
    })
```

## 为什么选择这个方案

### 方案对比

| 方案 | 优点 | 缺点 |
|------|------|------|
| **修改前端为 snake_case** | • 符合 Rust 约定<br>• 与其他命令一致<br>• 不需要额外配置 | • 前端代码风格不统一 |
| 修改后端为 camelCase | • 前端代码风格统一 | • 违反 Rust 约定<br>• 需要修改所有相关代码 |
| 使用 serde rename | • 两边都保持原风格 | • 增加配置复杂度<br>• 不是 Tauri 推荐方式 |

**选择方案1**：修改前端为 snake_case，因为：
1. Tauri 官方推荐在前端使用 snake_case 调用 Rust 命令
2. 项目中其他命令调用已经使用 snake_case（如 `execution_id`）
3. 保持代码一致性

## 验证其他命令调用

检查项目中其他 Tauri 命令调用，确保都使用 snake_case：

```typescript
// ✅ 正确的命名约定
await invoke('stop_execution', { execution_id: id })
await invoke('dispatch_query', { 
    query: text,
    conversation_id: convId,
    message_id: msgId,
    execution_id: execId
})
```

## 最佳实践

### 1. Tauri 命令参数命名规范

**始终使用 snake_case**：
```typescript
// ✅ 推荐
await invoke('my_command', {
    user_id: '123',
    task_name: 'test',
    is_active: true
})

// ❌ 避免
await invoke('my_command', {
    userId: '123',
    taskName: 'test',
    isActive: true
})
```

### 2. 内部变量可以使用 camelCase

```typescript
// 内部变量使用 camelCase
const executionId = `exec_${Date.now()}`
const conversationId = 'conv_123'

// 但传递给 Tauri 时转换为 snake_case
await invoke('stop_execution', {
    execution_id: executionId,  // 参数名用 snake_case
})
```

### 3. 类型定义建议

```typescript
// 定义接口时可以使用 snake_case 匹配后端
interface StopExecutionParams {
    execution_id: string;
}

// 或者使用映射
interface StopExecutionParams {
    executionId: string;
}

function callStopExecution(params: StopExecutionParams) {
    return invoke('stop_execution', {
        execution_id: params.executionId,  // 显式映射
    })
}
```

## 修改的文件

- `/src/components/AIChat.vue` - 修改 `stop_execution` 调用参数

## 测试验证

### 测试步骤
1. 启动应用
2. 开始一个对话
3. 点击"停止"按钮
4. 验证执行被正确取消

### 预期结果
- ✅ 不再报错 "missing required key executionId"
- ✅ 执行被成功取消
- ✅ 日志显示 "🛑 Stopping execution: {id}"

## 相关问题排查

如果遇到类似的参数匹配错误：

1. **检查命名约定**：
   ```
   Error: missing required key xxxYyy
   ```
   → 将 `xxxYyy` 改为 `xxx_yyy`

2. **检查参数类型**：
   ```
   Error: invalid type: expected string, found number
   ```
   → 确保前端传递的类型与后端定义一致

3. **检查必需参数**：
   ```
   Error: missing required key xxx
   ```
   → 确保所有必需参数都已传递

## 总结

通过将前端的 `executionId` 改为 `execution_id`，解决了 Tauri 命令参数匹配问题。这是 Tauri 框架的命名约定要求，前端调用 Rust 命令时应使用 snake_case 参数名。

**关键点**：
- ✅ Tauri 命令参数使用 snake_case
- ✅ 保持与项目其他命令调用一致
- ✅ 遵循 Tauri 官方推荐实践

