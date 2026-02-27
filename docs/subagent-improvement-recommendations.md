# Subagent 工具使用率改进建议

## 问题诊断

### 当前状况
从日志分析发现，虽然 `subagent_run` 工具已经在能力组中配置并成功加载，但主 Agent 在执行 CTF 加固题时并未主动调用该工具。

### 根本原因
1. **能力组指令为空**：`CTF竞赛全能专家` 能力组的 `instructions` 和 `additional_notes` 字段为空
2. **缺少使用引导**：LLM 不知道何时、如何使用 subagent 工具
3. **工具描述不够突出**：subagent 的使用场景在工具元数据中不够明确

## 改进方案

### 方案 1: 增强能力组指令（推荐）

为 `CTF竞赛全能专家` 能力组添加详细的 `instructions`，明确指导何时使用 subagent。

**实施步骤**：

1. 更新数据库中的能力组配置：

```sql
UPDATE ability_groups 
SET instructions = '
你是一名 CTF 竞赛专家，擅长解决各类 CTF 题目。在处理复杂任务时，应该合理委派子任务：

## 任务分解策略

对于复杂的 CTF 题目，使用 **subagent_run** 工具将任务分解为多个独立子任务：

### 何时使用 Subagent：
1. **信息收集阶段**：需要并行收集多个目标的信息（端口扫描、子域名枚举、漏洞查询）
2. **多步骤分析**：需要先完成侦察，再进行漏洞分析，最后生成利用代码
3. **验证与审查**：需要对生成的 payload 或脚本进行独立验证
4. **并行处理**：多个独立目标需要同时处理（如多个 IP、多个服务）

### 典型工作流：

#### 1. Web 应用加固题
```
主任务: 分析并加固 Web 应用
  ↓
  子任务1 (Researcher): 收集目标信息（技术栈、开放端口、已知漏洞）
  子任务2 (Analyzer): 分析源码或配置文件，识别安全问题
  子任务3 (Fixer): 生成修复方案和加固代码
  子任务4 (Validator): 验证修复方案的有效性
  ↓
主任务: 汇总结果，生成最终报告
```

#### 2. 二进制加固题
```
主任务: 分析二进制文件并加固
  ↓
  子任务1 (Scanner): 使用工具扫描二进制文件（checksec、strings、file）
  子任务2 (Analyzer): 分析漏洞类型（栈溢出、格式化字符串等）
  子任务3 (Patcher): 生成补丁或防护代码
  ↓
主任务: 编译并测试加固后的二进制
```

#### 3. 网络服务加固
```
主任务: 加固网络服务
  ↓
  子任务1 (Recon): 端口扫描 + 服务识别
  子任务2 (Vuln Scanner): 漏洞扫描（SQL注入、XSS、命令注入）
  子任务3 (Config Auditor): 审查配置文件（nginx、ssh、mysql等）
  子任务4 (Hardener): 生成加固配置和防火墙规则
  ↓
主任务: 应用加固措施并验证
```

### Subagent 调用示例：

**示例 1: 信息收集**
```json
{
  "parent_execution_id": "<current_execution_id>",
  "task": "扫描目标 192.168.1.100 的所有开放端口，并识别运行的服务版本",
  "role": "Port Scanner",
  "tool_config": {
    "enabled": true,
    "selection_strategy": "Manual",
    "fixed_tools": ["port_scan", "http_request", "shell"]
  },
  "max_iterations": 5
}
```

**示例 2: 源码分析**
```json
{
  "parent_execution_id": "<current_execution_id>",
  "task": "分析 /workspace/context/app.py 文件，识别所有潜在的安全漏洞（SQL注入、XSS、命令注入、路径遍历）",
  "role": "Code Analyzer",
  "system_prompt": "你是一名代码安全审计专家，专注于发现 Python Web 应用中的安全漏洞。",
  "tool_config": {
    "enabled": true,
    "selection_strategy": "Manual",
    "fixed_tools": ["shell", "http_request"]
  },
  "max_iterations": 8
}
```

**示例 3: 加固方案验证**
```json
{
  "parent_execution_id": "<current_execution_id>",
  "task": "验证我生成的 nginx 加固配置是否有效，检查是否存在配置错误或遗漏的安全措施",
  "role": "Config Validator",
  "inherit_parent_tools": true,
  "max_iterations": 4
}
```

### 注意事项：
- 每个子任务应该有**明确的目标**和**清晰的角色定位**
- 避免为简单任务创建 subagent（如单个命令执行）
- 合理控制并发（最多 2-3 个并行子任务）
- 子任务完成后，主任务负责**汇总和决策**

## 工具使用优先级

1. **简单命令执行** → 直接使用 `shell` 或 `terminal_server`
2. **单一工具调用** → 直接调用对应工具（如 `port_scan`、`http_request`）
3. **需要多步骤推理的子任务** → 使用 `subagent_run`
4. **需要专家审查** → 使用 `tenth_man_review`
5. **需要记录重要信息** → 使用 `memory_manager`
6. **需要跟踪任务进度** → 使用 `todos`
'
WHERE name = 'CTF竞赛全能专家';
```

2. 清理数据库中无效的工具名：

```sql
UPDATE ability_groups 
SET tool_ids = '["http_request","memory_manager","tenth_man_review","todos","shell","terminal_server","subagent_run","port_scan","web_search","ocr"]'
WHERE name = 'CTF竞赛全能专家';
```

### 方案 2: 优化 Subagent 工具元数据

