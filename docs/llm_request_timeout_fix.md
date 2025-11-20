# LLM请求超时问题修复

## 问题描述

用户发送消息后，AI助手没有响应。从日志分析发现：

```
INFO sentinel_ai_lib::services::ai: 1464: 发送流式消息请求 - 模型: deepseek-chat
```

日志在这一行后停止，没有任何错误信息或后续输出，说明请求在等待LLM响应时无限期挂起。

## 根本原因

1. **缺少超时保护**：`agent.stream_prompt().multi_turn().await` 调用没有设置超时，如果API服务器无响应，会永久阻塞
2. **错误处理不足**：没有捕获网络连接失败、API配置错误等问题
3. **日志不详细**：无法诊断是哪个环节出现问题（agent创建、请求发送、等待响应）

## 可能的触发场景

- 网络连接问题（无法连接到deepseek API）
- API base URL配置错误
- API服务器响应慢或无响应
- 防火墙或代理阻止连接
- API密钥验证失败但没有返回错误

## 解决方案

### 1. 添加超时保护

在 `/src-tauri/src/services/ai.rs` 的 `send_message_stream` 方法中添加120秒超时：

```rust
// 添加超时保护，防止请求无限期挂起
let stream_result = tokio::time::timeout(
    std::time::Duration::from_secs(120), // 2分钟超时
    agent
        .stream_prompt(user_input)
        .with_hook(logger)
        .multi_turn(100)
).await;

let mut stream_iter = match stream_result {
    Ok(iter) => iter,
    Err(_) => {
        error!("LLM request timeout after 120 seconds for provider '{}' model '{}'", provider, model);
        return Err(anyhow::anyhow!(
            "LLM request timeout: The AI service did not respond within 120 seconds. Please check your network connection and API configuration."
        ));
    }
};
```

### 2. 增强诊断日志

添加详细的日志来帮助诊断问题：

```rust
info!("Creating agent for provider '{}' with model '{}'", provider, model);
info!("API base URL: {:?}", self.config.api_base);
info!("Has API key: {}", self.config.api_key.is_some());

// ... agent创建 ...

info!("Successfully created agent builder for '{}' / '{}'", provider, model);
info!("Building agent...");
// ... agent.build() ...
info!("Agent built successfully, starting stream request...");
```

### 3. 改进错误处理（已在之前修复）

在 `dispatch_with_react` 中添加回退机制：

```rust
// 获取默认 AI 服务
let ai_service = match ai_service_manager.get_default_chat_model().await {
    Ok(Some((provider, model))) => {
        match ai_service_manager.get_provider_config(&provider).await {
            Ok(Some(mut provider_config)) => {
                // 使用配置的provider
                provider_config.model = model;
                // ...
                Arc::new(ai_svc)
            }
            _ => {
                // Provider配置获取失败，回退到 "default" 服务
                log::warn!("Failed to get provider config for '{}', falling back to 'default' service", provider);
                match ai_service_manager.get_service("default") {
                    Some(service) => Arc::new(service),
                    None => {
                        return Err(format!("Failed to get AI provider config for '{}' and no default service available", provider));
                    }
                }
            }
        }
    }
    _ => {
        // 没有配置默认模型，回退到 "default" 服务
        log::warn!("No default chat model configured, trying to use 'default' service");
        match ai_service_manager.get_service("default") {
            Some(service) => Arc::new(service),
            None => {
                return Err("No default AI model configured and no default service available".to_string());
            }
        }
    }
};
```

## 修改的文件

1. `/src-tauri/src/services/ai.rs`
   - 添加超时保护（第1660-1680行）
   - 增强诊断日志（第1632-1668行）

2. `/src-tauri/src/commands/ai_commands.rs`
   - 改进AI服务获取的回退机制（第1390-1427行）

## 预期效果

### 修复前
- 请求卡住，无任何响应
- 日志停在"发送流式消息请求"
- 无法诊断问题原因
- 需要重启应用

### 修复后
- 120秒后自动超时并返回错误
- 详细日志显示问题发生在哪个环节
- 用户收到明确的错误提示
- 可以重试或检查配置

## 测试建议

1. **正常场景**：验证正常请求仍然工作
2. **网络断开**：断开网络，验证超时机制
3. **错误配置**：使用错误的API base URL，验证错误提示
4. **慢速响应**：模拟慢速API，验证超时时间合理

## 后续优化建议

1. **可配置超时**：将120秒超时改为可配置参数
2. **重试机制**：添加自动重试逻辑（指数退避）
3. **健康检查**：启动时检查API连接状态
4. **用户提示**：在前端显示"正在连接AI服务..."状态

## 相关日志示例

### 修复后的正常日志
```
INFO Creating agent for provider 'deepseek' with model 'deepseek-chat'
INFO API base URL: Some("https://api.deepseek.com")
INFO Has API key: true
INFO Successfully created agent builder for 'deepseek' / 'deepseek-chat'
INFO Building agent...
INFO Agent built successfully, starting stream request...
INFO [Stream response chunks...]
```

### 超时错误日志
```
INFO Creating agent for provider 'deepseek' with model 'deepseek-chat'
INFO API base URL: Some("https://api.deepseek.com")
INFO Has API key: true
INFO Successfully created agent builder for 'deepseek' / 'deepseek-chat'
INFO Building agent...
INFO Agent built successfully, starting stream request...
ERROR LLM request timeout after 120 seconds for provider 'deepseek' model 'deepseek-chat'
```

## 编译状态

✅ 编译成功，无错误
```bash
cd /Users/a1024/code/ai/sentinel-ai/src-tauri
cargo check --lib
# Finished `dev` profile [unoptimized] target(s) in 19.73s
```

## 总结

通过添加超时保护和增强日志，解决了LLM请求无限期挂起的问题。现在即使API服务无响应，系统也会在120秒后超时并返回明确的错误信息，大大提升了用户体验和问题诊断能力。

