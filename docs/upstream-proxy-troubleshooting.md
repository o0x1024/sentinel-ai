# Upstream Proxy 故障排查指南

## 问题描述

设置了 upstream proxy 为本机 10809 端口后，无法通过代理访问 Google 等网站，但浏览器直接设置代理为 10809 端口可以正常访问。

## 排查步骤

### 1. 确认 Upstream Proxy 配置已启用

1. 打开应用的 **Proxy Configuration** 页面
2. 找到 **Upstream Proxy Servers** 部分
3. 确认以下配置：
   - ✅ **Enabled** 复选框已勾选（这是最关键的！）
   - **Destination Host**: `*` (匹配所有目标主机)
   - **Proxy Host**: `127.0.0.1`
   - **Proxy Port**: `10809`
   - **Auth Type**: 根据你的代理服务器选择（通常选择 "None"）

4. 点击保存后，**重启代理服务**

### 2. 查看日志确认配置

重启应用后，查看日志文件：`logs/sentinel-ai.log.2025-12-17`

应该看到类似以下的日志：

```
INFO sentinel_passive::proxy: Checking upstream proxy config: Some(UpstreamProxyConfig { enabled: true, destination_host: "*", proxy_host: "127.0.0.1", proxy_port: 10809, ... })
INFO sentinel_passive::proxy: Upstream proxy found - enabled: true, host: 127.0.0.1, port: 10809
INFO sentinel_passive::proxy: Use upstream proxy decision: true
INFO sentinel_passive::proxy: Starting HTTPS MITM proxy on port 8080 with upstream proxy 127.0.0.1:10809 (destination: *)
INFO sentinel_passive::proxy: Creating upstream proxy connector: host=127.0.0.1, port=10809, auth_type=
INFO sentinel_passive::proxy: Upstream proxy URI: http://127.0.0.1:10809
INFO sentinel_passive::proxy: Creating ProxyConnector from HTTP proxy
INFO sentinel_passive::proxy: Wrapping ProxyConnector with HTTPS connector (insecure TLS)
INFO sentinel_passive::proxy: Upstream proxy connector created successfully
```

如果看到的是：
```
INFO sentinel_passive::proxy: Starting HTTPS MITM proxy on port 8080 (ignoring upstream cert errors)
```

说明 upstream proxy **没有被启用**，请返回步骤 1 检查配置。

### 3. 常见问题

#### 问题 1: Enabled 复选框未勾选

**症状**: 日志显示 `Use upstream proxy decision: false`

**解决方案**: 
1. 在 UI 中勾选 Upstream Proxy 的 "Enabled" 复选框
2. 保存配置
3. 重启代理服务

#### 问题 2: 配置未保存到数据库

**症状**: 重启应用后配置丢失

**解决方案**:
1. 检查数据库文件权限：`/Users/a1024/Library/Application Support/sentinel-ai/database.db`
2. 确保应用有写入权限
3. 重新配置并保存

#### 问题 3: 代理协议不匹配

**症状**: 连接超时或连接被拒绝

**说明**: 10809 端口通常是混合代理端口（支持 HTTP 和 SOCKS5）。当前实现使用 HTTP 协议连接 upstream proxy。

**验证方法**:
```bash
# 测试 HTTP 代理是否工作
curl -x http://127.0.0.1:10809 http://www.google.com
```

如果上述命令失败，说明 10809 端口可能只支持 SOCKS5 协议，需要修改代码以支持 SOCKS5。

### 4. 调试技巧

#### 查看实时日志

```bash
tail -f /Users/a1024/code/ai/sentinel-ai/logs/sentinel-ai.log.2025-12-17
```

#### 测试 upstream proxy 连接性

```bash
# 测试 HTTP 代理
curl -v -x http://127.0.0.1:10809 http://www.google.com

# 测试 HTTPS 代理 (HTTP CONNECT)
curl -v -x http://127.0.0.1:10809 https://www.google.com
```

## 当前实现状态

✅ 已实现：
- Upstream proxy 配置 UI
- HTTP/HTTPS 请求通过 upstream proxy 转发
- 配置持久化到数据库
- 详细的调试日志

⚠️ 限制：
- 仅支持 HTTP 协议的 upstream proxy
- Basic 认证暂未实现（已预留接口）
- WebSocket 暂不通过 upstream proxy（直接连接）

🔄 待实现：
- SOCKS5 upstream proxy 支持
- Basic 认证完整实现
- WebSocket 通过 upstream proxy

## 下一步

如果按照以上步骤仍然无法解决问题，请提供：
1. 完整的日志输出
2. Upstream proxy 的类型（HTTP/SOCKS5/混合）
3. 是否需要认证
4. `curl` 测试的输出结果

