# 方案B: 高级AI插件生成 - 使用指南

## 概述

方案B提供了基于AI的智能插件生成系统，可以根据网站分析结果自动生成高质量的安全检测插件。

## 核心工作流

```
1. 启动被动代理
    ↓
2. 浏览目标网站
    ↓
3. 分析网站结构 (analyze_website)
    ↓
4. 生成检测插件 (generate_advanced_plugin)
    ↓
5. 审核并启用插件
    ↓
6. 执行被动扫描
    ↓
7. 查看漏洞发现
```

## 详细步骤

### Step 1: 启动被动扫描代理

使用MCP工具或Tauri命令启动代理：

```json
{
  "tool": "start_passive_scan",
  "parameters": {
    "port": 8080
  }
}
```

配置浏览器代理：
- HTTP Proxy: `127.0.0.1:8080`
- HTTPS Proxy: `127.0.0.1:8080`

### Step 2: 浏览目标网站

在浏览器中访问目标网站，确保：
- ✅ 访问主要功能页面
- ✅ 触发各种API调用
- ✅ 测试登录、搜索、提交表单等功能
- ✅ 浏览至少10-20个不同的页面

代理会自动记录所有HTTP/HTTPS流量到数据库。

### Step 3: 分析网站结构

使用 `analyze_website` 工具分析收集到的流量：

```json
{
  "tool": "analyze_website",
  "parameters": {
    "domain": "example.com",
    "limit": 1000
  }
}
```

**返回结果示例**：
```json
{
  "domain": "example.com",
  "total_requests": 234,
  "unique_endpoints": 45,
  "api_endpoints": [
    {
      "path": "/api/users",
      "method": "GET",
      "frequency": 12,
      "parameters": [
        {
          "name": "id",
          "type": "number",
          "location": "query",
          "required": true,
          "sample_values": ["1", "2", "100"]
        },
        {
          "name": "role",
          "type": "string",
          "location": "query",
          "required": false,
          "sample_values": ["admin", "user"]
        }
      ],
      "auth_required": true,
      "response_types": ["application/json"]
    },
    {
      "path": "/api/login",
      "method": "POST",
      "frequency": 5,
      "parameters": [
        {
          "name": "username",
          "type": "string",
          "location": "body",
          "required": true
        },
        {
          "name": "password",
          "type": "string",
          "location": "body",
          "required": true
        }
      ]
    }
  ],
  "tech_stack": {
    "server": "nginx/1.20.1",
    "framework": "Express.js",
    "database": "MySQL",
    "language": "Node.js",
    "features": ["REST API", "JWT Auth", "Rate Limiting"]
  },
  "security_observations": [
    "HTTPS enabled",
    "CORS headers present",
    "CSP header missing"
  ]
}
```

### Step 4: 生成检测插件

基于分析结果，使用 `generate_advanced_plugin` 生成插件：

#### 示例 1: 生成SQL注入检测插件

```json
{
  "tool": "generate_advanced_plugin",
  "parameters": {
    "analysis": {
      "domain": "example.com",
      "api_endpoints": [
        {
          "path": "/api/users",
          "method": "GET",
          "parameters": [
            {"name": "id", "type": "number", "location": "query"}
          ]
        }
      ],
      "tech_stack": {
        "database": "MySQL"
      }
    },
    "vuln_types": ["sqli"],
    "target_endpoints": ["/api/users"],
    "requirements": "Focus on numeric parameter SQL injection"
  }
}
```

**返回结果**：
```json
{
  "plugins": [
    {
      "name": "sqli_detector_mysql",
      "description": "SQL injection detector for MySQL backend",
      "code": "// TypeScript plugin code...",
      "status": "pending_review",
      "quality_score": 85.0,
      "quality_breakdown": {
        "logic_score": 90.0,
        "security_score": 100.0,
        "code_quality_score": 70.0
      },
      "vuln_type": "sqli",
      "target_tech": "MySQL",
      "generated_at": "2025-11-13T10:30:00Z",
      "model_used": "gpt-4"
    }
  ],
  "summary": "Generated 1 plugin(s)...",
  "statistics": {
    "total": 1,
    "pending_review": 1,
    "validation_failed": 0,
    "average_quality": 85.0
  }
}
```

#### 示例 2: 批量生成多类型插件

```json
{
  "tool": "generate_advanced_plugin",
  "parameters": {
    "analysis": {
      "domain": "shop.example.com",
      "api_endpoints": [
        {
          "path": "/api/products",
          "method": "GET",
          "parameters": [
            {"name": "category", "type": "string", "location": "query"}
          ]
        },
        {
          "path": "/api/checkout",
          "method": "POST",
          "parameters": [
            {"name": "user_id", "type": "number", "location": "body"},
            {"name": "items", "type": "array", "location": "body"}
          ]
        }
      ],
      "tech_stack": {
        "framework": "Express.js",
        "database": "MongoDB"
      }
    },
    "vuln_types": ["sqli", "xss", "idor"],
    "requirements": "E-commerce security focus, check price manipulation"
  }
}
```

