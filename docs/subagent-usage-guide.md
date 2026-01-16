# Subagent 使用指南

## 概述

Subagent 是一个内置工具，允许主 Agent 委派子任务给独立的子 Agent 执行。子 Agent 拥有自己的执行上下文、工具集和迭代限制，适合用于：
- 并行处理多个独立子任务
- 委派需要专注处理的复杂子问题
- 角色分工（如 Researcher、Validator、Executor）

## 工具参数

### 必需参数
- `parent_execution_id` (string): 父执行 ID，通常从系统上下文获取
- `task` (string): 子 Agent 要执行的任务描述

### 可选参数
- `role` (string): 角色标签，用于前端显示（如 "Researcher"、"Validator"）
- `system_prompt` (string): 自定义系统提示词，覆盖父 Agent 的提示词
- `tool_config` (object): 工具配置 JSON，指定子 Agent 可用的工具集
- `max_iterations` (integer): 最大迭代次数，默认 6
- `timeout_secs` (integer): 超时秒数，默认继承父 Agent
- `inherit_parent_llm` (boolean): 是否继承父 LLM 配置，默认 true（当前仅支持 true）
- `inherit_parent_tools` (boolean): 是否继承父工具配置，默认 false

## 使用示例

### 示例 1: 基础委派（使用默认配置）
```json
{
  "parent_execution_id": "current_execution_id",
  "task": "Research the latest CVE vulnerabilities for nginx",
  "role": "Researcher"
}
```
子 Agent 将使用默认工具集（所有工具，除了 `subagent_run` 本身）。

### 示例 2: 指定工具集
```json
{
  "parent_execution_id": "current_execution_id",
  "task": "Scan ports on 192.168.1.1 and identify services",
  "role": "Scanner",
  "tool_config": {
    "enabled": true,
    "selection_strategy": "Manual",
    "max_tools": 10,
    "fixed_tools": ["port_scan", "http_request", "local_time"],
    "disabled_tools": []
  }
}
```

### 示例 3: 继承父工具集
```json
{
  "parent_execution_id": "current_execution_id",
  "task": "Validate the scan results and generate a report",
  "role": "Validator",
  "inherit_parent_tools": true
}
```
子 Agent 将使用与父 Agent 相同的工具配置（但仍会禁用 `subagent_run` 防递归）。

### 示例 4: 自定义系统提示词
```json
{
  "parent_execution_id": "current_execution_id",
  "task": "Analyze the HTTP response for SQL injection vulnerabilities",
  "role": "Security Analyst",
  "system_prompt": "You are a security expert specializing in SQL injection detection. Focus on finding evidence of database errors, union-based injection, and time-based blind injection patterns.",
  "tool_config": {
    "enabled": true,
    "selection_strategy": "Manual",
    "fixed_tools": ["http_request", "shell"]
  }
}
```

## 工具配置说明

`tool_config` 对象支持以下字段：
- `enabled` (boolean): 是否启用工具调用
- `selection_strategy` (string): 工具选择策略
  - `"All"`: 所有工具（不推荐）
  - `"Keyword"`: 关键词匹配（快速，免费）
  - `"LLM"`: LLM 智能分析（准确，有成本）
  - `"Hybrid"`: 混合策略（推荐）
  - `"Manual"`: 手动指定工具列表
  - `"Ability"`: 能力组模式
- `max_tools` (integer): 最大工具数量
- `fixed_tools` (array): 固定启用的工具列表
- `disabled_tools` (array): 禁用的工具列表

## 前端展示

子 Agent 执行时会在 Agent 视图顶部显示一个面板，包含：
- 子 Agent 列表（角色、状态、任务）
- 实时状态更新（running/completed/failed）
- 执行结果摘要
- 使用的工具集

## 并发控制

当前实现：
- 全局并发上限：3 个子 Agent
- 单父任务上限：2 个子 Agent
- 超额请求会返回错误："Subagent concurrency limit reached"

## 防递归机制

子 Agent 的工具配置会自动禁用 `subagent_run`，防止无限递归调用。即使用户在 `tool_config` 中显式启用，系统也会强制禁用。

## 事件流

后端发射的事件：
- `subagent:start`: 子 Agent 开始执行
  - `execution_id`: 子 Agent 执行 ID
  - `parent_execution_id`: 父执行 ID
  - `role`: 角色标签
  - `task`: 任务描述
  
- `subagent:done`: 子 Agent 执行完成
  - `execution_id`: 子 Agent 执行 ID
  - `parent_execution_id`: 父执行 ID
  - `success`: 是否成功
  - `output`: 输出结果
  
- `subagent:error`: 子 Agent 执行失败
  - `execution_id`: 子 Agent 执行 ID
  - `parent_execution_id`: 父执行 ID
  - `error`: 错误信息

子 Agent 的流式输出（`agent:chunk`、`agent:tool_call_*` 等）会使用子 Agent 的 `execution_id`，与父 Agent 隔离。

## 最佳实践

1. **明确角色定义**：使用 `role` 参数给子 Agent 清晰的角色定位
2. **限制工具集**：为子 Agent 指定最小必要工具集，避免 token 浪费
3. **控制迭代次数**：根据任务复杂度设置合理的 `max_iterations`
4. **避免过度委派**：不要为简单任务创建子 Agent，会增加延迟和成本
5. **并行处理**：可以同时启动多个子 Agent 处理独立任务（注意并发限制）

## 典型使用场景

### 场景 1: 信息收集 + 分析分离
```
主 Agent: "我需要分析 example.com 的安全性"
  ↓
  委派 Subagent 1 (Researcher): "收集 example.com 的所有子域名和开放端口"
  委派 Subagent 2 (Analyzer): "分析已知的 example.com CVE 漏洞"
  ↓
主 Agent: 汇总两个子 Agent 的结果，生成综合报告
```

### 场景 2: 多目标并行扫描
```
主 Agent: "扫描 192.168.1.1-10 的端口"
  ↓
  委派 Subagent 1: "扫描 192.168.1.1-5"
  委派 Subagent 2: "扫描 192.168.1.6-10"
  ↓
主 Agent: 合并扫描结果
```

### 场景 3: 验证与审查
```
主 Agent: "生成 SQL 注入 payload"
  ↓
  委派 Subagent (Validator): "验证这些 payload 的有效性和安全性"
  ↓
主 Agent: 根据验证结果调整 payload
```

## 注意事项

- 子 Agent 会创建独立的对话历史（独立 `execution_id`）
- 子 Agent 的历史不会自动合并到父 Agent 的对话中
- 子 Agent 执行完成后，父 Agent 只会收到结果摘要（通过工具返回值）
- 如需查看子 Agent 的完整执行过程，可在前端 Subagent 面板中展开查看
- 子 Agent 不支持文档附件和图片附件（当前版本）

## 故障排查

### 问题：前端看不到 subagent_run 工具
**原因**：`get_builtin_tools_with_status` 函数未包含该工具
**解决**：已在最新版本中修复

### 问题：子 Agent 无法调用某些工具
**原因**：工具被 `disabled_tools` 禁用或未在 `fixed_tools` 中指定
**解决**：检查 `tool_config` 配置，确保所需工具已启用

### 问题：并发限制错误
**原因**：超过全局或单父任务的并发上限
**解决**：等待现有子 Agent 完成，或调整并发限制（需修改代码）

### 问题：子 Agent 递归调用
**原因**：系统会自动防止，但如果出现，检查 `normalize_tool_config` 逻辑
**解决**：确保 `subagent_run` 始终在 `disabled_tools` 中
