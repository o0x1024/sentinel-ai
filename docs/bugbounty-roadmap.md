# Bug Bounty 功能实现路线图

> 方案：B（ASM）为主线 + A（工作流模板化）做执行器 + D（证据/报告工程化）做产出

## 当前已实现

- [x] Program/Scope/Finding/Evidence/Submission 基础 CRUD
- [x] Finding 去重指纹（fingerprint）
- [x] 前端 BugBounty 基础面板
- [x] 报告模板面板（本地存储）
- [x] 数据库表结构（bounty_programs/scopes/findings/evidence/submissions/change_events）
- [x] ChangeEvent 模型定义

---

## P2: 变更监控 → 自动触发工作流 ✅

### B1. ChangeEvent 后端服务 ✅
- [x] ChangeEvent DB 操作层（bounty.rs 增加 change_event CRUD）
- [x] ChangeMonitor 服务实现（替换 placeholder）
- [x] bounty_commands 增加 change_event 相关命令
- [x] 变更事件与工作流触发器打通

### B2. ChangeEvent 前端 ✅
- [x] ChangeEventsPanel 变更事件列表页
- [x] ChangeEventDetailModal 详情弹窗
- [x] 变更事件与工作流运行结果聚合展示

---

## P1: ASM 攻击面管理 ✅

### B3. 资产归并与规范化 ✅
- [x] Bounty Asset 模型（与现有 Asset 打通或扩展）
- [x] URL canonicalization 工具函数
- [x] Scope → Asset 自动关联
- [x] 资产去重与归并逻辑

### B4. 指纹与标签体系 ✅
- [x] 高价值面标签定义（admin/upload/export/api 等）
- [x] 指纹规则引擎（可热更新）
- [x] 技术栈识别标签

### B5. 优先级评分 ✅
- [x] 资产/Scope 优先级评分算法
- [x] TopN 高价值队列展示
- [x] 基于标签+变化频率+历史产出的综合评分

---

## A: 工作流模板化（执行器） ✅

### A1. Bounty 工作流模板库 ✅
- [x] 内置 5-8 个高频模板（子域→存活→指纹→目录→漏洞探测）
- [x] 模板与 Program/Scope 绑定
- [x] 模板一键执行

### A2. 步骤级产物沉淀 ✅
- [x] Workflow 步骤输出自动入库
- [x] 步骤产物 → Finding/Evidence 转换
- [x] 工作流运行结果与 Bounty 数据关联

### A3. 变更触发工作流 ✅
- [x] ChangeEvent → Workflow 自动触发
- [x] 触发条件配置（事件类型/严重程度/标签匹配）
- [x] 触发历史记录

---

## D: 证据/报告工程化（产出） ✅

### D1. 自动证据采集 ✅
- [x] Traffic 插件 → Evidence 自动生成
- [x] Workflow 执行 → Evidence 自动生成
- [x] 截图/请求响应自动归档

### D2. 一键导出提交包 ✅
- [x] Finding → 提交报告模板渲染
- [x] 中英双语报告生成
- [x] Evidence 打包导出（ZIP）

### D3. 提交/复测运营 ✅
- [x] 提交沟通时间线（communications）
- [x] 复测提醒（到期自动提醒）
- [x] 一键重跑验证工作流

---

## 实施顺序

1. **P2-B1**: ChangeEvent 后端服务（当前）
2. **P2-B2**: ChangeEvent 前端
3. **D1**: 自动证据采集
4. **D2**: 一键导出提交包
5. **A1**: Bounty 工作流模板库
6. **A2**: 步骤级产物沉淀
7. **A3**: 变更触发工作流
8. **P1-B3**: 资产归并与规范化
9. **P1-B4**: 指纹与标签体系
10. **P1-B5**: 优先级评分
11. **D3**: 提交/复测运营

---

## P0: 数据流编排与产物协议 ✅ (2026-01-22)

### P0-1. 统一工作流产物协议 ✅
- [x] ArtifactType 枚举（Finding/Evidence/Asset/Subdomains/LiveHosts/Technologies/Endpoints/Secrets/Directories/RawData）
- [x] WorkflowArtifact 标准结构（id/step_id/execution_id/type/data/metadata）
- [x] ArtifactExtractor 自动类型检测与提取
- [x] 各类产物的标准化数据结构（FindingArtifact/SubdomainsArtifact/LiveHostsArtifact 等）

