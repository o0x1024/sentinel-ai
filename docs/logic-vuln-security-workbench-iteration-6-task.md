# 安全工作台迭代 6 任务单

本文档记录安全工作台从“执行草案层”推进到“只读执行层”的当前实现。

关联文档：

- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-5-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-5-task.md)

---

## 1. 本轮目标

上一轮已经完成：

- 对象边界分析
- Replay Plan
- 执行草案持久化

但还停留在“建议和草案”层，没有真正形成工作台内部的执行闭环。

因此本轮只做一件事：

`把只读型执行草案落成可点击执行的工作台运行记录，并把结果回流到验证面板。`

---

## 2. 本轮范围

### 2.1 包含

1. 为工作台增加 `ExecutionRun` 数据结构
2. 后端持久化执行结果
3. 只允许执行 `readOnly=true` 且 `GET/HEAD` 的草案
4. 基于目标字段和值生成有限变异请求
5. 保存每次候选值尝试的：
   - 目标 URL
   - 状态码
   - 响应摘要
   - 结果分类
6. 在工作台 `验证与对比` 面板展示执行结果

### 2.2 不包含

1. 非只读执行
2. 并发执行
3. 批量枚举
4. 自动写入正式漏洞 evidence
5. 复杂 body 级变异执行

---

## 3. 实现结果

本轮完成后，工作台链路已经变成：

`对象池 -> 建议 -> Replay Plan -> 执行草案 -> 只读执行记录`

具体包括：

1. 后端新增执行结果持久化键：
   - `security_workbench_execution_runs_v1`
2. case detail 现在会同时返回：
   - `executionDrafts`
   - `executionRuns`
3. 工作台支持直接执行只读草案
4. 执行结果会按候选值拆成 attempt 记录
5. 验证面板会优先展示工作台执行记录，再展示已有 system agent 验证 evidence
6. attempt 结果带结构化差异信号：
   - 状态码是否变化
   - 响应长度变化
   - 相似度等级
   - JSON 字段变化路径
7. 只读执行已支持：
   - path 参数替换
   - query 参数替换
   - JSON body 字段替换
   - 表单 body 字段替换

---

## 4. 当前限制

这一轮仍然是“安全可控优先”，所以明确保留了限制：

1. 当前自动执行限制为 `GET/HEAD/POST`
2. 只支持 `path.*`、`query.*`、`body.*` 三类字段替换
3. 只执行最多 5 个候选值
4. 只保存响应摘要，不做完整响应持久化
5. 非只读草案会被直接阻断，不自动执行

---

## 5. 下一轮建议

如果继续推进，下一轮最值得做的是：

1. 为只读执行结果增加更明确的差异比较
2. 支持对 `body.*` 的安全只读变异
3. 把执行结果和 finding 状态建议挂钩
4. 再决定是否引入需要人工确认的非只读执行入口
