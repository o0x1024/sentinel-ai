# Demo Business Service

这个本地服务用于给当前 `System Agent` 制造更贴近真实项目的业务流量。它不再是“明显写给安全测试看的接口集合”，而是一个带已有业务数据、标准门户入口、移动工作台入口、财务集成入口、客服入口的费用与资金平台。

服务文件：

- [demo-business-service.mjs](/Users/like/code/sentinel/sentinel-ai/scripts/demo-business-service.mjs)
- [demo-business-service-ui.html](/Users/like/code/sentinel/sentinel-ai/scripts/demo-business-service-ui.html)

## 业务背景

当前 demo 模拟的是一个企业费用与资金平台，包含 4 类业务对象：

- `费用报销单 expense claim`
- `付款申请 payment request`
- `营销券 coupon`
- `客户订单 order`

平台默认会带一批已有状态的数据，便于测试“同类业务对象在不同状态、不同入口、不同身份下的访问差异”：

- 已支付的报销单
- 已提交待审批的报销单
- 已审核待确认的付款申请
- 不同租户下的订单

这比“启动后全空、所有请求都是新造对象”的状态更接近真实项目。

## 启动

```bash
cd /Users/like/code/sentinel/sentinel-ai
npm run demo:biz-service
```

默认监听：

- `http://127.0.0.1:7788`
- Web 界面：`http://127.0.0.1:7788/`

可选环境变量：

```bash
DEMO_BIZ_HOST=127.0.0.1 DEMO_BIZ_PORT=7788 npm run demo:biz-service
```

## Demo 用户

- `alice`
- `bob`
- `manager`
- `finance`
- `admin`

认证方式二选一：

```http
Authorization: Bearer portal-alice-session
```

或：

```http
x-demo-user: alice
```

## Web 界面测试

启动服务后，直接打开：

```text
http://127.0.0.1:7788/
```

页面里已经内置：

- 身份切换
- 标准协作流程
- 移动端/集成端/客服端辅助流程
- 一键跑完整场景
- 当前平台状态查看
- 实时响应输出面板

适合直接配合浏览器代理做测试，不需要手工拼全部 `curl`。

## 推荐测试流量

### 1. 登录拿 token

```bash
curl -s http://127.0.0.1:7788/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"alice"}'
```

### 2. 标准费用报销流程

先创建一张报销单：

```bash
curl -s -X POST http://127.0.0.1:7788/api/expense/claims/create \
  -H 'Authorization: Bearer portal-alice-session' \
  -H 'content-type: application/json' \
  -d '{
    "item":"客户拜访差旅",
    "amount":860,
    "claimType":"travel",
    "costCenter":"CC-SALES-01",
    "expensePolicyId":"POL-TRAVEL-2026",
    "vendorId":"vendor-air-cn",
    "businessLine":"regional-sales"
  }'
```

再依次提交、经理审批、财务打款：

```bash
curl -s -X POST http://127.0.0.1:7788/api/expense/claims/ec-a-0903/submit \
  -H 'Authorization: Bearer portal-alice-session'

curl -s -X POST http://127.0.0.1:7788/api/workflow/expense-claims/ec-a-0903/approve \
  -H 'Authorization: Bearer portal-manager-session'

curl -s -X POST http://127.0.0.1:7788/api/treasury/expense-claims/ec-a-0903/disburse \
  -H 'Authorization: Bearer portal-finance-session'
```

### 3. 移动工作台审批跳步

不经过标准审批角色和流程，直接通过移动工作台完成任务：

```bash
curl -s -X POST http://127.0.0.1:7788/api/mobile/workbench/expense-claims/ec-a-0902/task-complete \
  -H 'Authorization: Bearer portal-alice-session'
```

### 4. 财务集成端重复执行出款

对同一张报销单连续调用两次集成执行接口：

```bash
curl -s -X POST http://127.0.0.1:7788/api/integrations/treasury/expense-claims/ec-a-0902/execute \
  -H 'Authorization: Bearer portal-alice-session'

curl -s -X POST http://127.0.0.1:7788/api/integrations/treasury/expense-claims/ec-a-0902/execute \
  -H 'Authorization: Bearer portal-alice-session'
```

### 5. 付款申请跳过审核直接确认

```bash
curl -s -X POST http://127.0.0.1:7788/api/mobile/treasury/payment-confirmations \
  -H 'Authorization: Bearer portal-alice-session' \
  -H 'content-type: application/json' \
  -d '{
    "transferId":"pr-missing-001",
    "amount":5200,
    "toAccount":"supplier-main",
    "paymentCategory":"vendor-settlement",
    "beneficiaryVendorId":"vendor-ic-301"
  }'
```

### 6. 营销券重复领取

```bash
curl -s -X POST http://127.0.0.1:7788/api/marketing/coupons/SPRING-2026/claim \
  -H 'Authorization: Bearer portal-alice-session'

curl -s -X POST http://127.0.0.1:7788/api/channel-partner/campaigns/OPS-BONUS/activate \
  -H 'Authorization: Bearer portal-alice-session'

curl -s -X POST http://127.0.0.1:7788/api/channel-partner/campaigns/OPS-BONUS/activate \
  -H 'Authorization: Bearer portal-alice-session'
```

### 7. 客服订单快照越权

先读取自己租户内的正常详情：

```bash
curl -s http://127.0.0.1:7788/api/customer/orders/ord-a-1001/detail \
  -H 'Authorization: Bearer portal-alice-session'
```

再读取其他租户订单的客服快照：

```bash
curl -s http://127.0.0.1:7788/api/customer-service/orders/ord-b-2001/snapshot \
  -H 'Authorization: Bearer portal-alice-session'
```

## 调试接口

查看当前内存状态：

```bash
curl -s http://127.0.0.1:7788/api/platform/state \
  -H 'Authorization: Bearer portal-admin-session'
```

重置平台到默认业务状态：

```bash
curl -s -X POST http://127.0.0.1:7788/api/platform/reset \
  -H 'Authorization: Bearer portal-admin-session'
```

健康检查：

```bash
curl -s http://127.0.0.1:7788/health
```

## 使用建议

为了测试当前这条“结构化上下文 -> AI 语义抽象 -> 假设生成 -> LLM triage -> 可选 replay 验证”的链路，建议这样用：

1. 启动这个 demo 服务。
2. 打开 Sentinel 代理监听。
3. 先跑一遍标准流程，制造基线样本。
4. 再跑移动端、集成端、客服端这些辅助入口，制造同对象、同动作、不同入口的对照流量。
5. 重点观察：
   - 流量历史记录
   - `traffic_logic_triage`
   - Security Center 里的 `contextExtraction`、`semanticAbstraction`、`logicHypotheses`
   - `traffic_active_verifier` 的验证结果
