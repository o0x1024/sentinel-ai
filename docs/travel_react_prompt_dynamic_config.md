# Travel 内置 ReAct Prompt 动态配置

## 问题背景

Travel 架构内置了一个 ReAct 执行器用于处理复杂任务，但其 prompt 是硬编码在代码中的：

```rust
// react_executor.rs
fn default_system_prompt(&self) -> String {
    r#"You are a helpful AI assistant using the ReAct framework...
    ...
    "#.to_string()
}
```

**问题**:
- ❌ Prompt 硬编码，无法通过前端修改
- ❌ 不支持多语言（只有英文）
- ❌ 无法根据不同任务类型定制 prompt
- ❌ 与其他架构的 prompt 管理方式不一致

## 解决方案

将 Travel 内置的 ReAct prompt 改为从数据库动态读取，并在 `PromptManagement.vue` 中支持配置。

## 实现细节

### 1. 修改 ReAct 执行器读取逻辑

#### 修改前 ❌

```rust
// react_executor.rs
async fn build_thought_prompt(...) -> Result<(String, String)> {
    // 硬编码使用 ReAct 架构的 prompt
    let system_prompt = if let Some(repo) = &self.prompt_repo {
        repo.get_template_by_arch_stage(
            ArchitectureType::ReAct,  // ❌ 使用通用 ReAct
            StageType::Planning,
        )
        .await
        .ok()
        .flatten()
        .map(|p| p.content)
        .unwrap_or_else(|| self.default_system_prompt())
    } else {
        self.default_system_prompt()
    };
    
    // 英文 user prompt
    user_prompt.push_str("Now, what's your next thought and action?");
    
    Ok((system_prompt, user_prompt))
}

fn default_system_prompt(&self) -> String {
    r#"You are a helpful AI assistant using the ReAct framework.
    ..."#.to_string()
}
```

#### 修改后 ✅

```rust
// react_executor.rs
async fn build_thought_prompt(...) -> Result<(String, String)> {
    use crate::models::prompt::{ArchitectureType, StageType};
    
    // 从数据库读取 Travel Act 阶段的 prompt（用于 ReAct 执行）
    let system_prompt = if let Some(repo) = &self.prompt_repo {
        // ✅ 使用 Travel 架构的 Act 阶段 prompt
        if let Ok(Some(template)) = repo
            .get_template_by_arch_stage(ArchitectureType::Travel, StageType::Act)
            .await
        {
            log::info!("Travel ReAct: Using Travel Act prompt from database");
            template.content
        } else {
            log::warn!("Travel ReAct: Travel Act prompt not found, using default");
            self.default_system_prompt()
        }
    } else {
        log::warn!("Travel ReAct: No prompt repository available, using default");
        self.default_system_prompt()
    };

    // 获取可用工具列表
    let tools_description = self.build_tools_information().await;

    // 替换工具占位符
    let system_prompt = system_prompt.replace("{tools}", &tools_description);

    // ✅ 中文 user prompt
    let mut user_prompt = String::new();
    user_prompt.push_str(&format!("任务: {}\n\n", task));

    if !context_history.is_empty() {
        user_prompt.push_str("历史记录:\n");
        for entry in context_history {
            user_prompt.push_str(&format!("{}\n", entry));
        }
        user_prompt.push_str("\n");
    }

    user_prompt.push_str("现在，你的下一步思考和行动是什么？");

    Ok((system_prompt, user_prompt))
}

/// 默认系统提示词（中文版 ReAct）
fn default_system_prompt(&self) -> String {
    r#"你是 Travel 安全测试智能体的执行者，使用 ReAct（推理 + 行动）框架进行安全测试。

## 可用工具

{tools}

## 执行格式

### 需要使用工具时：

```
Thought: [你对下一步的推理和分析]

Action: [工具名称]

Action Input: {"参数名": "参数值"}
```

### 有足够信息回答时：

```
Thought: [你的最终推理]

Final Answer: [你对任务的完整答案]
```

## 关键规则

1. **单步执行**: 一次只执行一个 Action
2. **等待观察**: 执行 Action 后等待 Observation，不要自己输出 "Observation:"
3. **不要提前规划**: 不要一次性输出多个步骤
4. **基于实际结果**: 下一步行动必须基于真实的 Observation

## 安全测试最佳实践

1. **系统化侦察**: 先收集信息，再进行测试
2. **渐进式测试**: 从被动扫描到主动测试
3. **记录发现**: 详细记录所有发现的漏洞和安全问题
4. **合法性**: 确保在授权范围内进行测试

现在开始执行任务！
"#.to_string()
}
```

