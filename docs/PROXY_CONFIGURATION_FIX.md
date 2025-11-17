# 代理配置组件修复

## 修复日期
2025-11-14

## 问题描述

1. **修改代理监听未生效**: 保存配置后，界面没有更新，导致用户看不到最新的配置
2. **选中行无法编辑**: 只能通过点击"编辑"按钮编辑，不支持双击行直接编辑

## 问题分析

### 问题1：配置未生效

**原因**:
- `watch(refreshTrigger)` 调用了不存在的 `loadConfig()` 函数
- `saveConfiguration()` 保存后没有重新加载配置
- 导致界面显示的数据与实际保存的数据不同步

### 问题2：选中行无法编辑

**原因**:
- 表格行没有添加双击事件处理
- 缺少 `editListenerByIndex()` 函数来直接编辑指定索引的监听器

## 修复方案

### 1. 添加 `loadConfig()` 函数

创建一个通用的配置加载函数，用于：
- 初始化时加载配置
- 保存后重新加载配置
- 父组件触发刷新时加载配置

```typescript
const loadConfig = async () => {
  try {
    console.log('[ProxyConfiguration] Loading config...')
    // 加载代理配置
    const configResponse = await invoke<any>('get_proxy_config')
    if (configResponse.success && configResponse.data) {
      proxyConfig.value = configResponse.data
      requestBodySizeMB.value = Math.round(configResponse.data.max_request_body_size / (1024 * 1024))
      responseBodySizeMB.value = Math.round(configResponse.data.max_response_body_size / (1024 * 1024))
      proxyListeners.value[0].interface = `127.0.0.1:${configResponse.data.start_port}`
    }
    
    // 检查代理实际运行状态
    const statusResponse = await invoke<any>('get_proxy_status')
    if (statusResponse.success && statusResponse.data) {
      const isRunning = statusResponse.data.running
      const actualPort = statusResponse.data.port
      
      // 同步运行状态到界面
      if (isRunning && actualPort > 0) {
        const listenerIndex = proxyListeners.value.findIndex(
          l => l.interface === `127.0.0.1:${actualPort}`
        )
        if (listenerIndex !== -1) {
          proxyListeners.value[listenerIndex].running = true
        } else {
          proxyListeners.value[0].interface = `127.0.0.1:${actualPort}`
          proxyListeners.value[0].running = true
        }
        console.log(`[ProxyConfiguration] Proxy is running on port ${actualPort}`)
      } else {
        proxyListeners.value.forEach(listener => {
          listener.running = false
        })
        console.log('[ProxyConfiguration] Proxy is not running')
      }
    }
  } catch (error) {
    console.error('[ProxyConfiguration] Failed to load config or status:', error)
    proxyListeners.value.forEach(listener => {
      listener.running = false
    })
  }
}
```

### 2. 修改 `saveConfiguration()` 函数

保存后自动重新加载配置：

```typescript
const saveConfiguration = async () => {
  try {
    console.log('[ProxyConfiguration] Saving configuration...', proxyConfig.value)
    
    const response = await invoke<any>('save_proxy_config', { 
      config: proxyConfig.value 
    })
    
    if (response.success) {
      dialog.toast.success('配置已保存，重启代理后生效')
      // ✅ 重新加载配置以同步界面
      await loadConfig()
    } else {
      throw new Error(response.error || '保存失败')
    }
  } catch (error: any) {
    console.error('[ProxyConfiguration] Failed to save configuration:', error)
    dialog.toast.error(`保存配置失败: ${error}`)
  }
}
```

### 3. 添加双击编辑功能

#### 3.1 添加 `editListenerByIndex()` 函数

```typescript
const editListenerByIndex = (index: number) => {
  const listener = proxyListeners.value[index]
  
  // 解析接口字符串
  const [host, portStr] = listener.interface.split(':')
  const port = parseInt(portStr)
  
  // 填充编辑表单
  editingIndex.value = index
  editingListener.value = {
    host,
    port,
    certificate: listener.certificate,
    tlsProtocols: listener.tlsProtocols,
    supportHTTP2: listener.supportHTTP2,
    invisible: listener.invisible,
    redirect: listener.redirect
  }
  
  // 打开对话框
  editDialogRef.value?.showModal()
}
```

#### 3.2 修改 `editListener()` 函数

复用 `editListenerByIndex()` 函数：

```typescript
const editListener = () => {
  if (selectedListeners.value.length !== 1) {
    dialog.toast.warning('请选择一个监听器进行编辑')
    return
  }
  
  const index = selectedListeners.value[0]
  editListenerByIndex(index)
}
```

#### 3.3 在表格行上添加双击事件

```vue
<tr 
  v-for="(listener, index) in proxyListeners" 
  :key="index"
  :class="{ 'bg-base-200': selectedListeners.includes(index) }"
  @dblclick="editListenerByIndex(index)"
  class="cursor-pointer hover:bg-base-300 transition-colors"
>
```

### 4. 修改 `onMounted()` 函数

简化初始化逻辑，复用 `loadConfig()` 函数：

```typescript
onMounted(async () => {
  await loadConfig()
  
  // 监听代理状态变化事件
  const unlisten = await listen('proxy:status', (event: any) => {
    // ... 事件处理逻辑
  })
  
  onUnmounted(() => {
    unlisten()
  })
})
```

## 改进效果

### 1. 配置同步

- ✅ 保存配置后自动刷新界面
- ✅ 父组件触发刷新时正确加载配置
- ✅ 界面显示与实际配置保持一致

### 2. 用户体验

- ✅ 支持双击行直接编辑
- ✅ 鼠标悬停时显示高亮效果
- ✅ 光标变为手型，提示可点击
- ✅ 保留原有的选择编辑功能

### 3. 日志输出

添加了详细的日志，便于调试：
- `[ProxyConfiguration] Loading config...`
- `[ProxyConfiguration] Saving configuration...`
- `[ProxyConfiguration] Proxy is running on port XXX`
- `[ProxyConfiguration] Proxy is not running`
- `[ProxyConfiguration] Refresh triggered by parent`

## 测试建议

### 测试场景1：保存配置

1. 修改代理配置（如起始端口、MITM 设置等）
2. 点击"保存配置"按钮
3. 验证：
   - 提示"配置已保存，重启代理后生效"
   - 界面显示的配置与修改后的一致
   - 监听器列表正确更新

### 测试场景2：双击编辑

1. 在监听器列表中双击任意行
2. 验证：
   - 编辑对话框正确打开
   - 表单中填充了该监听器的当前配置
   - 修改并保存后，列表正确更新

### 测试场景3：选择编辑

1. 勾选一个监听器
2. 点击"编辑"按钮
3. 验证：功能与双击编辑一致

### 测试场景4：页面刷新

1. 切换到其他页面
2. 切换回代理配置页面
3. 验证：
   - 配置正确加载
   - 代理运行状态正确显示
   - 监听器列表正确显示

## 相关文件

- `src/components/ProxyConfiguration.vue` - 主要修改文件
- `src/views/PassiveScan.vue` - 父组件，提供 `refreshTrigger`

## 注意事项

1. **配置生效时机**: 修改配置后需要重启代理才能生效
2. **运行状态同步**: 组件会自动监听代理状态事件，保持界面与实际状态同步
3. **错误处理**: 所有操作都有完善的错误处理和用户提示

---

**修复完成**: 2025-11-14
**测试状态**: 待测试

