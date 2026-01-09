# Plugin API 设计分析：op_emit_finding vs Return Value

## 问题

是否可以移除 `op_emit_finding`，直接通过插件的 `return` 值来返回漏洞发现（findings），在 Rust 中获取返回值进行处理？

## 当前架构分析

### 现有实现

#### 1. **插件端（JavaScript）**

```javascript
// 方式 1: 使用 Sentinel.emitFinding() 发送漏洞
export async function scan_transaction(ctx) {
    // 检测到漏洞
    Sentinel.emitFinding({
        title: 'SQL Injection',
        severity: 'critical',
        url: ctx.request.url,
        evidence: payload,
    });
    
    // 可以发送多个漏洞
    Sentinel.emitFinding({
        title: 'XSS Detected',
        severity: 'high',
        // ...
    });
    
    // 函数不需要返回漏洞
    return;  // 或者不返回任何值
}

globalThis.scan_transaction = scan_transaction;
```

#### 2. **Rust 端处理**

```rust
// plugin_ops.rs
#[op2]
fn op_emit_finding(state: &mut OpState, #[serde] finding: JsFinding) -> bool {
    let ctx = state.borrow::<PluginContext>().clone();
    let rust_finding = Finding::from(finding.clone());
    
    // 收集到 PluginContext 的 findings 列表中
    ctx.findings.lock().unwrap().push(rust_finding);
    
    true
}

// plugin_engine.rs
pub async fn execute_agent(&mut self, input: &serde_json::Value) 
    -> Result<(Vec<Finding>, Option<serde_json::Value>)> 
{
    // 调用插件函数
    self.call_plugin_function("analyze", input).await?;
    
    // 收集通过 Sentinel.emitFinding() 发送的漏洞
    let (findings, last_result) = {
        let op_state = self.runtime.op_state();
        let op_state_borrow = op_state.borrow();
        let plugin_ctx = op_state_borrow.borrow::<PluginContext>();
        (plugin_ctx.take_findings(), plugin_ctx.take_last_result())
    };
    
    Ok((findings, last_result))
}
```

### 工作流程

```
Plugin JS Code
    ↓
Sentinel.emitFinding({...})  // 第一个漏洞
    ↓
op_emit_finding(state, finding)
    ↓
ctx.findings.push(finding)   // 存入列表
    ↓
Sentinel.emitFinding({...})  // 第二个漏洞
    ↓
op_emit_finding(state, finding)
    ↓
ctx.findings.push(finding)   // 存入列表
    ↓
函数执行完毕
    ↓
Rust 从 PluginContext 中 take_findings()
    ↓
返回所有漏洞 Vec<Finding>
```

## 改进方案：使用 Return Value

### 方案设计

#### 插件端（JavaScript）

```javascript
// 方式 2: 直接返回漏洞数组
export async function scan_transaction(ctx) {
    const findings = [];
    
    // 检测到漏洞，添加到数组
    findings.push({
        title: 'SQL Injection',
        severity: 'critical',
        url: ctx.request.url,
        evidence: payload,
    });
    
    // 检测到另一个漏洞
    findings.push({
        title: 'XSS Detected',
        severity: 'high',
        // ...
    });
    
    // 返回漏洞数组
    return { findings };
}

globalThis.scan_transaction = scan_transaction;
```

#### Rust 端处理

```rust
// plugin_engine.rs
pub async fn execute_agent(&mut self, input: &serde_json::Value) 
    -> Result<(Vec<Finding>, Option<serde_json::Value>)> 
{
    // 调用插件函数
    self.call_plugin_function("analyze", input).await?;
    
    // 从返回值中获取 findings
    let return_value = {
        let op_state = self.runtime.op_state();
        let op_state_borrow = op_state.borrow();
        let plugin_ctx = op_state_borrow.borrow::<PluginContext>();
        plugin_ctx.take_last_result()
    };
    
    // 解析返回值中的 findings 字段
    let findings = if let Some(value) = return_value {
        if let Some(findings_array) = value.get("findings") {
            // 将 JSON 数组转换为 Vec<Finding>
            serde_json::from_value::<Vec<JsFinding>>(findings_array.clone())
                .map(|js_findings| {
                    js_findings.into_iter()
                        .map(|js| Finding::from(js))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    
    Ok((findings, return_value))
}
```

## 方案对比