增强 `subagent_run` 工具的描述，使其更容易被 LLM 选择。

**当前描述**：
```
"Spawn a subagent to run a focused task. Use this to delegate a subtask or parallelize work."
```

**改进后的描述**：
```
"Delegate complex subtasks to a specialized subagent. REQUIRED for: 1) Multi-step analysis that needs focused attention (recon→analysis→exploit), 2) Parallel processing of independent targets, 3) Role-based task separation (Scanner/Analyzer/Validator). Each subagent runs independently with its own tools and context. Use this to break down complex CTF challenges into manageable pieces."
```

**实施位置**：
- `src-tauri/sentinel-tools/src/buildin_tools/subagent_tool.rs` 中的 `DESCRIPTION` 常量
- `src-tauri/src/agents/tool_router.rs` 中的 `ToolMetadata`

### 方案 3: 添加示例对话到系统提示词

在 Agent 的系统提示词中添加使用 subagent 的示例对话。

**实施位置**：`src-tauri/src/agents/agent_builder.rs`

```rust
pub const CTF_EXPERT_PREAMBLE: &str = r#"
你是一名 CTF 竞赛专家。对于复杂任务，你应该使用 subagent_run 工具进行任务分解。

示例对话：

用户: 请分析并加固这个 Web 应用
助手: 我将把这个任务分解为几个步骤：
<tool_call>
{
  "name": "subagent_run",
  "arguments": {
    "parent_execution_id": "...",
    "task": "扫描目标 example.com 的所有开放端口和服务",
    "role": "Recon Agent"
  }
}
</tool_call>

<tool_call>
{
  "name": "subagent_run",
  "arguments": {
    "parent_execution_id": "...",
    "task": "分析 Web 应用源码，识别安全漏洞",
    "role": "Code Analyzer"
  }
}
</tool_call>
"#;
```

### 方案 4: 在工具路由中提升 Subagent 优先级

修改 `tool_router.rs` 中的工具元数据，增加 subagent 的权重。

```rust
ToolMetadata {
    id: SubagentTool::NAME.to_string(),
    name: SubagentTool::NAME.to_string(),
    description: "CRITICAL TOOL for complex tasks: Delegate multi-step subtasks to specialized subagents. Use for parallel processing, role-based separation, or focused analysis. Essential for CTF challenges requiring recon→analysis→exploit workflow.".to_string(),
    category: ToolCategory::AI,
    tags: vec![
        "delegation".to_string(),
        "parallel".to_string(),
        "multi-step".to_string(),
        "ctf".to_string(),      // 新增
        "analysis".to_string(),  // 新增
        "workflow".to_string(),  // 新增
    ],
    cost_estimate: ToolCostEstimate::High,
}
```

## 实施优先级

### 高优先级（立即实施）
1. ✅ **方案 1**: 更新 CTF 能力组的 instructions
2. ✅ **方案 2**: 优化 subagent_run 工具描述

### 中优先级（短期实施）
3. **方案 4**: 增加工具标签和优先级提示

### 低优先级（长期优化）
4. **方案 3**: 添加示例对话（可能增加 token 消耗）

## 预期效果

实施方案 1 和 2 后，LLM 应该能够：
1. **识别适合委派的任务**：多步骤分析、并行处理、角色分工
2. **主动调用 subagent_run**：在遇到复杂 CTF 题目时自动分解任务
3. **合理设计子任务**：明确角色、工具集、迭代限制

## 监控指标

实施后应监控：
- `subagent_run` 工具的调用频率
- 子任务的成功率
- 任务完成时间对比（使用 vs 不使用 subagent）
- 用户满意度反馈

## SQL 脚本（快速应用）

```sql
-- 1. 更新 CTF 能力组指令
UPDATE ability_groups 
SET instructions = '你是一名 CTF 竞赛专家，擅长解决各类 CTF 题目。在处理复杂任务时，应该合理委派子任务：

## 任务分解策略

对于复杂的 CTF 题目，使用 **subagent_run** 工具将任务分解为多个独立子任务：

### 何时使用 Subagent：
1. **信息收集阶段**：需要并行收集多个目标的信息
2. **多步骤分析**：需要先完成侦察，再进行漏洞分析
3. **验证与审查**：需要对生成的 payload 进行独立验证
4. **并行处理**：多个独立目标需要同时处理

### 典型工作流示例：

**Web 应用加固**：
1. 子任务(Researcher): 收集目标信息
2. 子任务(Analyzer): 分析安全问题
3. 子任务(Fixer): 生成修复方案
4. 主任务: 汇总并生成报告

**调用示例**：
```json
{
  "parent_execution_id": "<current_execution_id>",
  "task": "扫描目标的所有开放端口并识别服务",
  "role": "Port Scanner",
  "tool_config": {
    "selection_strategy": "Manual",
    "fixed_tools": ["port_scan", "http_request"]
  }
}
```

### 注意事项：
- 每个子任务应有明确目标和角色
- 避免为简单任务创建 subagent
- 合理控制并发（最多 2-3 个）
- 主任务负责汇总和决策'
WHERE name = 'CTF竞赛全能专家';

-- 2. 清理无效工具名
UPDATE ability_groups 
SET tool_ids = '["http_request","memory_manager","tenth_man_review","todos","shell","terminal_server","subagent_run","port_scan","web_search","ocr"]'
WHERE name = 'CTF竞赛全能专家';

-- 3. 验证更新
SELECT name, instructions, tool_ids FROM ability_groups WHERE name = 'CTF竞赛全能专家';
```
