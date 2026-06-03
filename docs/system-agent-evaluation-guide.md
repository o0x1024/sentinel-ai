# System Agent 逻辑漏洞最小评测基线

这份文档用于给当前 `traffic_logic_triage` / `traffic_active_verifier` 提供一套可重复的最小评测基线。

目标不是自动给出最终检出率，而是先稳定产出：

- 基线样本
- 候选样本
- 场景请求序列
- 预期标签

然后再和安全中心中的结果逐条对照。

## 1. 前置条件

先启动本地业务服务：

```bash
cd /Users/like/code/sentinel/sentinel-ai
npm run demo:biz-service
```

然后确认：

- Sentinel 代理已开启
- `Traffic Logic Triage` 已启用
- scope 已限制到本地测试站点
- 如果要观察增强效果，可开启浏览器扩展行为采集

## 2. 评测脚本

项目内置了一个最小评测脚本：

- [system-agent-eval.mjs](/Users/like/code/sentinel/sentinel-ai/scripts/system-agent-eval.mjs)
- [system-agent-eval-compare.mjs](/Users/like/code/sentinel/sentinel-ai/scripts/system-agent-eval-compare.mjs)

运行：

```bash
cd /Users/like/code/sentinel/sentinel-ai
npm run demo:eval
```

可选环境变量：

```bash
SYSTEM_AGENT_EVAL_BASE_URL=http://127.0.0.1:7788 npm run demo:eval
```

对照脚本运行方式：

```bash
cd /Users/like/code/sentinel/sentinel-ai
npm run demo:eval:compare -- --eval .tmp/system-agent-eval-report.json --findings .tmp/system-agent-findings-snapshot.json
```

## 3. 脚本会做什么

脚本会先调用：

- `POST /api/platform/reset`

把本地业务服务重置到干净状态，然后串行执行 5 组场景：

1. `reimbursement-primary`
   - 报销协作流程
   - 期望：`negative_control`

2. `reimbursement-secondary`
   - 报销处理入口
   - 期望：`logic_candidate`

3. `transfer-primary`
   - 转账协作流程
   - 期望：`negative_control`

4. `transfer-secondary`
   - 转账确认入口
   - 期望：`logic_candidate`

5. `activity-and-order`
   - 活动与订单访问
   - 期望：`mixed_candidate`

输出是结构化 JSON，包含：

- 运行时间
- 服务地址
- reset 结果
- 每组场景的请求序列
- 每组场景的预期标签

## 4. 如何做结果对照

推荐流程：

1. 先把评测输出保存到文件：

```bash
cd /Users/like/code/sentinel/sentinel-ai
npm run demo:eval > .tmp/system-agent-eval-report.json
```

2. 从 Sentinel 导出或保存 findings 快照，至少保留：
   - `id`
   - `title`
   - `url`
   - `status`
   - `analysisStage`
   - `severity`
   - `vulnType`

3. 运行对照脚本：

```bash
cd /Users/like/code/sentinel/sentinel-ai
npm run demo:eval:compare -- --eval .tmp/system-agent-eval-report.json --findings .tmp/system-agent-findings-snapshot.json
```

4. 再进入 Sentinel 做人工复核：
   - 查看流量历史，确认请求都进入历史记录
   - 查看 `Agent 管理 -> Traffic Logic Triage -> 运行记录`
   - 查看安全中心中对应的候选、已验证、误报

## 5. 当前最小评判标准

在当前阶段，可以先用这套人工标准：

- `negative_control`
  - 不进入正式漏洞列表
  - 若进入 `candidate`，应被视为误报候选

- `logic_candidate`
  - 至少形成 `candidate`
  - 如果验证成功，可升级为 `reviewed`

- `mixed_candidate`
  - 应能观察到至少一类：
    - `workflow`
    - `logic`
    - `bola/bfla/idor`

## 6. 推荐记录方式

建议配合 [system-agent-evaluation-workbook.md](/Users/like/code/sentinel/sentinel-ai/docs/system-agent-evaluation-workbook.md) 记录每轮结果：

- 时间
- 行为来源模式
  - `proxy_inferred`
  - `browser_extension`
- scope
- 每个场景的：
  - 预期标签
  - 实际 finding 状态
  - 是否验证成功
  - 是否被标成误报

## 7. 下一步建议

这套最小基线稳定后，再继续扩：

- 真阳性样本集
- 假阳性样本集
- 漏报样本集
- 浏览器行为增强 vs 弱行为推断对照统计
