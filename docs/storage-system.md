# 存储系统设计文档

## 概述

Sentinel AI 的存储系统提供了可靠的、跨应用重启的数据持久化能力。系统基于 `localStorage`，并在此基础上提供了更强大的功能，包括缓存管理、版本控制、过期清理等。

## 核心特性

### 1. 持久化存储
- ✅ **跨应用重启**：数据在应用重启后仍然有效
- ✅ **内存缓存**：自动使用内存缓存提高读取性能
- ✅ **异步 API**：所有操作都是异步的，便于未来扩展
- ✅ **类型安全**：完整的 TypeScript 类型支持

### 2. 缓存管理
- ✅ **版本控制**：自动检测缓存版本，不兼容时自动清理
- ✅ **过期机制**：支持设置缓存过期时间
- ✅ **自动清理**：应用启动时自动清理过期数据

### 3. 错误处理
- ✅ **异常捕获**：所有操作都有完善的错误处理
- ✅ **日志记录**：详细的日志帮助调试
- ✅ **降级方案**：缓存失败时不影响应用正常运行

## 架构设计

```
┌─────────────────────────────────────────┐
│          应用层 (Components)              │
│  PluginStoreSection.vue, etc.           │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│         存储服务层 (Services)              │
│  PluginStoreCache, ViewModeStorage      │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│       核心存储层 (StorageService)         │
│  Memory Cache + localStorage            │
└─────────────────────────────────────────┘
```

## API 文档

### StorageService

核心存储服务类，提供基础的存储操作。

```typescript
import { storage } from '@/services/storage'

// 存储数据
await storage.setItem('key', { data: 'value' })

// 读取数据
const data = await storage.getItem('key')

// 删除数据
await storage.removeItem('key')

// 检查是否存在
const exists = await storage.hasItem('key')

// 获取所有键
const keys = await storage.keys()

// 清空所有数据
await storage.clear()
```

### PluginStoreCache

插件商店专用的缓存管理类。

```typescript
import { PluginStoreCache } from '@/services/storage'

// 保存插件列表到缓存
await PluginStoreCache.save(plugins)

// 从缓存加载插件列表
const cache = await PluginStoreCache.load()
if (cache) {
  console.log('Plugins:', cache.plugins)
  console.log('Cache time:', new Date(cache.timestamp))
}

// 检查缓存是否有效
const isValid = await PluginStoreCache.isValid()

// 获取缓存时间
const timestamp = await PluginStoreCache.getCacheTime()

// 清除缓存
await PluginStoreCache.clear()
```

**缓存策略：**
- 缓存时长：5分钟
- 版本号：1.0.0
- 过期后自动失效，需要重新获取

### ViewModeStorage

视图模式持久化类。

```typescript
import { ViewModeStorage } from '@/services/storage'

// 保存视图模式
await ViewModeStorage.save('card')

// 加载视图模式（默认返回 'list'）
const mode = await ViewModeStorage.load()
```

### 初始化

在应用启动时自动初始化存储系统。

```typescript
import { initializeStorage } from '@/services/storage'

// 在 main.ts 中调用
await initializeStorage()
```

**初始化操作：**
1. 检查存储是否可用
2. 清理超过 24 小时的过期数据
3. 记录存储统计信息

## 使用示例

### 插件商店缓存

```vue
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { PluginStoreCache } from '@/services/storage'

const plugins = ref([])
const loading = ref(false)

// 加载插件
const loadPlugins = async () => {
  // 先尝试从缓存加载
  const cache = await PluginStoreCache.load()
  if (cache) {
    plugins.value = cache.plugins
    console.log('Loaded from cache')
    return
  }

  // 缓存失效，从服务器获取
  loading.value = true
  try {
    const response = await fetchPlugins()
    plugins.value = response.plugins
    
    // 保存到缓存
    await PluginStoreCache.save(response.plugins)
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadPlugins()
})
</script>
```

### 视图模式持久化

```vue
<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { ViewModeStorage } from '@/services/storage'

const viewMode = ref<'list' | 'card'>('list')

// 加载保存的视图模式
onMounted(async () => {
  viewMode.value = await ViewModeStorage.load()
})

// 自动保存视图模式变化
watch(viewMode, async (newMode) => {
  await ViewModeStorage.save(newMode)
})
</script>
```

## 数据结构

### 插件缓存数据结构

```typescript
interface PluginStoreCacheData {
  plugins: any[]        // 插件列表
  timestamp: number     // 缓存时间戳
  version: string       // 缓存版本号
}
```

## 性能优化

1. **内存缓存**
   - 首次读取后缓存在内存中
   - 避免重复的 JSON 解析
   - 显著提高读取性能

2. **按需加载**
   - 只在需要时才读取数据
   - 支持懒加载策略

3. **批量清理**
   - 应用启动时批量清理过期数据
   - 避免存储空间浪费

## 最佳实践

### 1. 使用专用的缓存类

❌ **不推荐**：直接使用 localStorage
```typescript
localStorage.setItem('plugins', JSON.stringify(plugins))
```

✅ **推荐**：使用专用的缓存类
```typescript
await PluginStoreCache.save(plugins)
```

### 2. 处理缓存失效

```typescript
const cache = await PluginStoreCache.load()
if (cache) {
  // 使用缓存
  return cache.plugins
}

// 缓存失效，获取新数据
const data = await fetchFromServer()
await PluginStoreCache.save(data)
return data
```

### 3. 错误处理

```typescript
try {
  await storage.setItem('key', value)
} catch (error) {
  console.error('Failed to save:', error)
  // 降级处理：应用继续运行，只是不保存缓存
}
```

## 测试

运行存储系统的单元测试：

```bash
# 运行所有测试
yarn test

# 只运行存储测试
yarn test storage.test.ts

# 监视模式
yarn test:watch
```

## 故障排查

### 问题：缓存在应用重启后丢失

**原因：** localStorage 被清理或禁用

**解决方案：**
1. 检查浏览器/WebView 设置
2. 确认存储配额未超限
3. 查看控制台错误日志

### 问题：缓存总是过期

**原因：** 系统时间不准确或缓存时间设置过短

**解决方案：**
1. 检查系统时间
2. 调整 `CACHE_DURATION` 常量
3. 查看缓存时间戳日志

### 问题：存储空间不足

**原因：** 缓存数据过大

**解决方案：**
1. 运行 `initializeStorage()` 清理过期数据
2. 减少缓存的数据量
3. 使用压缩（如果需要）

## 未来计划

- [ ] 支持 IndexedDB（更大存储空间）
- [ ] 数据压缩（减少存储占用）
- [ ] 加密敏感数据
- [ ] 云同步支持
- [ ] 更精细的缓存策略（LRU 等）

## 相关文件

- `src/services/storage.ts` - 核心存储服务
- `src/services/__tests__/storage.test.ts` - 单元测试
- `src/components/PluginManagement/PluginStoreSection.vue` - 使用示例
- `src/main.ts` - 初始化配置

## 参考资料

- [Web Storage API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API)
- [Tauri Storage](https://tauri.app/v1/guides/features/storage/)
- [localStorage 最佳实践](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)

