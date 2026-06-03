# 安全工作台迭代 8 任务单

本文档记录把“建议状态 / 建议结论”从前端推断层下沉到后端统一评估层的这一轮实现。

关联文档：

- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-7-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-7-task.md)

---

## 1. 本轮目标

上一轮已经实现了：

- 执行结果建议
- 建议状态
- 建议结论
- 应用到草稿 / 应用并保存

但当时这层逻辑完全在前端：

- 前端自己汇总 execution runs
- 前端自己推断 suggested status
- 前端自己生成 suggested conclusion

这会带来两个问题：

1. 前后端规则容易分叉
2. 后续如果其它页面也要消费建议结果，就会重复实现

因此本轮目标只有一个：

`把建议层下沉到后端，作为 case detail 的统一输出。`

---

## 2. 本轮实现

### 2.1 后端新增统一建议输出

`security_workbench_get_case_detail` 现在除了返回：

- `caseItem`
- `notes`
- `executionDrafts`
- `executionRuns`

还会额外返回：

- `assessmentSuggestion`

它由后端统一基于 execution runs 推断得到。

### 2.2 统一建议字段

建议结果统一包含：

- `title`
- `summary`
- `suggestedStatus`
- `suggestedConclusion`
- `confidence`
- `signals`

### 2.3 前端切换为直接消费后端结果

案件概览和验证面板都不再自己推断建议，而是直接展示后端给出的：

- 建议状态
- 建议结论
- 建议置信度
- 建议支撑信号

---

## 3. 当前收益

这轮做完之后，建议层已经具备统一出口：

1. 后续其它页面可以直接复用
2. `sync to finding` 前可以直接消费统一建议
3. 不再存在前端和后端两套状态建议逻辑

---

## 4. 下一轮建议

如果继续推进，下一轮最值得做的是：

1. 在 `sync to finding` 前增加建议确认
2. 允许用户“按建议回写”
3. 在后端把建议结果和 case 状态流转进一步联动
