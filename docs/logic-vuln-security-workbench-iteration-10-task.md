# 安全工作台迭代 10 任务单

本文档记录把“采用了哪条建议回写”变成案件活动记录的这一轮实现。

关联文档：

- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-9-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-9-task.md)

---

## 1. 本轮目标

上一轮已经支持：

- 普通回写
- 按建议回写

但这些关键动作还没有被记录成结构化案件活动，因此虽然能执行，复盘时仍然缺一条明确的追溯链。

因此本轮目标是：

`为工作台增加独立活动流，记录执行草案和漏洞回写动作。`

---

## 2. 本轮实现

### 2.1 新增活动流数据

后端新增独立活动存储，不与人工备注混用。

当前支持的活动类型：

- `draft_execution`
- `finding_sync`
- `suggestion_sync`

### 2.2 活动记录触发点

当前会在下面几个动作发生时自动写入活动：

1. 执行只读草案
2. 普通回写漏洞状态
3. 按建议回写漏洞状态

### 2.3 Review 面板展示

复盘记录页现在会优先展示活动流，再展示人工备注。

这样能直接看出：

- 什么时间执行了哪条草案
- 什么时间回写了漏洞
- 是否采用了建议回写

---

## 3. 当前收益

这一轮做完后，安全工作台的可追溯性明显增强：

1. 系统行为和人工备注分层
2. 关键动作有独立时间线
3. “采用了哪条建议回写”不再只能靠人工记忆

---

## 4. 当前限制

这一轮活动流仍然是轻量版：

1. 只记录关键摘要
2. 不记录完整 before/after 快照
3. 还没有做活动筛选
4. 还没有把活动单独拆成独立 tab

---

## 5. 下一轮建议

如果继续推进，下一轮最值得做的是：

1. 为活动记录增加结构化 before/after 字段
2. 支持按活动类型筛选
3. 把活动流和案件时间线进一步合并