### Step 5: 审核并启用插件

#### 5.1 查看生成的插件代码

```typescript
// 生成的插件示例
export const plugin = {
  metadata: {
    name: "sqli_detector_mysql",
    version: "1.0.0",
    description: "Detects SQL injection in MySQL queries",
    author: "AI Generator",
    severity: "high",
    category: "injection",
    tags: ["sqli", "mysql", "database"]
  },

  scan_request: async (ctx: RequestContext) => {
    const { url, method, headers, body } = ctx.request;
    
    // Parse URL parameters
    const urlObj = new URL(url);
    const params = urlObj.searchParams;
    
    // SQL injection payloads for MySQL
    const payloads = [
      "' OR '1'='1",
      "1' UNION SELECT NULL--",
      "1' AND 1=1--",
      "' OR 1=1#"
    ];
    
    for (const [key, value] of params.entries()) {
      if (value && typeof value === 'string') {
        for (const payload of payloads) {
          const testUrl = url.replace(value, encodeURIComponent(payload));
          
          // Emit finding for testing
          await op_emit_finding({
            vuln_type: "sqli",
            severity: "high",
            title: `Potential SQL Injection in parameter: ${key}`,
            description: `Parameter '${key}' may be vulnerable to SQL injection`,
            endpoint: url,
            method: method,
            evidence: {
              parameter: key,
              original_value: value,
              test_payload: payload
            }
          });
        }
      }
    }
  },

  scan_response: async (ctx: ResponseContext) => {
    const { response, request } = ctx;
    
    // Check for SQL error messages in response
    const sqlErrors = [
      "You have an error in your SQL syntax",
      "mysql_fetch_array()",
      "mysqli_error",
      "SQL syntax.*MySQL",
      "Warning.*mysql_.*"
    ];
    
    for (const error of sqlErrors) {
      if (response.body && response.body.includes(error)) {
        await op_emit_finding({
          vuln_type: "sqli",
          severity: "critical",
          title: "SQL Error Message Detected",
          description: "Response contains SQL error message, indicating potential SQL injection vulnerability",
          endpoint: request.url,
          method: request.method,
          evidence: {
            error_pattern: error,
            response_snippet: response.body.substring(0, 200)
          }
        });
      }
    }
  }
};
```

#### 5.2 审核质量评分

- **Logic Score**: 检查是否包含必要的扫描逻辑
- **Security Score**: 检查是否使用了危险函数
- **Code Quality Score**: 检查代码规范性

#### 5.3 加载插件

```json
{
  "tool": "load_plugin",
  "parameters": {
    "code": "...插件代码...",
    "enabled": true
  }
}
```

或保存为文件后加载：

```json
{
  "tool": "load_plugin",
  "parameters": {
    "path": "/path/to/sqli_detector_mysql.ts",
    "enabled": true
  }
}
```

### Step 6: 执行被动扫描

插件启用后，继续浏览目标网站，插件会自动检测漏洞：

```
浏览器 → 代理 → 插件引擎 → 检测逻辑 → 发现漏洞
```

### Step 7: 查看漏洞发现

```json
{
  "tool": "list_findings",
  "parameters": {
    "severity": "high",
    "vuln_type": "sqli",
    "limit": 50
  }
}
```

**返回结果**：
```json
{
  "findings": [
    {
      "id": "find_001",
      "vuln_type": "sqli",
      "severity": "high",
      "title": "SQL Injection in user ID parameter",
      "description": "Parameter 'id' in /api/users is vulnerable to SQL injection",
      "endpoint": "https://example.com/api/users?id=1",
      "method": "GET",
      "evidence": {
        "parameter": "id",
        "payload": "' OR '1'='1",
        "response_time_diff": "250ms"
      },
      "plugin_name": "sqli_detector_mysql",
      "discovered_at": "2025-11-13T10:35:22Z",
      "status": "new"
    }
  ],
  "total": 1
}
```

## 高级用法

### 1. 针对特定端点生成插件

```json
{
  "vuln_types": ["idor"],
  "target_endpoints": [
    "/api/users/:id",
    "/api/orders/:id",
    "/profile/:userId"
  ],
  "requirements": "Check for horizontal privilege escalation by testing different user IDs"
}
```

### 2. 结合技术栈信息

