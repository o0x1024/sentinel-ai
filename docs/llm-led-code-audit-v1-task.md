# LLM主导代码审计方案（V1）任务清单

更新日期：2026-02-24

## 1. 目标与范围

V1 目标是把“LLM 主导 + 工具验证 + 生命周期治理”落地为可运行闭环：

1. LLM 可主导审计过程与子任务分解（Subagent）。
2. 审计发现可结构化持久化，并进入生命周期状态机。
3. 审计结论可由 Judge/Verifier 复核并回写生命周期。
4. 前端可筛选、查看、流转审计发现。
5. 生命周期操作受显式权限能力控制（Capability）。

## 2. 任务看板（V1）

### 2.1 引擎与执行编排

- [x] 审计模式上下文下发（`audit_mode` / `verification_level`）到执行器与子代理。
- [x] 子代理角色化策略（planner/access/auth/state/payment/judge/reviewer）。
- [x] 子代理工具白名单按角色约束。
- [x] 审计场景迭代上限按验证级别动态调整（low/medium/high）。
- [x] 子代理递归深度保护（防止无限委派）。

### 2.2 Judge 输出规范化

- [x] Judge 输出结构化 schema 校验（含 `verdict/confidence/evidence_refs/rationale`）。
- [x] 非法输出自动降级为 `uncertain`。
- [x] “无证据却 confirmed/probable”自动降级。

### 2.3 审计发现数据模型与生命周期

- [x] `agent_audit_findings` 扩展生命周期字段：
  `lifecycle_stage`、`verification_status`、`required_evidence_json`、`verifier_json`、`judge_json`、`provenance_json`、`last_transition_at`。
- [x] 生命周期索引与兼容迁移（PG/SQLite/MySQL 兼容路径）。
- [x] 查询过滤支持 `lifecycle_stage`。
- [x] 新增生命周期流转命令：`transition_agent_audit_finding_lifecycle`。

### 2.4 Upsert与状态机约束

- [x] `audit_finding_upsert` 支持生命周期/验证/judge/verifier/provenance 字段。
- [x] upsert 时执行生命周期合法流转校验。
- [x] `status -> lifecycle` 推导映射落地。

### 2.5 Judge 回写闭环

- [x] Judge/Reviewer 子代理完成后，解析输出并尝试回写对应 finding 生命周期。
- [x] 通过 `conversation_id + finding_id` 定位记录，执行受状态机约束的迁移。

### 2.6 前端可用性

- [x] 审计发现列表支持生命周期筛选。
- [x] 列表展示生命周期列。
- [x] 详情展示验证状态、required evidence、judge/verifier 上下文。
- [x] 前端支持生命周期流转操作并回刷列表。

### 2.7 权限治理（Capability）

- [x] 角色模型新增显式能力字段：`capabilities`（持久化为 `capabilities_json`）。
- [x] 角色创建/更新/查询全链路透传能力字段。
- [x] 生命周期操作改为显式能力判定：
  `audit.lifecycle.transition`（`is_system` 仍可放行）。
- [x] 前端保留紧急本地开关覆盖：
  `localStorage['security:auditLifecycleTransitionEnabled']='true'`。

### 2.8 测试与文档

- [x] 生命周期状态机与规范化逻辑单测（Rust）已补齐。
- [x] 生命周期权限判定单测（Vitest）已补齐。
- [x] 生命周期调用链集成测试（Vitest）已补齐。
- [x] 审计技能文档已补充生命周期字段与流转示例。

### 2.9 工具可用性（read_file）

- [x] `read_file` 支持“文件读取 + 目录列表”统一入口（目录返回 `entries`）。
- [x] Docker 模式下增加路径存在性预检查，避免直接报 `sed: can't read`。
- [x] 路径不存在时返回可操作诊断：`/workspace` 顶层目录提示 + 同名文件候选 + 下一步建议。
- [x] CPG 四件套工具已接入 `ToolServer` 注册链路（`build_cpg` / `query_cpg` / `cpg_taint_analysis` / `cpg_security_scan`）。
- [x] 审计模式新增强制工具可用性预检：强制工具未注册时直接失败，避免静默降级。
- [x] 前端强制工具列表移除过时项 `taint_slice_lite`，与后端能力保持一致。

## 3. 本轮已完成模块清单

### 3.1 后端（Rust）

- `src-tauri/src/agents/subagent_executor.rs`
- `src-tauri/src/agents/executor/mod.rs`
- `src-tauri/src/agents/context_engineering/policy.rs`
- `src-tauri/src/agents/executor/run_with_tools.rs`
- `src-tauri/src/commands/code_audit_commands.rs`
- `src-tauri/src/commands/role.rs`
- `src-tauri/src/services/http_gateway.rs`
- `src-tauri/sentinel-db/src/database_service/{service.rs,traffic.rs,ai.rs,init.rs}`
- `src-tauri/sentinel-core/src/models/ai.rs`

### 3.2 前端（Vue/TS）

- `src/components/SecurityCenter/CodeAuditFindingsPanel.vue`
- `src/components/RoleManagement.vue`
- `src/composables/useRoleManagement.ts`
- `src/types/role.ts`
- `src/utils/auditLifecycleAccess.ts`

### 3.3 测试

- `src-tauri/src/commands/code_audit_commands.rs`（内置 tests）
- `src/tests/unit/security/audit-lifecycle-access.test.ts`
- `src/tests/integration/code-audit-lifecycle-flow.test.ts`

### 3.4 审计技能文档

- `src-tauri/skills/cpg-code-audit/SKILL.md`

## 4. V1 当前结论

V1 已达到“可运行闭环”状态：  
LLM 主导审计 -> 结构化落库 -> 生命周期状态机 -> Judge/Verifier 回写 -> 前端可视化与可控流转 -> 能力权限约束。

## 5. V1.1 建议任务（下一阶段）

- [x] 将默认安全角色初始化时显式写入 `audit.lifecycle.transition`（弱化对 `is_system` 兜底依赖）。
- [x] 增加端到端/流程用例：覆盖 `candidate -> confirmed -> archived` 生命周期路径校验。
- [x] 补充前端角色管理 UI 的能力模板选择（而非纯文本输入）。
- [x] 加入审计质量门禁指标（证据率、uncertain占比、误报回退率），并接入安全中心面板与 `audit_report` 输出。
- [x] 质量门禁阈值可配置化（`save/get_agent_audit_quality_gate_thresholds` + 前端阈值设置）。
- [x] 阈值分层策略：全局默认 + 会话覆盖 + 运行时(CI)覆盖优先级。
  优先级: `runtime_override > conversation_override > global_config > builtin_default`
