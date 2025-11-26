# Gemini 代理问题诊断和修复方案

## 问题描述

当配置全局代理（如本机 10809 端口的 Clash）后：
- **DeepSeek 正常**：请求通过代理，能在 BurpSuite 看到流量
- **Gemini 异常**：请求无响应，程序卡死，BurpSuite 看不到流量

## 根本原因分析

### 1. reqwest 不会自动读取环境变量代理

**关键发现**：即使你在 `global_proxy.rs` 中设置了 `HTTP_PROXY` 和 `HTTPS_PROXY` 环境变量，`reqwest::Client` 默认**不会**自动读取这些变量。

```rust
// ❌ 这样创建的客户端不会使用代理，即使环境变量已设置
let client = reqwest::Client::builder().build()?;

// ✅ 必须显式应用代理配置
let builder = reqwest::Client::builder();
let builder = apply_proxy_to_client(builder).await;
let client = builder.build()?;
```

### 2. rig 库的限制

`rig` 库（版本 0.24.0）的 `DynClientBuilder` 在内部创建 `reqwest::Client` 时：
- 不提供接口让你传入自定义的 HTTP 客户端
- 内部创建的客户端可能没有正确配置代理支持
- 即使设置了环境变量，也不保证会读取

### 3. TLS 支持缺失

原 `Cargo.toml` 配置：
```toml
reqwest = { version = "0.12.0", features = ["json", "stream", "socks"] }
```

**问题**：
- 缺少 TLS 后端 (`rustls-tls` 或 `native-tls`)
- Gemini 使用 HTTPS，没有 TLS 支持会导致连接失败
- 即使有代理，TLS 握手也会阻塞

### 4. API 环境变量未设置

rig 库要求每个 provider 都要从环境变量中读取配置：
- Gemini: `GEMINI_API_KEY` 和 `GEMINI_API_BASE`
- OpenAI: `OPENAI_API_KEY` 和 `OPENAI_API_BASE`
- Anthropic: `ANTHROPIC_API_KEY` 等

但你的代码在调用 `DynClientBuilder` 之前**没有设置这些环境变量**。

## 修复方案

### 1. 添加 reqwest TLS 支持 ✅

```toml
# Cargo.toml
reqwest = { version = "0.12.0", features = [
    "json", 
    "stream", 
    "socks", 
    "rustls-tls",              # ✅ 启用 HTTPS 支持
    "rustls-tls-native-roots", # ✅ 使用系统根证书
    "cookies"                   # ✅ Cookie 管理
] }
```

### 2. 为 rig 设置 API 环境变量 ✅

在 `ai.rs` 的 `send_message_stream_with_save_control` 中，创建 agent 之前：

```rust
// 为 rig 库设置必需的环境变量
if let Some(api_key) = &self.config.api_key {
    match provider.as_str() {
        "gemini" | "google" => {
            std::env::set_var("GEMINI_API_KEY", api_key);
            if let Some(base) = &self.config.api_base {
                std::env::set_var("GEMINI_API_BASE", base);
            }
        }
        "openai" => {
            std::env::set_var("OPENAI_API_KEY", api_key);
            // ...
        }
        // 其他 providers...
    }
}
```

### 3. 完善代理环境变量设置 ✅

在 `global_proxy.rs` 中，同时设置大小写两种格式的环境变量：

```rust
std::env::set_var("HTTP_PROXY", &url);
std::env::set_var("HTTPS_PROXY", &url);
std::env::set_var("http_proxy", &url);  // ✅ 小写版本
std::env::set_var("https_proxy", &url); // ✅ 小写版本
```

### 4. 手动应用代理到所有 reqwest 客户端 ✅

对于应用中其他直接创建的 `reqwest::Client`（如 Tavily 搜索）：

```rust
// ai.rs - Tavily 搜索
let client = {
    let builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(30));
    let builder = crate::utils::global_proxy::apply_proxy_to_client(builder).await;
    builder.build()?
};
```

## ⚠️ 重要说明和限制

### rig 库的代理支持不确定

由于 `rig` 库内部创建 HTTP 客户端的方式不受我们控制，**即使设置了所有环境变量，也不能 100% 保证 rig 会使用代理**。

可能的情况：
1. **最佳情况**：rig 内部在创建 `reqwest::Client` 时检查了环境变量，代理生效
2. **中等情况**：rig 部分 provider 支持代理，部分不支持
3. **最坏情况**：rig 完全忽略代理环境变量

### 进一步的验证步骤

1. **查看 rig 源码**：检查具体的 Gemini provider 实现
   ```bash
   # 查看 rig-core 0.24.0 的 Gemini 实现
   cargo doc --open --package rig-core
   ```

2. **网络抓包验证**：
   - 在 BurpSuite 中设置过滤器：目标域名 `generativelanguage.googleapis.com`
   - 观察是否有任何连接尝试（包括失败的）
   - 检查是否是 TLS 握手失败

3. **测试不同场景**：
   ```
   场景 A：Clash 10809 + Gemini → 观察是否有流量
   场景 B：关闭代理 + Gemini + 直连 Google → 是否正常工作
   场景 C：BurpSuite 8080 + 导入 CA 证书 + Gemini → 是否能抓到包
   ```

### 如果问题仍然存在

如果以上修改后 Gemini 仍然无法通过代理工作，可能需要：

1. **Fork rig 库并修改源码**：
   - 直接修改 Gemini provider 的客户端创建逻辑
   - 强制应用代理配置

2. **切换到其他 AI 框架**：
   - 使用更低level的 HTTP 库直接调用 Gemini API
   - 放弃 rig，自己实现 Gemini 客户端

3. **使用网络层代理**：
   - 在系统层面强制所有流量走代理（如透明代理）
   - 使用 VPN 而不是 HTTP 代理

## 测试验证

修改完成后，建议按以下步骤测试：

1. **重新构建项目**：
   ```bash
   cd src-tauri
   cargo clean
   cargo build --release
   ```

2. **启动 Clash 在 10809 端口**

3. **在 UI 中配置代理**：
   - 协议：HTTP 或 SOCKS5H
   - 主机：127.0.0.1
   - 端口：10809

4. **测试 Gemini**：
   - 发送一条简单消息
   - 观察 Clash 日志
   - 观察应用日志（查找 `Global proxy enabled` 和 `LLM Request` 日志）

5. **检查日志输出**：
   ```
   应该看到：
   INFO - Global proxy enabled (http): http://127.0.0.1:10809
   INFO - LLM Request - Provider: gemini, Model: gemini-2.5-pro
   DEBUG - Environment variables set: HTTP_PROXY=..., HTTPS_PROXY=...
   ```

## 结论

通过以上修改：
1. ✅ 确保 reqwest 有正确的 TLS 支持
2. ✅ 为 rig 设置必需的 API 环境变量
3. ✅ 完善代理环境变量配置
4. ✅ 对所有手动创建的 HTTP 客户端应用代理

但由于 rig 库的内部实现限制，**最终能否成功让 Gemini 通过代理工作，取决于 rig 的 Gemini provider 实现是否支持从环境变量读取代理配置**。

如果问题仍然存在，建议直接查看 rig 的源码或考虑切换到其他 AI 框架。
