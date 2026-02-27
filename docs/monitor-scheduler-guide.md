# 资产监控调度器使用指南

## 概述

资产监控调度器是 BugBounty 模块的核心功能之一，提供自动化的资产变更监控和工作流触发能力。

## 功能特性

### 1. 自动化监控
- **DNS 记录监控**: 检测子域名新增、删除、IP 变更
- **SSL 证书监控**: 监控证书更新、过期、指纹变化
- **内容变更监控**: 检测网页内容、响应头变化
- **API 端点监控**: 发现新增或删除的 API 端点

### 2. 灵活的调度策略
- 支持多种检查间隔：每小时、每 6 小时、每 12 小时、每天、每周
- 每个项目可以创建多个监控任务
- 独立的启用/禁用控制

### 3. 自动工作流触发
- 检测到变更时自动生成 ChangeEvent
- 根据严重程度自动触发绑定的工作流
- 支持手动触发立即执行

## 快速开始

### 步骤 1: 创建项目并配置

1. 在 **项目管理** 标签页创建一个 Bug Bounty 项目
2. 添加测试范围 (Scope)，例如：`*.example.com`
3. 绑定工作流模板到项目

### 步骤 2: 创建监控任务

1. 切换到 **监控调度** 标签页
2. 点击 **创建默认任务** 按钮，系统会自动创建两个监控任务：
   - **DNS & Certificate Monitor** (每 6 小时)
   - **Content & API Monitor** (每 24 小时)

或者手动创建自定义任务：

```
任务名称: 高频 DNS 监控
检查间隔: 每小时
监控类型:
  ✓ DNS 记录监控
  ✓ SSL 证书监控
  ✗ 内容变更监控
  ✗ API 端点监控
自动触发: ✓ 检测到变更时自动触发工作流
```

### 步骤 3: 启动调度器

1. 点击 **启动** 按钮启动监控调度器
2. 调度器将按照设定的间隔自动执行监控任务
3. 检测到变更时会：
   - 生成 ChangeEvent 记录
   - 如果开启了自动触发，会自动执行绑定的工作流

### 步骤 4: 查看监控结果

- **统计面板**: 查看总运行次数、检测到的事件数量
- **变更监控** 标签页: 查看所有检测到的变更事件
- **工作流** 标签页: 查看自动触发的工作流执行记录

## 监控任务配置

### DNS 记录监控

监控内容：
- 新增子域名
- 删除的子域名
- IP 地址变更
- DNS 记录类型变化

触发条件：
- 新增记录 → High severity → 自动触发
- 删除记录 → Medium severity
- IP 变更 → Medium severity

### SSL 证书监控

监控内容：
- 证书指纹变化
- 证书过期时间
- 证书颁发者变更

触发条件：
- 证书更新 → Medium severity
- 即将过期 → High severity

### 内容变更监控

监控内容：
- 页面内容哈希
- 响应头变化
- 状态码变化

触发条件：
- 内容变化 → Low severity
- 状态码变化 → Medium severity

### API 端点监控

监控内容：
- 新增 API 端点
- 删除的端点
- 参数变化

触发条件：
- 新增端点 → High severity → 自动触发
- 删除端点 → Low severity

## 工作流集成

### 自动触发流程

```
1. 监控任务执行
   ↓
2. 检测到变更 (例如：新增子域名)
   ↓
3. 生成 ChangeEvent (severity: High)
   ↓
4. 检查是否开启自动触发
   ↓
5. 查找绑定的工作流 (auto_run_on_change: true)
   ↓
6. 自动执行工作流
   ↓
7. 工作流产生 Findings
   ↓
8. 通知用户
```

### 推荐的工作流配置

**场景 1: 新子域名发现**
```
监控任务: DNS Monitor (每 6 小时)
  ↓
检测到: 新增子域名 api.example.com
  ↓
自动触发工作流:
  1. HTTP 探测 → 确认存活
  2. 技术指纹识别 → 识别技术栈
  3. 端口扫描 → 发现开放端口
  4. 漏洞扫描 → 检测常见漏洞
```

