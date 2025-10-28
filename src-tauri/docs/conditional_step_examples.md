# 条件判断步骤使用指南

## 概述

条件判断步骤 (`StepType::Conditional`) 允许在执行计划中根据动态条件执行不同的操作。这为智能任务执行提供了分支逻辑能力。

## 基本结构

```json
{
  "id": "condition_check_1",
  "name": "检查扫描结果",
  "description": "根据扫描结果决定后续操作",
  "step_type": "Conditional",
  "parameters": {
    "condition": "条件表达式",
    "on_true": "条件为真时的操作",
    "on_false": "条件为假时的操作"
  }
}
```

## 支持的条件表达式

### 1. 简单条件

#### 内置函数
- `non_empty_output(step_name)` - 检查步骤输出是否非空
- `schema_nonempty(step_name)` - 检查步骤输出对象是否非空
- `is_true(variable_name)` - 检查变量是否为真
- `gt(variable_name, number)` - 数值大于比较
- `contains(variable_name, "substring")` - 字符串包含检查

#### 比较操作符
- `==` - 等于
- `!=` - 不等于
- `>` - 大于
- `<` - 小于
- `>=` - 大于等于
- `<=` - 小于等于

#### 示例
```json
{
  "condition": "non_empty_output(port_scan)",
  "on_true": "continue",
  "on_false": "skip"
}
```

```json
{
  "condition": "vulnerability_count > 5",
  "on_true": {
    "type": "set_variable",
    "name": "alert_level",
    "value": "high"
  }
}
```

### 2. 复杂条件组合

#### 逻辑操作符
- `AND` - 逻辑与
- `OR` - 逻辑或
- `NOT` 或 `!` - 逻辑非
- `()` - 括号分组

#### 示例
```json
{
  "condition": "non_empty_output(scan_result) AND vulnerability_count > 0",
  "on_true": {
    "type": "emit_event",
    "data": {
      "event_type": "vulnerabilities_found",
      "severity": "medium"
    }
  }
}
```

```json
{
  "condition": "(scan_status == \"completed\") AND (NOT error_occurred)",
  "on_true": "continue",
  "on_false": {
    "type": "log_message",
    "message": "扫描未正常完成",
    "level": "error"
  }
}
```

## 支持的操作类型

### 1. 简单字符串操作
- `"continue"` - 继续执行
- `"skip"` - 跳过后续步骤
- `"abort"` - 中止执行

### 2. 复杂对象操作

#### 设置变量
```json
{
  "type": "set_variable",
  "name": "variable_name",
  "value": "variable_value"
}
```

#### 发送事件
```json
{
  "type": "emit_event",
  "data": {
    "event_type": "custom_event",
    "payload": "event_data"
  }
}
```

#### 记录日志
```json
{
  "type": "log_message",
  "message": "日志消息内容",
  "level": "info"  // "error", "warn", "info", "debug"
}
```

## 完整示例

### 安全扫描条件判断

```json
{
  "id": "vulnerability_assessment",
  "name": "漏洞评估决策",
  "description": "根据扫描结果决定是否需要深度分析",
  "step_type": "Conditional",
  "parameters": {
    "condition": "(vulnerability_count > 10) OR (critical_vulnerabilities > 0)",
    "on_true": {
      "type": "set_variable",
      "name": "requires_deep_scan",
      "value": true
    },
    "on_false": {
      "type": "log_message",
      "message": "未发现严重漏洞，跳过深度扫描",
      "level": "info"
    }
  },
  "estimated_duration": 1,
  "retry_config": {
    "max_retries": 0,
    "retry_interval": 0,
    "backoff_strategy": "Fixed"
  },
  "preconditions": ["non_empty_output(initial_scan)"],
  "postconditions": []
}
```

### 端口扫描结果处理

```json
{
  "id": "port_scan_analysis",
  "name": "端口扫描结果分析",
  "description": "根据开放端口数量决定后续扫描策略",
  "step_type": "Conditional",
  "parameters": {
    "condition": "open_ports_count >= 5 AND contains(service_list, \"http\")",
    "on_true": {
      "type": "emit_event",
      "data": {
        "event_type": "web_services_detected",
        "action": "start_web_vulnerability_scan"
      }
    },
    "on_false": {
      "type": "set_variable",
      "name": "scan_strategy",
      "value": "minimal"
    }
  }
}
```

## 变量访问

条件判断可以访问以下数据：

1. **步骤结果**: `step_result_{step_name}` 格式
2. **共享变量**: 直接使用变量名
3. **条件结果**: `condition_result_{step_name}` 格式（自动生成）

## 最佳实践

1. **条件表达式简洁性**: 保持条件表达式简单明了，复杂逻辑可拆分为多个条件步骤
2. **错误处理**: 为关键条件提供 `on_false` 操作，避免执行流程中断
3. **变量命名**: 使用描述性的变量名，便于条件表达式理解
4. **日志记录**: 在关键决策点添加日志记录，便于调试和审计
5. **前置条件**: 合理设置前置条件，确保条件判断所需数据已准备就绪

## 调试技巧

1. 使用 `log_message` 操作记录条件评估过程
2. 检查共享数据中的变量值和步骤结果
3. 使用简单条件测试，逐步构建复杂表达式
4. 查看执行日志中的条件评估信息

这个条件判断系统为智能任务执行提供了强大的分支控制能力，支持复杂的业务逻辑实现。
