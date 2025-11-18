# Orchestrator 架构完整设置指南

## ✅ 已完成的修复

### 1. Agent 保存错误修复

**问题**: 保存 Orchestrator Agent 时报错 "unknown variant `orchestrator`"

**修复内容**:
- ✅ 在 `AgentEngine` 枚举中添加 `Orchestrator` 变体
- ✅ 在 `ArchitectureType` 枚举中添加 `Orchestrator` 类型
- ✅ 在所有相关的匹配语句中添加 Orchestrator 分支
- ✅ 编译测试通过

**修改文件**:
- `src-tauri/src/commands/ai_commands.rs` - 添加 AgentEngine::Orchestrator
- `src-tauri/sentinel-core/src/models/prompt.rs` - 添加 ArchitectureType::Orchestrator
- `src-tauri/src/services/prompt_db.rs` - 添加 orchestrator 字符串映射
- `src-tauri/sentinel-db/src/database/prompt_dao.rs` - 添加 orchestrator 解析

### 2. Prompt 动态配置支持

**问题**: Orchestrator 架构需要支持动态 Prompt 配置

**实现方案**:

#### 方案 A: 使用现有 Prompt 管理系统（推荐）

Orchestrator 已经集成到 Prompt 管理系统中，可以通过以下方式配置：

1. **在 Prompt 管理页面创建 Orchestrator Prompt**:
   - 打开 "Prompt 管理" 页面
   - 点击 "新增模板"
   - 架构类型: 选择 "Orchestrator"
   - 阶段: 选择 "Planning" 或 "Execution"
   - 内容: 粘贴自定义 Prompt

2. **默认 Prompt 位置**:
   - `src-tauri/src/engines/orchestrator/prompt.md` - 系统默认 Prompt
   - 可以在 Prompt 管理中覆盖

3. **Prompt 变量支持**:
   - `{{task_kind}}` - 任务类型
   - `{{primary_target}}` - 目标
   - `{{current_stage}}` - 当前阶段
   - `{{previous_steps}}` - 之前的步骤
   - `{{findings}}` - 已发现的问题
   - `{{auth_context}}` - 认证上下文

#### 方案 B: 在 Agent 配置中直接设置

在创建 Orchestrator Agent 时，可以通过 Prompt 配置来自定义：

```json
{
  "name": "安全测试编排器",
  "engine": "orchestrator",
  "prompts": {
    "system": "你是一个安全测试编排系统...",
    "planner": "创建测试计划时...",
    "executor": "执行测试时..."
  }
}
```

## 📋 完整使用流程

### 步骤 1: 创建 Orchestrator Agent

1. 打开 "Agent 管理" 页面
2. 点击 "新增Agent"
3. 填写信息:
   - **名称**: `安全测试编排器`
   - **引擎**: 选择 `orchestrator (安全测试编排)`
   - **描述**: `智能安全测试编排系统，协调 ReWOO、Plan-and-Execute、LLM-Compiler`
   - **启用状态**: ✅ 勾选
4. 点击保存

### 步骤 2: (可选) 配置自定义 Prompt

**方式 1: 通过 Prompt 管理**

1. 打开 "Prompt 管理" 页面
2. 点击 "新增模板"
3. 填写:
   - 架构: `Orchestrator`
   - 阶段: `Planning`
   - 名称: `Orchestrator 规划 Prompt`
   - 内容: 自定义 Prompt 内容
4. 保存并设置为默认

**方式 2: 在 Agent 中配置**

在 Agent 编辑页面的 Prompt 配置区域设置自定义 Prompt

### 步骤 3: 使用 Orchestrator

1. 打开 "AI 智能助手" 页面
2. 在顶部下拉菜单选择 "安全测试编排器"
3. 输入测试需求:

```
对 https://api.example.com 进行安全测试

重点检查:
1. 认证和授权机制
2. API 接口安全
3. 常见漏洞（注入、越权等）
```

4. 查看执行过程和结果

## 🎯 Orchestrator Prompt 最佳实践

### 1. 系统 Prompt 结构

