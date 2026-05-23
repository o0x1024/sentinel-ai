# 顺丰国际关务系统 (IBU-SDM-CORE) 安全测试报告

**目标网站**: https://ibu-sdm-core.sf-express.com/#/loginPage  
**系统名称**: 顺丰国际关务系统 (SF International Customs System)  
**分析日期**: 2026-05-21  
**分析方法**: 通过暴露的 webpack:// source maps 下载完整前端源码逆向分析  
**风险等级**: 🔴 **严重 (CRITICAL)**

---

## 📋 一、总体概述

顺丰国际关务系统是一个集报关、核注清单、物流账册、跨境服务、海关对接等核心业务功能的企业级关务管理平台。通过其暴露的 `webpack://` source maps，成功下载并分析了 **808 个前端源码文件**，涵盖完整的 API 调用、认证流程、业务逻辑和路由结构。

---

## 🔴 二、严重风险 (CRITICAL)

### 2.1 Source Map 泄露 - 完整前端源码暴露

**风险等级**: 🔴 CRITICAL  
**CWE**: CWE-540 (Source Code Exposure)  
**文件**: 
- `app.f13b5a22.js.map` (2.6 MB, 808 sources)
- `chunk-libs.d3e043c3.js.map` (18.9 MB)
- `chunk-elementUI.fe739060.js.map` (2.6 MB)
- `runtime.a1536464.js.map`

**详情**:  
生产环境的 JavaScript source maps 可公开访问，任何人都可以：
- 下载完整的前端源码（808个源文件）
- 获取所有 API 接口端点及参数结构
- 了解完整的业务逻辑和数据流
- 获取认证机制、权限控制逻辑
- 发现所有路由和页面结构

**影响**: 攻击者可以利用完整的前端源码进行深度分析，发现所有 API 端点、认证漏洞和业务逻辑缺陷。

**修复建议**:
- 生产环境禁用 source maps（`productionSourceMap: false`）
- 如必需，仅对内部 IP 开放 source map 访问
- 在 Nginx/OpenResty 层配置拒绝 `.map` 文件的外网访问

```nginx
location ~ \.map$ {
    deny all;
}
```

---

### 2.2 硬编码认证凭证 (Hardcoded Credentials)

**风险等级**: 🔴 CRITICAL  
**CWE**: CWE-798 (Hardcoded Credentials)  
**文件**: `src/utils/cas.js`

**详情**:  
在 CAS 认证模块中发现硬编码的应用凭证：

```javascript
// src/utils/cas.js - Line 20-22
init() {
    this.appKey = "IBU-SDM-CORE-CAS";
    this.appSecret = "IBU-SDM-CORE-CAS";  // ⚠️ 密钥与Key相同!
    this.deviceId = "abc123";              // ⚠️ 硬编码的默认设备ID!
    this.ENV = process.env?.["VUE_APP_ENV_TAG"] ?? "SIT";
}
```

然后这些凭证被发送到 CAS 登录接口：

```javascript
// src/utils/cas.js - Line 100-107
casLogin({
    appKey: this.appKey,      // "IBU-SDM-CORE-CAS"
    appSecret: this.appSecret, // "IBU-SDM-CORE-CAS" 
    deviceId: this.deviceId,   // "abc123"
    ticket: searchParams.ticket,
    service: location.origin,
})
```

**影响**:
- `appKey` 和 `appSecret` 完全相同且容易猜解
- `deviceId` 固定为 `abc123`，丧失设备验证意义
- 攻击者获取这些凭证后可以构造合法的 CAS 登录请求
- 可能绕过部分 API 网关的设备验证机制

**修复建议**:
- 将 `appSecret` 移至后端/环境变量，绝不出现在前端代码中
- 使用安全的随机字符串作为 `appSecret`
- `deviceId` 应在客户端动态生成（如使用浏览器指纹）
- 对 appKey/appSecret 采用非对称加密方案

---

### 2.3 API 网关绕过及内部架构暴露

**风险等级**: 🔴 CRITICAL  
**CWE**: CWE-200 (Information Exposure)

**详情**:  
前端源码暴露了完整的后端微服务架构：

