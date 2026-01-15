# Vision Explorer 卡住问题修复

## 问题描述

Vision Explorer 在执行 Navigate 操作后卡住，没有继续执行后续步骤。

## 问题分析

通过分析日志和代码，发现以下问题：

### 1. 缺少超时机制

**位置**: `action_executor.rs::execute_navigate()`

**问题**: 
- 调用 `capture_observation()` 没有超时保护
- 如果 Vision LLM 响应慢或卡住，整个流程会永久等待

### 2. Vision LLM 调用可能超时

**位置**: `perception.rs::analyze()`

**问题**:
- Vision LLM API 调用没有超时限制
- 网络问题或 API 响应慢会导致无限等待

### 3. 缺少详细日志

**问题**:
- 无法定位具体卡在哪个步骤
- 难以诊断问题

## 修复方案

### 1. 添加 observation 捕获超时 (action_executor.rs)

```rust
// 在 execute_navigate 中添加 30 秒超时
let observation = match tokio::time::timeout(
    tokio::time::Duration::from_secs(30),
    self.capture_observation()
).await {
    Ok(Ok(obs)) => {
        info!("Observation captured successfully");
        Some(obs)
    }
    Ok(Err(e)) => {
        warn!("Failed to capture observation: {}", e);
        None
    }
    Err(_) => {
        warn!("Observation capture timed out after 30s");
        None
    }
};
```

**效果**:
- 即使 observation 捕获失败，Navigate 操作也能返回结果
- ReAct 循环可以继续执行
- 超时后会记录警告日志

### 2. 添加 Vision LLM 调用超时 (perception.rs)

```rust
// 为 Vision LLM 调用添加 60 秒超时
let response = tokio::time::timeout(
    tokio::time::Duration::from_secs(60),
    self.llm_client.chat(Some(&system_prompt), &user_prompt, &[], Some(&image))
).await;

match response {
    Ok(Ok(text)) => {
        debug!("Vision LLM response received, parsing...");
        self.parse_llm_response(&text, context)
    }
    Ok(Err(err)) => {
        warn!("Vision LLM failed, fallback to DOM-only: {}", err);
        Ok(self.analyze_dom_only(context))
    }
    Err(_) => {
        warn!("Vision LLM timed out after 60s, fallback to DOM-only");
        Ok(self.analyze_dom_only(context))
    }
}
```

**效果**:
- Vision LLM 超时后自动降级到 DOM-only 分析
- 确保即使 Vision API 有问题，也能继续工作
- 提供更好的容错能力

### 3. 增强日志记录

**action_executor.rs**:
```rust
info!("Navigation successful to: {}", result.url);
info!("Capturing observation after navigation...");
info!("Observation captured successfully");
warn!("Failed to capture observation: {}", e);
warn!("Observation capture timed out after 30s");
```

**perception.rs**:
```rust
debug!("Calling Vision LLM for page analysis...");
debug!("Vision LLM response received, parsing...");
warn!("Vision LLM timed out after 60s, fallback to DOM-only");
```

**capture_page_context**:
```rust
debug!("Starting page context capture");
debug!("Capturing screenshot...");
debug!("Screenshot captured");
debug!("Getting HTML content...");
debug!("HTML content retrieved, length: {}", html.len());
debug!("Getting URL and title...");
debug!("URL: {}, Title: {}", url, title);
debug!("Getting viewport info...");
debug!("Viewport info retrieved");
debug!("Page context capture completed");
```

## 超时时间设置

| 操作 | 超时时间 | 说明 |
|------|---------|------|
| Observation 捕获 | 30 秒 | 包括截图、HTML、Vision 分析 |
| Vision LLM 调用 | 60 秒 | Vision API 响应时间 |
| 页面加载等待 | 1 秒 | 等待动态内容加载 |

## 降级策略

### Vision LLM 失败时

1. **超时** → 使用 DOM-only 分析
2. **API 错误** → 使用 DOM-only 分析
3. **解析错误** → 使用 DOM-only 分析

### Observation 捕获失败时

1. **超时** → 返回 None，继续执行
2. **错误** → 返回 None，继续执行
3. **部分失败** → 使用可用的部分数据

## 测试建议

### 1. 正常场景测试

```bash
# 测试正常页面加载
vision_explorer("http://testphp.vulnweb.com/")
```

**预期**:
- 页面成功加载
- Observation 成功捕获
- Vision LLM 正常分析

### 2. 慢速网络测试

```bash
# 测试慢速网络场景
# 使用网络限速工具模拟慢速连接
```

**预期**:
- 超时后降级到 DOM-only
- 任务继续执行
- 日志显示降级信息

### 3. Vision API 故障测试

```bash
# 临时禁用 Vision API 或使用无效配置
```

**预期**:
- 自动降级到 DOM-only
- 不影响整体流程
- 日志显示降级原因

## 监控指标

建议监控以下指标：

1. **Observation 捕获成功率**
   - 成功次数 / 总次数
   - 目标: > 95%

2. **Vision LLM 超时率**
   - 超时次数 / 总调用次数
   - 目标: < 5%

3. **平均响应时间**
   - Vision LLM 平均响应时间
   - 目标: < 10 秒

4. **降级使用率**
   - DOM-only 使用次数 / 总次数
   - 目标: < 10%

## 后续优化建议

### 1. 动态超时调整

根据历史响应时间动态调整超时值：
```rust
let avg_response_time = get_avg_response_time();
let timeout = std::cmp::max(avg_response_time * 2, 30_000);
```

### 2. 重试机制

对于临时性故障，添加重试：
```rust
for attempt in 1..=3 {
    match call_vision_llm().await {
        Ok(result) => return Ok(result),
        Err(e) if is_retryable(&e) && attempt < 3 => {
            warn!("Attempt {} failed, retrying...", attempt);
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        Err(e) => return Err(e),
    }
}
```

### 3. 缓存机制

对相同页面的分析结果进行缓存：
```rust
let cache_key = format!("{}:{}", url, html_hash);
if let Some(cached) = cache.get(&cache_key) {
    return Ok(cached);
}
```

### 4. 并行处理

将截图和 HTML 获取并行化：
```rust
let (screenshot, html) = tokio::join!(
    service.screenshot(false),
    service.get_content(None)
);
```

## 相关文件

- `src-tauri/src/engines/vision_explorer_v2/action_executor.rs`
- `src-tauri/src/engines/vision_explorer_v2/perception.rs`

## 更新日期

2026-01-15

## 版本

v1.0
