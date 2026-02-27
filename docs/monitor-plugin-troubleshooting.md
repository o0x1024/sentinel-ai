# 监控插件配置问题排查指南

## 问题：插件下拉列表为空

### 原因分析

插件下拉列表为空可能有以下几个原因：

1. **Tool Server 中没有注册监控相关的插件**
2. **插件名称与预期不匹配**
3. **插件未启用或加载失败**

### 排查步骤

#### 1. 检查控制台日志

打开浏览器开发者工具 (F12)，在创建监控任务时查看控制台输出：

**预期看到：**
```
✅ Loaded available plugins from backend: [...]
Plugins for dns: [...]
```

**如果看到：**
```
⚠️ Backend returned empty plugin list, using fallback plugins
📋 Using fallback plugins: [...]
```
说明后端没有返回插件，但前端会使用默认插件列表，功能仍然可用。

#### 2. 检查后端日志

查看应用程序日志，搜索以下关键字：

```
Loading available plugins for monitoring
Checking tool: name=...
Matched plugin: ... -> monitor_type=...
Found N monitor plugins
```

**示例日志：**
```
INFO Loading available plugins for monitoring, total tools: 15
DEBUG Checking tool: name=subdomain_enumerator, category=recon, enabled=true
INFO Matched plugin: subdomain_enumerator -> monitor_type=dns
DEBUG Checking tool: name=cert_monitor, category=monitor, enabled=true
INFO Matched plugin: cert_monitor -> monitor_type=cert
INFO Found 9 monitor plugins
```

#### 3. 检查插件注册状态

在后端日志中查找插件注册信息：

```bash
# macOS/Linux
tail -f ~/Library/Application\ Support/sentinel-ai/logs/app.log | grep -i plugin

# 或者直接打开日志文件
open ~/Library/Application\ Support/sentinel-ai/logs/
```

#### 4. 验证 Tool Server 状态

通过以下命令检查 Tool Server 中的工具列表：

1. 打开应用的开发者工具
2. 在控制台执行：
```javascript
await __TAURI__.invoke('list_tool_server_tools')
```

应该能看到类似输出：
```json
[
  { "name": "subdomain_enumerator", "category": "recon", "enabled": true },
  { "name": "cert_monitor", "category": "monitor", "enabled": true },
  ...
]
```

### 解决方案

#### 方案 1: 使用后备插件列表（推荐）

即使后端没有返回插件，前端会自动使用以下默认插件列表：

**DNS 监控：**
- subdomain_enumerator
- dns_resolver

**证书监控：**
- cert_monitor
- ssl_scanner

**内容监控：**
- content_monitor
- http_prober

**API 监控：**
- api_monitor
- js_analyzer
- js_link_finder

这些插件即使后端未加载，也可以在配置中使用。

#### 方案 2: 初始化插件

如果插件未注册到 Tool Server，需要：

1. 确保插件文件在正确的位置：
   ```
   sentinel-plugin/plugins/agent/
   ├── subdomain_enumerator.ts
   ├── cert_monitor.ts
   ├── content_monitor.ts
   ├── api_monitor.ts
   └── ...
   ```

2. 检查 `plugins.json` 配置：
   ```json
   {
     "plugins": [
       {
         "id": "subdomain_enumerator",
         "enabled": true,
         "category": "recon"
       }
     ]
   }
   ```

3. 重启应用以重新加载插件

#### 方案 3: 手动注册插件

如果插件存在但未被识别为监控插件，可以修改匹配规则：

在 `monitor_commands.rs` 的 `monitor_get_available_plugins` 函数中，插件名称匹配规则为：

```rust
match tool.name.as_str() {
    "subdomain_enumerator" | "dns_resolver" => "dns",
    "cert_monitor" | "ssl_scanner" => "cert",
    "content_monitor" | "http_prober" => "content",
    "api_monitor" | "js_analyzer" | "js_link_finder" => "api",
    _ => // 或基于 category 匹配
}
```

### 常见问题

#### Q1: 为什么有些插件显示但不可选？

**A:** 检查 `is_available` 字段，如果为 `false` 表示插件已禁用。

#### Q2: 插件配置后不生效？

**A:** 确保：
1. 监控调度器已启动
2. 任务已启用
3. 配置了至少一个主插件

#### Q3: 所有插件都失败了怎么办？

**A:** 
1. 查看任务执行日志
2. 检查插件输入参数是否正确
3. 尝试使用备用插件
4. 查看后端错误日志

### 验证修复

修复后，应该能看到：

**前端控制台：**
```
✅ Loaded available plugins from backend: Array(9)
Plugins for dns: Array(2)
  - subdomain_enumerator
  - dns_resolver
```

**下拉列表：**
- 能看到对应监控类型的插件列表
- 每个插件显示名称和描述

**保存后：**
- 任务配置正确保存
- 插件配置包含在任务中
- 监控调度器能正常执行

### 联系支持

如果以上方法都无法解决问题，请提供：

1. 浏览器控制台完整日志
2. 应用后端日志相关部分
3. `plugins.json` 配置文件内容
4. Tool Server 工具列表输出

---

**最后更新：** 2024年（根据实际日期更新）
**相关文档：** monitor-plugin-configuration.md
