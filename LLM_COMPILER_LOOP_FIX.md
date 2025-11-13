# LLM-Compiler 循环执行问题修复报告

## 问题描述

LLM-Compiler 引擎在执行简单任务（如端口扫描）时出现循环重复执行的问题：
- 任务成功完成（30/30任务全部成功）
- 但 Joiner 持续返回 `Continue` 决策
- 执行10轮后才因达到最大轮次而停止
- 每轮都重复执行相同的端口扫描任务

## 根本原因分析

### 1. 提示词模板占位符不匹配

**问题**：数据库中存储的 Joiner 提示词使用了 `{current_task}` 和 `{dependencies}` 占位符，但代码中的占位符替换逻辑没有处理这些变量。

**影响**：
- AI 收到未替换的模板变量
- AI 返回"请提供具体的任务内容"
- Joiner 无法解析 AI 响应
- 默认返回 `Continue` 决策

**证据**：
```
[LLM Request Log]
当前任务：{current_task}
依赖结果：{dependencies}

[AI Response]
我正在等待您提供具体的 **当前任务** 和 **依赖结果** 信息...
```

### 2. 目标完成度评估不准确

**问题**：对于扫描类任务，即使所有扫描成功完成，目标完成度仍被评估为50%。

**原因**：
- `heuristic_completion_estimate` 方法只检查是否发现了漏洞、开放端口等
- 没有发现问题的扫描被认为是"不完整"的结果
- 实际上，完成扫描本身就是一个有价值的结果

### 3. 决策阈值过于严格

**问题**：Joiner 的完成判断条件过于严格：
- 要求 `goal_completion >= 0.8` 才认为完成
- 对于高成功率的简单任务没有提前结束机制

## 修复方案

### 1. 扩展占位符替换逻辑

在 `joiner.rs` 的 `build_ai_decision_prompt` 方法中添加了额外的占位符支持：

```rust
let replaced = Self::apply_placeholders(&dynamic, vec![
    // 原有占位符
    ("{{USER_QUERY}}", original_query),
    ("{original_query}", original_query),
    // ... 其他占位符 ...
    
    // 新增占位符支持
    ("{current_task}", original_query),
    ("{dependencies}", &execution_summary),
    ("{original_plan}", &original_plan),
    ("{execution_status}", &execution_status),
    ("{error_info}", &error_info),
]);
```

添加了辅助方法：
- `format_error_info()`: 格式化错误信息
- `format_execution_context()`: 格式化执行上下文

### 2. 改进目标完成度评估

更新了 `heuristic_completion_estimate` 方法：

```rust
// 对于扫描类任务，即使没有发现问题也算完成
if output.contains_key("scan_results") || 
   output.contains_key("scanned_ports") ||
   output.contains_key("closed_ports") ||
   output.contains_key("scan_summary") ||
   output.contains_key("execution_success") {
    score += 0.4; // 扫描完成本身就是一个有价值的结果
    factors += 1;
}

// 改进成功判断条件
if output.get("success").and_then(|v| v.as_bool()).unwrap_or(false) ||
   output.get("execution_success").and_then(|v| v.as_bool()).unwrap_or(false) {
    score += 0.3;
    factors += 1;
}
```

### 3. 优化决策逻辑

改进了 `synthesize_decision` 方法的完成判断条件：

```rust
// 决策逻辑 - 改进判断条件
let should_complete = 
    goal_completion >= 0.7 ||                      // 目标完成度高（降低阈值）
    (goal_completion >= 0.5 && round >= 3) ||      // 中等完成度且已执行多轮
    success_rate < 0.3 ||                           // 成功率太低
    overall_risk > 0.8 ||                           // 风险太高
    (success_rate >= 0.9 && round >= 2) ||         // 高成功率且已执行2轮以上
    round >= self.config.max_iterations;            // 达到最大轮次
```

## 预期效果

### 修复前
- ❌ 执行10轮，重复相同任务30次
- ❌ 目标完成度固定在50%
- ❌ AI 无法理解提示词模板
- ❌ 浪费资源和时间

### 修复后
- ✅ 提示词占位符正确替换
- ✅ AI 能理解任务上下文
- ✅ 扫描类任务完成度正确评估（70-90%）
- ✅ 高成功率任务在2-3轮后正确结束
- ✅ 资源利用更高效

## 测试建议

1. **简单端口扫描测试**
   - 扫描3个目标的常见端口
   - 预期：2-3轮后完成，不再循环

2. **有发现结果的测试**
   - 扫描已知有开放端口的目标
   - 预期：目标完成度 > 80%，1-2轮完成

3. **复杂任务测试**
   - 需要多步骤依赖的任务
   - 预期：正常执行，合理轮次后完成

## 相关文件

- `src-tauri/src/engines/llm_compiler/joiner.rs` - Joiner 决策器
- `src-tauri/src/engines/llm_compiler/engine_adapter.rs` - LLMCompiler 引擎适配器
- `src-tauri/src/engines/llm_compiler/planner.rs` - 规划器

## 后续优化建议

1. **数据库提示词标准化**
   - 统一所有提示词模板的占位符格式
   - 建立占位符命名规范

2. **智能决策改进**
   - 增加基于任务类型的动态阈值
   - 根据历史执行模式学习最优决策点

3. **性能监控**
   - 添加轮次统计
   - 记录决策准确率
   - 识别异常循环模式

## 版本信息

- 修复日期：2025-11-13
- 影响版本：所有使用 LLMCompiler 引擎的版本
- 编译状态：✅ 通过（118 warnings）