```json
{
  "analysis": {
    "tech_stack": {
      "framework": "Django",
      "database": "PostgreSQL",
      "features": ["JWT Auth", "GraphQL"]
    }
  },
  "vuln_types": ["sqli"],
  "requirements": "Generate PostgreSQL-specific payloads, include GraphQL injection tests"
}
```

### 3. 迭代优化插件

如果生成的插件质量不高：

```json
{
  "requirements": "Previous plugin had low quality score. Please:
    1. Add more comprehensive payload list
    2. Include time-based blind SQLi detection
    3. Add better error message detection
    4. Improve code documentation"
}
```

## 最佳实践

### 1. 充分的网站探索
- ⏰ 至少浏览15-30分钟
- 🔍 覆盖主要功能模块
- 🧪 触发各种状态（登录/未登录、成功/失败）
- 📊 收集足够的API调用样本（建议 > 100个请求）

### 2. 合理的漏洞类型选择
- **SQL注入**: 适用于数据库查询端点
- **XSS**: 适用于用户输入展示的页面
- **IDOR**: 适用于资源访问控制端点
- **信息泄露**: 适用于错误页面、调试接口
- **CSRF**: 适用于状态修改操作

### 3. 质量阈值设置
- ✅ `quality_score >= 70`: 可以直接使用
- ⚠️ `quality_score 40-70`: 需要人工审核
- ❌ `quality_score < 40`: 建议重新生成

### 4. 插件组合策略
```
基础覆盖: sqli + xss + idor
深度检测: + csrf + info_leak + auth_bypass
特定场景: + file_upload + xxe + ssrf
```

## 故障排查

### 问题1: 分析结果为空
**原因**: 
- 代理未启动
- 未浏览目标网站
- 域名不匹配

**解决**:
```bash
# 检查代理状态
curl -x http://127.0.0.1:8080 https://example.com

# 检查数据库记录
SELECT COUNT(*) FROM proxy_requests WHERE host LIKE '%example.com%';
```

### 问题2: 插件生成失败
**原因**:
- AI服务未配置
- 分析结果不完整
- LLM响应超时

**解决**:
```json
// 检查AI服务
{
  "tool": "list_ai_services"
}

// 简化请求
{
  "vuln_types": ["sqli"],  // 只生成一个类型
  "requirements": "Simple detection logic"
}
```

### 问题3: 质量评分过低
**原因**:
- LLM生成的代码不完整
- 缺少必要的检测逻辑
- 包含不安全的代码

**解决**:
1. 增加更详细的`requirements`
2. 提供Few-shot示例（Day 6功能）
3. 手动修改代码后重新评分

## 性能优化

### 并行生成
```rust
// 对多个漏洞类型并行生成
let handles: Vec<_> = vuln_types.iter().map(|vuln_type| {
    let generator = generator.clone();
    tokio::spawn(async move {
        generator.generate_single(vuln_type).await
    })
}).collect();

let plugins = futures::future::join_all(handles).await;
```

### 缓存策略
- 缓存常见技术栈的prompt模板
- 缓存高质量插件代码作为示例
- 缓存网站分析结果（24小时）

## 安全建议

### 1. 代码审核
**必须审核的内容**:
- ❗ 所有`eval()`、`Function()`调用
- ❗ 网络请求（是否向外部发送数据）
- ❗ 文件系统访问
- ❗ 敏感信息处理

### 2. 沙箱执行
目前插件在Deno Core沙箱中执行，但仍需注意：
- ⚠️ 限制网络访问权限
- ⚠️ 限制文件系统访问
- ⚠️ 监控CPU和内存使用

### 3. 权限控制
- 🔒 只允许授权用户生成插件
- 🔒 插件审核后才能启用
- 🔒 记录所有插件操作日志

## 后续功能预告

### Day 5: 插件审核UI
- 🎨 可视化插件代码编辑器
- 📊 质量评分仪表板
- ✅ 一键批准/拒绝工作流

### Day 6: Few-shot学习
- 📚 高质量插件示例库
- 🧠 智能prompt优化
- 🔄 迭代式质量提升

### Day 7: 端到端集成
- 🚀 一键式完整工作流
- 📈 性能监控和优化
- 📖 完整的API文档

## 总结

方案B提供了强大的AI驱动插件生成能力：

✅ **智能分析**: 自动理解网站结构和技术栈
✅ **智能生成**: 基于上下文生成针对性检测插件
✅ **质量保证**: 多维度质量评分和验证
✅ **易于使用**: 简单的MCP工具接口

**适用场景**:
- 🎯 快速为新目标生成检测插件
- 🎯 针对特定技术栈定制检测逻辑
- 🎯 批量生成多类型安全检测插件
- 🎯 持续学习和优化检测能力

开始使用方案B，让AI成为你的安全测试助手！ 🚀

