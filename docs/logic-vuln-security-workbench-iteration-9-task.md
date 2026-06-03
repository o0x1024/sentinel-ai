# 安全工作台迭代 9 任务单

本文档记录“按建议回写”这一轮实现，也就是把后端统一建议结果真正接入案件状态流转。

关联文档：

- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-8-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-8-task.md)

---

## 1. 本轮目标

上一轮已经完成：

- 后端统一输出 `assessmentSuggestion`
- 前端概览和验证面板统一消费后端建议

但状态流转仍然分成两条：

1. 手工改案件状态后回写
2. 看建议但不真正把建议接入回写动作

因此本轮目标是：

`让工作台支持“按建议回写”，并在回写前确认是否先把建议状态和建议结论应用到案件。`

---

## 2. 本轮实现

### 2.1 新增独立入口

案件概览中的建议卡片现在有单独入口：

- `按建议回写`

它不替代原有的 `回写到漏洞`，而是一个更安全的增强流。

### 2.2 回写前确认

前端会在执行“按建议回写”前弹出确认框，展示：

- 建议状态
- 建议摘要
- 回写动作说明

### 2.3 后端支持可选地先应用建议

`security_workbench_sync_case_to_finding` 现在支持：

- `applySuggestionToCase`

当该参数为 `true` 时：

1. 后端先读取统一建议结果
2. 把建议状态和建议结论应用到案件
3. 再按更新后的案件状态同步到漏洞状态

这样保证：

- 建议层与回写层使用同一套后端规则
- 不会再出现前端自己拼状态、后端自己映射状态的分叉

---

## 3. 当前收益

这轮做完之后，工作台已经具备两条明确回写路径：

1. 普通回写
   - 保持人工当前案件状态

2. 按建议回写
   - 先应用后端建议
   - 再同步漏洞状态

这让工作台真正从“建议展示”走到了“建议参与状态流转”。

---

## 4. 下一轮建议

如果继续推进，下一轮最值得做的是：

1. 在回写成功后把“采用了哪条建议”记成案件活动记录
2. 增加“建议误报回写”和“建议继续补样本”的更细粒度动作
3. 把建议使用历史纳入案件复盘信息
