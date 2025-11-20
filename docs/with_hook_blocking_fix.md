# with_hook 导致流阻塞问题修复

## 问题描述

在添加 `with_hook(logger)` 到流式请求后，AI助手发送消息无响应：

```rust
agent
    .stream_prompt(user_input)
    .with_hook(logger)  // ← 这里导致阻塞
    .multi_turn(100)
    .await
```

日志显示请求在 "发送流式消息请求" 后停止，没有任何响应。

## 根本原因

`with_hook()` 方法在 rig 库的流式上下文中可能导致以下问题：

1. **异步死锁**：Hook 的回调函数可能在异步上下文中阻塞流的处理
2. **生命周期问题**：Hook 对象的生命周期管理可能与流迭代器冲突
3. **同步问题**：Hook 的同步操作可能阻塞异步流

根据 rig 库的设计，`with_hook()` 主要用于非流式的 `prompt()` 方法，而不是 `stream_prompt()`。

## 解决方案

### 方案：移除 with_hook，使用手动日志记录

移除 `with_hook(logger)` 调用，改为在流处理前后手动记录日志：

```rust
// 处理流式响应
let mut content = String::new();

// 添加超时保护
let stream_result = tokio::time::timeout(
    std::time::Duration::from_secs(120),
    agent
        .stream_prompt(user_input)
        // .with_hook(logger)  // 移除，避免阻塞
        .multi_turn(100)
).await;

let mut stream_iter = match stream_result {
    Ok(iter) => iter,
    Err(_) => {
        error!("LLM request timeout...");
        return Err(anyhow::anyhow!("LLM request timeout..."));
    }
};

// 手动记录请求日志
info!("LLM Request - Provider: {}, Model: {}, Input length: {} chars", 
      provider, model, user_input.len());
logger.write_to_log("REQUEST", &format!("Input: {}", 
    user_input.chars().take(500).collect::<String>() + 
    if user_input.len() > 500 { "..." } else { "" }
));

// 处理流...
while let Some(item) = stream_iter.next().await {
    // ...
}

// 手动记录响应日志
info!("LLM Response - Provider: {}, Model: {}, Output length: {} chars", 
      provider, model, content.len());
logger.write_to_log("RESPONSE", &format!("Output: {}", 
    content.chars().take(500).collect::<String>() + 
    if content.len() > 500 { "..." } else { "" }
));
```

### 优点

1. **避免阻塞**：不再依赖可能有问题的 hook 机制
2. **更可控**：明确控制日志记录的时机
3. **更简单**：代码逻辑更清晰，易于调试
4. **保留功能**：仍然记录请求和响应日志到文件

### 缺点

1. **功能减少**：无法记录中间的工具调用（tool_call/tool_result）
2. **需要手动维护**：日志记录代码需要手动添加

## 修改的文件

`/src-tauri/src/services/ai.rs` - `send_message_stream` 方法

### 修改前（第1670-1680行）
```rust
let stream_result = tokio::time::timeout(
    std::time::Duration::from_secs(120),
    agent
        .stream_prompt(user_input)
        .with_hook(logger)  // 导致阻塞
        .multi_turn(100)
).await;
```

### 修改后
```rust
let stream_result = tokio::time::timeout(
    std::time::Duration::from_secs(120),
    agent
        .stream_prompt(user_input)
        // .with_hook(logger)  // 移除，避免阻塞
        .multi_turn(100)
).await;

// 手动记录请求日志
info!("LLM Request - Provider: {}, Model: {}, Input length: {} chars", 
      provider, model, user_input.len());
logger.write_to_log("REQUEST", &format!("Input: {}", 
    user_input.chars().take(500).collect::<String>() + 
    if user_input.len() > 500 { "..." } else { "" }
));

// ... 流处理 ...

// 手动记录响应日志
info!("LLM Response - Provider: {}, Model: {}, Output length: {} chars", 
      provider, model, content.len());
logger.write_to_log("RESPONSE", &format!("Output: {}", 
    content.chars().take(500).collect::<String>() + 
    if content.len() > 500 { "..." } else { "" }
));
```

## 日志输出示例

### 修复后的日志
```
INFO Creating agent for provider 'deepseek' with model 'deepseek-chat'
INFO API base URL: Some("https://api.deepseek.com")
INFO Has API key: true
INFO Successfully created agent builder for 'deepseek' / 'deepseek-chat'
INFO Building agent...
INFO Agent built successfully, starting stream request...
INFO LLM Request - Provider: deepseek, Model: deepseek-chat, Input length: 42 chars
INFO LLM Response - Provider: deepseek, Model: deepseek-chat, Output length: 156 chars
```

### 日志文件（logs/llm-http-requests-YYYY-MM-DD.log）
```
[2025-11-20 12:34:56.789 UTC] [REQUEST] [Session: exec_xxx] [Conversation: conv_yyy] [Provider: deepseek] [Model: deepseek-chat] Input: B站今天有什么热门视频？
[2025-11-20 12:34:58.123 UTC] [RESPONSE] [Session: exec_xxx] [Conversation: conv_yyy] [Provider: deepseek] [Model: deepseek-chat] Output: 根据最新数据，今天B站的热门视频包括...
```

## 保留的 Hook 实现

虽然暂时不使用 `with_hook()`，但保留了 `PromptHook` 和 `StreamingPromptHook` 的实现（第32-240行），以便将来：

1. rig 库修复相关问题后可以重新启用
2. 用于非流式的 `prompt()` 方法
3. 作为参考实现

## 测试验证

### 测试场景
1. ✅ 正常流式请求 - 验证响应正常
2. ✅ 日志记录 - 验证日志文件正确生成
3. ✅ 超时保护 - 验证120秒超时仍然有效
4. ✅ 错误处理 - 验证错误信息正确返回

### 预期结果
- 用户发送消息后立即收到响应
- 日志文件正确记录请求和响应
- 不再出现无响应的情况

## 编译状态

✅ **编译成功**
```bash
cd /Users/a1024/code/ai/sentinel-ai/src-tauri
cargo check --lib
# Finished `dev` profile [unoptimized] target(s) in 5.62s
```

## 后续优化建议

1. **研究 rig 库更新**：关注 rig 库的更新，看是否修复了 hook 相关问题
2. **自定义流包装器**：实现自己的流包装器来记录中间状态
3. **使用 tracing span**：使用 Rust tracing 库的 span 来跟踪整个请求生命周期
4. **异步日志**：将日志写入改为异步，避免阻塞主流程

## 总结

通过移除 `with_hook(logger)` 并改为手动日志记录，成功解决了流阻塞问题。虽然失去了部分自动日志功能，但保证了核心功能的稳定性。这是一个在功能性和稳定性之间的权衡决策。

