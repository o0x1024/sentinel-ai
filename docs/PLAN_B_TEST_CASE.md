# 方案B测试案例 - 完整使用指南

## 📋 测试概述

本测试案例演示如何使用方案B的AI插件生成系统，为目标网站自动生成安全检测插件。

**测试目标**: 使用DVWA (Damn Vulnerable Web Application) 作为测试目标  
**预计时间**: 30-45分钟  
**所需工具**: Sentinel AI + 被动代理 + AI服务 (OpenAI/Claude)

---

## 🎯 测试场景

我们将为DVWA应用生成以下类型的安全检测插件：
1. **SQL注入检测插件** - 针对数据库查询参数
2. **XSS检测插件** - 针对输入/输出点
3. **IDOR检测插件** - 针对用户ID参数

---

## 📝 前置准备

### 1. 环境准备

```bash
# 1. 确保Sentinel AI已编译
cd /Users/a1024/code/ai/sentinel-ai/src-tauri
cargo build --release

# 2. 准备DVWA测试环境
# 方式A: Docker快速启动
docker run --rm -it -p 80:80 vulnerables/web-dvwa

# 方式B: 或使用其他测试网站
# 例如: http://testphp.vulnweb.com
```

### 2. 配置AI服务

确保配置了AI服务（OpenAI或Claude）：

```bash
# 检查AI配置
# 在Sentinel AI UI中: 设置 -> AI服务 -> 配置API Key
```

---

## 🚀 测试步骤

### Step 1: 启动被动代理

在Sentinel AI应用中，启动被动代理扫描：

**方式1: UI操作**
```
1. 打开Sentinel AI应用
2. 导航到 "被动扫描" 页面
3. 点击 "启动代理" 按钮
4. 记录代理端口 (默认: 8080)
```

**方式2: MCP工具调用**
```json
{
  "tool": "start_passive_scan",
  "parameters": {
    "port": 8080,
    "target_domain": "localhost"
  }
}
```

**验证代理运行**:
```bash
# 检查代理端口
curl -x http://localhost:8080 http://www.baidu.com
# 应该返回百度首页HTML
```

---

### Step 2: 配置浏览器代理

配置浏览器使用Sentinel代理：

**Chrome/Edge**:
```
1. 打开设置 -> 系统 -> 代理设置
2. HTTP代理: localhost:8080
3. HTTPS代理: localhost:8080
```

**Firefox**:
```
1. 设置 -> 常规 -> 网络设置
2. 手动代理配置
3. HTTP代理: localhost, 端口: 8080
4. 勾选 "为所有协议使用此代理服务器"
```

**或使用curl测试**:
```bash
export http_proxy=http://localhost:8080
export https_proxy=http://localhost:8080
```

---

### Step 3: 浏览目标网站

**目标**: 收集至少100个HTTP请求，覆盖主要功能

访问DVWA主要功能：

```
1. 登录页面
   http://localhost/login.php
   - 用户名: admin
   - 密码: password

2. SQL注入页面
   http://localhost/vulnerabilities/sqli/
   - 输入不同的User ID: 1, 2, 3, admin, ' OR '1'='1

3. XSS页面
   http://localhost/vulnerabilities/xss_reflected/
   - 输入测试数据: <script>alert(1)</script>

4. 用户信息页面
   http://localhost/vulnerabilities/view_user.php?id=1
   - 尝试不同的ID: 1, 2, 3, 999

5. 其他功能
   - 文件上传
   - 命令注入
   - CSRF测试页面
```

**建议操作时间**: 15-20分钟

**实时监控**:
```bash
# 在Sentinel AI中查看捕获的请求
# UI: 被动扫描 -> 请求列表
# 确认已捕获 100+ 请求
```

---

### Step 4: 分析网站结构

使用 `analyze_website` 工具分析收集的流量：

**方式1: AI Agent调用**

向Sentinel AI发送消息：
```
请帮我分析localhost网站的结构，使用analyze_website工具
```

