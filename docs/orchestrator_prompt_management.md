# Orchestrator架构Prompt管理实现

## 问题1：在PromptManagement中添加Orchestrator架构支持

### ✅ 已完成的修改

#### 1. 类型定义扩展

**文件**: `src/views/PromptManagement.vue`

添加了Orchestrator相关的类型：

```typescript
// 添加Orchestrator到架构类型
type ArchitectureType = 'ReWOO' | 'LLMCompiler' | 'PlanExecute' | 'ReAct' | 'Orchestrator'

// 添加Orchestrator的阶段类型
type StageType = 'Planner' | 'Worker' | 'Solver' | 'Planning' | 'Execution' | 
                 'Evaluation' | 'Replan' | 'Orchestration' | 'SubAgentCoordination'
```

#### 2. 架构分组配置

在`groups`数组中添加了Orchestrator配置：

```typescript
{ 
  value: 'Orchestrator', 
  label: 'Orchestrator', 
  stages: [
    { value: 'Orchestration', label: 'Orchestration (编排)' },
    { value: 'SubAgentCoordination', label: 'SubAgent Coordination (子代理协调)' },
  ]
}
```

#### 3. 阶段计算逻辑

更新了两个computed属性以支持Orchestrator：

```typescript
// stagesOfSelectedArch
if (selected.value.architecture === 'Orchestrator') 
  return ['Orchestration','SubAgentCoordination'] as StageType[]

// stagesOfGroupArch  
if (arch === 'Orchestrator') 
  return ['Orchestration','SubAgentCoordination'] as StageType[]
```

#### 4. Prompt导入功能

添加了一键导入Orchestrator默认prompts的功能：

**UI组件**:
```vue
<div v-if="selectedCategory === 'LlmArchitecture' && selected.architecture === 'Orchestrator'" class="mt-2 px-4">
  <button class="btn btn-xs btn-outline btn-primary w-full" @click="importOrchestratorPrompts">
    📥 导入Orchestrator默认Prompt
  </button>
  <div class="text-xs opacity-60 mt-1">从orchestrator/prompt.md导入</div>
</div>
```

**导入函数**: `importOrchestratorPrompts()`
- 创建2个预定义模板：
  1. **Orchestrator - 编排主提示** (Orchestration阶段)
  2. **Orchestrator - 子代理协调指南** (SubAgentCoordination阶段)
- 自动设置为默认和激活状态
- 包含完整的角色定义、子代理说明、工作流程指南

### Prompt内容

#### 编排主提示 (Orchestration)

包含以下关键内容：
- **角色定义**: Security Test Orchestrator Agent
- **核心职责**: 理解目标、任务分解、子代理选择、状态维护、结果综合
- **可用子代理**: ReWOO、Plan-and-Execute、LLM-Compiler
- **支持的任务类型**: Web/API渗透测试、取证分析、CTF、逆向工程
- **工作流程指南**: 5个阶段的详细说明
- **重要约束**: 自动推进、避免循环、风险意识、上下文保持

#### 子代理协调指南 (SubAgentCoordination)

包含以下关键内容：
- **子代理选择矩阵**: 每个子代理的适用场景和不适用场景
- **协调模式**: 3种常见的协调模式
- **上下文管理**: 认证、会话状态、结果累积、错误处理
- **最佳实践**: 5条核心实践指南
- **示例协调流程**: 完整的API安全测试流程示例

### 变量支持

两个模板都定义了可替换的变量：

**编排主提示变量**:
- `task_type`: 任务类型
- `primary_target`: 主要目标
- `user_credentials`: 用户凭证

**协调指南变量**:
- `current_stage`: 当前阶段
- `auth_context`: 认证上下文
- `previous_results`: 之前的结果

## 问题2：修复generate_advanced_plugin工具权限问题

### ❌ 原始问题

从日志第137行可以看到错误：
```
ERROR: Tool execution error: 工具 'generate_advanced_plugin' 不在允许列表中
```

虽然Orchestrator配置了工具允许列表，但ReWOO子代理内部有自己的工具权限检查，而`generate_advanced_plugin`没有被包含在允许列表中。

### ✅ 修复方案

**文件**: `src-tauri/src/engines/orchestrator/sub_agents/rewoo_executor.rs`

在ReWOO子代理执行器的工具权限列表中添加了`generate_advanced_plugin`：