### P0-2. 数据流编排 ✅
- [x] PluginPortRegistry 插件端口注册表
- [x] DataFlowResolver 上下游数据自动传递
- [x] 支持 JSON Path 提取（如 `subdomains[*].subdomain`）
- [x] 自动填充下游步骤的输入参数

### P0-3. 失败重试与限流 ✅
- [x] RetryConfig 统一重试配置
- [x] BackoffStrategy 支持 Fixed/Linear/Exponential
- [x] RateLimiter 全局+每主机并发控制
- [x] RetryExecutor 统一执行器

### P0-4. 编排集成 ✅
- [x] WorkflowOrchestrator 编排器
- [x] bounty_resolve_step_inputs 命令
- [x] bounty_process_step_output 命令
- [x] bounty_sink_artifacts 命令（自动落库）
- [x] bounty_get_plugin_ports / bounty_list_plugin_ports 命令

---

## P1: 前端编排可视化 ✅ (2026-01-22)

### P1-1. 插件端口可视化 ✅
- [x] WorkflowStepDetail 组件 - 步骤详情展示
- [x] 输入端口展示（参数名/期望类型/必填标识）
- [x] 输出端口展示（端口名/产物类型）
- [x] 端口类型颜色编码（不同产物类型不同颜色）

### P1-2. 数据流连线可视化 ✅
- [x] WorkflowDataFlow 组件 - 数据流图
- [x] 步骤分层布局（按依赖关系自动分层）
- [x] 上下游连接指示器
- [x] 数据传递路径展示（JSON Path 提取）

### P1-3. 产物预览面板 ✅
- [x] 产物类型检测与图标
- [x] Subdomains 列表预览
- [x] Live Hosts 状态码展示
- [x] Finding 严重级别展示
- [x] 步骤配置折叠展示

### P1-4. 执行状态实时展示 ✅
- [x] WorkflowExecutionPanel 组件
- [x] 进度条与步骤计数
- [x] 限流器状态（全局/单主机并发）
- [x] 重试状态统计
- [x] 产物汇总统计
- [x] 执行日志流

---

## 进度追踪

| 模块 | 状态 | 开始时间 | 完成时间 |
|------|------|----------|----------|
| P2-B1 ChangeEvent 后端 | ✅ 完成 | 2026-01-21 | 2026-01-21 |
| P2-B2 ChangeEvent 前端 | ✅ 完成 | 2026-01-21 | 2026-01-21 |
| D1 自动证据采集 | ✅ 完成 | 2026-01-21 | 2026-01-21 |
| D2 一键导出提交包 | ✅ 完成 | 2026-01-21 | 2026-01-21 |
| A1 工作流模板库 | ✅ 完成 | 2026-01-21 | 2026-01-21 |
| A2 步骤级产物沉淀 | ✅ 完成 | 2026-01-21 | 2026-01-21 |
| A3 变更触发工作流 | ✅ 完成 | 2026-01-21 | 2026-01-21 |
| P1-B3 资产归并 | ✅ 完成 | 2026-01-21 | 2026-01-21 |
| P1-B4 指纹标签 | ✅ 完成 | 2026-01-21 | 2026-01-21 |
| P1-B5 优先级评分 | ✅ 完成 | 2026-01-21 | 2026-01-21 |
| D3 提交复测运营 | ✅ 完成 | 2026-01-21 | 2026-01-21 |
| P0-1 产物协议 | ✅ 完成 | 2026-01-22 | 2026-01-22 |
| P0-2 数据流编排 | ✅ 完成 | 2026-01-22 | 2026-01-22 |
| P0-3 重试限流 | ✅ 完成 | 2026-01-22 | 2026-01-22 |
| P0-4 编排集成 | ✅ 完成 | 2026-01-22 | 2026-01-22 |
| P1-1 端口可视化 | ✅ 完成 | 2026-01-22 | 2026-01-22 |
| P1-2 数据流连线 | ✅ 完成 | 2026-01-22 | 2026-01-22 |
| P1-3 产物预览 | ✅ 完成 | 2026-01-22 | 2026-01-22 |
| P1-4 执行状态 | ✅ 完成 | 2026-01-22 | 2026-01-22 |
