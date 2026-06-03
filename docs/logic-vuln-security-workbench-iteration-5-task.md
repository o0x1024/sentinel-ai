# 安全工作台迭代五任务文档

本轮任务把安全工作台从“Replay Plan”推进到“执行草案层”，但仍然不直接发起真实 replay。

关联文档：

- [logic-vuln-ultimate-solution.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-ultimate-solution.md)
- [logic-vuln-security-workbench-implementation.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-implementation.md)
- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-4-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-4-task.md)

---

## 1. 本次目标

把 Replay Plan 变成真正可追踪的执行草案对象，并挂到 case 下管理。

本轮聚焦：

1. 从 plan 保存成 draft
2. draft 落到后端持久化
3. draft 状态可维护
4. draft 摘要可复制
5. case detail 能直接看到所有 draft

---

## 2. 本次范围

### 包含

1. 新增执行草案数据结构
2. 新增后端 execution draft 持久化
3. 新增创建 execution draft 命令
4. 新增更新 execution draft 状态命令
5. case detail 返回 execution drafts
6. 工作台详情新增 `执行草案` tab
7. Replay Plan 支持“保存为执行草案”

### 不包含

1. 自动发起 replay 请求
2. 自动批量执行
3. 自动验证队列调度
4. 执行结果自动写回验证证据

---

## 3. 交付物

### 后端

1. execution draft 持久化支持
2. `security_workbench_create_execution_draft`
3. `security_workbench_update_execution_draft`
4. case detail 返回 drafts

### 前端

1. Replay Plan -> 执行草案
2. `执行草案` tab
3. 草案状态切换
4. 草案摘要复制

---

## 4. 验收标准

1. 用户可把 Replay Plan 保存成执行草案
2. 刷新页面后执行草案仍然存在
3. 工作台详情中可以查看所有执行草案
4. 可修改执行草案状态
5. 仍然不会自动发请求

---

## 5. 当前实现结论

本轮完成后，安全工作台已经具备：

`对象池 -> 建议 -> Replay Plan -> 执行草案`

也就是说，工作台已经从“给建议”进一步走到了“形成可追踪执行对象”的阶段。

下一轮优先级建议：

1. 把执行草案变成可点击的只读 replay 执行
2. 接入执行结果摘要和对比
3. 再决定是否接真正的主动验证链路
