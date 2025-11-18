# Orchestrator 架构使用指南

## 一、概述

Orchestrator 是一个基于 ReAct 的智能安全测试编排系统，它能够：
- 自动选择和调度 ReWOO、Plan-and-Execute、LLM-Compiler 三大子 Agent
- 支持 Web/API 渗透测试、取证分析、CTF 解题、逆向工程等多种安全任务
- 维护完整的测试会话状态，支持认证上下文在步骤间传递
- 提供清晰的可视化展示和进度跟踪

## 二、快速开始

### 2.1 基本使用流程

```
用户 → AI 助手页面 → Orchestrator 自动识别安全任务 → 调度子 Agent → 展示结果
```

### 2.2 典型使用场景

#### 场景 1：Web/API 渗透测试

**用户输入示例：**
```
对 https://api.example.com 进行安全测试，我有测试账号：
用户名：test@example.com
密码：Test123!@#

重点检查：
1. 认证和授权机制
2. API 接口安全
3. 常见漏洞（注入、越权等）
```

**Orchestrator 工作流程：**
1. 识别任务类型：`APIPentest`
2. 调用 **ReWOO** 生成全局测试计划
3. 调用 **Plan-and-Execute** 执行登录流程，获取认证 Token
4. 更新 `AuthContext`，保存 Token/Cookie
5. 调用 **Plan-and-Execute** 枚举 API 端点
6. 调用 **LLM-Compiler** 生成漏洞测试脚本
7. 调用 **Plan-and-Execute** 执行漏洞测试
8. 记录发现的安全问题
9. 生成测试报告

#### 场景 2：取证分析

**用户输入示例：**
```
分析这份服务器日志，找出可疑的入侵行为：
/var/log/apache2/access.log

重点关注：
- 异常访问模式
- 可疑的 User-Agent
- 攻击特征
```

**Orchestrator 工作流程：**
1. 识别任务类型：`Forensics`
2. 调用 **ReWOO** 规划分析策略
3. 调用 **LLM-Compiler** 生成日志解析脚本
4. 调用 **Plan-and-Execute** 执行日志分析
5. 提取 IOC（威胁指标）
6. 重建攻击时间线
7. 生成取证报告

#### 场景 3：CTF 解题

**用户输入示例：**
```
帮我解这道 CTF Web 题：
URL: http://ctf.example.com:8080/challenge
提示：SQL 注入

目标：获取 flag
```

**Orchestrator 工作流程：**
1. 识别任务类型：`CTF`
2. 调用 **ReWOO** 分析题目和规划解题策略
3. 调用 **Plan-and-Execute** 探测注入点
4. 调用 **LLM-Compiler** 生成 SQL 注入 payload
5. 调用 **Plan-and-Execute** 执行注入攻击
6. 提取 flag
7. 生成 writeup

#### 场景 4：逆向工程

**用户输入示例：**
```
分析这个可疑的二进制文件：
/path/to/suspicious.exe

需要了解：
- 文件基本信息
- 是否有恶意行为
- 主要功能
```

**Orchestrator 工作流程：**
1. 识别任务类型：`ReverseEngineering`
2. 调用 **ReWOO** 规划分析流程
3. 调用 **LLM-Compiler** 生成静态分析脚本
4. 调用 **Plan-and-Execute** 执行静态分析
5. 调用 **LLM-Compiler** 生成动态分析脚本
6. 调用 **Plan-and-Execute** 执行动态分析
7. 总结恶意行为
8. 生成分析报告

## 三、前端使用方式

### 3.1 在 AI 助手页面使用

1. 打开 AI 助手页面
2. 在对话框中输入安全测试需求
3. Orchestrator 会自动识别任务类型并开始执行
4. 实时查看执行进度和结果

### 3.2 消息展示说明

#### 会话概要卡片
```
🎯 Web 渗透测试                    [信息收集]
目标: https://api.example.com
正在进行 Web/API 安全测试...

步骤: 5    发现: 2    高危: 1
```

