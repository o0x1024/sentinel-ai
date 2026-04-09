# 逻辑漏洞安全工作台实施补充

本文档是 [logic-vuln-ultimate-solution.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-ultimate-solution.md) 的工程化补充，专门展开：

- 后端接口草案
- 前端路由设计
- case 状态机
- 从现有漏洞中心迁移到安全工作台的落地步骤

配套 MVP 任务清单见：

- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)

---

## 1. 后端接口草案

第一阶段不建议一开始就设计成完整的复杂工作流系统，后端接口应围绕“从 finding 进入 case、在 case 中查看证据、写入复盘结论、回写 finding”这条闭环最小实现。

### 1.1 Case 管理接口

#### `get_or_create_workbench_case`

用途：

- 从 finding 详情进入工作台时调用
- 如果该 finding 已经绑定 case，则直接返回
- 如果未绑定，则创建主 case

输入建议：

- `findingId`
- `entrySource`

输出建议：

- `case`
- `created`
- `linkedFinding`

#### `list_workbench_cases`

用途：

- 安全工作台案件列表页

输入建议：

- `status`
- `owner`
- `findingType`
- `source`
- `search`
- `page`
- `pageSize`

输出建议：

- `items`
- `total`
- `stats`

#### `get_workbench_case_detail`

用途：

- 获取案件详情页基础数据

输出建议：

- `case`
- `finding`
- `latestSession`
- `evidenceLinks`
- `verificationRuns`
- `notes`

#### `update_workbench_case`

用途：

- 更新案件状态、负责人、当前结论、优先级

输入建议：

- `caseId`
- `status`
- `owner`
- `assignee`
- `currentConclusion`
- `priority`

### 1.2 Evidence 与链路接口

#### `list_workbench_case_evidence`

用途：

- 拉取案件关联证据链

输出建议：

- `baselineEvidence`
- `relatedEvidence`
- `generatedEvidence`
- `timelineEntries`

#### `set_workbench_case_baseline`

用途：

- 将某条请求或证据标记为案件基线

输入建议：

- `caseId`
- `evidenceId`

#### `link_evidence_to_workbench_case`

用途：

- 手动把其它 evidence 关联到案件

输入建议：

- `caseId`
- `evidenceId`
- `role`

### 1.3 验证执行接口

#### `list_workbench_verification_runs`

用途：

- 展示案件中已有的验证执行历史

#### `create_workbench_verification_run`

用途：

- 从工作台中发起一次验证动作

输入建议：

- `caseId`
- `strategy`
- `baselineRequestId`
- `mutationPlan`
- `notes`

输出建议：

- `runId`
- `accepted`
- `queued`

#### `get_workbench_verification_run`

用途：

- 拉取单次验证执行结果与差异摘要

输出建议：

- `baseline`
- `mutated`
- `requestDiff`
- `responseDiff`
- `result`
- `summary`

### 1.4 复盘与回写接口

#### `list_workbench_notes`

用途：

- 获取案件的人工记录和复盘笔记

#### `create_workbench_note`

用途：

- 写入观察记录、结论记录、误报原因、修复建议

输入建议：

- `caseId`
- `kind`
- `body`

#### `sync_workbench_case_to_finding`

用途：

- 将案件结论回写到 finding

输入建议：

- `caseId`
- `targetFindingStatus`
- `summaryPatch`
- `labels`

输出建议：

- `findingUpdated`
- `caseUpdated`

### 1.5 第一阶段接口设计原则

1. 一个接口只处理一类职责
2. 不在第一阶段引入复杂批处理
3. 所有接口都必须返回稳定的结构化数据
4. case 和 finding 的同步动作要显式触发，不隐式联动

---

## 2. 前端路由设计

建议在 Security Center 中引入独立路由，而不是把工作台完全塞进弹窗或现有详情里。

### 2.1 推荐路由

- `/security-center/vulnerabilities`
  - 漏洞台账

- `/security-center/workbench`
  - 案件列表

- `/security-center/workbench/:caseId`
  - 案件详情

### 2.2 从漏洞中心进入工作台

建议在漏洞详情页和列表行都提供入口：

- `进入工作台`
- `打开案件`

交互建议：

1. 如果 finding 已绑定 case，则直接跳转案件详情
2. 如果未绑定 case，则先调用 `get_or_create_workbench_case`
3. 创建成功后跳转 `/security-center/workbench/:caseId`

### 2.3 工作台内部路由策略

第一阶段不建议把四个面板拆成子路由，保持案件详情页单页 tabs 即可。

推荐 tabs：

