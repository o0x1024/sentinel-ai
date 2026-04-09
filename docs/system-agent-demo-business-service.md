# 报销协作平台 Demo

这个 demo 现在已经改成一个更贴近真实业务系统的本地服务，不再使用纯内存 `Map`，而是使用 SQLite 持久化业务状态。它的目标不是“展示很多安全场景”，而是稳定提供两类更适合当前链路评估的样本：

- 平行越权
- 流程绕过

相关文件：

- [demo-business-service.mjs](/Users/like/code/sentinel/sentinel-ai/scripts/demo-business-service.mjs)
- [demo-business-service-db.mjs](/Users/like/code/sentinel/sentinel-ai/scripts/demo-business-service-db.mjs)
- [demo-business-service-ui.html](/Users/like/code/sentinel/sentinel-ai/scripts/demo-business-service-ui.html)

## 业务背景

当前 demo 模拟的是一个企业报销协作平台，核心对象只有一类：

- `报销单 expense claim`

但它具备 3 种真实系统中常见入口：

1. 标准门户
2. 移动协作端
3. 财务集成端

标准门户负责正常业务流：

`员工创建 -> 员工提交 -> 经理审批 -> 财务放款`

移动协作端和财务集成端则保留了两条遗留弱入口，用来稳定制造风险样本：

- `移动协作端弱摘要接口`
- `移动工作台快速审批`
- `财务集成端直接放款`

## SQLite

默认数据库文件：

```text
/Users/like/code/sentinel/sentinel-ai/scripts/.demo-business-service.sqlite
```

可通过环境变量覆盖：

```bash
DEMO_BIZ_DB=/tmp/demo-expense.sqlite npm run demo:biz-service
```

这个数据库会持久化：

- 用户
- 报销单
- 报销单流程事件

因此你重启 demo 后，状态不会像纯内存版本那样完全丢失；如果要恢复到初始数据，用重置接口即可。

## 启动

```bash
cd /Users/like/code/sentinel/sentinel-ai
npm run demo:biz-service
```

默认监听：

- `http://127.0.0.1:7788`
- Web 页面：`http://127.0.0.1:7788/`

可选环境变量：

```bash
DEMO_BIZ_HOST=127.0.0.1 DEMO_BIZ_PORT=7788 npm run demo:biz-service
```

## Demo 用户

- `alice`：employee / tenant-a
- `carol`：employee / tenant-a
- `bob`：employee / tenant-b
- `manager`：manager / tenant-a
- `finance`：finance / tenant-a
- `admin`：admin / tenant-a

认证方式二选一：

```http
Authorization: Bearer portal-alice-session
```

或：

```http
x-demo-user: alice
```

## 初始数据

默认会带 4 张报销单：

- `ec-a-1001`：alice 的草稿
- `ec-a-1002`：carol 的已提交单据
- `ec-a-1003`：alice 的已审批待放款单据
- `ec-b-2001`：bob 的已提交单据

这组数据的目的是：

- 给标准流程提供基线
- 给平行越权提供“同租户不同申请人”的对象
- 给流程绕过提供“已提交但未审批”的对象

## 风险设计

### 1. 平行越权

安全入口：

```text
GET /api/portal/expense-claims/{id}/detail
```

标准门户会做正常对象边界校验：

- 员工只能看自己的报销单
- 经理 / 财务只能看自己租户内的单据
- 管理员除外

弱入口：

```text
GET /api/mobile/collab/expense-claims/{id}/summary
```

这个接口只校验租户，不校验申请人归属。  
因此 `alice` 理论上不该看到 `carol` 的报销单摘要，但在这个弱入口里可以成功访问。

### 2. 流程绕过

标准流程入口：

```text
POST /api/portal/expense-claims/{id}/submit
POST /api/workflow/expense-claims/{id}/approve
POST /api/treasury/expense-claims/{id}/disburse
```

弱入口：

```text
POST /api/mobile/workbench/expense-claims/{id}/quick-approve
POST /api/integrations/treasury/expense-claims/{id}/release
```

这两条弱入口故意保留：

- `quick-approve`：允许同租户普通员工直接把报销单推进到已审批
- `release`：允许同租户用户直接放款，不要求标准审批状态和财务角色

这组样本更适合测试当前对：

- workflow
- logic
- BFLa / 敏感动作越权

的识别效果。

## Web 页面使用

打开：

```text
http://127.0.0.1:7788/
```

页面里已经内置 3 组动作：

1. 标准业务流程
2. 平行越权风险
3. 流程绕过风险

适合直接配合浏览器代理做测试，不需要手工拼完整 `curl`。  
如果你的目标是评估当前 Sentinel 对越权和逻辑漏洞的检出率，优先点这 3 个一键动作：

