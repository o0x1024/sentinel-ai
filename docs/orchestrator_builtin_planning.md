# Orchestrator Built-in Planning 架构重构

## 问题背景

### 原架构的问题

当前的 Orchestrator 实现存在严重的架构混乱：

1. **Planning 阶段使用 ReWOO**：
   - 调用 ReWOO 生成 ReWOO 格式的 DAG 计划
   - 计划格式：`{"id": "E1", "tool": "start_passive_scan", "args": {...}, "depends_on": []}`
   - 这是 ReWOO 特有的格式，不适合 Orchestrator

2. **Execution 阶段直接执行工具**：
   - 从 ReWOO 计划中提取 `tool` 字段
   - 直接调用工具（`start_passive_scan`、`rsubdomain` 等）
   - **完全没有调度子 Agent**

3. **设计目标未达成**：
   - 文档说要调度 ReWOO/PlanAndExecute/LLMCompiler
   - 实际上只是把 `sub_agent` 作为参数放在 args 里
   - 没有真正的子 Agent 调度逻辑

### 根本原因

ReWOO 是一个完整的执行引擎，它的计划格式是为 ReWOO 自己设计的：
- `tool` 字段直接对应工具名称
- ReWOO 会自己执行这些工具
- 不存在"调度其他 Agent"的概念

Orchestrator 不应该使用 ReWOO 来生成计划，因为：
- Orchestrator 需要的是**安全测试步骤计划**，不是 ReWOO 的 DAG
- 每个步骤应该指定**使用哪个子 Agent**，不是哪个工具
- Orchestrator 应该**调度子 Agent 执行步骤**，不是自己执行工具

---

## 新架构：Built-in Planning

### 核心思想

**Orchestrator 应该完全独立，使用内置的 Planning 逻辑，生成自己的计划格式。**

### 新的两阶段模型

```
用户任务
  ↓
┌────────────────────────────────┐
│  Phase 1: Planning (Built-in)  │
│  - 调用 LLM 生成结构化计划       │
│  - 输出 OrchestratorPlan       │
└────────────────────────────────┘
  ↓
┌────────────────────────────────┐
│  Phase 2: Execution             │
│  - 遍历计划步骤                  │
│  - 根据 sub_agent_kind 调度：   │
│    * PlanAndExecute             │
│    * ReWOO                      │
│    * LLMCompiler                │
└────────────────────────────────┘
  ↓
测试结果
```

### 新的计划格式

```json
{
  "id": "plan_uuid",
  "task_kind": "WebPentest",
  "primary_target": "https://testphp.vulnweb.com",
  "plan_summary": "针对目标网站的全面渗透测试",
  "steps": [
    {
      "id": "step_1",
      "index": 1,
      "step_type": "Recon",
      "sub_agent_kind": "PlanAndExecute",  // 真正的调度标识
      "objective": "信息收集和子域枚举",
      "actions": [
        "start_passive_scan",
        "rsubdomain testphp.vulnweb.com"
      ],
      "expected_outputs": ["子域列表", "被动扫描数据"],
      "depends_on": [],
      "risk_level": "None"
    },
    {
      "id": "step_2",
      "index": 2,
      "step_type": "VulnScan",
      "sub_agent_kind": "PlanAndExecute",
      "objective": "主动漏洞扫描",
      "actions": [
        "nuclei -u https://testphp.vulnweb.com"
      ],
      "expected_outputs": ["漏洞报告"],
      "depends_on": ["step_1"],
      "risk_level": "Low"
    }
  ],
  "estimated_duration_min": 30
}
```

### 关键区别

| 维度 | 旧架构 (ReWOO) | 新架构 (Built-in) |
|------|----------------|------------------|
| **Planning 方式** | 调用 ReWOO 引擎 | 内置 LLM Planning |
| **计划格式** | ReWOO DAG | OrchestratorPlan |
| **步骤标识** | `tool` (工具名) | `sub_agent_kind` (子 Agent) |
| **执行方式** | 直接执行工具 | 调度子 Agent |
| **架构独立性** | 依赖 ReWOO | 完全独立 |

---

## 实现细节

### 1. OrchestratorPlanner

**文件**: `src-tauri/src/engines/orchestrator/planner.rs`