| 维度 | 当前方案（op_emit_finding） | 新方案（Return Value） |
|------|---------------------------|---------------------|
| **API 风格** | 回调风格（类似事件发射） | 函数式风格（返回值） |
| **代码直观性** | ❌ 不直观，需要理解特殊 API | ✅ 非常直观，标准函数返回 |
| **多漏洞支持** | ✅ 简单，多次调用即可 | ✅ 简单，添加到数组 |
| **Node.js 兼容** | ❌ 自定义 API，非标准 | ✅ 标准 JavaScript 返回值 |
| **类型安全** | ⚠️ 运行时验证 | ⚠️ 运行时验证（相同） |
| **错误处理** | ⚠️ 隐式（op 调用失败静默） | ✅ 显式（返回值验证） |
| **实时发送** | ✅ 检测到即发送 | ❌ 需要收集完才返回 |
| **流式处理** | ✅ 支持流式发送 | ❌ 必须等待全部完成 |
| **性能** | ✅ 无需序列化返回值 | ⚠️ 需要序列化整个对象 |
| **向后兼容** | ✅ 已有大量插件使用 | ❌ 需要迁移现有插件 |

## 深入分析

### 优势对比

#### 当前方案（op_emit_finding）的优势

1. **流式发送** ⭐⭐⭐
   ```javascript
   // 可以边检测边发送，不需要等待所有检测完成
   for (const payload of sqlPayloads) {
       const result = await test(payload);
       if (isVulnerable(result)) {
           Sentinel.emitFinding({...});  // 立即发送
       }
   }
   ```

2. **实时反馈** ⭐⭐⭐
   - 用户可以实时看到检测进度
   - 适合长时间运行的扫描任务
   - 可以实现进度条或实时更新

3. **内存效率** ⭐⭐
   - 不需要在内存中累积所有漏洞
   - findings 逐个发送到 Rust 端
   - 适合大规模扫描（可能检测出数百个漏洞）

4. **错误隔离** ⭐⭐
   ```javascript
   // 即使后续代码出错，已发送的 findings 仍然保留
   Sentinel.emitFinding({...});  // ✅ 已保存
   Sentinel.emitFinding({...});  // ✅ 已保存
   throw new Error('Oops!');     // ❌ 错误，但前面的漏洞不丢失
   ```

#### 新方案（Return Value）的优势

1. **代码直观** ⭐⭐⭐
   ```javascript
   // 标准的函数返回值，任何 JS 开发者都能理解
   return { 
       success: true,
       findings: [...] 
   };
   ```

2. **Node.js 兼容** ⭐⭐⭐
   - 没有任何自定义 API
   - 纯标准 JavaScript
   - 易于测试和调试

3. **类型明确** ⭐⭐
   ```javascript
   // 返回值结构清晰
   interface Result {
       success: boolean;
       findings: Finding[];
       metadata?: any;
   }
   ```

4. **测试友好** ⭐⭐⭐
   ```javascript
   // 可以直接测试函数返回值，无需 mock Sentinel API
   const result = await scan_transaction(ctx);
   assert.equal(result.findings.length, 2);
   assert.equal(result.findings[0].severity, 'high');
   ```

### 劣势对比

#### 当前方案的劣势

1. **非标准 API** ❌
   - 需要学习 `Sentinel.emitFinding()`
   - 不符合 Node.js 开发习惯
   - 增加学习成本

2. **测试困难** ❌
   ```javascript
   // 测试时需要 mock Sentinel API
   // 无法直接验证函数返回值
   ```

3. **不够直观** ❌
   - 看不出函数会产生什么输出
   - 副作用不明显

#### 新方案的劣势

1. **无法流式发送** ❌❌❌
   ```javascript
   // 必须等待所有检测完成
   const findings = [];
   for (const payload of sqlPayloads) {  // 可能需要很长时间
       const result = await test(payload);
       if (isVulnerable(result)) {
           findings.push({...});
       }
   }
   return { findings };  // 用户必须等到这里才能看到结果
   ```

2. **内存占用** ❌
   - 必须在内存中累积所有 findings
   - 对于大规模扫描可能是问题

3. **错误丢失风险** ❌
   ```javascript
   const findings = [];
   findings.push({...});  // 找到漏洞
   findings.push({...});  // 找到漏洞
   throw new Error('Oops!');  // ❌ 错误导致 findings 全部丢失
   ```

4. **向后兼容** ❌❌
   - 需要迁移现有所有插件
   - 可能影响已发布的插件

## 推荐方案：混合模式（最佳实践）

### 设计思路

**同时支持两种方式**，让插件开发者根据场景选择：

1. **保留 `Sentinel.emitFinding()`** - 用于实时发送、流式处理
2. **支持返回值** - 用于简单场景、标准化 API

### 实现方案

#### JavaScript 端

