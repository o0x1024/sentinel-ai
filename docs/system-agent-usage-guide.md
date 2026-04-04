# System Agent 使用文档

本文说明如何使用当前的 System Agent 能力，实现：

- 通过代理流量自动发现风险
- 在 Security Center 中查看风险结果
- 用测试业务服务快速验证 `traffic_logic_triage` 和 `traffic_active_verifier`

## 1. 功能概览

当前已具备的主链路：

1. 浏览器或客户端请求经过代理
2. 系统基于流量做弱行为推断、过程图归纳、不变量匹配、skill 推荐
3. `Traffic Logic Triage` 被动分析风险
4. 风险写入 Security Center
5. `Traffic Active Verifier` 可手动或自动做后续验证

## 2. 前置条件

需要确认以下几项：

- 代理监听器已经开启
- 流量能够进入历史记录
- 已初始化默认 System Agent
- 已配置可用的 AI 服务

如果没有初始化默认 Agent，可在 `Agent 管理` 页面点击“初始化默认 Agent”。

## 3. 启动测试业务服务

项目内置了一个用于制造越权和逻辑漏洞测试流量的 Web 服务。

在项目根目录执行：

```bash
cd /Users/like/code/sentinel/sentinel-ai
npm run demo:biz-service
```

启动后访问：

```text
http://127.0.0.1:7788/
```

相关文件：

- [demo-business-service.mjs](/Users/like/code/sentinel/sentinel-ai/scripts/demo-business-service.mjs)
- [demo-business-service-ui.html](/Users/like/code/sentinel/sentinel-ai/scripts/demo-business-service-ui.html)
- [system-agent-demo-business-service.md](/Users/like/code/sentinel/sentinel-ai/docs/system-agent-demo-business-service.md)

## 4. 配置行为特征来源

进入代理配置页面，在“行为特征来源”中确认：

- 默认模式：`代理侧弱行为推断`
- 增强模式：`浏览器扩展行为采集`

当前推荐直接使用默认模式。

说明：

- `代理侧弱行为推断` 不依赖浏览器扩展，默认可工作
- ` ewaqsmmc` 当前是增强入口，未连接扩展时会自动回退到默认模式

## 5. 配置 Agent

进入 Sidebar 下的 `系统设置 -> Agent管理`。

### 5.1 Traffic Logic Triage

确认以下配置：

- `已启用`
- `允许自动模式`：开启
- `仅记录影子结果`
  - 如果你希望直接写入 Security Center，请关闭
  - 如果你只想先静默观察，请开启

建议保留默认工具绑定，不要随意禁掉分析工具。

### 5.2 Traffic Active Verifier

确认以下配置：

- `已启用`
- `允许主动重放`
  - 需要做自动或手动验证时开启
- `允许自动模式`
  - 开启后，triage 发现风险时可自动接 verifier
  - 如果你只想先看 passive finding，可以关闭
- `作用域主机`
  - 建议填写允许主动验证的 host
  - 例如：

```text
127.0.0.1
localhost
```

## 6. 推荐测试流程

### 6.1 最容易触发风险发现的操作方式

打开测试业务服务 Web 页面后，推荐按下面顺序操作：

1. 选择身份 `alice`
2. 点击“对象越权：跨租户订单”
3. 点击“对象越权：项目成员列表”
4. 点击“审批越权”
5. 点击“重复兑换”
6. 点击“跳步执行”或“重复支付”

这样最容易制造出：

- 对象越权
- 业务跳步
- 重复消费
- 审批流程异常

### 6.2 更稳定的触发方式

不要一次性连续狂点所有按钮。建议：

- 每次点 1 到 2 个按钮
- 间隔 1 到 3 秒

这样更容易让 triage 和 verifier 正常处理完，不会因为并发队列堆积而延迟显示。

## 7. 如何查看检测结果

### 7.1 历史记录

先到流量历史里确认请求已经经过代理并被记录。

如果历史里没有这批请求，System Agent 也不会有输入。

### 7.2 Security Center

进入 Security Center 的漏洞列表页面：

- 来源会显示 `System Agent`
- 阶段可能显示：
  - `被动分诊`
  - `Agent 已验证`

你可以继续查看详情，详情里会展示：

- 命中的 skills
- 过程图摘要
- 不变量命中
- triage 上下文
- verifier 结果

### 7.3 Agent 管理

在 `Agent管理` 页面可以查看：

- 运行记录
- 最近发现
- skill 命中效果
- 自动保存配置版本

## 8. 手动验证

如果 `Traffic Active Verifier` 没有开启自动模式，也可以在 Security Center 中对某条 finding 手动点击验证。

手动验证会：

- 根据 triage 产出的 `verificationPlan`
- 选择合适的保守验证策略
- 将结果写回 evidence

## 9. 常见问题

### 9.1 为什么没有发现写入 Security Center

常见原因：

- `Traffic Logic Triage` 没有启用
- `仅记录影子结果` 开启了
- 流量没有经过代理
- 该轮 triage 置信度过低，没有写入 finding

### 9.2 为什么日志里出现 cooldown 或 concurrency 提示

当前 runtime 已支持：

- 冷却分组
- 并发排队
- 重试
- 死信

但如果短时间内制造大量流量，仍可能出现队列积压。建议测试时分批触发。

### 9.3 为什么浏览器扩展模式没有生效

当前默认可工作的模式是：

- `代理侧弱行为推断`

浏览器扩展模式目前是增强入口，未连接扩展时系统会自动回退。

## 10. 推荐的最小验证路径

如果你只是想确认当前整套能力是否正常，按这个最小路径就够了：

1. 启动 demo 服务
2. 开启代理监听器
3. 在 Agent 管理里启用：
   - `Traffic Logic Triage`
   - `Traffic Active Verifier`
4. 在行为特征来源里保持默认：`代理侧弱行为推断`
5. 用浏览器访问 demo 页面并点击：
   - 跨租户订单
   - 审批越权
   - 重复兑换
6. 到 Security Center 看 `System Agent` finding
7. 如有需要，手动点击验证

## 11. 相关文件

- Agent 管理页：
  - [AgentManagement.vue](/Users/like/code/sentinel/sentinel-ai/src/views/AgentManagement.vue)
  - [SystemAgentSettings.vue](/Users/like/code/sentinel/sentinel-ai/src/components/Settings/SystemAgentSettings.vue)
- 运行时：
  - [runtime.rs](/Users/like/code/sentinel/sentinel-ai/src-tauri/src/services/system_agents/runtime.rs)
  - [verifier.rs](/Users/like/code/sentinel/sentinel-ai/src-tauri/src/services/system_agents/verifier.rs)
- 行为与逻辑分析：
  - [behavior_session.rs](/Users/like/code/sentinel/sentinel-ai/src-tauri/src/services/system_agents/behavior_session.rs)
  - [process_graph.rs](/Users/like/code/sentinel/sentinel-ai/src-tauri/src/services/system_agents/process_graph.rs)
  - [logic_invariants.rs](/Users/like/code/sentinel/sentinel-ai/src-tauri/src/services/system_agents/logic_invariants.rs)
  - [skill_recommendation.rs](/Users/like/code/sentinel/sentinel-ai/src-tauri/src/services/system_agents/skill_recommendation.rs)
- 测试服务：
  - [demo-business-service.mjs](/Users/like/code/sentinel/sentinel-ai/scripts/demo-business-service.mjs)