**关键改进**:
- ✅ 从 `ArchitectureType::Travel` + `StageType::Act` 读取 prompt
- ✅ 默认 prompt 改为中文版本
- ✅ User prompt 也改为中文
- ✅ 添加详细的日志记录

### 2. 更新数据库 Prompt 模板

#### 更新 `src-tauri/prompts/travel/act.md`

```markdown
# Travel OODA - Act (执行) 阶段 - ReAct 模式

你是 Travel 安全测试智能体的执行者，使用 ReAct（推理 + 行动）框架进行安全测试。

## 可用工具

{tools}

## 执行格式

### 需要使用工具时：

```
Thought: [你对下一步的推理和分析]

Action: [工具名称]

Action Input: {"参数名": "参数值"}
```

**重要**: 
- 输出 Action Input 后**立即停止**
- **不要**输出 "Observation:"（系统会自动返回）
- **不要**输出下一个 "Thought:"
- **等待**系统返回 Observation

### 有足够信息回答时：

```
Thought: [你的最终推理]

Final Answer: [你对任务的完整答案]
```

## 关键规则

1. **单步执行**: 一次只执行一个 Action，不要提前规划多个步骤
2. **等待观察**: 执行 Action 后等待 Observation，基于实际结果决策
3. **不要输出 Observation**: 工具执行结果由系统自动返回
4. **基于实际结果**: 下一步行动必须基于真实的 Observation，不要假设

## 安全测试最佳实践

### 1. 系统化侦察
先收集信息，再进行测试：

```
Thought: 需要先了解目标网站的结构和技术栈
Action: analyze_website
Action Input: {"domain": "example.com"}

[等待 Observation]

Thought: 根据网站分析结果，发现使用 PHP + MySQL，接下来测试数据库注入
Action: http_request
Action Input: {"url": "http://example.com/page.php?id=1'", "method": "GET"}
```

### 2. 渐进式测试
从被动扫描到主动测试：

```
第1轮: Thought → Action (被动扫描) → Observation
第2轮: Thought → Action (主动测试) → Observation
第3轮: Thought → Action (漏洞验证) → Observation
最后: Thought → Final Answer
```

### 3. 记录发现
详细记录所有发现的漏洞：

```
Thought: 测试完成，发现了 SQL 注入漏洞
Final Answer: 
在 http://example.com/page.php?id= 参数发现 SQL 注入漏洞
- 类型: 基于错误的 SQL 注入
- 严重性: 高危
- 验证: 使用 ' OR '1'='1 payload 成功绕过认证
- 建议: 使用参数化查询防止 SQL 注入
```

## ReAct 执行示例

### 示例 1: Web 渗透测试

```
Thought: 用户要求对 http://testphp.vulnweb.com 进行安全测试，首先需要分析网站结构
Action: analyze_website
Action Input: {"domain": "testphp.vulnweb.com"}

[系统返回 Observation: 发现网站使用 PHP + MySQL，有登录页面和搜索功能]

Thought: 网站有搜索功能，可能存在 SQL 注入，先测试搜索参数
Action: http_request
Action Input: {"url": "http://testphp.vulnweb.com/search.php?test=1'", "method": "GET"}

[系统返回 Observation: 返回 SQL 错误信息]

Thought: 确认存在 SQL 注入漏洞，现在验证可利用性
Action: http_request
Action Input: {"url": "http://testphp.vulnweb.com/search.php?test=1' OR '1'='1", "method": "GET"}

[系统返回 Observation: 成功绕过，返回所有数据]

Thought: 已确认 SQL 注入漏洞，可以生成报告
Final Answer: 
发现高危 SQL 注入漏洞：
- 位置: http://testphp.vulnweb.com/search.php?test=
- 类型: 基于错误的 SQL 注入
- 严重性: 高危
- 验证: 使用 ' OR '1'='1 payload 成功绕过
- 建议: 使用参数化查询或预编译语句
```

...
```

**特点**:
- ✅ 专门为 Travel 的 ReAct 执行设计
- ✅ 包含 `{tools}` 占位符
- ✅ 中文提示词
- ✅ 详细的安全测试最佳实践
- ✅ 完整的执行示例

### 3. 前端已支持（无需修改）

`PromptManagement.vue` 已经支持 Travel 架构的所有阶段，包括 Act 阶段：