**场景 2: 证书变更**
```
监控任务: Certificate Monitor (每 12 小时)
  ↓
检测到: SSL 证书更新
  ↓
手动审查: 检查是否有安全问题
```

**场景 3: 新 API 端点**
```
监控任务: API Monitor (每天)
  ↓
检测到: 新增 /api/v2/admin 端点
  ↓
自动触发工作流:
  1. API 安全扫描
  2. 权限测试
  3. SQL 注入检测
```

## 最佳实践

### 1. 监控频率设置

- **高价值目标**: 每小时检查一次
- **一般目标**: 每 6-12 小时检查一次
- **低优先级**: 每天或每周检查一次

### 2. 资源优化

- 避免同时运行过多监控任务
- 合理设置检查间隔，避免频繁请求
- 使用速率限制保护目标服务器

### 3. 告警策略

- High/Critical 变更 → 自动触发工作流
- Medium 变更 → 生成事件，手动审查
- Low 变更 → 仅记录，定期回顾

### 4. 数据管理

- 定期清理已处理的变更事件
- 保留重要的历史快照
- 导出关键发现到报告

## 故障排查

### 调度器无法启动

**问题**: 点击启动按钮后没有反应

**解决方案**:
1. 检查是否有监控任务
2. 查看浏览器控制台错误信息
3. 重启应用

### 未检测到变更

**问题**: 监控任务运行但没有检测到变更

**原因**:
1. 没有进行过首次扫描，缺少基线快照
2. 监控配置未启用相应的监控类型
3. 资产确实没有变化

**解决方案**:
1. 手动运行一次完整的侦察工作流建立基线
2. 检查监控任务配置
3. 使用监控插件手动触发检测

### 工作流未自动触发

**问题**: 检测到变更但工作流没有自动执行

**检查清单**:
- [ ] 工作流绑定已创建
- [ ] 绑定设置了 `auto_run_on_change: true`
- [ ] 绑定状态为 `enabled`
- [ ] 变更事件的严重程度达到阈值
- [ ] 监控任务开启了 `auto_trigger_enabled`

## API 参考

### 启动/停止调度器

```typescript
// 启动调度器
await invoke('monitor_start_scheduler')

// 停止调度器
await invoke('monitor_stop_scheduler')

// 检查状态
const isRunning = await invoke('monitor_is_running')
```

### 管理监控任务

```typescript
// 创建任务
const taskId = await invoke('monitor_create_task', {
  request: {
    program_id: 'prog-123',
    name: 'DNS Monitor',
    interval_secs: 3600, // 1 hour
    config: {
      enable_dns_monitoring: true,
      enable_cert_monitoring: true,
      enable_content_monitoring: false,
      enable_api_monitoring: false,
      auto_trigger_enabled: true,
    }
  }
})

// 列出任务
const tasks = await invoke('monitor_list_tasks', {
  programId: 'prog-123'
})

// 启用/禁用任务
await invoke('monitor_enable_task', { taskId: 'task-123' })
await invoke('monitor_disable_task', { taskId: 'task-123' })

// 立即触发任务
await invoke('monitor_trigger_task', { taskId: 'task-123' })

// 删除任务
await invoke('monitor_delete_task', { taskId: 'task-123' })
```

### 创建默认任务

```typescript
// 为项目创建默认监控任务
const taskIds = await invoke('monitor_create_default_tasks', {
  programId: 'prog-123'
})
// 返回: ['task-1', 'task-2']
```

## 事件监听

```typescript
import { listen } from '@tauri-apps/api/event'

// 监听变更检测事件
const unlisten = await listen('monitor:change-detected', (event) => {
  console.log('Change detected:', event.payload)
  // 显示通知或更新 UI
})

// 监听调度器状态变化
await listen('monitor:scheduler-started', () => {
  console.log('Scheduler started')
})

await listen('monitor:scheduler-stopped', () => {
  console.log('Scheduler stopped')
})
```

## 总结

资产监控调度器提供了强大的自动化能力，可以：

✅ 持续监控资产变化
✅ 自动触发安全测试工作流
✅ 及时发现新的攻击面
✅ 提高漏洞挖掘效率

通过合理配置监控任务和工作流绑定，可以实现 7x24 小时的自动化漏洞挖掘。
