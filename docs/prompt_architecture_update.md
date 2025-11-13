# Prompt 架构阶段修正记录

**更新时间**: 2025-11-13  
**更新内容**: 修正前端提示词管理界面的架构阶段定义，使其与后端实现保持一致

---

## 问题描述

前端 `PromptManagement.vue` 中定义的各架构阶段与后端实现不一致，特别是 **LLMCompiler** 架构缺少 **Evaluation** 阶段。

---

## 修正内容

### 1. LLMCompiler 架构阶段（主要修改）

**修改前**:
```typescript
{ value: 'LLMCompiler', label: 'LLMCompiler', stages: [
  { value: 'Planning', label: 'Planning' },
  { value: 'Execution', label: 'Execution' },
  { value: 'Replan', label: 'Replan' },  // ❌ 缺少 Evaluation
]}
```

**修改后**:
```typescript
{ value: 'LLMCompiler', label: 'LLMCompiler', stages: [
  { value: 'Planning', label: 'Planning (规划)' },
  { value: 'Execution', label: 'Execution (执行)' },
  { value: 'Evaluation', label: 'Evaluation (评估)' },  // ✅ 新增
  { value: 'Replan', label: 'Replan (重规划)' },
]}
```

### 2. 其他架构（优化显示）

为所有架构的阶段添加了中文说明，提升用户体验：

#### ReWOO 架构
```typescript
{ value: 'ReWOO', label: 'ReWOO', stages: [
  { value: 'Planner', label: 'Planner (规划器)' },
  { value: 'Worker', label: 'Worker (执行器)' },
  { value: 'Solver', label: 'Solver (求解器)' },
]}
```

#### Plan&Execute 架构
```typescript
{ value: 'PlanExecute', label: 'Plan&Execute', stages: [
  { value: 'Planning', label: 'Planning (规划)' },
  { value: 'Execution', label: 'Execution (执行)' },
  { value: 'Replan', label: 'Replan (重规划)' },
]}
```

#### ReAct 架构
```typescript
{ value: 'ReAct', label: 'ReAct', stages: [
  { value: 'Planning', label: 'Planning (规划)' },
  { value: 'Execution', label: 'Execution (执行)' },
]}
```

---

## 后端相应修改

### 1. 枚举类型更新

**文件**: `src-tauri/sentinel-core/src/models/prompt.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StageType {
    // ReWOO stages
    Planner,
    Worker,
    Solver,
    // LLMCompiler & Plan&Execute stages
    Planning,
    Execution,
    Evaluation,  // ✅ 新增：LLMCompiler Joiner/Evaluator stage
    Replan,
}
```

### 2. 映射关系更新

**文件**: `src-tauri/src/utils/prompt_resolver.rs`

```rust
impl CanonicalStage {
    pub fn to_architecture_stage(&self, arch: &ArchitectureType) -> Option<StageType> {
        match (self, arch) {
            // ...
            (CanonicalStage::Evaluator, ArchitectureType::LLMCompiler) => 
                Some(StageType::Evaluation),  // ✅ 新增映射
            // ...
        }
    }
}
```

### 3. 字符串转换函数更新

**文件**: 
- `src-tauri/src/services/prompt_db.rs`
- `src-tauri/sentinel-db/src/database/prompt_dao.rs`

```rust
fn stage_str(s: &StageType) -> &'static str {
    match s {
        // ...
        StageType::Evaluation => "evaluation",  // ✅ 新增
        // ...
    }
}

fn parse_stage(s: &str) -> StageType {
    match s.to_lowercase().as_str() {
        // ...
        "evaluation" => StageType::Evaluation,  // ✅ 新增
        // ...
    }
}
```

### 4. TypeScript 类型定义更新

**文件**: `src/views/PromptManagement.vue`

```typescript
type StageType = 'Planner' | 'Worker' | 'Solver' | 
                 'Planning' | 'Execution' | 'Evaluation' | 'Replan'
                 // ✅ 新增 'Evaluation'
```

---

## LLM Compiler 架构完整阶段说明

### 1. Planning (规划阶段)
- **职责**: 将用户任务拆解为可并行的 DAG 执行计划
- **对应组件**: `LlmCompilerPlanner`
- **使用模型**: 规划器模型（从 ModelSettings 配置）
- **提示词**: Planning stage prompt

### 2. Execution (执行阶段)
- **职责**: 并行执行 DAG 中的任务
- **对应组件**: `ParallelExecutorPool`
- **使用模型**: 执行器模型（工具调用）
- **提示词**: Execution stage prompt（用于生成最终响应）

