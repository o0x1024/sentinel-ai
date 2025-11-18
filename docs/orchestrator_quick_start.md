# Orchestrator 快速开始指南

## 如何使用 Orchestrator 架构

### 方法 1：在 Agent 管理器中创建 Orchestrator Agent

1. **打开 Agent 管理页面**
   - 在左侧菜单点击 "Agent管理"

2. **创建新 Agent**
   - 点击 "新增Agent" 按钮
   - 填写以下信息：
     - **名称**: `安全测试编排器` 或任意名称
     - **引擎**: 选择 `orchestrator (安全测试编排)`
     - **描述**: `基于 ReAct 的智能安全测试编排系统`
     - **启用状态**: 勾选启用
   - 点击保存

3. **在 AI 助手中使用**
   - 打开 "AI智能助手" 页面
   - 在顶部下拉菜单中选择刚创建的 Orchestrator Agent
   - 输入安全测试需求，例如：
     ```
     对 https://api.example.com 进行安全测试
     ```

### 方法 2：直接在 AI 助手中使用（如果已有 Orchestrator Agent）

1. 打开 AI 智能助手
2. 选择 Orchestrator Agent
3. 输入测试目标和需求

## 支持的任务类型

Orchestrator 会自动识别以下任务类型：

- **Web 渗透测试**: 包含 "web"、"网站" 关键词
- **API 渗透测试**: 包含 "api"、"接口" 关键词  
- **取证分析**: 包含 "取证"、"forensic" 关键词
- **CTF 解题**: 包含 "ctf" 关键词
- **逆向工程**: 包含 "逆向"、"reverse" 关键词

## 示例输入

### Web/API 安全测试
```
对 https://api.example.com 进行安全测试，重点检查：
1. 认证和授权机制
2. API 接口安全
3. 常见漏洞（注入、越权等）
```

### 取证分析
```
分析这份服务器日志，找出可疑的入侵行为：
/var/log/apache2/access.log
```

### CTF 解题
```
帮我解这道 CTF Web 题：
URL: http://ctf.example.com:8080/challenge
提示：SQL 注入
```

## 工作流程

1. **任务识别**: Orchestrator 分析你的输入，确定任务类型
2. **创建会话**: 创建安全测试会话，记录所有状态
3. **规划阶段**: 调用 ReWOO 生成全局测试计划
4. **执行阶段**: 调用 Plan-and-Execute 执行具体测试步骤
5. **代码生成**: 必要时调用 LLM-Compiler 生成测试脚本
6. **结果汇总**: 整理发现和生成报告

## 查看结果

在 AI 助手界面中，你会看到：

- **会话概要卡片**: 显示任务类型、目标、当前阶段
- **步骤卡片**: 每个步骤的执行情况
- **风险标识**: 高危/中危/低危的可视化标记
- **详细输出**: 可展开查看每步的详细信息

## 注意事项

1. **当前状态**: ✅ Orchestrator 架构已完成并可使用
   - 后端引擎完整实现
   - 前端展示组件已集成
   - 编译测试通过
2. **测试环境**: 请在授权的测试环境中使用
3. **日志查看**: 可在 `src-tauri/logs/` 目录查看详细日志

## 实现状态

- ✅ 领域模型定义完成
- ✅ 会话管理器实现完成
- ✅ ExecutionEngine trait 实现完成
- ✅ 三大子 Agent 执行器框架完成
- ✅ 前端展示组件完成
- ✅ Agent 管理器集成完成
- ✅ 编译测试通过

## 下一步

- 查看 [完整使用指南](./orchestrator_usage_guide.md)
- 查看 [实现计划](./orchestrator_agent_implementation_plan.md)

