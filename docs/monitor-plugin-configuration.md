# 监控插件配置功能

## 概述

监控插件配置功能允许用户为每种监控类型手动指定使用的插件，支持主插件和备用插件机制，提高监控系统的灵活性和容错性。

## 功能特性

### 1. 四种监控类型

- **DNS记录监控**: 监控域名DNS记录变化
- **SSL证书监控**: 监控SSL/TLS证书变更和过期
- **内容变更监控**: 监控网页内容变化
- **API端点监控**: 监控API端点的新增和删除

### 2. 插件配置选项

每种监控类型都支持：
- **主插件**: 默认使用的插件
- **备用插件**: 主插件失效时自动切换的备份插件（支持多个）
- **自定义参数**: 为插件提供额外的配置参数

## 使用方法

### 创建监控任务

1. 进入 Bug Bounty 模块的 "监控" 标签页
2. 点击 "创建任务" 按钮
3. 填写任务基本信息：
   - 任务名称
   - 检查间隔

### 配置监控类型和插件

对于每种监控类型：

1. **启用监控**
   - 勾选相应的监控类型复选框

2. **添加插件配置**
   - 点击 "添加插件" 按钮
   - 从下拉列表中选择主插件
   - （可选）点击 "添加备用" 添加备用插件

3. **配置多个插件组**
   - 可以为同一监控类型配置多个插件组
   - 每个插件组独立运行

### 插件配置示例

#### DNS监控配置
```
主插件: subdomain_enumerator
备用插件: 
  - dns_resolver
```

#### SSL证书监控配置
```
主插件: cert_monitor
备用插件: (无)
```

#### 内容监控配置
```
主插件: content_monitor
备用插件: 
  - http_prober
```

#### API监控配置
```
主插件: api_monitor
备用插件:
  - js_analyzer
  - js_link_finder
```

## API接口

### 获取可用插件列表

```typescript
const plugins = await invoke('monitor_get_available_plugins')
// 返回: Array<{
//   id: string,
//   name: string,
//   category: string,
//   monitor_type: string, // 'dns' | 'cert' | 'content' | 'api'
//   description?: string,
//   is_available: boolean
// }>
```

### 测试插件可用性

```typescript
const isAvailable = await invoke('monitor_test_plugin', {
  plugin_id: 'subdomain_enumerator'
})
```

### 更新任务插件配置

```typescript
await invoke('monitor_update_task_plugins', {
  task_id: 'task-uuid',
  request: {
    monitor_type: 'dns',
    plugins: [
      {
        plugin_id: 'subdomain_enumerator',
        fallback_plugins: ['dns_resolver'],
        plugin_params: {}
      }
    ]
  }
})
```

## 数据结构

### MonitorPluginConfig

```typescript
interface MonitorPluginConfig {
  plugin_id: string           // 主插件ID
  fallback_plugins: string[]  // 备用插件ID列表
  plugin_params: any          // 自定义参数
}
```

### ChangeMonitorConfig

```typescript
interface ChangeMonitorConfig {
  // DNS监控
  enable_dns_monitoring: boolean
  dns_plugins: MonitorPluginConfig[]
  
  // 证书监控
  enable_cert_monitoring: boolean
  cert_plugins: MonitorPluginConfig[]
  
  // 内容监控
  enable_content_monitoring: boolean
  content_plugins: MonitorPluginConfig[]
  
  // API监控
  enable_api_monitoring: boolean
  api_plugins: MonitorPluginConfig[]
  
  // 其他配置
  auto_trigger_enabled: boolean
  auto_trigger_min_severity: string
  check_interval_secs: number
}
```

## 优势

1. **灵活性**: 用户可以根据需求选择最适合的插件
2. **容错性**: 主插件失效时自动切换到备用插件
3. **可扩展性**: 支持添加新插件而不影响现有配置
4. **多样性**: 可以为同一监控类型配置多个不同的插件
5. **独立性**: 插件改名或删除不会导致整个监控系统失效

## 注意事项

1. 至少需要配置一个主插件才能启用监控
2. 备用插件是可选的，但建议配置以提高可靠性
3. 如果所有配置的插件都不可用，该监控类型将暂停工作
4. 插件执行失败会记录在日志中，便于排查问题

## 故障排查

### 问题：监控任务不执行

**解决方案：**
1. 检查插件是否可用：使用 `monitor_test_plugin` 命令
2. 查看调度器状态：确认监控调度器正在运行
3. 检查任务配置：确认至少配置了一个可用插件

### 问题：插件执行失败

**解决方案：**
1. 检查插件参数是否正确
2. 查看后端日志了解详细错误信息
3. 尝试配置备用插件

### 问题：找不到合适的插件

**解决方案：**
1. 使用 `monitor_get_available_plugins` 查看所有可用插件
2. 根据 `monitor_type` 字段筛选合适的插件
3. 如需要，可以开发自定义插件

## 未来改进

- [ ] 支持插件性能监控和统计
- [ ] 支持插件权重配置（优先级）
- [ ] 支持条件触发（根据资产类型选择不同插件）
- [ ] 支持插件参数模板
- [ ] 支持批量导入/导出插件配置