**方式2: 直接MCP工具调用**

```json
{
  "tool": "analyze_website",
  "parameters": {
    "domain": "localhost"
  }
}
```

**预期输出**:

```
🔍 Website Analysis: localhost
Total Requests Analyzed: 156

📊 API Endpoints Discovered: 12

1. GET /login.php (pattern: /login.php, hits: 8)
   Query params: username:string, password:string

2. POST /vulnerabilities/sqli/ (pattern: /vulnerabilities/sqli/, hits: 15)
   Body params: id:string, Submit:string

3. GET /vulnerabilities/xss_reflected/ (pattern: /vulnerabilities/xss_reflected/, hits: 10)
   Query params: name:string

4. GET /vulnerabilities/view_user.php (pattern: /vulnerabilities/view_user.php, hits: 12)
   Query params: id:integer

... and 8 more endpoints

🛠️  Technology Stack Detected:
   Server: Apache/2.4.41
   Language: PHP
   Database: MySQL
   Framework: Custom

📋 Unique Parameters Found: 23
   id, username, password, Submit, name, file, command, token, ...

📦 Static Resources: 45
🔌 API Endpoints: 12
```

**分析结果保存**:
```bash
# 分析结果会自动保存在内存中
# 同时返回JSON格式数据供下一步使用
```

---

### Step 5: 生成安全插件

使用分析结果生成针对性的安全插件：

**方式1: AI Agent调用**

```
基于刚才的分析结果，请使用generate_advanced_plugin工具生成以下插件：
1. SQL注入检测插件 - 针对id参数
2. XSS检测插件 - 针对name参数  
3. IDOR检测插件 - 针对view_user.php
```

**方式2: MCP工具调用**

```json
{
  "tool": "generate_advanced_plugin",
  "parameters": {
    "analysis": {
      // 从Step 4返回的analysis对象
      "domain": "localhost",
      "endpoints": [...],
      "tech_stack": {...},
      ...
    },
    "vuln_types": ["sqli", "xss", "idor"],
    "target_endpoints": [
      "/vulnerabilities/sqli/",
      "/vulnerabilities/xss_reflected/",
      "/vulnerabilities/view_user.php"
    ],
    "requirements": "Focus on parameter injection and authentication bypass. Use MySQL-specific payloads."
  }
}
```

**预期输出**:

```
🤖 AI Plugin Generation Complete
Generated 3 plugins

1. MySQL SQL Injection Detector (ID: plugin-sqli-20251113-001)
   Type: sqli
   Quality Score: 82.5/100
   Status: PendingReview
   Model: gpt-4
   Quality Breakdown:
     - Syntax: 95%
     - Logic: 85%
     - Security: 80%
     - Code Quality: 75%
   ✅ Validation: PASSED

2. Reflected XSS Detector (ID: plugin-xss-20251113-002)
   Type: xss
   Quality Score: 78.0/100
   Status: PendingReview
   Model: gpt-4
   Quality Breakdown:
     - Syntax: 90%
     - Logic: 80%
     - Security: 75%
     - Code Quality: 70%
   ✅ Validation: PASSED
   ⚠️  Warnings:
      - Consider adding CSP header detection

3. IDOR User Access Detector (ID: plugin-idor-20251113-003)
   Type: idor
   Quality Score: 75.5/100
   Status: PendingReview
   Model: gpt-4
   Quality Breakdown:
     - Syntax: 92%
     - Logic: 78%
     - Security: 70%
     - Code Quality: 72%
   ✅ Validation: PASSED

📊 Summary:
   - Pending Review: 3
   - Validation Failed: 0
   - Average Quality: 78.7/100
```

**生成时间**: 每个插件约5-15秒（取决于LLM响应速度）

---

### Step 6: 审核和管理插件

打开插件审核UI进行人工审核：

**访问审核界面**:
```
http://localhost:1420/plugin-review
```

**审核操作**:

1. **查看插件列表**
   - 所有待审核的插件会显示在列表中
   - 显示质量评分、漏洞类型、生成时间

