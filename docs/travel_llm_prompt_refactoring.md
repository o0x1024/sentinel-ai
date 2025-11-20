# Travel LLM Prompt 重构 - 参考 ReWOO 模式

## 重构目标

将 Travel 的 LLM 调用方式改为参考 ReWOO 的实现：
1. **分离 system 和 user prompt**
2. **从数据库读取 prompt 模板**
3. **避免硬编码 prompt**

## 问题分析

### 原有实现 ❌

```rust
// 硬编码的单一 prompt
let prompt = format!(r#"你是一个安全测试专家...
**任务类型**: {}
**目标**: {}
...
"#, task_type, target, query, available_tools);

// 直接调用，没有 system prompt
let response = ai_service
    .send_message_stream(Some(&prompt), None, None, None, false, true, None)
    .await?;
```

**问题**:
- ❌ prompt 硬编码在代码中，难以维护和更新
- ❌ 没有分离 system 和 user prompt
- ❌ 无法通过数据库动态配置 prompt
- ❌ 与其他架构（ReWOO、ReAct）的实现方式不一致

## 重构实现

### 1. 参考 ReWOO 的实现模式

#### ReWOO 的 LLM 调用流程

```rust
// rewoo_planner.rs
pub async fn plan(...) -> Result<ReWOOPlan> {
    // 1. 构建 system prompt 和 user prompt
    let (system_prompt, user_prompt) = self.build_planning_prompt(...).await?;
    
    // 2. 调用 LLM
    let plan_string = self.call_llm(&system_prompt, &user_prompt, execution_id).await?;
    
    // 3. 解析结果
    let steps = self.parse_plan(&plan_string)?;
    
    Ok(ReWOOPlan { steps, ... })
}

async fn build_planning_prompt(...) -> Result<(String, String)> {
    // 从数据库获取模板
    let system_template = if let Ok(Some(template)) = self.prompt_repo
        .get_template_by_arch_stage(ArchitectureType::ReWOO, StageType::Planner)
        .await
    {
        template.content
    } else {
        // Fallback 到默认模板
        DEFAULT_PLANNER_PROMPT.to_string()
    };
    
    // 填充占位符
    let system_prompt = system_template.replace("{tools}", &tools_desc);
    let user_prompt = query.to_string();
    
    Ok((system_prompt, user_prompt))
}

async fn call_llm(&self, system_prompt: &str, user_prompt: &str, ...) -> Result<String> {
    let content = ai_service
        .send_message_stream(
            Some(user_prompt),
            Some(system_prompt),  // ✅ 分离的 system prompt
            None,                  // 不关联会话
            Some(execution_id),
            false,                 // 不流式发送到前端
            false,                 // 不是最终消息
            None,
        )
        .await?;
    
    Ok(content)
}
```

### 2. Travel 的新实现

#### 修改后的代码结构

```rust
// ooda_executor.rs
async fn plan_observation_with_llm(...) -> Result<HashMap<String, serde_json::Value>> {
    // 1. 构建可用工具列表
    let available_tools = self.get_available_tools_for_observation().await;
    
    // 2. 构建 system prompt 和 user prompt（参考 ReWOO）
    let (system_prompt, user_prompt) = self.build_observation_planning_prompt(
        task_type,
        target,
        query,
        &available_tools,
    ).await?;

    // 3. 调用 LLM（参考 ReWOO）
    let response = ai_service
        .send_message_stream(
            Some(&user_prompt),
            Some(&system_prompt),  // ✅ 分离的 system prompt
            None,                   // 不关联会话
            None,                   // 无 execution_id
            false,                  // 不流式发送到前端
            false,                  // 不是最终消息
            None,
        )
        .await?;

    // 4. 解析 LLM 响应
    let plan: serde_json::Value = self.parse_llm_observation_plan(&response)?;

    // 5. 执行规划的步骤
    let mut observations = HashMap::new();
    if let Some(steps) = plan.get("steps").and_then(|s| s.as_array()) {
        for (idx, step) in steps.iter().enumerate() {
            // ... 执行工具 ...
        }
    }

    Ok(observations)
}
```

#### 新增：构建 prompt 的方法

