# ReWOO Prompt分离修复

## 问题描述

从日志文件 `llm-http-requests-2025-11-17.log` 中发现，ReWOO的Planner和Solver在调用LLM时存在严重问题：

1. **System prompt重复出现在User input中**：整个system prompt的内容被完整地发送到了user input，导致模板内容重复
2. **占位符未正确填充**：`{task}`、`{tools}`等占位符没有被正确替换
3. **结构混乱**：LLM收到的消息结构不清晰，影响生成质量

### 日志证据

**Planner问题（行2-195）**:
- System prompt: 完整的Planning模板
- User input: **相同的完整Planning模板**（重复发送）

**Solver问题（行411-820）**:
- System prompt: 完整的Solver模板
- User input: **相同的完整Solver模板**（重复发送）
- 结果：LLM回复说缺少信息（因为占位符未填充）

## 根本原因

### 原始实现问题

1. **`build_planning_prompt`/`build_solving_prompt`返回单个字符串**：
   - 原本这些函数返回一个包含所有内容的字符串
   - 这个字符串被作为user prompt发送给LLM
   - 然后在`call_llm`中又从数据库读取system prompt
   - 导致模板内容被重复发送

2. **占位符填充时机错误**：
   - 占位符在构建"user prompt"时填充
   - 但system prompt中仍然包含未填充的占位符
   - 或者两者都包含了填充后的内容（重复）

## 解决方案

### 1. 修改函数签名

将`build_xxx_prompt`函数改为返回`(String, String)`元组：
- 第一个String：system prompt
- 第二个String：user prompt

```rust
// Planner
async fn build_planning_prompt(
    &self,
    query: &str,
    available_tools: &[String],
    context: Option<&str>,
) -> Result<(String, String)>

// Solver
async fn build_solving_prompt(
    &self,
    query: &str,
    plan_string: &str,
    tool_results: &HashMap<String, serde_json::Value>,
) -> Result<(String, String)>
```

### 2. 正确分离System和User Prompt

**Planner逻辑**:
```rust
// 1. 从数据库获取模板
let system_template = self.prompt_repo
    .get_template_by_arch_stage(ArchitectureType::ReWOO, StageType::Planner)
    .await?;

// 2. 填充system prompt中的占位符（除了{task}）
let mut system_prompt = system_template
    .replace("{tools}", &tools_desc)
    .replace("{context}", ctx);

// 3. 替换{task}占位符为用户输入
let system_part = system_prompt.replace("{task}", query);

// 4. user prompt就是用户的输入
let user_part = query.to_string();

Ok((system_part, user_part))
```

**Solver逻辑**:
```rust
// 1. 从数据库获取模板
let system_template = self.prompt_repo
    .get_template_by_arch_stage(ArchitectureType::ReWOO, StageType::Solver)
    .await?;

// 2. 填充system prompt中的占位符（除了{task}）
let mut system_prompt = system_template
    .replace("{execution_plan}", plan_string)
    .replace("{execution_results}", &results_str);

// 3. 替换{task}占位符为用户输入
let system_part = system_prompt.replace("{task}", query);

// 4. user prompt就是用户的输入
let user_part = query.to_string();

Ok((system_part, user_part))
```

### 3. 修改call_llm函数签名

```rust
// Planner
async fn call_llm(
    &self, 
    system_prompt: &str,  // 新增参数
    user_prompt: &str,     // 改名并使用传入的值
    execution_id: &str
) -> Result<String>

// Solver
async fn call_llm(
    &self,
    system_prompt: &str,   // 新增参数
    user_prompt: &str,     // 改名并使用传入的值
    execution_id: &str
) -> Result<String>
```

### 4. 移除重复的System Prompt获取

在`call_llm`中，不再从数据库获取system prompt，而是直接使用传入的参数：

```rust
// 旧代码（删除）:
// let system_prompt = match self.prompt_repo.get_active_prompt(...).await {
//     Ok(Some(p)) => p,
//     _ => "fallback prompt".to_string(),
// };

// 新代码：直接使用传入的参数
let content = ai_service
    .send_message_stream(
        Some(user_prompt),      // 使用传入的user prompt
        Some(system_prompt),    // 使用传入的system prompt
        None,
        Some(execution_id.to_string()),
        false,
        false,
        None,
    )
    .await?;
```

### 5. 更新调用点

```rust
// Planner
let (system_prompt, user_prompt) = self.build_planning_prompt(query, available_tools, context).await?;
let plan_string = self.call_llm(&system_prompt, &user_prompt, execution_id).await?;

// Solver  
let (system_prompt, user_prompt) = self.build_solving_prompt(query, plan_string, tool_results).await?;
let answer = self.call_llm(&system_prompt, &user_prompt, execution_id).await?;
```

## 模板占位符说明

### Planner模板占位符

- `{tools}`: 可用工具列表（逗号分隔的工具名称）
- `{context}`: 上下文信息（可选）
- `{task}`: 用户的任务/查询

### Solver模板占位符

- `{task}`: 用户的原始任务/查询
- `{execution_plan}`: 执行计划字符串
- `{execution_results}`: 工具执行结果字符串

## 预期效果

修复后，LLM请求应该是：

**Planner**:
- System prompt: 包含Planning指南、可用工具列表、用户任务等完整信息
- User prompt: 仅包含用户的查询（如："对 http://testphp.vulnweb.com 进行全面的安全渗透测试"）

**Solver**:
- System prompt: 包含Solver指南、原始任务、执行计划、执行结果等完整信息
- User prompt: 仅包含用户的原始查询

这样LLM就能：
1. 从system prompt中理解角色、任务要求和可用信息
2. 从user prompt中获取用户的具体需求
3. 生成符合要求的输出

## 文件修改列表

1. **src-tauri/src/engines/rewoo/rewoo_planner.rs**
   - 修改`build_planning_prompt`返回类型为`(String, String)`
   - 修改`call_llm`签名，接受`system_prompt`和`user_prompt`参数
   - 更新`plan`函数的调用逻辑

2. **src-tauri/src/engines/rewoo/rewoo_solver.rs**
   - 修改`build_solving_prompt`返回类型为`(String, String)`
   - 修改`call_llm`签名，接受`system_prompt`和`user_prompt`参数
   - 更新`solve`函数的调用逻辑

3. **src-tauri/src/engines/rewoo/prompt.md**
   - 确保模板包含`{task}`占位符
   - 占位符应该在需要用户输入的地方

## 测试验证

修复后应该验证：

1. **检查LLM请求日志**：
   - System prompt应该包含完整的指南和已填充的占位符
   - User prompt应该仅包含用户的输入
   - 两者不应该重复

2. **检查LLM响应**：
   - Planner应该生成有效的JSON计划
   - Solver应该生成基于实际结果的报告

3. **功能测试**：
   - 执行完整的ReWOO流程
   - 验证计划生成正确
   - 验证最终答案质量

## 注意事项

1. **模板来源**：
   - 优先从数据库的`PromptManagement`获取
   - 如果数据库中没有，回退到`prompt.md`中的默认模板

2. **占位符一致性**：
   - 确保代码中替换的占位符与模板中使用的占位符一致
   - 大小写敏感：`{task}` ≠ `{Task}`

3. **User Prompt内容**：
   - 应该简洁，仅包含用户的输入
   - 不应该包含任何模板内容或指南
   - 让LLM从system prompt中获取所有指导信息

4. **向后兼容性**：
   - 如果模板中不包含某个占位符，replace操作不会报错
   - 但应该确保模板正确包含所有必需的占位符

