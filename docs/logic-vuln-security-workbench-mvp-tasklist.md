# 安全工作台 MVP 实施任务清单

本文档用于把“漏洞台账 + 安全工作台”方案进一步落成可执行任务清单。  
目标不是描述终局能力，而是定义第一阶段 MVP 的交付范围、实现顺序、验收标准和风险控制。

关联文档：

- [logic-vuln-ultimate-solution.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-ultimate-solution.md)
- [logic-vuln-security-workbench-implementation.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-implementation.md)
- [logic-vuln-security-workbench-iteration-1-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-1-task.md)
- [logic-vuln-security-workbench-iteration-2-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-2-task.md)
- [logic-vuln-security-workbench-iteration-6-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-6-task.md)
- [logic-vuln-security-workbench-iteration-7-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-7-task.md)
- [logic-vuln-security-workbench-iteration-8-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-8-task.md)
- [logic-vuln-security-workbench-iteration-9-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-9-task.md)
- [logic-vuln-security-workbench-iteration-10-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-10-task.md)

---

## 0. 当前实现状态

截至当前实现，MVP 的主体闭环已经打通，状态如下：

### 已完成

1. 漏洞详情和漏洞列表都可以进入安全工作台
2. Security Center 内已经有独立的 `安全工作台` 入口和稳定路径
3. `finding -> case` 已经从前端本地存储升级成后端持久化
4. 工作台案件列表支持基础搜索、状态筛选和分页
5. 工作台案件详情支持查看概览、证据链、验证记录和复盘备注
6. 工作台支持保存案件状态、负责人、优先级、结论和基线证据
7. 工作台支持把案件状态显式回写到 finding 状态
8. 工作台已经支持对象引用池、行为链推断和水平越权测试建议
9. 工作台已经支持可复制的 replay plan
10. 工作台已经支持可持久化的执行草案
11. 工作台已经支持只读执行草案和执行结果回流到验证面板
12. 工作台执行结果已经带有结构化差异信号，如状态码变化、响应长度变化和字段变化
13. 工作台只读执行已经支持 `path.*`、`query.*`、`body.*` 三类目标字段
14. 工作台已经支持基于执行结果生成“建议状态”和“建议结论”，并可直接应用到案件草稿
15. 建议状态/建议结论已经下沉到后端统一评估，前端不再维护另一套推断逻辑
16. 工作台已经支持“按建议回写”漏洞状态，回写前会确认是否先把建议状态和建议结论应用到案件
17. 工作台已经支持活动记录，可追溯执行草案、普通回写和按建议回写三类关键动作

### 暂未完成

1. 对象关系图
2. 非只读自动测试计划执行和主动验证发起
3. 团队协作评论
4. 独立 case 数据表

一句话总结：

`当前已完成“漏洞台账 -> 工作台调查 -> 对象边界建议 -> replay plan -> 执行草案 -> 只读执行结果回流”的 MVP 闭环，但还没有进入非只读自动验证阶段。`

---

## 1. MVP 目标

第一阶段 MVP 只解决一个问题：

`让安全中心从“只能看 finding 结果”，升级成“可以围绕 finding 进行调查、验证、复盘，并把结论回写”。`

MVP 范围内必须打通的最小闭环：

1. 从漏洞详情进入安全工作台
2. 自动创建或打开与 finding 绑定的 case
3. 在工作台中查看案件概览、请求与证据链
4. 在工作台中记录复盘备注
5. 在工作台中查看已有验证结果
6. 将案件结论回写到 finding

MVP 明确**不要求**一次完成：

- 行为链视图
- 对象池视图
- 自动测试计划
- 对象关系图
- 同类 case 聚类
- 团队协作评论

这些能力属于后续增强项。

---

## 2. MVP 交付范围

### 2.1 页面范围

MVP 需要交付：

1. 漏洞详情中的 `进入工作台` 入口
2. 安全工作台案件列表页
3. 安全工作台案件详情页

案件详情页包含 4 个核心面板：

1. 案件概览
2. 请求与证据链
3. 验证与对比
4. 复盘记录

### 2.2 数据范围

MVP 需要引入：

- `WorkbenchCase`
- `WorkbenchNote`
- `WorkbenchEvidenceLink`
- `WorkbenchVerificationRun` 的只读接入层

MVP 可以暂时不实现：

- 完整 `WorkbenchSession`
- 自动 case 聚类
- 跨 case 知识沉淀

### 2.3 回写范围

