# 安全工作台迭代 7 任务单

本文档记录安全工作台把“执行结果”进一步推进到“建议状态 / 建议结论”的这一轮实现。

关联文档：

- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-6-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-6-task.md)

---

## 1. 本轮目标

上一轮已经实现：

- 对象池
- Replay Plan
- 执行草案
- 只读执行结果
- 结构化差异信号

但案件状态和案件结论仍然完全靠人工手填，执行结果和案件判断之间没有闭环。

因此本轮只做一件事：

`根据工作台执行结果自动生成建议状态和建议结论，并允许直接应用到案件草稿。`

---

## 2. 本轮实现

### 2.1 新增建议推断层

新增了独立的前端推断模块：

- `securityWorkbenchAssessment.ts`

它会根据工作台执行结果聚合：

- `changed`
- `blocked`
- `same`
- `not_found`
- `error`

并结合差异信号，给出：

- 建议标题
- 建议摘要
- 建议状态
- 建议结论
- 建议置信度
- 关键支撑信号

### 2.2 案件概览联动

案件概览现在会直接展示“执行结果建议”卡片，并提供两个动作：

1. 应用到草稿
2. 应用并保存

### 2.3 验证面板联动

验证面板顶部也会展示同一份建议摘要，方便在看自动执行结果时立即判断：

- 当前更像待验证
- 还是继续调查
- 还是结果偏弱

---

## 3. 当前推断规则

当前规则故意保持保守：

1. 如果观察到 `changed`
   - 建议状态：`awaiting_verification`
   - 含义：已看到对象边界变化，但仍建议人工确认

2. 如果全部 `blocked`
   - 建议状态：`investigating`
   - 含义：边界当前看起来正常，但不直接当成误报

3. 如果主要是 `same`
   - 建议状态：`investigating`
   - 含义：未看到明显变化，继续补样本

4. 如果结果很弱或没有 attempt
   - 建议状态：维持 `investigating`
   - 含义：当前不建议推进状态

---

## 4. 当前限制

这一层目前仍然是“前端建议层”，还没有做成后端统一策略：

1. 建议只用于工作台
2. 不自动改 finding
3. 不自动推进 case 状态
4. 仍然需要人工点击应用或保存

---

## 5. 下一轮建议

如果继续推进，下一轮最值得做的是：

1. 把建议层下沉成后端统一评估逻辑
2. 让建议结果参与 `sync to finding` 前的状态确认
3. 增加“建议误报”与“建议继续补样本”的更细粒度分支
