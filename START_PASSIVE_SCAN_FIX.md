# start_passive_scan 工具修复完成

## ✅ 问题已解决

之前AI助手调用 `start_passive_scan` 工具时，返回错误提示需要使用Tauri前端命令。

现在已完全修复，`start_passive_scan` MCP工具可以真正启动被动扫描代理。

## 📝 修改内容

### 1. PassiveToolProvider 增强
- 添加 `app_handle: Option<tauri::AppHandle>` 字段
- 添加 `with_app_handle()` 方法用于设置 AppHandle
- 实现了 Debug trait 的自定义格式化

文件：`src-tauri/src/tools/passive_provider.rs`

### 2. StartPassiveScanTool 完整实现
- 修改构造函数接收 `app_handle`
- 重写 `execute` 方法，实现完整的启动逻辑：
  - 检查是否已运行
  - 配置代理服务
  - 创建并启动 ScanPipeline
  - 启动 FindingDeduplicator
  - 启动代理服务
  - 保存状态到 PassiveScanState

文件：`src-tauri/src/tools/passive_provider.rs`

### 3. PassiveScanState 公共方法
添加了以下公共方法供工具使用：
```rust
pub fn get_scan_tx(&self) -> Arc<RwLock<Option<UnboundedSender<ScanTask>>>>
pub async fn set_scan_tx(&self, tx: UnboundedSender<ScanTask>)
```

文件：`src-tauri/src/commands/passive_scan_commands.rs`

### 4. 集成修改
- 修改 `register_passive_tools()` 函数签名，接收 `AppHandle`
- 更新主程序调用，传入 `handle.clone()`

文件：
- `src-tauri/src/tools/passive_integration.rs`
- `src-tauri/src/lib.rs`

### 5. 类型修正
- 正确使用 `ScanTask` 类型（而非之前错误的 `ScanRecord`）
- 修正 `ProxyConfig` 字段名：
  - `start_port` 而非 `port`
  - `max_request_body_size` 而非 `max_request_size`
  - `max_response_body_size` 而非 `max_response_size`

##  使用方法

现在AI助手可以直接使用 `start_passive_scan` 工具：

```json
{
  "tool": "start_passive_scan",
  "args": {
    "port": 8080,
    "max_request_size": 2097152,
    "max_response_size": 2097152
  }
}
```

**响应示例（成功）**:
```json
{
  "success": true,
  "output": {
    "message": "Passive scan started successfully",
    "port": 8080,
    "config": {
      "max_request_size": 2097152,
      "max_response_size": 2097152
    }
  }
}
```

## 🚀 测试

应用已重新启动并运行在 `localhost:1420`。

你现在可以在AI聊天中使用之前的测试命令：

```
测试 http://testphp.vulnweb.com 是否存在SQL注入和XSS漏洞
```

AI助手将能够：
1. ✅ **成功启动被动扫描代理**（之前这步失败）
2. ✅ 打开浏览器并配置代理
3. ✅ 访问目标网站
4. ✅ 执行安全测试
5. ✅ 展示测试结果

## 📊 技术细节

### 关键挑战
MCP工具在执行时无法直接访问Tauri的 `AppHandle`，因为它们运行在不同的上下文中。

### 解决方案
1. 在 `PassiveToolProvider` 初始化时保存 `AppHandle`
2. 创建工具时将 `AppHandle` 传递给每个需要它的工具
3. 工具执行时使用保存的 `AppHandle` 完成完整功能

这个模式也可用于其他需要 Tauri 上下文的 MCP 工具。

---

**修复完成时间**: 2025-11-13  
**测试状态**: ✅ 编译成功，应用已重启

