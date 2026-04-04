# Demo Business Service

这个本地服务用于给当前 `System Agent` 能力制造真实业务流量，重点覆盖：

- 对象越权 / IDOR
- 审批越权
- 重复兑换
- 跳步执行
- 重复支付

服务文件：

- [demo-business-service.mjs](/Users/like/code/sentinel/sentinel-ai/scripts/demo-business-service.mjs)

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
- `admin`
- `finance`

认证方式二选一：

```http
Authorization: Bearer demo-alice
```

或：

```http
x-demo-user: alice
```

## 推荐测试流量

## Web 界面测试

启动服务后，直接打开：

```text
http://127.0.0.1:7788/
```

页面里已经内置：

- 身份切换
- 单场景按钮
- 一键跑完整场景
- 当前状态查看
- 实时响应输出面板

适合直接配合浏览器代理做测试，不需要再手敲 `curl`。

## 推荐测试流量

### 1. 登录拿 token

```bash
curl -s http://127.0.0.1:7788/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"alice"}'
```

### 2. 制造对象越权流量

先用 `alice` 访问自己的订单：

```bash
curl -s http://127.0.0.1:7788/api/orders/ord-1001 \
  -H 'Authorization: Bearer demo-alice'
```

再用 `alice` 访问 `bob` 的订单：

```bash
curl -s http://127.0.0.1:7788/api/orders/ord-2001 \
  -H 'Authorization: Bearer demo-alice'
```

项目成员列表也有同类问题：

```bash
curl -s http://127.0.0.1:7788/api/projects/proj-200/members \
  -H 'Authorization: Bearer demo-alice'
```

### 3. 制造审批越权流量

```bash
curl -s -X POST http://127.0.0.1:7788/api/orders/ord-2001/approve \
  -H 'Authorization: Bearer demo-alice'
```

### 4. 制造重复兑换逻辑漏洞流量

连续请求两次：

```bash
curl -s -X POST http://127.0.0.1:7788/api/coupons/redeem \
  -H 'Authorization: Bearer demo-alice' \
  -H 'content-type: application/json' \
  -d '{"couponCode":"WELCOME-50","orderId":"ord-1001"}'
```

### 5. 制造跳步执行逻辑漏洞流量

不调用 `prepare`，直接 `confirm`：

```bash
curl -s -X POST http://127.0.0.1:7788/api/transfers/confirm \
  -H 'Authorization: Bearer demo-alice' \
  -H 'content-type: application/json' \
  -d '{"transferId":"tr-missing","amount":100,"toAccount":"merchant-main"}'
```

### 6. 制造重复支付流量

连续请求两次：

```bash
curl -s -X POST http://127.0.0.1:7788/api/invoices/inv-100/pay \
  -H 'Authorization: Bearer demo-alice'
```

## 调试接口

查看当前内存状态：

```bash
curl -s http://127.0.0.1:7788/api/demo/state \
  -H 'Authorization: Bearer demo-admin'
```

健康检查：

```bash
curl -s http://127.0.0.1:7788/health
```

## 使用建议

为了测试当前这套“AI 从手动调用推进到系统级无感能力”的链路，建议这样用：

1. 启动这个 demo 服务
2. 让 Sentinel 代理监听器打开
3. 浏览器或 `curl` 通过代理访问这些接口
4. 重点混合不同身份访问同类资源
5. 观察：
   - 流量历史记录
   - 被动类 `traffic_logic_triage`
   - Security Center 中的 finding
   - `traffic_active_verifier` 的后续验证结果