#### 步骤卡片
```
#1  [ReWOO 规划]  [Low]  已完成
决定创建多分支测试计划，包含认证测试、接口枚举和漏洞扫描

▶ 详细输出
```

### 3.3 状态和风险等级

**步骤状态：**
- ⏳ 等待中 (pending)
- ▶️ 执行中 (running)
- ✅ 已完成 (completed)
- ❌ 失败 (failed)

**风险等级：**
- 🔵 Info - 信息性
- 🟢 Low - 低风险
- 🟡 Medium - 中风险
- 🟠 High - 高风险
- 🔴 Critical - 严重

## 四、后端 API 使用

### 4.1 创建测试会话

```rust
use crate::managers::SecurityTestManager;
use crate::models::security_testing::*;

// 创建会话管理器
let manager = SecurityTestManager::new();

// 创建 Web 渗透测试会话
let session = manager.create_session(
    SecurityTaskKind::WebPentest,
    "https://api.example.com".to_string(),
    "API 安全测试".to_string()
).await?;

println!("会话 ID: {}", session.id);
```

### 4.2 添加测试步骤

```rust
// 创建一个步骤
let step = TestStep::new(
    1,
    SubAgentKind::ReWOO,
    TestStepType::PlanSecurityTest,
    "创建全局测试计划".to_string()
).with_risk_impact(RiskImpact::Low);

// 添加到会话
manager.add_step(&session.id, step).await?;
```

### 4.3 更新认证上下文

```rust
// 创建认证上下文
let mut auth_context = AuthContext::new();
auth_context.add_token("bearer".to_string(), "eyJhbGc...".to_string());
auth_context.add_cookie("session".to_string(), "abc123".to_string());

// 更新到会话
manager.update_auth_context(&session.id, auth_context).await?;
```

### 4.4 记录安全发现

```rust
// 创建安全发现
let finding = Finding::new(
    "/api/users/{id}".to_string(),
    RiskImpact::High,
    "IDOR 漏洞允许访问其他用户数据".to_string(),
    "通过修改 URL 中的用户 ID，可以访问任意用户的敏感信息".to_string(),
    "请求: GET /api/users/123\n响应: {...}".to_string()
)
.with_method("GET".to_string())
.with_reproduction_steps(vec![
    "登录系统获取有效 token".to_string(),
    "访问 /api/users/123".to_string(),
    "修改 ID 为 456".to_string(),
    "观察到可以访问其他用户数据".to_string(),
]);

// 添加到会话
manager.add_finding(&session.id, finding).await?;
```

### 4.5 更新会话阶段

```rust
// 更新到漏洞扫描阶段
manager.update_stage(&session.id, TestStage::VulnScan).await?;
```

### 4.6 查询会话统计

```rust
// 获取统计信息
let stats = manager.get_session_stats(&session.id).await?;

println!("总步骤: {}", stats.total_steps);
println!("已完成: {}", stats.completed_steps);
println!("总发现: {}", stats.total_findings);
println!("高危发现: {}", stats.high_findings);
```

## 五、调用子 Agent

### 5.1 调用 ReWOO 规划 Agent

```rust
use crate::engines::orchestrator::*;
use crate::agents::orchestrator::*;

// 创建 Orchestrator 适配器
let orchestrator = OrchestratorEngineAdapter::new(
    Arc::new(manager)
);

// 注册 ReWOO 子 Agent
orchestrator.register_sub_agent(
    SubAgentKind::ReWOO,
    Arc::new(ReWOOSubAgentExecutor::new())
).await;

// 构建上下文
let context = orchestrator.build_sub_agent_context(
    &session.id,
    "创建 Web 渗透测试计划".to_string()
).await?;

// 创建请求
let request = SubAgentRequest::new(
    SubAgentKind::ReWOO,
    session.id.clone(),
    context
);

// 执行
let response = orchestrator.execute_sub_agent(request).await?;

// 处理响应
if let SubAgentOutput::Plan { nodes, summary, .. } = response.output {
    println!("计划: {}", summary);
    for node in nodes {
        println!("- {:?}: {}", node.step_type, node.description);
    }
}
```