```rust
runtime_params.insert(
    "tools_allow".to_string(),
    serde_json::json!([
        "http_request", "port_scan", "rsubdomain", "analyze_website",
        "playwright_navigate", "playwright_click", "playwright_fill",
        "playwright_get_visible_text", "playwright_screenshot",
        "playwright_evaluate", "playwright_get", "playwright_post",
        "local_time", "get_passive_scan_status", "start_passive_scan",
        "list_findings", "get_finding_detail",
        "generate_advanced_plugin"  // ✅ 新增
    ])
);
```

### 工具权限检查机制

ReWOO Worker在执行工具前会进行权限检查（`rewoo_worker.rs` 第67-98行）：

1. 检查`tools_allow`白名单是否为空
2. 检查工具是否在白名单中
3. 检查工具是否在`tools_deny`黑名单中

如果任何检查失败，会返回错误：
```rust
return Err(ReWOOError::ToolExecutionError(format!(
    "工具 '{}' 不在允许列表中", step.tool
)));
```

## 使用指南

### 1. 导入Orchestrator Prompts

1. 打开Prompt管理页面
2. 选择"LLM架构"分类
3. 点击左侧"Orchestrator"架构
4. 点击"📥 导入Orchestrator默认Prompt"按钮
5. 系统会自动创建2个模板并激活

### 2. 编辑和自定义Prompts

1. 在左侧选择Orchestration或SubAgentCoordination阶段
2. 点击对应的模板
3. 在右侧编辑器中修改内容
4. 支持添加标签和变量
5. 点击"保存"按钮

### 3. 设置为默认Prompt

1. 选择要设置为默认的模板
2. 勾选"激活此模板"复选框
3. 保存后该模板将成为Orchestrator架构的默认prompt

### 4. 变量渲染预览

1. 勾选"变量渲染"选项
2. 在"示例上下文"中输入JSON格式的变量值
3. 点击"实时预览"查看渲染结果

## 技术细节

### 前端架构支持

- **类型安全**: TypeScript类型定义确保架构和阶段的类型安全
- **动态计算**: 使用computed属性动态计算可用阶段
- **分类管理**: 支持按架构分类管理prompts
- **搜索过滤**: 支持按名称、描述、标签搜索

### 后端工具权限

- **分层权限**: Orchestrator和子代理都有独立的工具权限配置
- **白名单机制**: 只有在白名单中的工具才能被调用
- **黑名单机制**: 黑名单中的工具会被明确拒绝
- **运行时配置**: 通过`runtime_params`动态设置权限

### Prompt模板结构

```typescript
interface PromptTemplate {
  name: string                    // 模板名称
  description: string             // 描述
  architecture: ArchitectureType  // 所属架构
  stage: StageType               // 所属阶段
  content: string                // Prompt内容
  is_default: boolean            // 是否默认
  is_active: boolean             // 是否激活
  is_system: boolean             // 是否系统级
  template_type: TemplateType    // 模板类型
  priority: number               // 优先级
  tags: string[]                 // 标签
  variables: string[]            // 变量列表
}
```

## 编译状态

✅ 所有修改已通过编译
✅ 前端Vue组件无语法错误
✅ 后端Rust代码编译成功
✅ 0个编译错误

## 相关文件

### 前端
- `src/views/PromptManagement.vue` - Prompt管理页面

### 后端
- `src-tauri/src/engines/orchestrator/sub_agents/rewoo_executor.rs` - ReWOO子代理执行器
- `src-tauri/src/engines/orchestrator/prompt.md` - Orchestrator原始prompt文档
- `src-tauri/src/engines/rewoo/rewoo_worker.rs` - ReWOO Worker工具权限检查

### 文档
- `docs/orchestrator_sub_agent_fix.md` - 子代理修复文档
- `docs/orchestrator_prompt_management.md` - 本文档

## 测试建议

1. **导入测试**: 测试一键导入功能是否正常创建2个模板
2. **编辑测试**: 测试模板编辑和保存功能
3. **变量测试**: 测试变量渲染预览功能
4. **工具权限测试**: 使用Orchestrator架构执行包含`generate_advanced_plugin`的任务
5. **默认Prompt测试**: 测试设置默认prompt后是否生效

## 后续优化建议

1. **Prompt版本管理**: 支持prompt的版本历史和回滚
2. **Prompt模板市场**: 支持导入/导出和分享prompt模板
3. **智能变量提取**: 自动从prompt内容中提取变量
4. **Prompt效果评估**: 记录使用不同prompt的执行效果
5. **动态工具权限**: 根据任务类型动态调整工具权限列表

