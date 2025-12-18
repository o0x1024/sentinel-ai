# WebSocket 支持实现计划

## 概述

在现有的代理系统中增加完整的 WebSocket 请求拦截、编辑和历史记录功能，采用**方案三：统一列表 + 子分组**设计。

**重要变更**：使用**内存缓存**替代数据库存储 HTTP 和 WebSocket 请求历史记录。

## ✅ 已完成的工作

### Phase 1: 内存缓存模块 ✅
- [x] 创建了 `sentinel-passive/src/history_cache.rs`
- [x] 支持 HTTP 请求和 WebSocket 连接/消息的存储
- [x] LRU 缓存策略（默认 2000 条 HTTP + 200 个 WebSocket 连接）
- [x] 应用关闭时自动清空（内存自动释放）

### Phase 2: 后端集成 ✅
- [x] `ScanPipeline` 改为使用 `history_cache` 存储请求历史
- [x] `PassiveScanState` 添加了 `history_cache` 字段
- [x] 启动代理时将 `history_cache` 传递给 `ScanPipeline`

### Phase 3: HTTP Tauri 命令更新 ✅
- [x] `list_proxy_requests` - 从内存缓存查询
- [x] `get_proxy_request` - 从内存缓存获取
- [x] `clear_proxy_requests` - 清空内存缓存
- [x] `count_proxy_requests` - 统计缓存数量

### Phase 4: WebSocket 后端扩展 ✅
- [x] 扩展 `InterceptState` 添加 `websocket_enabled` 和 `pending_websocket_tx`
- [x] 添加 `WebSocketConnectionContext` 和 `WebSocketMessageContext` 类型
- [x] 添加 `PendingInterceptWebSocketMessage` 用于 WebSocket 拦截
- [x] 扩展 `ScanTask` 添加 `WebSocketConnection` 和 `WebSocketMessage` 变体
- [x] 增强 `WebSocketHandler` 实现以记录消息到扫描器
- [x] 在 `ScanPipeline` 中添加 `process_websocket_connection` 和 `process_websocket_message` 方法
- [x] WebSocket 消息存储到内存缓存
- [x] 发送 `proxy:websocket_connection` 和 `proxy:websocket_message` 事件到前端

### Phase 5: WebSocket Tauri 命令 ✅
- [x] `list_websocket_connections` - 查询 WebSocket 连接列表
- [x] `list_websocket_messages` - 查询指定连接的消息
- [x] `clear_websocket_history` - 清空 WebSocket 历史
- [x] `get_history_stats` - 获取缓存统计信息
- [x] `clear_all_history` - 清空所有历史 (HTTP + WebSocket)
- [x] 命令已注册到 Tauri

### Phase 6: 前端基础支持 ✅
- [x] 创建 `src/services/proxy_history.ts` - 前端 API 服务
- [x] 在 `ProxyHistory.vue` 添加协议类型切换 tabs (All/HTTP/WS)
- [x] 添加 `protocolFilter` 状态变量
- [x] 前端类型检查通过

### Phase 7: 前端 WebSocket 完整支持 ✅
- [x] **WebSocket 列表展示** - 在 `ProxyHistory.vue` 实现
  - 支持展开/折叠 WebSocket 连接
  - 显示连接状态、URL、时间等信息
- [x] **消息列表** - 内嵌在连接展开区域
  - 支持显示消息方向 (Up/Down)
  - 支持显示消息类型 (Text/Binary/Control)
  - Base64 二进制与文本区分截断展示
- [x] **握手详情** - 支持 Tab 切换查看握手请求/响应 Header
- [x] **事件监听** - 实时监听后端 `proxy:websocket_connection` 和 `proxy:websocket_message` 事件
- [x] **状态管理** - 切换协议时自动清理选中的 HTTP 请求
- [x] **去冗余** - 优化代码结构，移除冗余函数

---

## 待完成的工作

### Phase 8: 前端拦截器改造（可选）

1. **WebSocket 拦截控制**
   - 新增 WebSocket 拦截开关
   - 支持 WebSocket 消息拦截和编辑（仅 Text 类型）

---

## 文件修改清单

### 已修改 ✅

| 文件 | 修改内容 |
|------|----------|
| `sentinel-passive/src/history_cache.rs` | **新增** - 内存缓存模块 |
| `sentinel-passive/src/lib.rs` | 导出 history_cache 模块和 WebSocket 类型 |
| `sentinel-passive/src/scanner.rs` | 使用 history_cache，添加 WebSocket 处理方法 |
| `sentinel-passive/src/proxy.rs` | 扩展 InterceptState，增强 WebSocketHandler |
| `src-tauri/src/commands/passive_scan_commands.rs` | 添加 history_cache，HTTP/WebSocket 命令 |
| `src-tauri/src/lib.rs` | 注册新的 WebSocket 命令 |
| `src/services/proxy_history.ts` | **新增** - 前端 API 服务 |
| `src/components/passive/ProxyHistory.vue` | 实现完整的 WebSocket 历史查看功能 |

---

## 后端事件总结

### 已实现的后端事件

| 事件名 | 数据类型 | 描述 |
|--------|----------|------|
| `proxy:request` | `HttpRequestRecord` | HTTP 请求完成 |
| `proxy:websocket_connection` | `WebSocketConnectionContext` | WebSocket 连接建立 |
| `proxy:websocket_message` | `WebSocketMessageContext` | WebSocket 消息 |

### 已实现的 Tauri 命令

| 命令名 | 返回类型 | 描述 |
|--------|----------|------|
| `list_proxy_requests` | `Vec<HttpRequestRecord>` | HTTP 列表 |
| `get_proxy_request` | `Option<HttpRequestRecord>` | HTTP 详情 |
| `clear_proxy_requests` | `u64` | 清空 HTTP 历史 |
| `count_proxy_requests` | `i64` | HTTP 数量 |
| `list_websocket_connections` | `Vec<WebSocketConnectionRecord>` | WS 连接列表 |
| `list_websocket_messages` | `Vec<WebSocketMessageRecord>` | WS 消息列表 |
| `clear_websocket_history` | `u64` | 清空 WS 历史 |
| `get_history_stats` | `HistoryCacheStats` | 缓存统计 |
| `clear_all_history` | `String` | 清空全部历史 |

---

## 构建验证

```bash
# 后端
cd src-tauri && cargo check  # ✅ 通过

# 前端
npm run type-check  # ✅ 通过
```