- `一键标准基线（自动切角色）`
- `一键平行越权样本`
- `一键流程绕过样本`
- `整组高风险样本`

注意：

- 页面已经刻意拦住了“普通员工直接点标准审批/标准放款”这种高噪音路径。
- 如果当前身份不是正确角色，点击标准审批/放款不会真的发请求，而是提示你先用自动基线或切到正确角色。
- 这样做的目的，是减少 `403 candidate` 噪音，让工作台更稳定地承载真实风险样本。

## 推荐测试流程

### A. 标准基线路径

1. 先点 `一键标准基线（自动切角色）`

这个动作会自动执行：

- `alice` 新建报销单
- `alice` 提交
- `manager` 审批
- `finance` 放款

并自动把新建单号回填到页面输入框。

用途：

- 生成一条完整成功的安全基线
- 避免手工切角色时误打出 403 噪音
- 给后续风险样本提供更稳定的对照

### B. 风险优先路径

1. 保持身份为 `alice`
2. 目标填 `ec-a-1002`
3. 先点 `一键平行越权样本`
4. 再点 `一键流程绕过样本`

这条路径优先制造：

- 安全详情失败
- 弱摘要成功
- employee 直接审批
- employee 直接放款

它比“先完整跑标准流程”更适合测试当前：

- 平行越权检出率
- workflow / logic 候选效果
- 自动验证是否能拿到有效 baseline

### C. 平行越权

1. 切换到 `alice`
2. 目标填 `ec-a-1002`
3. 先点 `安全详情`
4. 再点 `弱摘要越权读取`

预期：

- 安全详情应该被拒绝
- 弱摘要读取会成功

这组样本适合测试当前对对象边界/水平越权的识别效果。

### D. 流程绕过

1. 切换到 `alice`
2. 目标填 `ec-a-1002`
3. 点 `越权快速审批`
4. 再点 `越权直接放款`

或直接点：

```text
整组绕过流程
```

预期：

- 普通员工可以推动本不属于自己权限边界的敏感流程动作
- 工作台更容易产出 `workflow candidate`

## 期望检出结果对照表

下面这张表不是“绝对必须逐条命中”的承诺，而是当前 demo 在现有产品边界下的**推荐预期**。

产品边界先明确两点：

- `安全工作台`：承接 `candidate / triaging / false_positive / 调查态`
- `漏洞` tab：只展示正式漏洞，也就是 `open / reviewed / fixed`

因此：

- 只要还是 `candidate`，即使系统已经识别到风险，也**应该优先出现在安全工作台**
- 只有自动验证或人工验证足够强，最终状态被推进到正式态后，才会进 `漏洞` tab

### A. 标准安全动作

| 动作 | 预期 HTTP 结果 | 期望检出 |
| --- | --- | --- |
| `查看我的报销单` | 200 | 正常情况下不应产出 finding |
| `安全详情（alice -> ec-a-1001）` | 200 | 正常情况下不应产出 finding |
| `标准提交（alice -> 自己 draft）` | 200 | 正常情况下不应产出 finding；若出现 candidate，通常是当前 triage 噪音 |
| `标准审批（manager）` | 200 | 正常情况下不应产出 finding |
| `标准放款（finance）` | 200 | 正常情况下不应产出 finding |

### B. 平行越权样本

| 动作 | 预期 HTTP 结果 | 期望检出 |
| --- | --- | --- |
| `安全详情（alice -> ec-a-1002）` | 403 | 可作为“正常对象边界拒绝”基线，本身不一定单独产出 finding |
| `弱摘要越权读取（alice -> ec-a-1002）` | 200 | 应优先在 `安全工作台` 看到对象边界/越权候选 |
| `一键平行越权样本` | 先 403，再 200 | 这是最适合评估水平越权检出率的入口。优先看工作台中的对象边界分析、Replay Plan、自动验证结果 |

对这组样本，更合理的期望是：

- 第一阶段：`安全工作台` 出现候选
- 第二阶段：如果自动验证或人工验证确认“同一身份可读到不属于自己的对象”，再推进到 `漏洞` tab

### C. 流程绕过样本

| 动作 | 预期 HTTP 结果 | 期望检出 |
| --- | --- | --- |
| `越权快速审批（alice -> ec-a-1002）` | 200 | 应优先在 `安全工作台` 看到 `workflow` 或 `logic` candidate |
| `越权直接放款（alice -> ec-a-1002）` | 200 | 应优先在 `安全工作台` 看到 `workflow / logic / 敏感动作越权` 候选 |
| `一键流程绕过样本` | 连续 200 | 是当前评估流程绕过效果的主入口，优先看工作台和自动 verifier |

