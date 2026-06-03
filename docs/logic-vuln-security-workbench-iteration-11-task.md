# 安全工作台迭代 11 任务单

本文档记录把案件活动从“摘要记录”推进到“结构化前后态”的这一轮实现。

关联文档：

- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-10-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-10-task.md)

---

## 1. 本轮目标

上一轮已经把下面几类关键动作记录成案件活动：

- 执行只读草案
- 普通回写漏洞状态
- 按建议回写漏洞状态

但活动仍然只有摘要文字，复盘时缺少结构化的前后态。

因此本轮目标是：

`为案件活动增加 before/after 结构化快照，并在复盘页直接展示。`

---

## 2. 本轮实现

### 2.1 活动模型扩展

前后端活动模型都新增了可选的：

- `before`
- `after`

字段类型保持轻量，直接承载结构化 JSON 快照，不强行抽更重的固定 schema。

### 2.2 执行草案活动补充前后态

`draft_execution` 现在会记录：

前：

- 草案状态
- 目标方法
- 目标字段
- 候选值数量
- 是否只读

后：

- 执行状态
- 尝试次数
- `changed / blocked / same / not_found / error` 计数

这样复盘时可以直接看到某次执行前打算做什么、执行后实际观察到了什么。

### 2.3 回写活动补充前后态

`finding_sync` 和 `suggestion_sync` 现在会记录：

前：

- 案件状态
- 案件结论
- 漏洞状态

后：

- 案件状态
- 案件结论
- 漏洞状态
- 本次回写模式

这样可以稳定追溯一次回写到底改了哪些状态。

### 2.4 Review 面板展示

复盘页里的活动卡片现在支持直接展示：

- `前`
- `后`

两个结构化状态块，不再只是一段摘要。

---

## 3. 当前收益

这一轮完成后，安全工作台的可复盘性明显增强：

1. 执行动作不再只是“执行过”，而是能看见执行前后态
2. 漏洞回写不再只是“同步过”，而是能看见状态怎么变的
3. 系统关键动作具备了更完整的调查证据链

---

## 4. 当前限制

这一轮仍然保持轻量：

1. `before/after` 还是扁平快照，不做更深层差异树
2. 还没有活动筛选
3. 还没有把活动和完整案件时间线合并

---

## 5. 下一轮建议

如果继续推进，下一轮最值得做的是：

1. 给活动流增加类型筛选
2. 把活动和备注统一成案件时间线视图
3. 对执行活动补更强的对象差异摘要