### 5.2 调用 Plan-and-Execute 执行 Agent

```rust
// 注册 Plan-and-Execute 子 Agent
orchestrator.register_sub_agent(
    SubAgentKind::PlanAndExecute,
    Arc::new(PlanExecSubAgentExecutor::new())
).await;

// 构建上下文（带认证信息）
let mut context = orchestrator.build_sub_agent_context(
    &session.id,
    "枚举所有 API 端点".to_string()
).await?;

// 执行
let request = SubAgentRequest::new(
    SubAgentKind::PlanAndExecute,
    session.id.clone(),
    context
);

let response = orchestrator.execute_sub_agent(request).await?;

// 处理响应
if let SubAgentOutput::Execution { steps, final_result, auth_context_updated } = response.output {
    println!("执行结果: {}", final_result);
    
    // 如果认证上下文更新了，保存它
    if let Some(new_auth) = auth_context_updated {
        orchestrator.update_auth_context(&session.id, new_auth).await?;
    }
}
```

### 5.3 调用 LLM-Compiler 代码生成 Agent

```rust
// 注册 LLM-Compiler 子 Agent
orchestrator.register_sub_agent(
    SubAgentKind::LLMCompiler,
    Arc::new(CompilerSubAgentExecutor::new())
).await;

// 构建上下文
let context = orchestrator.build_sub_agent_context(
    &session.id,
    "生成 SQL 注入测试脚本".to_string()
).await?;

// 执行
let request = SubAgentRequest::new(
    SubAgentKind::LLMCompiler,
    session.id.clone(),
    context
);

let response = orchestrator.execute_sub_agent(request).await?;

// 处理响应
if let SubAgentOutput::Code { language, code, explanation, usage } = response.output {
    println!("语言: {}", language);
    println!("代码:\n{}", code);
    println!("说明: {}", explanation);
    println!("用法: {}", usage);
}
```

## 六、自定义扩展

### 6.1 添加新的安全任务类型

```rust
// 在 src-tauri/src/models/security_testing.rs 中添加
pub enum SecurityTaskKind {
    WebPentest,
    APIPentest,
    Forensics,
    CTF,
    ReverseEngineering,
    OtherSecurity,
    // 新增类型
    MobileAppPentest,  // 移动应用渗透测试
    CloudSecurity,     // 云安全评估
}

// 添加对应的阶段
pub enum TestStage {
    // ... 现有阶段
    
    // 移动应用测试阶段
    AppDecompile,
    StaticCodeAnalysis,
    DynamicTesting,
    
    // 云安全阶段
    ConfigurationReview,
    IAMAnalysis,
    NetworkSecurity,
}
```

### 6.2 实现自定义子 Agent

```rust
use crate::agents::orchestrator::*;
use async_trait::async_trait;

pub struct CustomSubAgentExecutor {
    // 自定义字段
}

#[async_trait]
impl SubAgentExecutor for CustomSubAgentExecutor {
    async fn execute(&self, request: SubAgentRequest) -> Result<SubAgentResponse> {
        // 实现自定义逻辑
        
        Ok(SubAgentResponse::success(
            SubAgentKind::Other,
            SubAgentOutput::Generic {
                content: "自定义输出".to_string(),
            }
        ))
    }
}

// 注册
orchestrator.register_sub_agent(
    SubAgentKind::Other,
    Arc::new(CustomSubAgentExecutor::new())
).await;
```

### 6.3 自定义前端展示

在 `src/composables/useOrchestratorMessage.ts` 中添加新的标签映射：