2. **查看插件详情**
   - 点击插件查看完整代码
   - 检查检测逻辑是否合理
   - 查看质量评分详情

3. **编辑插件（可选）**
   - 修改检测逻辑
   - 调整payload
   - 优化匹配规则

4. **批准插件**
   ```
   点击 "批准" 按钮
   - 插件状态变为 Approved
   - 自动部署到扫描引擎
   ```

5. **拒绝插件（如果需要）**
   ```
   点击 "拒绝" 按钮
   - 插件状态变为 Rejected
   - 可以重新生成
   ```

**示例审核流程**:

```typescript
// SQL注入插件代码示例
export default {
  name: "MySQL SQL Injection Detector",
  version: "1.0.0",
  
  match(request: Request): boolean {
    // 匹配含有id参数的请求
    const url = new URL(request.url);
    return url.searchParams.has('id') || 
           request.body?.includes('"id"');
  },
  
  async execute(request: Request): Promise<Finding[]> {
    const findings: Finding[] = [];
    
    // MySQL特定的SQL注入payload
    const payloads = [
      "' OR '1'='1",
      "1' UNION SELECT NULL,NULL,NULL--",
      "1' AND SLEEP(5)--"
    ];
    
    // 测试每个payload
    for (const payload of payloads) {
      const testReq = request.clone();
      // 替换参数值
      testReq.setParam('id', payload);
      
      const response = await fetch(testReq);
      
      // 检测SQL错误或异常行为
      if (this.detectSQLInjection(response)) {
        findings.push({
          severity: "high",
          type: "sqli",
          description: `SQL Injection found with payload: ${payload}`,
          evidence: response.body.substring(0, 500)
        });
      }
    }
    
    return findings;
  },
  
  detectSQLInjection(response: Response): boolean {
    const indicators = [
      /SQL syntax.*?error/i,
      /mysql_fetch_array/i,
      /You have an error in your SQL syntax/i
    ];
    
    return indicators.some(pattern => 
      pattern.test(response.body)
    );
  }
}
```

**审核要点**:
- ✅ 检测逻辑是否合理
- ✅ Payload是否针对目标技术栈
- ✅ 是否有误报风险
- ✅ 性能是否可接受（避免过多请求）

---

### Step 7: 执行扫描测试

插件批准后，自动加载到扫描引擎。继续浏览网站即可触发检测：

**自动扫描**:
```
1. 继续使用代理浏览DVWA
2. 访问之前的功能页面
3. 插件会自动对流量进行检测
4. 发现漏洞会自动记录
```

**主动扫描测试**:
```
也可以手动触发特定URL的扫描
UI: 工具 -> 插件管理 -> 测试插件
输入测试URL: http://localhost/vulnerabilities/sqli/?id=1
```

**预期结果**:

访问 `http://localhost/vulnerabilities/sqli/?id=1` 时：

```
🔍 Finding Detected!

Plugin: MySQL SQL Injection Detector
Severity: HIGH
Type: SQL Injection
URL: http://localhost/vulnerabilities/sqli/?id=1'
Parameter: id
Payload: ' OR '1'='1

Evidence:
  - SQL syntax error detected in response
  - Original value: 1
  - Injected value: ' OR '1'='1
  - Response contains database error message

Recommendation:
  - Use parameterized queries
  - Implement input validation
  - Apply least privilege principle for database user
```

---

### Step 8: 查看检测结果

**方式1: UI查看**
```
UI: 扫描结果 -> 漏洞列表
- 按严重程度排序
- 按漏洞类型过滤
- 查看详细信息
```

**方式2: MCP工具查询**
```json
{
  "tool": "list_findings",
  "parameters": {
    "severity": "high",
    "vuln_type": "sqli",
    "limit": 10
  }
}
```