**a) Base URL 配置**:
```javascript
// config.js
window._global = {
    _BASEURL: 'https://ibu-sdm-core.sf-express.com/api/',
    _CASURL_: "https://cas.sf-express.com/cas/login",
    _CASURL_LOGOUT: "https://cas.sf-express.com/cas/logout",
    _ICASURL_: "https://ibu-icas.sf-express.com",
    SfMonitorURL: 'http://bds-track.sf-express.com:8088/SfMonitor.globals.js',
    SDKURL: 'https://ibu-ibdp-access.sf-express.com/ibdp-data-access/writeData',
};
```

**b) 内部微服务端点**:
```javascript
// src/api/service.js - 服务URL配置
const baseServiceUrl           // 基础服务
const crossServiceUrl           // 跨境服务
const bondedServiceUrl          // 特殊监管区服务
const declarationServiceUrl     // 通关服务
const orderServiceUrl           // 订单服务
const customsServiceUrl         // 海关对接服务
```

**c) 内网地址泄露**:
```
http://ibu-sdm-core-apis.intsit.sfcloud.local:8000/apis-auth/login/cas
http://ibu-sdm-core-apis.intsit.sfcloud.local:8000/apis-auth/login/logout
```

**d) API 网关完整错误码暴露** (来自 `src/views/basp/common/axiosExt.vue.plugin.js`):
| 错误码 | 含义 |
|--------|------|
| 09020101 | 访问需要登录认证的接口时在请求头中没有设置token |
| 09020102 | 账号异地登录 |
| 09020201 | 未绑定接口，禁止访问 |
| 09020202 | 内部接口，禁止访问 |
| 09020301 | 访问过于频繁 |
| 09020401 | 网关根据session-id找不到密钥 |
| 09020402 | 没有在header中设置session-id |
| 09020501 | 网关没有该API的路由信息 |
| 09020502 | 后台微服务响应超时(默认20s) |

**影响**:
- 攻击者获得完整的后端架构图
- 了解 API 网关防护机制的弱点
- 可以针对性地绕过网关保护
- 内部地址可用于内网渗透的 pivot point

**修复建议**:
- 从生产环境 config.js 中移除内网地址
- 使用环境变量注入且不在前端暴露
- API 网关错误码不应在前端代码中完整映射

---

## 🟠 三、高风险 (HIGH)

### 3.1 完整的 API 接口清单暴露

**风险等级**: 🟠 HIGH  
**文件**: 所有 `src/api/` 下的文件

以下为所有发现的 API 模块及核心端点：

#### 用户认证模块 (`/api/user.js`)
| 端点 | 方法 | 风险 |
|------|------|------|
| `/apis-auth/login/cas` | POST | CAS 登录，含硬编码凭证 |
| `/apis-auth/login/logout` | GET | CAS 登出 |
| `/admin/queryUserInfo` | POST | 查询用户信息(含角色、租户、密钥) |
| `/admin/queryMenus` | GET | 查询菜单权限结构 |
| `/admin/queryPageBtn` | GET | 查询页面按钮权限 |
| `/admin/changeUserInfo` | POST | ⚠️ 切换角色/租户(潜在越权) |
| `/admin/loginOut` | POST | 登出清除Cookie |
| `/module/queryByOne/{code}` | GET | 模块信息查询 |
| `/baspDataGroupController/queryTenantKey` | GET | 租户Key查询 |
| `/tsUser/updateName` | GET | 更新用户名 |

#### 核注清单模块 (`/api/bonded/invt.js`)
| 端点 | 方法 | 风险 |
|------|------|------|
| `invtHeadService/findEntityList` | POST | 核注清单查询 |
| `invtHeadService/saveOrUpdate` | POST | 暂存核注清单 |
| `invtHeadService/deleteByIds` | POST | ⚠️ 删除核注清单 |
| `invtHeadService/declareInvt` | POST | ⚠️ 申报核注清单 |
| `invtHeadService/declareInvtBatch` | POST | ⚠️ 批量申报 |
| `invtHeadService/applyDelete` | POST | 申请删除 |
| `invtHeadService/saveDraft` | POST | 保存草稿 |
| `invtHeadService/billToInvtAutoSplit` | POST | 台账→核注自动拆单 |
| `passport/createFrom` | POST | 创建核放单 |