```rust
/// 构建观察规划的 prompt（返回 system prompt 和 user prompt）
async fn build_observation_planning_prompt(
    &self,
    task_type: &str,
    target: &str,
    query: &str,
    available_tools: &str,
) -> Result<(String, String)> {
    use crate::models::prompt::{ArchitectureType, StageType};
    
    // 从数据库获取 Travel Observe 阶段的 prompt 模板
    let system_template = if let Some(prompt_repo) = &self.engine_dispatcher.prompt_repo {
        if let Ok(Some(template)) = prompt_repo
            .get_template_by_arch_stage(ArchitectureType::Travel, StageType::Observe)
            .await
        {
            log::info!("Travel Observe Planner: Using prompt from database");
            template.content
        } else {
            log::warn!("Travel Observe template not found in database, using default template");
            self.get_default_observe_planning_prompt()
        }
    } else {
        log::warn!("No prompt repository available, using default template");
        self.get_default_observe_planning_prompt()
    };
    
    // 填充 system prompt 中的占位符
    let system_prompt = system_template
        .replace("{tools}", available_tools)
        .replace("{task_type}", task_type)
        .replace("{target}", target);
    
    // user prompt 是用户的查询
    let user_prompt = format!(
        "任务类型: {}\n目标: {}\n用户查询: {}",
        task_type,
        target,
        query
    );
    
    Ok((system_prompt, user_prompt))
}

/// 获取默认的观察规划 prompt
fn get_default_observe_planning_prompt(&self) -> String {
    r#"你是一个安全测试专家，负责规划 Observe (侦察) 阶段的信息收集流程。

**可用工具**:
{tools}

**任务类型说明**:
- web_pentest: Web 渗透测试 → 使用 analyze_website, http_request, port_scan
- api_pentest: API 安全测试 → 使用 http_request, analyze_website
- code_audit: 代码审计 → 不需要网络工具，直接分析代码
...

**请规划需要执行的观察步骤**，以 JSON 格式返回：

```json
{
  "steps": [
    {
      "tool": "工具名称",
      "args": {"参数名": "参数值"},
      "description": "步骤描述"
    }
  ],
  "reasoning": "规划理由"
}
```

只返回 JSON，不要其他文字。"#.to_string()
}
```

### 3. 更新数据库 Prompt 模板

#### 更新 `src-tauri/prompts/travel/observe.md`

```markdown
# Travel OODA - Observe (侦察) 阶段 - 智能规划模式

你是 Travel 安全测试智能体的侦察阶段规划者。你的任务是根据任务类型和目标，智能规划信息收集流程。

## 当前任务信息

- **任务类型**: {task_type}
- **目标**: {target}

## 阶段目标

根据任务类型，规划合适的侦察步骤：
- 识别目标的技术栈和架构（如适用）
- 发现所有可访问的资产和端点（如适用）
- 绘制攻击面地图（如适用）
- 记录网络拓扑和服务信息（如适用）

## 可用工具

{tools}

## 任务类型与侦察策略

### Web 渗透测试 (web_pentest)
- 使用 `analyze_website` 分析网站结构（参数: domain）
- 使用 `http_request` 获取 HTTP 响应（参数: url, method）
- 使用 `port_scan` 扫描端口（参数: target=IP地址, ports）
- 使用 `rsubdomain` 枚举子域名（参数: domain）

### API 安全测试 (api_pentest)
- 使用 `http_request` 测试 API 端点
- 使用 `analyze_website` 分析 API 服务器

### 代码审计 (code_audit)
- **不需要网络扫描工具**
- 直接记录代码路径和审计类型

### CTF 夺旗 (ctf)
- 根据题目类型选择：
  - Web CTF: 使用 `http_request`
  - Crypto/Pwn CTF: 不需要网络工具

...

## 输出格式

**必须**以 JSON 格式返回侦察规划：

```json
{
  "steps": [
    {
      "tool": "工具名称",
      "args": {"参数名": "参数值"},
      "description": "步骤描述"
    }
  ],
  "reasoning": "规划理由"
}
```

**重要提示**:
- 只返回 JSON，不要其他文字
- 工具参数必须正确：
  - `analyze_website` 需要 `domain`（域名），不是 `url`
  - `port_scan` 需要 `target`（IP地址），不是域名
  - `http_request` 需要 `url` 和 `method`
- 代码审计、移动安全等任务可以返回空的 `steps` 数组

## 规划示例

### 示例 1: Web 渗透测试

```json
{
  "steps": [
    {
      "tool": "analyze_website",
      "args": {"domain": "example.com"},
      "description": "分析网站结构和技术栈"
    },
    {
      "tool": "http_request",
      "args": {"url": "http://example.com", "method": "GET"},
      "description": "获取首页内容"
    },
    {
      "tool": "port_scan",
      "args": {"target": "192.168.1.1", "ports": "80,443,8080"},
      "description": "扫描常见 Web 端口"
    }
  ],
  "reasoning": "Web 渗透测试需要全面了解目标网站的结构、技术栈和开放端口"
}
```

### 示例 2: 代码审计

```json
{
  "steps": [],
  "reasoning": "代码审计是静态分析任务，不需要网络扫描工具"
}
```

现在请根据任务类型和目标，规划侦察步骤！
```

## 重构对比

### 修改前 ❌