```typescript
// PromptManagement.vue
const groups = ref<PromptGroup[]>([
  // ...
  {
    architecture: 'Travel',
    stages: [
      { name: 'Observe', label: '侦察' },
      { name: 'Orient', label: '分析' },
      { name: 'Decide', label: '决策' },
      { name: 'Act', label: '执行' },  // ✅ 已支持
    ],
  },
  // ...
]);
```

**用户可以通过前端**:
1. 选择 Travel 架构
2. 选择 Act 阶段
3. 编辑 prompt 内容
4. 保存到数据库

## Prompt 读取流程

```
Travel ReAct 执行器
       ↓
build_thought_prompt()
       ↓
尝试从数据库读取
prompt_repo.get_template_by_arch_stage(
    ArchitectureType::Travel,
    StageType::Act
)
       ↓
找到？
├─ 是 → 使用数据库 prompt
└─ 否 → 使用默认 prompt (default_system_prompt)
       ↓
填充 {tools} 占位符
       ↓
返回 (system_prompt, user_prompt)
```

## 数据库表结构

```sql
-- prompts 表
CREATE TABLE prompts (
    id INTEGER PRIMARY KEY,
    architecture TEXT NOT NULL,  -- 'Travel'
    stage TEXT NOT NULL,          -- 'Act'
    content TEXT NOT NULL,        -- prompt 模板内容
    ...
);

-- 示例数据
INSERT INTO prompts (architecture, stage, content) VALUES (
    'Travel',
    'Act',
    '你是 Travel 安全测试智能体的执行者...'
);
```

## 修改的文件

### 1. `src-tauri/src/engines/travel/react_executor.rs`

**主要修改**:
- ✅ `build_thought_prompt`: 改为从 `Travel` + `Act` 读取 prompt
- ✅ `default_system_prompt`: 改为中文版本
- ✅ User prompt: 改为中文

**代码行数**:
- 修改: ~50 行

### 2. `src-tauri/prompts/travel/act.md`

**主要修改**:
- ✅ 重构为 ReAct 模式
- ✅ 添加 `{tools}` 占位符
- ✅ 添加执行格式说明
- ✅ 添加关键规则
- ✅ 添加安全测试最佳实践
- ✅ 添加完整的 ReAct 执行示例

**内容变化**:
- 从通用执行指南 → ReAct 专用模板
- 添加详细的 Thought-Action-Observation 示例

### 3. `docs/travel_react_prompt_dynamic_config.md`

**新增文档**:
- 完整的实现说明
- 修改前后对比
- Prompt 读取流程
- 使用指南

## 编译状态

```bash
✅ Finished `dev` profile [unoptimized] target(s) in 11.54s
⚠️ 146 warnings (unused imports - 无关紧要)
❌ 0 errors
```

## 优势对比

### 修改前 ❌

```
硬编码 Prompt:
- 英文版本
- 无法修改
- 不支持定制
- 与其他架构不一致
```

### 修改后 ✅

```
动态 Prompt:
- 中文版本（默认）
- 可通过前端修改
- 支持定制
- 与其他架构一致
- 从数据库读取
```

## 使用指南

### 1. 通过前端修改 Prompt

1. 打开 `PromptManagement.vue`
2. 选择架构: `Travel`
3. 选择阶段: `Act (执行)`
4. 编辑 prompt 内容
5. 点击保存

### 2. Prompt 占位符

| 占位符 | 说明 | 示例 |
|--------|------|------|
| `{tools}` | 可用工具列表 | 自动填充 |

### 3. Prompt 模板结构

```markdown
# 标题

你是 Travel 安全测试智能体的执行者...

## 可用工具

{tools}

## 执行格式

### 需要使用工具时：
...

### 有足够信息回答时：
...

## 关键规则
...

## 安全测试最佳实践
...

## ReAct 执行示例
...
```

## 关键改进

1. **一致性**: 与其他架构（ReWOO、ReAct、Plan-and-Execute）的 prompt 管理方式保持一致
2. **可维护性**: Prompt 从数据库读取，无需修改代码即可更新
3. **灵活性**: 用户可以通过前端自定义 prompt
4. **本地化**: 默认使用中文 prompt，更适合中文用户
5. **专业性**: 针对安全测试场景定制的 ReAct prompt

## 后续优化建议

1. **多语言支持**: 在数据库中存储多语言版本的 prompt
2. **Prompt 模板库**: 提供多个预设的 prompt 模板供用户选择
3. **A/B 测试**: 支持多个 prompt 模板的效果对比
4. **Prompt 版本管理**: 支持 prompt 的版本控制和回滚

---

**实现日期**: 2025-11-20  
**实现人员**: AI Assistant  
**状态**: ✅ 已完成并编译通过