```javascript
// 方式 1: 使用 Sentinel.emitFinding()（推荐用于复杂扫描）
export async function scan_transaction_streaming(ctx) {
    for (const payload of payloads) {
        if (isVulnerable(payload)) {
            Sentinel.emitFinding({...});  // 实时发送
        }
    }
}

// 方式 2: 返回值（推荐用于简单检测）
export async function scan_transaction_simple(ctx) {
    return {
        success: true,
        findings: [
            { title: 'XSS', severity: 'high', ... },
            { title: 'SQLi', severity: 'critical', ... }
        ]
    };
}

// 方式 3: 混合模式（最灵活）
export async function scan_transaction_hybrid(ctx) {
    // 流式发送部分漏洞
    Sentinel.emitFinding({ title: 'Finding 1', ... });
    
    // 同时返回其他信息
    return {
        success: true,
        findings: [
            { title: 'Finding 2', ... }
        ],
        metadata: { scanned: 100, time: 1234 }
    };
}
```

#### Rust 端实现

```rust
pub async fn execute_agent(&mut self, input: &serde_json::Value) 
    -> Result<(Vec<Finding>, Option<serde_json::Value>)> 
{
    // 调用插件函数
    self.call_plugin_function("analyze", input).await?;
    
    // 方式 1: 从 op_emit_finding 收集的 findings
    let emitted_findings = {
        let op_state = self.runtime.op_state();
        let op_state_borrow = op_state.borrow();
        let plugin_ctx = op_state_borrow.borrow::<PluginContext>();
        plugin_ctx.take_findings()
    };
    
    // 方式 2: 从返回值中解析 findings
    let return_value = {
        let op_state = self.runtime.op_state();
        let op_state_borrow = op_state.borrow();
        let plugin_ctx = op_state_borrow.borrow::<PluginContext>();
        plugin_ctx.take_last_result()
    };
    
    let mut returned_findings = Vec::new();
    if let Some(ref value) = return_value {
        if let Some(findings_array) = value.get("findings") {
            if let Ok(js_findings) = serde_json::from_value::<Vec<JsFinding>>(findings_array.clone()) {
                returned_findings = js_findings.into_iter()
                    .map(|js| Finding::from(js))
                    .collect();
            }
        }
    }
    
    // 合并两种方式的 findings
    let mut all_findings = emitted_findings;
    all_findings.extend(returned_findings);
    
    Ok((all_findings, return_value))
}
```

### 使用建议

| 场景 | 推荐方式 | 原因 |
|------|---------|------|
| **简单检测**（1-10 个漏洞） | Return Value | 代码简洁，易于测试 |
| **复杂扫描**（可能很多漏洞） | Sentinel.emitFinding() | 实时反馈，内存效率高 |
| **长时间运行** | Sentinel.emitFinding() | 用户可以看到进度 |
| **单元测试** | Return Value | 易于验证 |
| **生产扫描** | Sentinel.emitFinding() | 容错性好，不怕中途出错 |

## 迁移计划

如果决定实施混合模式：

### 阶段 1: 实现支持（向后兼容）✅

1. ✅ 保留 `op_emit_finding`（不破坏现有插件）
2. ✅ 增加返回值 findings 解析逻辑
3. ✅ 合并两种来源的 findings
4. ✅ 更新文档说明两种用法

### 阶段 2: 推广新方式（可选）

1. 更新插件生成模板，默认使用返回值
2. 为简单场景推荐返回值方式
3. 为复杂场景推荐 `emitFinding()` 方式
4. 提供迁移指南

### 阶段 3: 长期维护（推荐保留两者）

不建议移除 `op_emit_finding`，原因：
- 流式发送是重要特性
- 已有插件依赖此 API
- 两种方式各有优势

## 结论

### 回答原问题

**是否可以移除 op_emit_finding？**

**技术上可以，但不推荐。**

推荐方案：
1. ✅ **保留 `op_emit_finding`** - 用于流式发送、实时反馈
2. ✅ **同时支持返回值** - 用于简单场景、标准化 API
3. ✅ **Rust 端合并两者** - 给插件开发者最大灵活性

### 最佳实践

```javascript
// 推荐：简单插件使用返回值
export async function simple_check(ctx) {
    return {
        findings: [{ title: 'Issue', ... }]
    };
}

// 推荐：复杂扫描使用 emitFinding
export async function comprehensive_scan(ctx) {
    for (const test of tests) {
        const result = await runTest(test);
        if (result.vulnerable) {
            Sentinel.emitFinding(result.finding);  // 实时反馈
        }
    }
}
```

这种混合模式既保持了向后兼容性，又提供了更直观的 Node.js 风格 API，是最优解。

---

*文档更新时间：2026-01-09*