MVP 允许回写 finding 的字段仅限：

- 状态
- 描述摘要补丁
- 标签
- 验证结果摘要

不在 MVP 中修改：

- 原始 evidence
- 原始请求记录
- 历史命中计数逻辑

---

## 3. 实施阶段

建议分成 5 个阶段。

### 阶段 0：基础建模

目标：

- 建立 workbench case 的基础数据模型
- 打通 finding 到 case 的绑定关系

产出：

- 数据结构定义
- 后端基础接口
- 类型定义

### 阶段 1：入口与案件列表

目标：

- 从漏洞中心进入工作台
- 可以浏览 workbench cases

产出：

- 漏洞详情入口
- workbench 列表页
- case 创建 / 打开能力

### 阶段 2：案件详情基础能力

目标：

- 可以查看案件概览、请求与证据链、复盘记录

产出：

- 案件详情容器页
- 概览面板
- 请求与证据链面板
- 复盘记录面板

### 阶段 3：验证与对比

目标：

- 可以在工作台里查看已有验证记录和结果差异

产出：

- 验证与对比面板
- 验证记录查询
- 基线/变异对比视图

### 阶段 4：回写闭环

目标：

- 可以把案件结论回写到 finding

产出：

- 回写动作
- 回写确认交互
- finding 摘要同步

---

## 4. 任务拆解

### 4.1 阶段 0：基础建模

#### 任务 0.1

定义前端类型：

- `WorkbenchCase`
- `WorkbenchNote`
- `WorkbenchEvidenceLink`
- `WorkbenchVerificationRunSummary`

验收标准：

- 类型文件独立
- 不与现有 finding 类型强耦合

#### 任务 0.2

定义后端数据模型和存储结构。

验收标准：

- case 可以通过 `finding_id` 稳定定位
- note 能独立追加

#### 任务 0.3

实现基础接口：

- `get_or_create_workbench_case`
- `list_workbench_cases`
- `get_workbench_case_detail`

验收标准：

- finding 能稳定生成或打开 case
- 无重复 case 冲突

### 4.2 阶段 1：入口与案件列表

#### 任务 1.1

在漏洞详情页增加 `进入工作台` 入口。

验收标准：

- 所有 finding 都可见该入口
- 点击后能打开或创建 case

#### 任务 1.2

新增 Security Center workbench 路由。

验收标准：

- 可独立进入案件列表页
- 支持从列表进入案件详情页

#### 任务 1.3

实现案件列表页。

建议字段：

- case 标题
- finding 标题
- finding 类型
- 当前状态
- 负责人
- 最近活动时间

验收标准：

- 支持分页
- 支持基本筛选
- 支持空态展示

### 4.3 阶段 2：案件详情基础能力

#### 任务 2.1

实现案件详情容器页。

验收标准：

- 具备 tabs 或分面板结构
- 支持独立路由打开

#### 任务 2.2

实现案件概览面板。

验收标准：

- 能稳定展示 finding 基础信息
- 能展示 case 当前状态、结论、负责人

#### 任务 2.3

实现请求与证据链面板。

验收标准：

- 能拉取并展示基线 evidence
- 能按时间顺序组织 related evidence
- 支持标记基线请求

#### 任务 2.4

实现复盘记录面板。

验收标准：

- 可新增 note
- 可展示历史 note
- note 至少支持 observation / conclusion / false_positive_reason

### 4.4 阶段 3：验证与对比

#### 任务 3.1

接入 verification run 列表。

验收标准：

- 能展示案件相关验证记录
- 可区分 success / failed / blocked

#### 任务 3.2

实现基线与变异结果对比视图。

验收标准：

- 能展示请求差异
- 能展示响应差异
- 能展示结果摘要

#### 任务 3.3

实现“设为当前验证结论”的交互。

验收标准：

- 用户可从已有 run 中选一个作为当前结论依据

### 4.5 阶段 4：回写闭环

#### 任务 4.1

实现 case 到 finding 的显式回写。

验收标准：

- 可回写 finding 状态
- 可回写摘要补丁
- 可回写验证结果摘要

#### 任务 4.2

实现回写确认弹窗。

验收标准：

- 用户能清楚看到将修改哪些字段
- 避免隐式覆盖 finding 内容

#### 任务 4.3

实现回写完成后的联动刷新。

验收标准：

- 漏洞详情和漏洞列表能看到最新状态

---

## 5. 前端拆分建议

MVP 阶段建议新增的前端文件结构如下：

