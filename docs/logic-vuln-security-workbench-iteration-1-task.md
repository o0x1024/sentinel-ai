# 安全工作台迭代一任务文档

本任务文档只描述当前实施批次，不重复方案总览。

关联文档：

- [logic-vuln-ultimate-solution.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-ultimate-solution.md)
- [logic-vuln-security-workbench-implementation.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-implementation.md)
- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)

---

## 1. 本次目标

把“安全工作台”从文档概念推进到可点击、可进入、可查看、可记录的前端 MVP 骨架。

本次不追求完整后端持久化，也不追求自动验证全闭环。

---

## 2. 本次范围

### 包含

1. 在安全中心增加 `安全工作台` 入口
2. 支持从漏洞详情进入工作台
3. 建立基于本地存储的 `finding -> case` 绑定
4. 实现工作台案件列表页
5. 实现工作台案件详情页骨架
6. 实现四个基础面板
   - 案件概览
   - 请求与证据链
   - 验证与对比
   - 复盘记录

### 不包含

1. 后端数据库持久化
2. 自动测试计划
3. 行为链 / 对象池 / 对象关系图
4. finding 状态正式回写
5. 团队协作与评论

---

## 3. 交付物

### 文档

- 当前任务文档

### 前端

- 工作台页面容器
- 案件列表
- 案件详情
- 本地存储支持层
- 基础展示与编辑组件

---

## 4. 任务拆解

### 任务 A：Security Center 接入

- 新增 `安全工作台` tab
- 支持 URL query 切换到 workbench

### 任务 B：本地 case 模型

- 定义 workbench case 类型
- 定义 note 类型
- 实现 localStorage 读写
- 实现 `getOrCreateCaseForFinding`

### 任务 C：案件列表

- 展示 case 列表
- 支持打开案件详情

### 任务 D：案件详情骨架

- 案件概览
- 请求与证据链
- 验证与对比
- 复盘记录

### 任务 E：漏洞详情入口

- 从漏洞详情中创建/打开案件
- 跳转到工作台

---

## 5. 验收标准

1. 用户能从漏洞详情进入安全工作台
2. 同一个 finding 再次进入时会打开同一个 case
3. 工作台列表能展示已有案件
4. 工作台详情能看到 finding 快照和 evidence
5. 用户能新增复盘备注并持久化到本地

---

## 6. 当前取舍

本轮选择前端本地存储而不是直接接后端，原因：

1. 先验证页面结构和交互闭环
2. 降低第一轮实现复杂度
3. 后续可以平滑替换为后端 case 存储

一句话总结：

`这次先把工作台“做出来并用起来”，下一轮再把它“做深并持久化”。`