#### 通关管理模块 (`/api/declaration/declaration.js`)
| 端点 | 方法 | 风险 |
|------|------|------|
| `declHeadService/queryData` | POST | 报关单查询 |
| `declHeadService/saveData` | POST | 保存报关单 |
| `declHeadService/sendDecl` | POST | ⚠️ 发送报关报文 |
| `declHeadService/sendDeclBatch` | POST | ⚠️ 批量发送 |
| `declHeadService/deleteMain` | POST | ⚠️ 删除报关单 |
| `declHeadService/declInvalid` | POST | ⚠️ 作废报关单 |
| `declHeadService/copyData` | POST | 复制报关单 |
| `declHeadService/logicCheck` | POST | 逻辑校验 |
| `declHeadService/updateBatch` | POST | 批量修改 |
| `declBodyService/insertOrUpdateDeclBody` | POST | 新增/修改表体 |
| `declBodyService/deleteDeclBody` | POST | 删除表体 |
| `declOcrService/enableTask` | POST | OCR识别任务 |
| `declHeadService/sendLlmOcrTask` | POST | ⚠️ 多模态AI OCR |
| `declHeadService/cancelLlmOcrTask` | POST | 取消AI OCR |
| `riskRule/getRules` | GET | 风险规则查询 |
| `riskRule/updateRules` | POST | ⚠️ 修改风险规则 |

#### 物流账册模块 (`/api/bonded/bwl.js`)
| 端点 | 方法 | 风险 |
|------|------|------|
| `bwlService/query` | POST | 账册查询 |
| `bwlService/addOrUpdateHead` | POST | 新增/修改表头 |
| `bwlService/delete` | POST | ⚠️ 删除账册 |
| `bwlService/declare` | POST | ⚠️ 申报账册 |
| `bwlService/declareForHead` | POST | ⚠️ 表头申报 |
| `bwlService/tempStore` | POST | 暂存 |

#### 跨境服务模块 (`/api/crossService/exportCrossInfo.js`)
| 端点 | 方法 | 风险 |
|------|------|------|
| `exportCrossInfo/findEntityList` | POST | 跨境数据查询 |
| `exportCrossInfo/sendEntrySignleData` | POST | ⚠️ 发送入境申报 |
| `exportCrossInfo/sendSelectedDatas` | POST | ⚠️ 批量发送 |
| `exportCrossInfo/deleteSelectDatas` | POST | ⚠️ 删除跨境数据 |
| `expDeclService/createDanJu` | POST | 创建出口单据 |
| `inDeclService/createDanJu` | POST | 创建进口单据 |

#### 通用服务 (`/api/common.js`)
| 端点 | 方法 | 风险 |
|------|------|------|
| `commonService/listAllDclEtps` | POST | 所有申报单位 |
| `commonService/listAllBussinessUnit` | POST | 所有经营单位 |
| `commonService/listAllMasterCuscd` | POST | 主管关区代码 |
| `commonService/listAllCurrency` | POST | 所有币制 |
| `clientBaseDataCacheService/queryAllCountryAreaList` | POST | 所有国家代码 |
| `clientBaseDataCacheService/queryAllCustomCodeList` | POST | 所有关区代码 |
| `clientBaseDataCacheService/queryAllHarborCodeList` | POST | 所有港口代码 |
| `clientBaseDataCacheService/queryAllPortCodeList` | POST | 所有口岸代码 |
| `taskRunLogService/query` | POST | 异步任务状态轮询 |
| `filePathService/download` | POST | ⚠️ 文件下载(路径穿越风险) |

#### 数据统计模块 (`/api/workData.js`)
| 端点 | 方法 | 风险 |
|------|------|------|
| `businessTotalSum/query` | POST | 业务总量统计 |
| `businessTotalSum/export` | POST | 业务总量导出 |
| `checkSum/query2` | POST | 查验率统计 |
| `passEfficiencyService/list` | POST | 通关时效 |
| `declFailStatisticService/listForLine` | POST | 申报差错率 |
| `skuActivityStatistic/list2` | POST | SKU活跃度 |

---

