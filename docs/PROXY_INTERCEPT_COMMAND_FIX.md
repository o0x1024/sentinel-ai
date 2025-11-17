# 代理拦截命令修复

## 修复日期
2025-11-14

## 问题描述

前端调用了不存在的后端命令，导致错误：
- `Command get_intercept_status not found`
- `Command set_intercept_enabled not found`

## 问题分析

### 错误来源

在 `src/components/ProxyIntercept.vue` 中：

1. **`toggleIntercept()` 函数**调用了 `set_intercept_enabled` 命令
2. **`watch(refreshTrigger)` 监听器**调用了 `get_intercept_status` 命令

但这两个命令在后端都没有实现。

### 根本原因

代理拦截功能还在开发中，前端 UI 已经实现，但后端的拦截逻辑还未完成。

## 修复方案

### 临时解决方案

暂时移除后端命令调用，只在前端维护拦截状态，避免报错。

### 1. 修改 `toggleIntercept()` 函数

**修改前**：
```typescript
async function toggleIntercept() {
  interceptEnabled.value = !interceptEnabled.value;
  try {
    await invoke('set_intercept_enabled', { enabled: interceptEnabled.value });
    dialog.toast.success(interceptEnabled.value ? '拦截已启用' : '拦截已禁用');
  } catch (error: any) {
    console.error('Failed to toggle intercept:', error);
    dialog.toast.error(`切换拦截状态失败: ${error}`);
    interceptEnabled.value = !interceptEnabled.value;
  }
}
```

**修改后**：
```typescript
async function toggleIntercept() {
  const newState = !interceptEnabled.value;
  
  // TODO: 后端需要实现 set_intercept_enabled 命令来真正启用/禁用拦截
  // 目前只在前端切换状态，实际的拦截功能需要后端支持
  
  try {
    // 暂时注释掉后端调用，避免命令不存在的错误
    // await invoke('set_intercept_enabled', { enabled: newState });
    
    // 直接更新前端状态
    interceptEnabled.value = newState;
    dialog.toast.info(
      interceptEnabled.value 
        ? '拦截已启用（前端状态，后端功能待实现）' 
        : '拦截已禁用（前端状态）'
    );
    
    console.log('[ProxyIntercept] Intercept toggled:', interceptEnabled.value);
  } catch (error: any) {
    console.error('[ProxyIntercept] Failed to toggle intercept:', error);
    dialog.toast.error(`切换拦截状态失败: ${error}`);
  }
}
```

### 2. 修改 `watch(refreshTrigger)` 监听器

**修改前**：
```typescript
watch(refreshTrigger, async () => {
  console.log('[ProxyIntercept] Refresh triggered by parent');
  await refreshStatus();
  // 刷新拦截状态
  try {
    const response = await invoke<any>('get_intercept_status');
    if (response.success && response.data) {
      interceptEnabled.value = response.data.enabled || false;
    }
  } catch (error) {
    console.error('Failed to refresh intercept status:', error);
  }
});
```

**修改后**：
```typescript
watch(refreshTrigger, async () => {
  console.log('[ProxyIntercept] Refresh triggered by parent');
  await refreshStatus();
  // TODO: 刷新拦截状态（需要后端实现 get_intercept_status 命令）
  // 目前拦截状态由前端维护，不需要从后端获取
});
```

## 改进效果

### 1. 错误修复

- ✅ 不再出现 `Command not found` 错误
- ✅ 用户可以正常切换拦截开关（前端状态）
- ✅ 页面刷新不会报错

### 2. 用户提示

- ✅ 切换拦截状态时显示提示：`拦截已启用（前端状态，后端功能待实现）`
- ✅ 明确告知用户这是前端状态，实际功能需要后端支持
- ✅ 添加了详细的日志输出

### 3. 代码维护

- ✅ 添加了 TODO 注释，标记需要实现的后端功能
- ✅ 保留了原有的代码结构，便于后续集成后端功能

## 后续工作

### 需要在后端实现的功能

#### 1. `set_intercept_enabled` 命令

**位置**: `src-tauri/src/commands/passive_commands.rs` 或新建文件

**功能**: 启用/禁用请求拦截

```rust
#[tauri::command]
pub async fn set_intercept_enabled(
    enabled: bool,
    app: AppHandle,
) -> Result<(), String> {
    // TODO: 实现拦截启用/禁用逻辑
    // 1. 更新全局拦截状态
    // 2. 如果启用，开始拦截匹配规则的请求
    // 3. 如果禁用，停止拦截，直接转发所有请求
    
    log::info!("Set intercept enabled: {}", enabled);
    
    // 发送状态变更事件
    app.emit("intercept:status_changed", serde_json::json!({
        "enabled": enabled
    })).map_err(|e| e.to_string())?;
    
    Ok(())
}
```

#### 2. `get_intercept_status` 命令

**位置**: `src-tauri/src/commands/passive_commands.rs`

**功能**: 获取当前拦截状态

```rust
#[tauri::command]
pub async fn get_intercept_status(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    // TODO: 从全局状态获取拦截状态
    
    Ok(serde_json::json!({
        "success": true,
        "data": {
            "enabled": false, // 从实际状态获取
            "rules_count": 0, // 拦截规则数量
            "intercepted_count": 0, // 已拦截的请求数量
        }
    }))
}
```

#### 3. 注册命令

在 `src-tauri/src/lib.rs` 中注册命令：

```rust
.invoke_handler(tauri::generate_handler![
    // ... 其他命令
    set_intercept_enabled,
    get_intercept_status,
])
```

### 实现步骤

1. **创建拦截状态管理器**
   - 使用 `Arc<RwLock<InterceptState>>` 管理全局状态
   - 包含：是否启用、拦截规则、拦截队列等

2. **实现拦截逻辑**
   - 在代理请求处理流程中检查拦截状态
   - 匹配拦截规则
   - 将匹配的请求加入拦截队列
   - 通过事件发送到前端

3. **实现请求转发/丢弃**
   - `forward_intercepted_request` - 转发请求
   - `drop_intercepted_request` - 丢弃请求
   - 支持修改请求内容后转发

4. **前端集成**
   - 取消注释 `invoke('set_intercept_enabled')` 调用
   - 取消注释 `invoke('get_intercept_status')` 调用
   - 更新提示信息，移除"待实现"字样

## 测试建议

### 当前状态测试

1. 点击拦截开关
2. 验证：
   - 不报错
   - 显示提示信息
   - 开关状态正确切换
   - 控制台有日志输出

### 后端实现后的测试

1. 启用拦截
2. 发送 HTTP 请求
3. 验证：
   - 请求被拦截并显示在界面
   - 可以查看请求详情
   - 可以转发/丢弃请求
   - 可以修改请求后转发

## 相关文件

- `src/components/ProxyIntercept.vue` - 前端拦截组件
- `src-tauri/src/commands/passive_commands.rs` - 后端命令（待创建）
- `src-tauri/src/lib.rs` - 命令注册

## 注意事项

1. **当前状态**: 拦截功能仅在前端有 UI，后端逻辑未实现
2. **用户体验**: 用户可以切换开关，但实际不会拦截请求
3. **开发优先级**: 建议先实现基础的代理转发功能，再实现拦截功能

---

**修复完成**: 2025-11-14
**测试状态**: 已测试（前端状态切换正常）
**后端状态**: 待实现