```markdown
# Security Test Orchestrator

你是一个安全测试编排系统，负责协调多个子 Agent 完成安全测试任务。

## 你的能力

- 调用 ReWOO Agent 进行全局规划
- 调用 Plan-and-Execute Agent 执行具体测试
- 调用 LLM-Compiler Agent 生成测试脚本

## 任务类型

- Web 渗透测试
- API 安全测试
- 取证分析
- CTF 解题
- 逆向工程

## 工作流程

1. 分析用户需求，识别任务类型
2. 调用 ReWOO 创建测试计划
3. 调用 Plan-and-Execute 执行测试步骤
4. 记录发现的安全问题
5. 生成测试报告
```

### 2. 规划阶段 Prompt

```markdown
当前任务: {{task_kind}}
目标: {{primary_target}}

请创建一个全面的安全测试计划，包括:
- 信息收集步骤
- 认证测试步骤
- 漏洞扫描步骤
- 利用验证步骤

已完成的步骤:
{{previous_steps}}

已发现的问题:
{{findings}}
```

### 3. 执行阶段 Prompt

```markdown
执行以下安全测试步骤:
{{current_step}}

认证上下文:
{{auth_context}}

请详细记录:
- 执行的操作
- 观察到的结果
- 发现的安全问题
- 风险等级评估
```

## 🔧 高级配置

### 1. 自定义子 Agent 选择策略

可以在 Orchestrator 的配置中指定何时使用哪个子 Agent:

```json
{
  "sub_agent_strategy": {
    "planning": "rewoo",
    "execution": "plan-execute",
    "code_generation": "llm-compiler"
  }
}
```

### 2. 测试策略配置

```json
{
  "testing_strategy": {
    "parallel_execution": false,
    "max_depth": 5,
    "timeout_seconds": 300,
    "risk_threshold": "medium"
  }
}
```

### 3. 认证配置

```json
{
  "auth_config": {
    "auto_maintain_session": true,
    "cookie_persistence": true,
    "token_refresh": true
  }
}
```

## 📊 监控和调试

### 查看执行日志

```bash
tail -f src-tauri/logs/sentinel-ai.log.$(date +%Y-%m-%d)
```

### 查看 Orchestrator 特定日志

```bash
grep "Orchestrator" src-tauri/logs/sentinel-ai.log.* | tail -50
```

### 查看子 Agent 调用

```bash
grep "sub-agent" src-tauri/logs/sentinel-ai.log.* | tail -50
```

## 🐛 常见问题

### Q: Agent 保存失败

**A**: 确保选择的是 `orchestrator (安全测试编排)` 选项，不是其他引擎类型。

### Q: 执行时没有调用子 Agent

**A**: 检查:
1. 子 Agent 是否已注册
2. Prompt 是否正确配置
3. 查看日志中的错误信息

### Q: 如何自定义 Orchestrator 行为

**A**: 有三种方式:
1. 修改系统 Prompt (`src-tauri/src/engines/orchestrator/prompt.md`)
2. 在 Prompt 管理中创建自定义 Prompt
3. 在 Agent 配置中设置 Prompt

### Q: 支持哪些任务类型

**A**: 目前支持:
- Web 渗透测试 (关键词: web, 网站)
- API 渗透测试 (关键词: api, 接口)
- 取证分析 (关键词: 取证, forensic)
- CTF 解题 (关键词: ctf)
- 逆向工程 (关键词: 逆向, reverse)

## 📚 相关文档

- [快速开始指南](./orchestrator_quick_start.md)
- [完整使用指南](./orchestrator_usage_guide.md)
- [实现计划](./orchestrator_agent_implementation_plan.md)

## ✅ 验证清单

- [x] Agent 可以成功保存
- [x] 在 AI 助手中可以选择 Orchestrator Agent
- [x] 可以正常执行安全测试任务
- [x] 支持 Prompt 动态配置
- [x] 编译测试通过
- [x] 前端界面正常显示

---

**最后更新**: 2025-11-18  
**状态**: ✅ 完全可用