### 3.2 权限提升风险 - 角色/租户切换接口

**风险等级**: 🟠 HIGH  
**端点**: `/admin/changeUserInfo` (POST)  
**参数**: `{ roleId, tenant }`

该接口允许用户切换角色和租户，如果后端没有严格验证当前用户是否有权切换到目标角色/租户，则存在横向越权和纵向提权风险。

**修复建议**:
- 后端严格校验用户是否属于目标租户
- 验证用户是否有目标角色的授权
- 添加审计日志记录角色/租户切换操作

---

### 3.3 文件下载路径穿越风险

**风险等级**: 🟠 HIGH  
**端点**: `filePathService/download` (POST)  
**参数**: `filePath` (通过 URL 参数传递)

```javascript
// src/common/js/asyncRequest.js
let filePathArr = filePath.split("/");
let fileName = filePathArr[filePathArr.length - 1];
filePath = encodeURIComponent(filePath);
let url = `${baseServiceUrl}/filePathService/download`;
asyncRequest.download1(url, filePath, resolve, reject, null, fileName);
```

**影响**: 如果后端未正确校验 `filePath` 参数，可能被利用进行路径穿越攻击 (`../../etc/passwd`)，读取服务器上的任意文件。

**修复建议**:
- 后端使用文件ID而非文件路径进行下载
- 严格校验和过滤路径参数

---

## 🟡 四、中风险 (MEDIUM)

### 4.1 Token 安全管理不当

**位置**: `src/utils/cas.js`, `src/store/modules/user.js`

- Token 同时存储在 localStorage 和 Cookie 中
- Token 过期时间固定为 3 天
- 登出时仅清除 Cookie，但 localStorage 中的 token 保留了

```javascript
Cookies.set("token", token, { expires: 3 });
localStorage.setItem("token", token);
```

**风险**: XSS 攻击可轻易读取 localStorage 中的 token。

**修复建议**:
- Token 仅使用 httpOnly Cookie 存储
- 缩短 Token 有效期
- 实现 refresh token 机制

### 4.2 多模态 AI (LLM) OCR 接口未授权风险

**端点**: 
- `declHeadService/sendLlmOcrTask` - 发起AI OCR
- `declHeadService/getLlmOcrResult?id=` - 获取AI OCR结果

这些接口涉及大模型调用，可能被滥用产生成本，且 `getLlmOcrResult` 通过 URL 参数传递 ID，存在 IDOR 风险。

### 4.3 内部监控系统暴露

```javascript
SfMonitorURL: 'http://bds-track.sf-express.com:8088/SfMonitor.globals.js'
```

顺丰监控系统(BDS Track)的 JS 文件路径暴露，结合端口 8088 非标准端口。

### 4.4 系统管理功能暴露

从路由分析发现系统管理模块包含：
- `/systemManage/userManage` - 用户管理
- `/systemManage/roleManage` - 角色管理
- `/systemManage/tenantManage` - 租户管理
- `/systemManage/moduleManage` - 模块管理

这些管理功能的接口端点未在前端源码中找到对应的 API 文件，推测可能使用另一套 API 路径。但路由结构暴露了所有管理页面路径。

---

## 🟢 五、低风险/信息泄露 (LOW/INFO)

### 5.1 开发/测试环境地址泄露
- `http://ibu-sdm-core-apis.intsit.sfcloud.local:8000` - 内部测试环境地址
- `VUE_APP_ENV_TAG: "PRO"` - 环境标识
- 多处 `DEVTOSIT` 条件判断暴露开发/测试逻辑

### 5.2 系统版本信息公开
```javascript
Cookies.set("sdm_load_version", sysVersion);
```

### 5.3 Cookie 管理细节
```javascript
Cas.clearAllCookie() // 仅清除 token, userName, _TOKEN_KEY_
// 未清除其他可能的会话 Cookie
```

### 5.4 CORS 配置
```javascript
withCredentials: true  // 允许携带凭证的跨域请求
```

---

## 📊 六、攻击面总结

### 攻击向量优先级