```text
SecurityCenter/
├── SecurityWorkbenchPage.vue
├── SecurityWorkbenchCaseList.vue
├── SecurityWorkbenchCaseDetail.vue
├── WorkbenchCaseOverviewPanel.vue
├── WorkbenchEvidenceChainPanel.vue
├── WorkbenchVerificationPanel.vue
├── WorkbenchReviewNotesPanel.vue
├── WorkbenchRequestCard.vue
├── WorkbenchResponseDiffCard.vue
├── WorkbenchTimelineRail.vue
├── securityWorkbenchTypes.ts
├── securityWorkbenchPresentation.ts
└── securityWorkbenchCaseSupport.ts
```

约束：

1. 不让任何单文件失控增长
2. 容器负责调度，面板负责展示
3. 通用卡片组件要可复用

---

## 6. 后端任务建议

后端第一阶段建议只做以下 6 个接口：

1. `get_or_create_workbench_case`
2. `list_workbench_cases`
3. `get_workbench_case_detail`
4. `list_workbench_case_evidence`
5. `list_workbench_notes`
6. `create_workbench_note`

第二阶段再补：

7. `list_workbench_verification_runs`
8. `sync_workbench_case_to_finding`

原因：

- 先把“读 + 记 + 关联”闭环打通
- 再接“验证 + 回写”

这样交付风险最低。

---

## 7. MVP 验收标准

MVP 完成后，必须满足以下验收条件：

### 7.1 入口闭环

- 能从 finding 打开或创建 case
- 一个 finding 不会重复创建多个主 case

### 7.2 阅读闭环

- 能在案件详情中稳定看到概览、证据链、复盘记录

### 7.3 记录闭环

- 调查人员能写入复盘备注
- 备注能长期保留并可回看

### 7.4 验证闭环

- 如果已有验证记录，能在工作台中查看

### 7.5 回写闭环

- 能把案件结论回写到 finding

如果以上 5 条不同时成立，则不应视为 MVP 完成。

---

## 8. 风险与控制

### 8.1 风险：工作台与漏洞中心双重维护

控制：

- finding 仍是事实源
- case 只维护调查态数据

### 8.2 风险：页面职责再次混乱

控制：

- 工作台承载过程
- 漏洞中心承载结果

### 8.3 风险：MVP 范围膨胀

控制：

- 第一阶段不做行为链、对象池、自动测试建议

### 8.4 风险：回写覆盖过多 finding 数据

控制：

- 回写必须显式确认
- 只允许更新少数字段

---

## 9. 下一步建议

如果要继续推进实现，建议按这个顺序进入开发：

1. 先做数据类型和 case 基础接口
2. 再加漏洞详情入口和 workbench 路由
3. 再做案件列表页和案件详情容器
4. 再做概览 / 证据链 / 复盘记录
5. 最后接入验证与回写

一句话总结：

`MVP 不追求智能，先把“进入案件、看证据、写结论、回写 finding”这条路径打通。`

---

## 10. 当前实现状态补充

截至当前迭代，MVP 主链路已经从“案件台账”推进到“可执行、可复盘”的状态，已完成：

1. finding 到 case 的创建、打开和回写闭环
2. 对象边界分析、建议生成和 replay plan 生成
3. 执行草案持久化和只读自动执行
4. 执行结果差异比较和建议状态生成
5. 普通回写、按建议回写和案件活动流
6. 活动记录的结构化 `before/after` 状态展示
7. 活动和备注合并后的案件时间线，以及活动类型筛选
8. 时间线搜索，以及从时间线节点跳转到证据、草案、执行结果
9. 案件详情的深链接参数，可直接通过 URL 定位到 tab、证据、草案和执行结果
10. 深链接目标的自动滚动定位，不再只切换 tab 和高亮
11. 复制当前定位链接，支持把当前案件内定位点直接分享出去
12. 案件列表的筛选和分页状态同步到 URL，并在案件详情中保留
13. 时间线节点级别的“复制定位链接”，不用先跳转再复制
14. 时间线节点独立锚点 ID 和 `timelineId` 深链接，备注节点也可精确定位
15. 时间线搜索和筛选同步到 URL，可完整恢复复盘视图
16. 非只读执行草案的人工确认执行入口

当前最适合作为下一阶段入口的方向是：

1. 活动流筛选和时间线聚合
2. 只读执行结果的更强对象差异归因
3. 人工确认后的非只读执行入口
