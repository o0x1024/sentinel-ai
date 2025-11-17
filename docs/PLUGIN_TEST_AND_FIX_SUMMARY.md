# 插件执行测试与自动修复功能实现总结

## 实施概述

为AI生成的插件添加了执行测试和自动修复功能，确保生成的插件代码可以正常执行。当插件执行失败时，系统会自动调用LLM进行修复，最多尝试3次。

## 修改的文件

### 1. src-tauri/src/generators/validator.rs

**新增内容：**

- `ExecutionTestResult` 结构体：存储执行测试结果
  - `success`: 测试是否成功
  - `error_message`: 错误消息
  - `error_details`: 详细错误信息

- `PluginValidator::test_plugin_execution()` 方法：
  - 创建隔离的Deno运行时
  - 注入Mock API（Sentinel和Deno.core.ops）
  - 测试必需函数的存在性和可调用性
  - 使用模拟数据调用插件函数
  - 返回详细的测试结果

- `extract_error_message()` 辅助函数：
  - 从详细错误堆栈中提取关键错误信息

**测试覆盖：**
- 函数存在性检查（get_metadata, scan_response, scan_request）
- 元数据正确性验证
- 函数可调用性测试
- API可用性验证

### 2. src-tauri/src/generators/advanced_generator.rs

**新增内容：**

- `MAX_FIX_ATTEMPTS` 常量：最大修复尝试次数（3次）

- `GeneratedPlugin` 结构体新增字段：
  - `execution_test: Option<ExecutionTestResult>`: 执行测试结果
  - `fix_attempts: u32`: 修复尝试次数

- `fix_plugin_code()` 方法：
  - 构建修复prompt
  - 调用LLM生成修复代码
  - 提取和清理修复后的代码

**修改内容：**

- `generate_single_plugin()` 方法：
  - 在验证后添加执行测试
  - 测试失败时自动调用修复流程
  - 重新验证和测试修复后的代码
  - 最多重试3次
  - 更新插件状态和描述

### 3. src-tauri/src/generators/prompt_templates.rs

**新增内容：**

- `build_fix_prompt()` 方法：构建插件修复prompt
  - 包含原始代码和错误信息
  - 提供详细的修复指导
  - 列出常见问题检查清单
  - 明确输出格式要求

**Prompt内容：**
- 错误信息展示
- 原始代码展示
- 修复指导（7个要点）
- 常见问题检查清单
- 输出格式要求

### 4. src-tauri/src/generators/mod.rs

**修改内容：**
- 导出 `ExecutionTestResult` 类型

## 工作流程

```
插件生成
    ↓
语法验证
    ↓
执行测试
    ↓
测试通过？
    ├─ 是 → 质量评分 → 自动审批 → 完成
    └─ 否 ↓
         构建修复prompt
              ↓
         调用LLM修复
              ↓
         提取修复代码
              ↓
         重新验证
              ↓
         重新测试
              ↓
         尝试次数 < 3？
              ├─ 是 → 返回测试步骤
              └─ 否 → 标记为ValidationFailed
```

## 关键特性

### 1. 隔离测试环境
- 使用独立的Deno运行时
- 不影响主系统
- 完整的Mock API支持

### 2. 智能错误提取
- 从复杂的错误堆栈中提取关键信息
- 提供简洁和详细两种错误信息
- 便于LLM理解和修复

### 3. 渐进式修复
- 最多3次修复尝试
- 每次修复后重新验证和测试
- 避免无限循环

### 4. 完整的结果追踪
- 记录执行测试结果
- 记录修复尝试次数
- 更新插件描述和状态

## 测试场景

执行测试验证以下内容：

1. **必需函数**
   - `get_metadata()` 存在且可调用
   - 返回包含id和name的对象

2. **扫描函数**
   - `scan_response()` 存在且可调用
   - 可选的 `scan_request()` 函数

3. **API调用**
   - `Sentinel.emitFinding()` 可用
   - `Deno.core.ops.op_emit_finding()` 可用

4. **异步支持**
   - 检测并支持异步函数
   - Promise返回值处理

## 修复Prompt结构

```markdown
# Plugin Code Fix Task

## Error Information
- Error: [错误消息]
- Detailed Error: [详细堆栈]

## Original Plugin Code
[原始TypeScript代码]

## Fix Instructions
1. 修复具体错误
2. 维护插件接口
3. 正确检测漏洞
4. 使用正确的TypeScript语法
5. 正确调用API
6. 包含错误处理
7. 确保可执行

## Common Issues to Check
- 函数签名错误
- 未定义的变量
- API使用错误
- 元数据字段缺失
- 语法错误
- 类型错误
- 属性访问错误

## Output Format
[仅返回修复后的TypeScript代码]
```

## 状态管理

插件状态根据测试结果更新：

| 测试结果 | 质量分数 | 最终状态 |
|---------|---------|---------|
| 失败（达到重试上限） | - | ValidationFailed |
| 成功 | 高（≥80） | Approved |
| 成功 | 中（60-80） | PendingReview |
| 成功 | 低（<60） | PendingReview |
| 成功 | 有安全问题 | Rejected |

## 日志示例

```
INFO: Generating xss plugin for example.com (with Few-shot learning)
INFO: Testing plugin execution
WARN: Plugin execution test failed (attempt 1/3): get_metadata function not found
INFO: Attempting to fix plugin code (attempt 1)
INFO: Plugin code fixed, re-validating...
INFO: Plugin execution test passed after fix attempt 1
INFO: Plugin ai_gen_xss_20241114_123456 auto-approved: High quality and passed all checks
```

## 性能影响

- **额外时间成本**：每次修复需要1次LLM调用（约2-5秒）
- **最大额外时间**：3次修复 × 5秒 = 15秒
- **成功率提升**：预计提升20-30%的插件可用率
- **资源消耗**：每个插件额外消耗0-3次LLM调用

## 编译结果

✅ 编译成功，无错误
⚠️ 115个警告（主要是未使用的导入和变量）

## 未来改进方向

1. **可配置的重试策略**
   - 支持自定义重试次数
   - 支持指数退避策略

2. **更智能的错误分析**
   - 使用LLM分析错误类型
   - 提供更精准的修复建议

3. **修复历史记录**
   - 保存每次修复的代码版本
   - 分析修复模式以改进生成质量

4. **测试覆盖率提升**
   - 添加更多测试场景
   - 支持异步函数的完整测试
   - 测试实际的漏洞检测逻辑

5. **性能优化**
   - 并行测试多个插件
   - 缓存常见错误的修复方案

## 相关文档

- [详细实现文档](./PLUGIN_EXECUTION_TEST_AND_FIX.md)
- [插件生成架构](./PLAN_B_ARCHITECTURE.md)
- [自动审批机制](./PLUGIN_REVIEW_INTEGRATION_SUMMARY.md)

