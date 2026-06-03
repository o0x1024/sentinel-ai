# 安全工作台迭代二任务文档

本任务文档描述从“前端本地骨架”推进到“后端持久化 MVP 闭环”的这一轮实施。

关联文档：

- [logic-vuln-ultimate-solution.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-ultimate-solution.md)
- [logic-vuln-security-workbench-implementation.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-implementation.md)
- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-1-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-1-task.md)

---

## 1. 本次目标

把安全工作台从“前端本地存储的演示型骨架”，推进成“真正可用的 MVP 工作流”。

本次重点不再是页面是否存在，而是闭环是否成立：

1. case 和 note 要落到后端持久化
2. 从 finding 进入工作台时，要能稳定拿到数据库里的完整 finding 快照
3. 工作台内的结论和状态，要能显式回写到 finding
4. 工作台列表要具备基本搜索、状态筛选和分页
5. 工作台要有稳定路径，而不只是 query tab

---

## 2. 本次范围

### 包含

1. 新增后端 `security_workbench_commands`
2. 基于现有 `proxy_config` 的 case / note 持久化
3. `get_or_create_case(finding_id)` 统一从数据库重建 finding 快照
4. 工作台详情接口
5. 工作台案件更新接口
6. 工作台备注追加接口
7. 工作台状态回写到 finding 状态
8. 前端 support 层从 localStorage 切换到 Tauri invoke
9. 工作台列表搜索、状态筛选、分页
10. `/security-center/workbench/:caseId?` 稳定路径

### 不包含

1. 独立 case 数据表
2. 行为链、对象池、对象关系图
3. 自动测试计划
4. 主动验证发起
5. 团队协作评论

---

## 3. 交付物

### 后端

1. `security_workbench_list_cases`
2. `security_workbench_get_or_create_case_for_finding`
3. `security_workbench_get_case_detail`
4. `security_workbench_update_case`
5. `security_workbench_add_note`
6. `security_workbench_sync_case_to_finding`

### 前端

1. 工作台支持后端加载案件列表
2. 工作台支持后端加载案件详情
3. 工作台支持保存案件信息和结论
4. 工作台支持新增备注
5. 工作台支持显式回写 finding
6. 工作台支持列表筛选和分页
7. 工作台支持独立路径进入

---

## 4. 任务拆解

### 任务 A：后端持久化

1. 新建独立命令文件，不把工作台逻辑继续塞进原有大文件
2. 复用现有 `proxy_config` 做 MVP 持久化
3. 支持 case 和 note 的独立读写

### 任务 B：finding 快照统一化

1. 从工作台入口只传 `finding_id`
2. 后端统一查询漏洞和 evidence
3. 避免前端列表项 evidence 不全导致的案件数据缺失

### 任务 C：工作台列表闭环

1. 列表查询
2. 搜索
3. 状态筛选
4. 分页

### 任务 D：工作台详情闭环

1. 加载案件详情
2. 保存案件元信息
3. 保存结论
4. 更新基线证据
5. 追加复盘备注

### 任务 E：finding 回写

1. 建立案件状态到 finding 状态的映射
2. 提供显式“回写到漏洞”动作
3. 回写后刷新漏洞中心

### 任务 F：稳定路径

1. 新增 `/security-center/workbench/:caseId?`
2. 支持直接打开案件详情
3. 支持从工作台回到案件列表

---

## 5. 验收标准

1. 从漏洞列表或漏洞详情进入工作台时，能稳定打开同一个 case
2. case 数据刷新后不会丢失 finding 快照
3. 工作台案件列表支持搜索、状态筛选和分页
4. 工作台内保存的结论、状态、备注会持久化到后端
5. 用户可以在工作台中显式把案件状态回写到 finding 状态
6. 刷新页面后，仍可通过稳定路径重新打开案件

---

## 6. 当前实现结论

本轮完成后，安全工作台已经不再是“只在前端本地玩的临时面板”，而是具备了最小可用的调查闭环：

`漏洞中心 -> 打开案件 -> 查看证据和验证记录 -> 记录结论 -> 回写漏洞状态`

下一轮如果继续推进，优先级应切到：

1. 行为链和对象池
2. 水平越权对象替换建议
3. 主动验证计划和执行入口