```typescript
const taskKindLabel = computed(() => {
  const kind = sessionData.value?.taskKind
  const labels: Record<string, string> = {
    'web_pentest': 'Web 渗透测试',
    'api_pentest': 'API 渗透测试',
    'forensics': '取证分析',
    'ctf': 'CTF 解题',
    'reverse_engineering': '逆向工程',
    'other_security': '其他安全任务',
    // 添加新类型
    'mobile_app_pentest': '移动应用测试',
    'cloud_security': '云安全评估',
  }
  return kind ? labels[kind] || kind : ''
})
```

## 七、最佳实践

### 7.1 任务描述要清晰

**好的示例：**
```
对 https://api.example.com 进行 API 安全测试
账号：test@example.com / Test123!
重点：认证、授权、注入漏洞
```

**不好的示例：**
```
测试一下这个网站
```

### 7.2 提供必要的上下文

- 目标 URL 或文件路径
- 测试账号（如果有）
- 测试范围和重点
- 已知信息或线索

### 7.3 合理利用认证上下文

Orchestrator 会自动维护认证状态，但你可以：
- 提供初始凭据
- 在步骤间检查认证是否有效
- 必要时重新认证

### 7.4 关注风险等级

- High/Critical 发现需要优先处理
- 可以根据风险等级调整测试深度
- 记录详细的复现步骤

### 7.5 利用子 Agent 的优势

- **ReWOO**：复杂任务的全局规划
- **Plan-and-Execute**：需要维持状态的线性操作
- **LLM-Compiler**：需要生成代码或脚本时

## 八、故障排查

### 8.1 会话创建失败

**问题：** 无法创建测试会话

**解决：**
- 检查任务类型是否正确
- 确认目标格式是否有效
- 查看后端日志

### 8.2 子 Agent 调用失败

**问题：** 子 Agent 返回错误

**解决：**
- 检查子 Agent 是否已注册
- 验证请求上下文是否完整
- 查看具体错误信息

### 8.3 认证上下文丢失

**问题：** 后续步骤无法使用认证信息

**解决：**
- 确认认证步骤成功完成
- 检查是否调用了 `update_auth_context`
- 验证 Token/Cookie 格式

### 8.4 前端显示异常

**问题：** 步骤或会话信息显示不正确

**解决：**
- 检查消息格式是否为 JSON
- 验证 `type` 字段是否为 `orchestrator_session` 或 `orchestrator_step`
- 查看浏览器控制台错误

## 九、示例代码

完整的使用示例请参考：
- 后端示例：`src-tauri/examples/orchestrator_example.rs`（待创建）
- 前端示例：`src/examples/OrchestratorExample.vue`（待创建）
- 集成测试：`src-tauri/tests/orchestrator_integration_test.rs`（待创建）

## 十、常见问题

**Q: Orchestrator 和直接使用 ReAct/ReWOO 有什么区别？**

A: Orchestrator 是一个更高层的编排系统，它：
- 专注于安全测试场景
- 自动选择最合适的子 Agent
- 维护完整的测试状态和上下文
- 提供统一的可视化界面

**Q: 可以同时运行多个测试会话吗？**

A: 可以。每个会话都有独立的 ID 和状态，互不干扰。

**Q: 如何持久化测试结果？**

A: 当前版本测试会话存储在内存中。后续版本将支持数据库持久化。

**Q: 支持哪些认证方式？**

A: 目前支持：
- Cookie
- Bearer Token
- API Key
- 自定义 Headers
- 用户名/密码（用于登录流程）

**Q: 如何生成测试报告？**

A: 可以通过 `session_to_messages` 方法获取完整的测试记录，然后使用模板生成报告。后续版本将提供内置的报告生成功能。

## 十一、下一步学习

1. 阅读 [Orchestrator 架构设计文档](./orchestrator_agent_implementation_plan.md)
2. 查看各子 Agent 的详细文档
3. 尝试运行示例代码
4. 根据实际需求进行定制化开发

---

**文档版本：** v1.0  
**最后更新：** 2025-11-18  
**维护者：** Sentinel-AI Team