- `overview`
- `evidence`
- `verification`
- `review`

这样有几个好处：

1. 心智简单
2. 便于从 finding 直接落进案件详情
3. 不会过早引入复杂路由同步问题

### 2.4 第二阶段路由扩展

如果后续面板复杂度明显升高，再考虑引入子路由：

- `/security-center/workbench/:caseId/overview`
- `/security-center/workbench/:caseId/evidence`
- `/security-center/workbench/:caseId/verification`
- `/security-center/workbench/:caseId/review`
- `/security-center/workbench/:caseId/object-pool`
- `/security-center/workbench/:caseId/behavior`

---

## 3. Case 状态机

案件状态不应简单复用 finding 状态，需要单独定义“调查态”状态机。

### 3.1 建议状态

- `new`
  - 刚从 finding 创建，尚未开始调查

- `investigating`
  - 正在分析请求链、证据链或对象关系

- `awaiting_verification`
  - 已有明确假设，等待执行或补充验证

- `verified`
  - 案件侧已经确认成立

- `false_positive`
  - 案件侧已确认为误报或误测

- `archived`
  - 已完成复盘并归档

### 3.2 状态转移建议

推荐转移关系：

```text
new -> investigating
investigating -> awaiting_verification
investigating -> false_positive
awaiting_verification -> verified
awaiting_verification -> false_positive
verified -> archived
false_positive -> archived
```

### 3.3 状态与 finding 的关系

case 状态和 finding 状态不必 1:1 同步。

建议关系如下：

- case `new / investigating / awaiting_verification`
  - finding 仍可能是 `candidate`

- case `verified`
  - finding 可被回写为 `reviewed` 或更明确的已验证状态

- case `false_positive`
  - finding 可被回写为 `false_positive`

- case `archived`
  - finding 不一定变化，只表示案件调查结束

### 3.4 状态设计原则

1. case 表示调查过程，不直接等于漏洞生命周期
2. 状态必须反映“现在调查处于哪一步”
3. 回写 finding 应由显式操作触发

---

## 4. 从现有漏洞中心迁移到工作台的落地步骤

为了避免一次性大改，建议按四步迁移。

### 4.1 第一步：只增加入口，不改变原有列表结构

做法：

- 在现有漏洞详情中新增 `进入工作台`
- 在列表行中新增 `打开案件`

目标：

- 不破坏现有漏洞中心的使用习惯
- 先建立 `finding -> case` 绑定能力

### 4.2 第二步：先做最小案件详情页

做法：

- 新增工作台路由
- 实现案件概览
- 实现请求与证据链
- 实现复盘记录

目标：

- 先让工作台具备“读得懂、记得住、能回写”的能力

### 4.3 第三步：接入验证与对比

做法：

- 引入验证执行记录
- 引入基线与变异对比视图
- 从工作台发起或查看验证动作

目标：

- 把“调查过程”从静态阅读升级成动态验证

### 4.4 第四步：逐步承接逻辑漏洞专属能力

做法：

- 把行为链、对象池、对象关系、自动测试建议逐步沉到工作台
- 漏洞详情只保留摘要和入口

目标：

- 避免漏洞详情页再次膨胀
- 让逻辑漏洞专属复杂能力只存在于工作台

### 4.5 最终迁移后的边界

迁移完成后，建议形成稳定边界：

`漏洞中心`

- 列表
- 状态
- 摘要
- 基础详情
- 证据浏览
- 时间线

`安全工作台`

- 调查过程
- 验证过程
- 对比过程
- 复盘过程
- 结论沉淀

### 4.6 迁移过程中的工程原则

1. 不推翻现有漏洞中心
2. 先加入口，再加页面，再迁专属能力
3. 逻辑漏洞专属复杂视图逐步下沉到工作台
4. 普通漏洞继续使用统一详情，不被逻辑漏洞能力污染

---

## 5. 工程化结论

如果继续往前推进，最合理的工程路径不是“重做安全中心”，而是：

1. 保留现有漏洞台账
2. 新增安全工作台路由和 case 层
3. 建立 finding 到 case 的绑定
4. 先打通最小闭环
5. 再逐步把复杂调查与逻辑漏洞专属能力迁入工作台

这样做的好处是：

- 不破坏当前已有漏洞中心能力
- 结构更清晰
- 后续逻辑漏洞扩展空间更大
- 也能兼容 SQLi、RCE、SSRF 等普通漏洞的复盘需求

一句话总结：

`先在现有漏洞中心旁边长出工作台，再逐步把“过程能力”迁过去，而不是直接把漏洞中心改造成工作台。`