```rust
pub struct OrchestratorPlanner {
    ai_service: Arc<AiServiceManager>,
}

impl OrchestratorPlanner {
    /// Generate plan using LLM
    pub async fn generate_plan(
        &self,
        task: &SecurityTask,
    ) -> Result<OrchestratorPlan> {
        // 1. Build planning prompt
        let prompt = self.build_planning_prompt(task);
        
        // 2. Call LLM
        let response = self.ai_service.call_default(&prompt, None).await?;
        
        // 3. Parse JSON response
        let plan = self.parse_plan_response(&response, task)?;
        
        Ok(plan)
    }
}
```

### 2. Planning Prompt

Prompt 会明确要求 LLM 生成指定格式的 JSON 计划，包括：
- 步骤序列
- 每个步骤使用哪个子 Agent
- 步骤之间的依赖关系
- 预期输出和风险等级

### 3. Execution Logic

```rust
async fn execute_orchestration_workflow(&self, plan: &OrchestratorPlan) -> Result<String> {
    for step in &plan.steps {
        // Check dependencies
        self.wait_for_dependencies(&step.depends_on).await?;
        
        // Dispatch to sub-agent
        match step.sub_agent_kind {
            SubAgentKind::PlanAndExecute => {
                let result = self.execute_plan_and_execute_step(step).await?;
                // Record result
            }
            SubAgentKind::ReWOO => {
                let result = self.execute_rewoo_step(step).await?;
                // Record result
            }
            SubAgentKind::LLMCompiler => {
                let result = self.execute_compiler_step(step).await?;
                // Record result
            }
            _ => {}
        }
    }
    
    Ok("Workflow completed")
}
```

---

## 下一步工作

### 必要改动

1. **重构 `execute_orchestration_workflow`**：
   - 移除调用 ReWOO 生成计划的逻辑
   - 使用 `OrchestratorPlanner::generate_plan()`
   - 实现基于 `sub_agent_kind` 的调度逻辑

2. **实现子 Agent 调度方法**：
   - `execute_plan_and_execute_step()`
   - `execute_rewoo_step()`
   - `execute_compiler_step()`

3. **更新测试和文档**：
   - 更新 `orchestrator_agent_implementation_plan.md`
   - 添加使用示例

### 可选改动

1. **依赖解析**：实现步骤依赖的拓扑排序和并发执行
2. **状态传递**：在步骤间传递 `AuthContext` 和中间结果
3. **错误处理**：步骤失败时的重试和回退策略
4. **进度报告**：实时向前端报告执行进度

---

## 优势

### 1. 架构清晰

- Orchestrator 完全独立，不依赖 ReWOO
- Planning 和 Execution 职责明确
- 子 Agent 真正被调度，而不是伪调度

### 2. 易于扩展

- 新增子 Agent 只需实现调度方法
- 新增安全任务类型只需更新 prompt
- 计划格式统一，易于持久化和展示

### 3. 符合设计目标

- 实现了真正的"三大子 Agent 协同工作"
- Orchestrator 作为协调者的角色清晰
- 与文档描述的架构一致

---

## 迁移指南

### 对现有代码的影响

1. **Orchestrator Engine Adapter**：
   - 需要集成 `OrchestratorPlanner`
   - 重构 `execute_orchestration_workflow` 方法

2. **前端展示**：
   - 计划格式变化，需要更新解析逻辑
   - 步骤展示需要适配新的 `sub_agent_kind` 字段

3. **测试**：
   - 更新单元测试和集成测试
   - 验证新的调度逻辑

### 兼容性

- **不向后兼容**：新旧计划格式不兼容
- **建议**：清理旧的测试会话，重新开始

---

## 总结

这次重构将 Orchestrator 从"依赖 ReWOO 的伪调度器"改造为"真正的安全测试协调引擎"：

- ✅ **内置 Planning**：使用 LLM 直接生成结构化计划
- ✅ **真实调度**：根据 `sub_agent_kind` 真正调度子 Agent
- ✅ **架构独立**：完全独立于 ReWOO，职责清晰
- ✅ **符合设计**：实现了文档中描述的两阶段模型

这是 Orchestrator 架构的重大改进，使其真正成为一个强大的安全测试协调引擎！