**方式3: 数据库查询**
```bash
sqlite3 /Users/a1024/Library/Application\ Support/sentinel-ai/database.db

SELECT 
  id, 
  severity, 
  vuln_type, 
  url, 
  created_at 
FROM findings 
WHERE severity = 'high' 
ORDER BY created_at DESC 
LIMIT 10;
```

---

## 📊 预期测试结果

### 性能指标

| 操作 | 预期时间 | 实际时间 | 状态 |
|------|---------|---------|------|
| 网站浏览 | 15-20分钟 | _填写_ | □ |
| 流量收集 | 100+ 请求 | _填写_ | □ |
| 网站分析 | < 2秒 | _填写_ | □ |
| 插件生成 | 15-45秒 | _填写_ | □ |
| 插件验证 | < 1秒 | _填写_ | □ |
| 插件审核 | 5-10分钟 | _填写_ | □ |
| 漏洞检测 | 实时 | _填写_ | □ |

### 质量指标

| 指标 | 目标值 | 实际值 | 达成 |
|------|--------|--------|------|
| 插件生成成功率 | > 90% | _填写_ | □ |
| 插件语法正确率 | > 95% | _填写_ | □ |
| 平均质量评分 | > 70分 | _填写_ | □ |
| 漏洞检测准确率 | > 80% | _填写_ | □ |
| 误报率 | < 20% | _填写_ | □ |

### 功能验证

- [ ] 被动代理正常启动
- [ ] 流量成功捕获
- [ ] 网站分析完整准确
- [ ] API端点正确识别
- [ ] 技术栈识别准确
- [ ] 插件生成成功
- [ ] 语法验证通过
- [ ] 质量评分合理
- [ ] 审核UI正常工作
- [ ] 插件批准/拒绝功能正常
- [ ] 插件自动部署成功
- [ ] 漏洞检测正常工作
- [ ] 结果记录准确

---

## 🐛 常见问题排查

### 问题1: 代理无法启动

**症状**: 启动代理时报错 "Port already in use"

**解决**:
```bash
# 检查端口占用
lsof -i :8080

# 杀死占用进程
kill -9 <PID>

# 或使用其他端口
start_passive_scan(port=8081)
```

### 问题2: 未捕获到流量

**症状**: 浏览网站后，请求列表为空

**排查**:
```bash
# 1. 确认代理正在运行
curl -x http://localhost:8080 http://www.baidu.com

# 2. 检查浏览器代理设置
# Chrome: chrome://settings/system
# Firefox: about:preferences#general

# 3. 检查证书是否信任
# 导入Sentinel根证书：src-tauri/ca/sentinel-ca.pem

# 4. 查看代理日志
tail -f logs/sentinel-ai.log.2025-11-13
```

### 问题3: 网站分析返回空结果

**症状**: analyze_website返回0个端点

**解决**:
```json
// 检查domain参数是否正确
{
  "tool": "analyze_website",
  "parameters": {
    "domain": "localhost"  // 确保与实际访问的domain一致
  }
}

// 如果DVWA运行在其他域名，使用实际域名
{
  "domain": "dvwa.local"
}
```

### 问题4: 插件生成失败

**症状**: generate_advanced_plugin报错 "AI service not available"

**排查**:
```bash
# 1. 检查AI服务配置
# UI: 设置 -> AI服务

# 2. 测试API连接
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer YOUR_API_KEY"

# 3. 查看AI服务日志
tail -f logs/llm-http-requests-2025-11-13.log

# 4. 尝试使用备用模型
# 在配置中切换到Claude或本地LLM
```

### 问题5: 插件质量分数过低

**症状**: 生成的插件质量分数 < 40分

**改进方法**:
```json
{
  "tool": "generate_advanced_plugin",
  "parameters": {
    "analysis": {...},
    "vuln_types": ["sqli"],
    // 添加更详细的requirements
    "requirements": `
      - Use MySQL-specific SQL injection payloads
      - Include time-based blind detection
      - Test for error-based injection
      - Add boolean-based detection
      - Include UNION-based testing
      - Use prepared statements in recommendations
    `
  }
}
```

