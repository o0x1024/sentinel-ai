# 安全工作台迭代 15 任务单

本文档记录把工作台深链接从“能打开目标面板”推进到“自动滚动到目标卡片”的这一轮实现。

关联文档：

- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-14-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-14-task.md)

---

## 1. 本轮目标

上一轮已经支持：

- `workbenchTab`
- `evidenceId`
- `draftId`
- `runId`

这些 URL 深链接参数。

但打开链接后仍然只是：

- 切到正确 tab
- 给目标卡片打高亮

如果列表很长，用户仍然需要自己往下找。

因此本轮目标是：

`为深链接目标增加自动滚动锚点。`

---

## 2. 本轮实现

### 2.1 证据链自动滚动

证据链面板现在会在收到 `selectedEvidenceId` 后：

- 高亮目标证据
- 自动滚动到目标证据卡片

### 2.2 执行草案自动滚动

执行草案面板现在会在收到 `selectedDraftId` 后：

- 高亮目标草案
- 自动滚动到目标草案卡片

### 2.3 执行结果自动滚动

验证面板现在会在收到 `selectedRunId` 后：

- 高亮目标执行结果
- 自动滚动到目标执行结果卡片

### 2.4 与现有深链接状态对齐

本轮没有新增新的 URL 参数，而是直接消费上一轮已经建立的：

- `evidenceId`
- `draftId`
- `runId`

因此整体导航模型没有变复杂，只是把最终落点补完整了。

---

## 3. 当前收益

这一轮完成后，工作台深链接体验明显更完整：

1. 打开链接后不只是切 tab，而是能直接看到目标卡片
2. 时间线跳转、URL 定位和界面最终落点三者一致
3. 复盘和分享案件时，不再需要人工再找一次目标记录

---

## 4. 当前限制

这一轮仍然保持轻量：

1. 还没有加入滚动完成提示
2. 还没有对备注节点做滚动锚点
3. 还没有支持浏览器返回前进时的滚动恢复优化

---

## 5. 下一轮建议

如果继续推进，下一轮最值得做的是：

1. 给案件时间线节点本身补深链接锚点
2. 支持复制“当前定位链接”
3. 把案件列表筛选和分页状态也同步到 URL