### 3. Evaluation (评估阶段) ✨
- **职责**: 评估执行结果，决定继续或完成
- **对应组件**: `IntelligentJoiner`
- **使用模型**: 评估器模型（从 ModelSettings 配置）
- **提示词**: Evaluation stage prompt
- **决策输出**: CONTINUE 或 COMPLETE

### 4. Replan (重规划阶段)
- **职责**: 根据评估反馈重新生成执行计划
- **对应组件**: `LlmCompilerPlanner` (replan 模式)
- **使用模型**: 规划器模型
- **提示词**: Replanning stage prompt

---

## 与提示词 SQL 文件的对应关系

生成的 `llm_compiler_prompts.sql` 中包含以下阶段的提示词：

| SQL 中的 stage | 对应的 StageType | 说明 |
|---------------|-----------------|------|
| `planner` | `Planning` | 规划阶段 |
| `executor` | `Execution` | 执行阶段（最终响应生成） |
| `evaluator` | `Evaluation` | ✅ 评估阶段 |
| `replanner` | `Replan` | 重规划阶段 |

**注意**: SQL 文件中使用小写字符串（如 `"planner"`），而 Rust 枚举使用驼峰命名（如 `Planning`），通过 `stage_str()` 和 `parse_stage()` 函数进行转换。

---

## 验证方法

### 1. 前端验证
1. 打开应用，进入 **设置 > 提示词管理**
2. 选择 **LLMCompiler** 架构
3. 应该看到 4 个阶段：
   - Planning (规划)
   - Execution (执行)
   - **Evaluation (评估)** ← 新增
   - Replan (重规划)

### 2. 后端验证
```bash
# 编译检查
cd src-tauri
cargo check

# 应该通过编译，没有关于 StageType::Evaluation 的错误
```

### 3. 数据库导入验证
```bash
# 导入提示词
cd src-tauri
sqlite3 sentinel-ai.db < ../docs/llm_compiler_prompts.sql

# 查询验证
sqlite3 sentinel-ai.db "SELECT stage FROM prompt_templates WHERE architecture='LLMCompiler';"

# 应该看到: planning, executor, evaluator, replanner
```

---

## 影响范围

### 直接影响
1. ✅ 前端提示词管理界面现在可以管理 Evaluation 阶段的提示词
2. ✅ LLM Compiler 引擎可以使用专门的评估器提示词
3. ✅ 模型设置界面中的评估器模型配置现在可以正确应用

### 间接影响
1. ✅ 提示词组（Prompt Group）可以配置 Evaluation 阶段
2. ✅ Agent 配置可以为 Evaluation 阶段指定特定提示词
3. ✅ 提示词解析器（PromptResolver）可以正确解析 Evaluation 阶段

---

## 兼容性说明

### 向后兼容
- ✅ 旧的提示词配置仍然有效
- ✅ 缺少 Evaluation 阶段配置时会使用默认值
- ✅ 现有的 Planning/Execution/Replan 阶段不受影响

### 数据迁移
**不需要**数据迁移，因为：
1. 枚举新增了变体，旧数据不受影响
2. 数据库表结构未改变
3. 字符串解析函数保持向后兼容

---

## 后续建议

1. **创建默认提示词**: 为 LLMCompiler 的 Evaluation 阶段创建默认提示词模板
2. **更新用户文档**: 说明 Evaluation 阶段的作用和配置方法
3. **添加示例**: 在文档中提供 Evaluation 阶段提示词的优化示例
4. **UI 优化**: 考虑在界面上为关键阶段（如 Evaluation）添加特殊标记

---

## 相关文件

### 已修改文件
1. `src/views/PromptManagement.vue` - 前端界面定义
2. `src-tauri/sentinel-core/src/models/prompt.rs` - 枚举类型
3. `src-tauri/src/utils/prompt_resolver.rs` - 映射关系
4. `src-tauri/src/services/prompt_db.rs` - 字符串转换
5. `src-tauri/sentinel-db/src/database/prompt_dao.rs` - 数据库访问

### 新增文件
1. `docs/llm_compiler_prompts.sql` - LLM Compiler 提示词 SQL
2. `docs/llm_compiler_prompts_guide.md` - 详细使用指南
3. `docs/llm_compiler_prompts_README.md` - 快速开始文档

---

## 测试清单

- [x] 前端编译通过
- [x] 后端编译通过
- [x] StageType 枚举完整性检查
- [x] 字符串转换函数完整性检查
- [x] 映射关系正确性检查
- [ ] 界面功能测试（需要启动应用）
- [ ] 提示词导入测试
- [ ] 端到端流程测试

---

**维护者**: Sentinel AI Team  
**最后更新**: 2025-11-13  
**状态**: ✅ 编译通过，等待功能测试