```rust
// 硬编码 prompt
let prompt = format!(r#"
你是一个安全测试专家...
**任务类型**: {}
**目标**: {}
**用户查询**: {}
**可用工具**: {}
...
"#, task_type, target, query, available_tools);

// 单一 prompt 调用
let response = ai_service
    .send_message_stream(Some(&prompt), None, None, None, false, true, None)
    .await?;
```

**问题**:
- ❌ prompt 硬编码，难以维护
- ❌ 没有 system/user 分离
- ❌ 无法从数据库配置

### 修改后 ✅

```rust
// 从数据库读取模板
let (system_prompt, user_prompt) = self.build_observation_planning_prompt(
    task_type, target, query, &available_tools
).await?;

// 分离的 system 和 user prompt
let response = ai_service
    .send_message_stream(
        Some(&user_prompt),
        Some(&system_prompt),  // ✅ system prompt
        None,
        None,
        false,
        false,
        None,
    )
    .await?;
```

**优势**:
- ✅ prompt 从数据库读取，易于更新
- ✅ system/user 分离，符合 LLM 最佳实践
- ✅ 与 ReWOO 等架构保持一致
- ✅ 支持占位符动态填充

## 占位符系统

### System Prompt 占位符

| 占位符 | 说明 | 示例值 |
|--------|------|--------|
| `{tools}` | 可用工具列表 | `- analyze_website(domain: string)...` |
| `{task_type}` | 任务类型 | `web_pentest` |
| `{target}` | 目标 | `http://example.com` |

### User Prompt 格式

```
任务类型: web_pentest
目标: http://example.com
用户查询: 对网站进行全面的安全测试
```

## 数据库集成

### Prompt 读取流程

```
1. 尝试从 PromptRepository 读取
   ↓
   prompt_repo.get_template_by_arch_stage(
       ArchitectureType::Travel,
       StageType::Observe
   )
   ↓
2. 如果找到，使用数据库模板
   ↓
3. 如果未找到，使用默认模板（代码中的 fallback）
   ↓
4. 填充占位符
   ↓
5. 返回 (system_prompt, user_prompt)
```

### 数据库表结构

```sql
-- prompts 表
CREATE TABLE prompts (
    id INTEGER PRIMARY KEY,
    architecture TEXT NOT NULL,  -- 'Travel'
    stage TEXT NOT NULL,          -- 'Observe'
    content TEXT NOT NULL,        -- prompt 模板内容
    ...
);
```

## 修改的文件

### 1. `src-tauri/src/engines/travel/ooda_executor.rs`

**主要修改**:
- ✅ 重构 `plan_observation_with_llm` 方法
- ✅ 新增 `build_observation_planning_prompt` 方法
- ✅ 新增 `get_default_observe_planning_prompt` 方法
- ✅ 修改 LLM 调用方式，分离 system 和 user prompt

**代码行数**:
- 新增: ~80 行
- 修改: ~20 行

### 2. `src-tauri/prompts/travel/observe.md`

**主要修改**:
- ✅ 添加 `{task_type}` 和 `{target}` 占位符
- ✅ 重构为智能规划模式
- ✅ 添加任务类型与侦察策略映射
- ✅ 更新输出格式和示例
- ✅ 强调工具参数正确性

**内容变化**:
- 从通用侦察指南 → 智能规划模板
- 添加多个任务类型的规划示例

## 编译状态

```bash
✅ Finished `dev` profile [unoptimized] target(s) in 25.74s
⚠️ 145 warnings (unused imports - 无关紧要)
❌ 0 errors
```

## 优势总结

### 1. 一致性 ✅

- 与 ReWOO、ReAct 等架构的实现方式保持一致
- 统一的 prompt 管理模式
- 统一的 LLM 调用方式

### 2. 可维护性 ✅

- prompt 从数据库读取，无需修改代码即可更新
- 清晰的 system/user prompt 分离
- 占位符系统便于动态填充

### 3. 灵活性 ✅

- 用户可以通过前端 `PromptManagement.vue` 自定义 prompt
- 支持多语言 prompt（中文/英文）
- 易于添加新的占位符

### 4. 最佳实践 ✅

- 符合 LLM 的 system/user prompt 最佳实践
- 更好的 prompt engineering
- 更清晰的角色定义

## 后续优化建议

1. **Orient、Decide、Act 阶段**: 也应该采用相同的模式
2. **Prompt 版本管理**: 支持 prompt 的版本控制和回滚
3. **A/B 测试**: 支持多个 prompt 模板的 A/B 测试
4. **Prompt 性能监控**: 记录不同 prompt 的效果

---

**实现日期**: 2025-11-20  
**实现人员**: AI Assistant  
**参考架构**: ReWOO Planner  
**状态**: ✅ 已完成并编译通过