| 优先级 | 攻击向量 | 利用难度 | 影响范围 |
|--------|----------|----------|----------|
| P0 | 硬编码凭证 + CAS登录绕过 | 低 | 完全系统访问 |
| P0 | 文件下载路径穿越 | 中 | 服务器文件泄露 |
| P1 | 权限提升 (changeUserInfo) | 中 | 跨租户数据访问 |
| P1 | API端点未授权访问 | 中 | 业务数据泄露 |
| P1 | 报关单/核注清单越权操作 | 中 | 海关申报数据篡改 |
| P2 | AI OCR接口滥用 | 低 | 资源消耗/成本 |
| P2 | 批量操作接口滥用 | 低 | 业务数据批量泄露 |
| P3 | 监控/内部系统探测 | 中 | 内网渗透跳板 |

### 已暴露的完整功能模块

```
顺丰国际关务系统
├── 系统管理 (basp)
│   ├── 模块管理
│   ├── 角色管理
│   ├── 用户管理
│   └── 租户管理
├── 字典管理 (backstageManage)
├── 基础资料 (baseData)
│   ├── 账册管理
│   ├── 物料管理
│   ├── 公告管理
│   └── 配置管理
├── 通关管理 (declaration) ⚠️ 核心业务
│   ├── 整合申报
│   ├── 两步申报
│   ├── 默认值设置
│   ├── 统计报表
│   ├── 个人偏好设置
│   ├── 报关委托管理
│   ├── 查验管理
│   └── 修撤单查询
├── 特殊监管区 (bonded)
│   ├── 物流账册 (bwl)
│   └── 核注清单 (invt)
├── 跨境服务 (crossService)
├── 海关对接 (customs)
├── 订单管理 (order)
├── 保证金管理 (margin)
├── 报表中心 (report)
├── 风险配置 (riskConfigs)
├── 香港关务 (hongKongCustomsAffairs)
├── 船材管理 (shipMaterials)
├── 温州非保税 (wenZhouUnBonded)
└── 企业设置 (enterpriseSettings)
```

---

## 🛡️ 七、修复优先级建议

### 立即修复 (24小时内)
1. ❗ **禁用生产环境 source maps** - 最简单且影响最大的修复
2. ❗ **移除前端硬编码的 appSecret** - 改用后端签名机制
3. ❗ **修复 deviceId 硬编码** - 使用动态设备指纹

### 短期修复 (1周内)
4. 🔶 **审计文件下载接口** - 防止路径穿越
5. 🔶 **加强 changeUserInfo 权限校验** - 防止越权
6. 🔶 **Token 改用 httpOnly Cookie** - 防止 XSS 窃取

### 中期修复 (1个月内)
7. 🔹 **移除前端代码中的内部地址** - 防止架构泄露
8. 🔹 **API 网关错误码脱敏** - 减少信息泄露
9. 🔹 **加强批量操作接口的频率限制**
10. 🔹 **AI OCR 接口增加配额控制**

---

## 📝 附录

### A. 发现的完整 API 端点列表

提取了 **150+ 个 API 端点**，涵盖7个微服务模块。完整列表见各模块 API 文件：
- `src/api/user.js` (认证)
- `src/api/common.js` (通用)
- `src/api/workData.js` (工作数据)
- `src/api/service.js` (服务URL配置)
- `src/api/bonded/invt.js` (核注清单)
- `src/api/bonded/bwl.js` (物流账册)
- `src/api/bonded/acmp.js` (ACMP)
- `src/api/declaration/declaration.js` (通关申报)
- `src/api/declaration/preference.js` (偏好设置)
- `src/api/crossService/exportCrossInfo.js` (跨境)
- `src/api/baseData/accountBookManager/businessUnit.js` (经营单位)
- `src/api/baseData/materialModule/announcement.js` (公告)

### B. 源码文件清单

共计提取 808 个源文件，已保存至 `/tmp/sf_src/` 目录，关键文件包括：
- `src_utils_request.js` - HTTP请求拦截器
- `src_utils_cas.js` - CAS认证模块
- `src_store_modules_user.js` - 用户状态管理
- `src_router_index.js` - 路由配置
- 所有 `src_api_*` 文件 - API接口定义

---

**免责声明**: 本报告仅用于安全评估目的，请勿利用发现的漏洞进行未授权测试。建议立即联系安全团队进行整改。
