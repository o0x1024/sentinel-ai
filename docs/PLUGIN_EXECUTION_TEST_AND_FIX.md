# 插件执行测试与自动修复功能

## 概述

本功能为AI生成的插件添加了执行测试和自动修复能力。当生成的插件代码无法正常执行时，系统会自动调用LLM进行修复，最多尝试3次。

## 功能特性

### 1. 插件执行测试

在插件生成后，系统会自动进行执行测试：

- 创建隔离的Deno运行时环境
- 模拟Sentinel API和Deno.core.ops
- 测试必需函数的存在和可调用性
- 使用模拟数据调用插件函数
- 捕获并记录详细的错误信息

### 2. 自动修复机制

当插件执行测试失败时：

1. 系统提取错误信息（错误消息和详细堆栈）
2. 构建修复prompt，包含：
   - 原始插件代码
   - 错误信息和详细堆栈
   - 漏洞类型上下文
   - 修复指导和常见问题检查清单
3. 调用LLM生成修复后的代码
4. 重新验证和测试修复后的代码
5. 最多重试3次，直到测试通过或达到重试上限

### 3. 结果追踪

生成的插件包含以下额外信息：

- `execution_test`: 执行测试结果
  - `success`: 是否成功
  - `error_message`: 错误消息（如果失败）
  - `error_details`: 详细错误信息
- `fix_attempts`: 修复尝试次数
- `description`: 包含修复信息的描述

## 技术实现

### 核心组件

#### 1. ExecutionTestResult (validator.rs)

```rust
pub struct ExecutionTestResult {
    pub success: bool,
    pub error_message: Option<String>,
    pub error_details: Option<String>,
}
```

#### 2. PluginValidator::test_plugin_execution (validator.rs)

执行测试方法：
- 创建隔离的Deno运行时
- 注入Mock API
- 执行插件代码并测试必需函数
- 返回详细的测试结果

#### 3. PromptTemplateBuilder::build_fix_prompt (prompt_templates.rs)

构建修复prompt：
- 包含原始代码和错误信息
- 提供详细的修复指导
- 列出常见问题检查清单
- 明确输出格式要求

#### 4. AdvancedPluginGenerator::fix_plugin_code (advanced_generator.rs)

修复流程：
- 调用LLM生成修复代码
- 提取和清理代码
- 返回修复后的代码

### 生成流程

```
1. 生成插件代码
   ↓
2. 语法验证
   ↓
3. 执行测试 ←─────┐
   ↓              │
4. 测试失败？     │
   ├─是 → 修复代码 ┘ (最多3次)
   └─否
   ↓
5. 质量评分
   ↓
6. 自动审批
   ↓
7. 返回结果
```

## 配置

### 最大重试次数

在 `advanced_generator.rs` 中定义：

```rust
const MAX_FIX_ATTEMPTS: u32 = 3;
```

可以根据需要调整此值。

## 测试场景

执行测试会验证以下内容：

1. **函数存在性**
   - `get_metadata()` 函数存在
   - `scan_response()` 函数存在
   - 可选的 `scan_request()` 函数

2. **元数据正确性**
   - 返回对象类型
   - 包含必需字段（id, name）

3. **函数可调用性**
   - 使用模拟数据调用函数
   - 检查是否抛出异常
   - 验证异步函数支持

4. **API可用性**
   - Sentinel.emitFinding() 可用
   - Deno.core.ops.op_emit_finding() 可用

## 错误处理

### 错误信息提取

系统会从详细的错误堆栈中提取关键错误信息：

```rust
fn extract_error_message(error: &str) -> String {
    if let Some(line) = error.lines().find(|l| l.contains("Error:")) {
        line.trim().to_string()
    } else if let Some(line) = error.lines().next() {
        line.trim().to_string()
    } else {
        error.to_string()
    }
}
```

### 常见错误类型

修复prompt包含常见问题检查清单：

- 缺失或错误的函数签名
- 未定义的变量或函数
- API使用错误
- 元数据字段缺失
- 语法错误
- TypeScript类型错误
- 访问未定义的属性

## 日志输出

系统会记录详细的日志：

```
INFO: Generating xss plugin for example.com (with Few-shot learning)
INFO: Testing plugin execution
WARN: Plugin execution test failed (attempt 1/3): get_metadata function not found
INFO: Attempting to fix plugin code (attempt 1)
INFO: Plugin code fixed, re-validating...
INFO: Plugin execution test passed after fix attempt 1
INFO: Plugin ai_gen_xss_20241114_123456 auto-approved: High quality and passed all checks
```

## 状态管理

插件状态会根据测试结果更新：

- 测试通过 + 高质量 → `Approved`
- 测试通过 + 中等质量 → `PendingReview`
- 测试失败（达到重试上限）→ `ValidationFailed`
- 安全问题 → `Rejected`

## 使用示例

生成插件时，系统会自动进行测试和修复：

```rust
let generator = AdvancedPluginGenerator::new(ai_manager);
let request = PluginGenerationRequest {
    analysis: website_analysis,
    vuln_types: vec!["xss".to_string(), "sqli".to_string()],
    target_endpoints: None,
    requirements: None,
};

let plugins = generator.generate(request).await?;

for plugin in plugins {
    println!("Plugin: {}", plugin.plugin_name);
    println!("Status: {:?}", plugin.status);
    println!("Fix attempts: {}", plugin.fix_attempts);
    
    if let Some(test) = plugin.execution_test {
        println!("Execution test: {}", if test.success { "PASS" } else { "FAIL" });
        if let Some(err) = test.error_message {
            println!("Error: {}", err);
        }
    }
}
```

## 性能考虑

- 每次修复需要额外的LLM调用
- 最多3次重试，避免无限循环
- 执行测试在隔离环境中进行，不影响主系统
- 测试使用模拟数据，不需要真实HTTP流量

## 未来改进

1. **可配置的重试策略**
   - 支持自定义重试次数
   - 支持指数退避

2. **更智能的错误分析**
   - 使用LLM分析错误类型
   - 提供更精准的修复建议

3. **修复历史记录**
   - 保存每次修复的代码版本
   - 分析修复模式以改进生成质量

4. **测试覆盖率提升**
   - 添加更多测试场景
   - 支持异步函数的完整测试

## 相关文件

- `src-tauri/src/generators/validator.rs` - 执行测试实现
- `src-tauri/src/generators/advanced_generator.rs` - 修复流程
- `src-tauri/src/generators/prompt_templates.rs` - 修复prompt模板
- `src-tauri/src/generators/mod.rs` - 类型导出