### 问题6: 审核UI无法访问

**症状**: http://localhost:1420/plugin-review 404

**解决**:
```bash
# 1. 确认应用正在运行
ps aux | grep sentinel-ai

# 2. 检查前端是否编译
cd /Users/a1024/code/ai/sentinel-ai
npm run build

# 3. 重启应用
# 关闭后重新启动Sentinel AI
```

---

## 📝 测试检查清单

### 环境准备
- [ ] Sentinel AI已编译并运行
- [ ] DVWA测试环境已启动
- [ ] AI服务已配置（API Key）
- [ ] 浏览器代理设置正确

### Step 1-3: 流量收集
- [ ] 被动代理成功启动
- [ ] 浏览器流量经过代理
- [ ] 已访问DVWA主要功能
- [ ] 捕获100+请求
- [ ] 请求包含多种HTTP方法（GET/POST）
- [ ] 请求包含查询参数和Body参数

### Step 4: 网站分析
- [ ] analyze_website工具调用成功
- [ ] 识别出10+个API端点
- [ ] 正确提取参数信息
- [ ] 技术栈识别准确（Apache/PHP/MySQL）
- [ ] 分析时间 < 5秒

### Step 5: 插件生成
- [ ] generate_advanced_plugin工具调用成功
- [ ] 生成3个插件（sqli/xss/idor）
- [ ] 所有插件语法验证通过
- [ ] 平均质量分数 > 70
- [ ] 生成时间合理（<1分钟）

### Step 6: 插件审核
- [ ] 审核UI可以访问
- [ ] 插件列表显示正常
- [ ] 可以查看插件详情和代码
- [ ] 可以编辑插件代码
- [ ] 批准功能正常工作
- [ ] 拒绝功能正常工作

### Step 7-8: 漏洞检测
- [ ] 已批准插件自动加载
- [ ] 继续浏览触发检测
- [ ] 成功检测到SQL注入
- [ ] 成功检测到XSS
- [ ] 成功检测到IDOR
- [ ] 漏洞记录到数据库
- [ ] 可以查看检测结果

### 质量验证
- [ ] 无误报（真实漏洞）
- [ ] 无漏报（已知漏洞都检测到）
- [ ] 插件代码质量良好
- [ ] 检测逻辑合理
- [ ] 性能影响可接受

---

## 🎓 学习要点

通过本测试，你应该掌握：

1. **方案B的完整工作流程**
   - 被动流量收集 → 智能分析 → AI生成 → 人工审核 → 自动检测

2. **核心工具使用**
   - `start_passive_scan` - 启动流量收集
   - `analyze_website` - 分析网站结构
   - `generate_advanced_plugin` - 生成检测插件

3. **插件质量评估**
   - 语法正确性（AST验证）
   - 逻辑合理性（沙箱测试）
   - 安全性检查
   - 代码质量

4. **审核和优化**
   - 如何审核AI生成的代码
   - 如何优化插件质量
   - 如何减少误报

---

## 📚 相关文档

- [PLAN_B_USAGE_GUIDE.md](./PLAN_B_USAGE_GUIDE.md) - 完整使用指南
- [PLAN_B_ARCHITECTURE.md](./PLAN_B_ARCHITECTURE.md) - 技术架构
- [PLAN_B_FINAL_SUMMARY.md](./PLAN_B_FINAL_SUMMARY.md) - 项目总结

---

## 💡 下一步

完成测试后，你可以：

1. **尝试其他目标**
   - 测试真实的Web应用
   - 使用不同的技术栈

2. **优化插件生成**
   - 提供更详细的requirements
   - 指定特定的target_endpoints
   - 训练质量模型

3. **扩展功能**
   - 添加新的漏洞类型
   - 创建自定义Few-shot示例
   - 开发新的验证规则

4. **生产部署**
   - 集成到CI/CD流程
   - 建立插件库
   - 团队协作和分享

---

**祝测试顺利！** 🚀

如有问题，请查看日志文件或提交Issue。