对这组样本，更合理的期望是：

- 第一阶段：`安全工作台` 出现 `workflow / logic` 候选
- 第二阶段：如果自动验证或人工确认表明普通员工确实越权推进了审批/放款，才推进到 `漏洞` tab

### D. 整组高风险样本

| 动作 | 预期 HTTP 结果 | 期望检出 |
| --- | --- | --- |
| `整组高风险样本` | 403 + 200 + 200 + 200 | 最适合观察当前链路是否同时对“对象边界”和“流程边界”给出候选 |

更具体地说，跑完这组动作后，理想状态应该是：

1. `安全工作台` 至少出现 2 类候选：
   - 平行越权 / 对象边界相关
   - workflow / logic 相关
2. 自动 verifier 如果拿到了有效 baseline，应追加 `system_agent_verification` evidence
3. 只有当验证足够强、状态被推进后，相关记录才进入 `漏洞` tab

### 如果结果和预期不一致，优先这样判断

1. `工作台有记录，漏洞没有记录`
   这通常是正常的，说明当前仍处于 `candidate` 阶段。

2. `只出现 create / submit 噪音，没有出现 weak summary / bypass`
   这通常表示你这轮操作主要打到了标准流程，而不是弱入口本身。

3. `自动 verifier 有 run，但漏洞仍没进正式台账`
   这通常表示 verifier 已运行，但没有得到足够强的确认结果。

4. `弱入口命中了，但仍只有工作台 candidate`
   这代表“系统已经识别到风险特征”，但还没有把它提升成正式漏洞；这时优先去工作台看对象边界分析和验证对比，而不是先看漏洞 tab。

## 关键接口

### 登录

```bash
curl -s http://127.0.0.1:7788/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"alice"}'
```

### 查看自己的报销单列表

```bash
curl -s http://127.0.0.1:7788/api/portal/expense-claims \
  -H 'Authorization: Bearer portal-alice-session'
```

### 正常详情

```bash
curl -s http://127.0.0.1:7788/api/portal/expense-claims/ec-a-1001/detail \
  -H 'Authorization: Bearer portal-alice-session'
```

### 平行越权样本

```bash
curl -s http://127.0.0.1:7788/api/mobile/collab/expense-claims/ec-a-1002/summary \
  -H 'Authorization: Bearer portal-alice-session'
```

### 正常提交

```bash
curl -s -X POST http://127.0.0.1:7788/api/portal/expense-claims/ec-a-1001/submit \
  -H 'Authorization: Bearer portal-alice-session'
```

### 正常审批

```bash
curl -s -X POST http://127.0.0.1:7788/api/workflow/expense-claims/ec-a-1002/approve \
  -H 'Authorization: Bearer portal-manager-session'
```

### 正常放款

```bash
curl -s -X POST http://127.0.0.1:7788/api/treasury/expense-claims/ec-a-1003/disburse \
  -H 'Authorization: Bearer portal-finance-session'
```

### 流程绕过样本：快速审批

```bash
curl -s -X POST http://127.0.0.1:7788/api/mobile/workbench/expense-claims/ec-a-1002/quick-approve \
  -H 'Authorization: Bearer portal-alice-session'
```

### 流程绕过样本：直接放款

```bash
curl -s -X POST http://127.0.0.1:7788/api/integrations/treasury/expense-claims/ec-a-1002/release \
  -H 'Authorization: Bearer portal-alice-session'
```

## 调试接口

查看当前数据库状态：

```bash
curl -s http://127.0.0.1:7788/api/platform/state \
  -H 'Authorization: Bearer portal-admin-session'
```

重置到默认 SQLite 数据：

```bash
curl -s -X POST http://127.0.0.1:7788/api/platform/reset \
  -H 'Authorization: Bearer portal-admin-session'
```

健康检查：

```bash
curl -s http://127.0.0.1:7788/health
```

## 用它怎么评估当前检测效果

建议按两类能力分别看：

### 对越权的评估

重点观察：

- `portal detail` 和 `mobile collab summary` 是否能形成对象边界对照
- 当前是否能对 `alice -> carol claim` 识别出对象越权候选
- 工作台里的对象边界分析是否能给出合理对象池和替换建议

### 对逻辑漏洞的评估

重点观察：

- `submit -> approve -> disburse` 的正常链路是否能被当作流程基线
- `quick-approve` 和 `release` 是否能被识别成流程跳步
- 当前 triage 更偏 `workflow` 还是能进一步抬到更强的敏感动作风险

这个 demo 的价值不在于“覆盖所有漏洞”，而在于：

**用一个足够真实、足够稳定、可持久化复盘的业务对象，把平行越权和流程绕过两类风险做成可重复测的样本。**
