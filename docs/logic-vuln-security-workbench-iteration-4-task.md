# 安全工作台迭代四任务文档

本轮任务把安全工作台从“建议层”推进到“计划层”，但仍然不直接执行主动验证请求。

关联文档：

- [logic-vuln-ultimate-solution.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-ultimate-solution.md)
- [logic-vuln-security-workbench-implementation.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-implementation.md)
- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-3-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-3-task.md)

---

## 1. 本次目标

把“对象边界建议”进一步整理成白帽可以直接使用的 replay plan。

本轮聚焦：

1. 明确目标请求
2. 明确目标字段
3. 明确候选值
4. 明确执行步骤
5. 明确停止条件
6. 保持只读优先和人工确认边界

---

## 2. 本次范围

### 包含

1. 新增 replay plan support 模块
2. 支持从水平越权建议生成计划对象
3. 支持在工作台详情新增 `Replay Plan` tab
4. 支持复制计划摘要到剪贴板

### 不包含

1. 自动发起 replay 请求
2. 自动批量替换
3. 自动写入主动验证队列
4. 自动生成对象关系图

---

## 3. 交付物

### 前端 support

1. `securityWorkbenchReplayPlan.ts`
2. replay plan 生成
3. replay plan 文本格式化

### 前端面板

1. `WorkbenchReplayPlanPanel.vue`
2. 工作台详情新增 `Replay Plan` tab

---

## 4. 验收标准

1. 工作台详情中可以看到 `Replay Plan` tab
2. 每条计划都能看到：
   - 目标请求
   - 目标字段
   - 候选值
   - 执行步骤
   - 停止条件
3. 计划可以复制成文本摘要
4. 本轮仍然不自动发请求

---

## 5. 当前实现结论

本轮完成后，安全工作台已经具备：

`对象池 -> 建议 -> Replay Plan`

这意味着工作台已经不只是告诉白帽“这里可疑”，而是开始告诉白帽“下一步该怎么测”。

下一轮优先级建议：

1. 把 replay plan 变成可点击的执行草案
2. 接入对象关系图和父子错配计划
3. 再决定是否接真正的主动验证执行
