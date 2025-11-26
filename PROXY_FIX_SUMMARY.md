# 全局代理配置修复总结

## 问题描述

开启全局代理后，LLM 请求没有走代理，导致使用需要 VPN 才能访问的 LLM 提供商（如 Google Gemini、OpenAI 等）时程序崩溃或请求失败。

## 根本原因

1. **rig 库的限制**：应用使用 `rig-core` 库的 `DynClientBuilder` 来创建 AI 客户端，该库内部会创建自己的 `reqwest::Client` 实例
2. **代理配置未传递**：我们的全局代理配置存储在 `GlobalProxyConfig` 中，但 `DynClientBuilder::new()` 不接受自定义 HTTP 客户端配置
3. **环境变量未设置**：虽然我们有全局代理配置，但没有设置标准的 `HTTP_PROXY`/`HTTPS_PROXY` 环境变量，而 `reqwest` 库在创建客户端时会自动读取这些环境变量

## 解决方案

### 1. 设置环境变量（核心修复）

修改 `src-tauri/src/utils/global_proxy.rs` 中的 `set_global_proxy()` 函数，在启用代理时设置环境变量：

```rust
pub async fn set_global_proxy(config: GlobalProxyConfig) {
    // ... 存储配置到全局变量 ...
    
    if config.enabled {
        if let Some(url) = config.build_proxy_url() {
            // 设置环境变量，让 reqwest 和其他 HTTP 库自动使用代理
            std::env::set_var("HTTP_PROXY", &url);
            std::env::set_var("HTTPS_PROXY", &url);
            
            // 设置 no_proxy 环境变量
            if let Some(no_proxy) = &config.no_proxy {
                std::env::set_var("NO_PROXY", no_proxy);
                std::env::set_var("no_proxy", no_proxy);
            }
        }
    } else {
        // 禁用代理时清除环境变量
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("NO_PROXY");
        std::env::remove_var("no_proxy");
    }
}
```

### 2. 清除代理时移除环境变量

修改 `clear_global_proxy()` 函数，确保禁用代理时清除环境变量：

```rust
pub async fn clear_global_proxy() {
    let mut proxy = GLOBAL_PROXY.write().await;
    *proxy = GlobalProxyConfig::default();
    
    // 清除代理环境变量
    std::env::remove_var("HTTP_PROXY");
    std::env::remove_var("HTTPS_PROXY");
    std::env::remove_var("NO_PROXY");
    std::env::remove_var("no_proxy");
}
```

### 3. 实现代理测试命令

重新实现 `src-tauri/src/commands/test_proxy.rs` 中的测试命令：

- `test_proxy_dynamic_update`: 验证环境变量是否正确设置
- `test_proxy_persistence`: 测试代理配置持久化
- `test_http_client_proxy_update`: 验证 HTTP 客户端能否使用代理
- `test_proxy_connection`: 实际测试代理连接

### 4. 注册测试命令

在 `src-tauri/src/lib.rs` 中注册所有测试命令：

```rust
commands::test_proxy::test_proxy_dynamic_update,
commands::test_proxy::test_proxy_persistence,
commands::test_proxy::test_http_client_proxy_update,
commands::test_proxy::test_proxy_connection,
commands::test_proxy::get_current_proxy_config,
```

## 工作原理

### reqwest 代理自动检测

`reqwest::Client` 在构建时会自动检测以下环境变量：

- `HTTP_PROXY` 或 `http_proxy`: HTTP 请求代理
- `HTTPS_PROXY` 或 `https_proxy`: HTTPS 请求代理
- `NO_PROXY` 或 `no_proxy`: 不走代理的地址列表

参考：[reqwest 文档 - Proxies](https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html#proxies)

### 为什么这个方案有效

1. **rig 库兼容**：`rig-core` 的 `DynClientBuilder` 内部使用 `reqwest::Client::default()` 或类似方法创建客户端
2. **自动应用**：`reqwest` 在创建客户端时会自动读取环境变量并配置代理
3. **全局生效**：环境变量对整个进程生效，包括所有第三方库创建的 HTTP 客户端
4. **标准方案**：这是 Rust 生态系统中的标准代理配置方式

## 测试验证

使用前端界面中的"代理配置动态更新测试"面板：

1. **动态更新测试**：验证环境变量是否正确设置
2. **持久化测试**：验证配置是否正确保存到数据库
3. **客户端更新测试**：验证新创建的 HTTP 客户端能否使用代理

## 注意事项

1. **进程级别**：环境变量是进程级别的，重启应用后需要从数据库重新加载代理配置
2. **已初始化的客户端**：已经创建的 `reqwest::Client` 实例不会受环境变量变化影响，只有新创建的客户端才会使用新的代理配置
3. **rig 库行为**：每次 LLM 请求时，`DynClientBuilder` 都会创建新的客户端实例，因此能及时应用代理配置
4. **安全性**：密码等敏感信息会在日志中被屏蔽（使用 `***` 替代）

## 相关文件

- `src-tauri/src/utils/global_proxy.rs` - 全局代理配置管理
- `src-tauri/src/commands/config.rs` - 代理配置命令（保存/读取）
- `src-tauri/src/commands/test_proxy.rs` - 代理测试命令
- `src-tauri/src/services/ai.rs` - AI 服务（使用 rig 库）
- `src/components/Settings/NetworkSettings.vue` - 前端代理设置界面

## 替代方案（未采用）

### 方案 A：修改 rig 库（不可行）
- 需要 fork rig 库并修改其内部实现
- 维护成本高，升级困难

### 方案 B：自定义 HTTP 客户端工厂（复杂）
- 需要实现自定义的客户端构建逻辑
- 与 rig 库的设计理念不符

### 方案 C：使用系统代理（平台相关）
- Windows/macOS/Linux 的系统代理设置方式不同
- 需要管理员权限
- 可能影响其他应用程序

## 结论

通过设置标准的 `HTTP_PROXY`/`HTTPS_PROXY` 环境变量，我们以最小的代码修改实现了全局代理功能，并且：

✅ 兼容 rig 库和其他第三方 HTTP 库  
✅ 符合 Rust 生态系统的标准实践  
✅ 无需修改第三方库源码  
✅ 支持动态更新（对新创建的客户端生效）  
✅ 提供完整的测试验证机制  
