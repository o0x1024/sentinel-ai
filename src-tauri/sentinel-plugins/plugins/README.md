# Sentinel AI 被动扫描插件开发指南

## 📚 概述

Sentinel AI 被动扫描系统使用 **Deno Core** 作为插件引擎，支持使用 JavaScript/TypeScript 编写自定义安全扫描插件。

## 🚀 快速开始

### 1. 插件目录

插件默认存放在：
```
~/.sentinel-ai/plugins/
```

### 2. 创建第一个插件

复制 `template.ts` 作为起点：

```bash
cp template.ts my-plugin.ts
```

修改插件元数据：

```typescript
function get_metadata(): PluginMetadata {
  return {
    id: "myname.sqli.basic",        // 唯一标识
    name: "Basic SQL Injection Scanner",
    version: "1.0.0",
    author: "Your Name",
    category: "sqli",
    default_severity: "high",
    tags: ["sql", "injection"],
    description: "Detects basic SQL injection patterns.",
  };
}
```

### 3. 实现扫描逻辑

#### 扫描请求（scan_request）

检测请求参数、头部、URL 中的安全问题：

```typescript
function scan_request(ctx: RequestContext): Finding[] {
  const findings: Finding[] = [];
  
  // 检测 SQL 注入符号
  for (const [key, value] of Object.entries(ctx.query_params)) {
    if (/['";]|--|\bOR\b/i.test(value)) {
      findings.push({
        vuln_type: "sqli",
        severity: "high",
        title: `SQL injection in ${key}`,
        description: "Detected SQL injection characters",
        evidence: value.slice(0, 100),
        location: `param:${key}`,
        confidence: "medium",
        cwe: "CWE-89",
      });
    }
  }
  
  return findings;
}
```

#### 扫描响应（scan_response）

检测响应体、头部中的安全问题：

```typescript
function scan_response(ctx: CombinedContext): Finding[] {
  const findings: Finding[] = [];
  
  const responseBody = decodeBody(ctx.response.body);
  
  // 检测数据库错误
  if (/mysql_fetch|SQL syntax/i.test(responseBody)) {
    findings.push({
      vuln_type: "sqli",
      severity: "critical",
      title: "SQL error in response",
      description: "Database error disclosed",
      evidence: responseBody.slice(0, 200),
      location: "response:body",
      confidence: "high",
    });
  }
  
  return findings;
}
```

### 4. 测试插件

将插件文件放入插件目录后，重启 Sentinel AI 或使用插件管理命令：

```javascript
// 在 Tauri 前端调用
await invoke('load_plugin', { path: '/path/to/my-plugin.ts' });
await invoke('enable_plugin', { pluginId: 'myname.sqli.basic' });
```

## 📋 插件 API 参考

### 必须实现的函数

#### `get_metadata(): PluginMetadata`

返回插件元数据，在插件加载时调用一次。

**返回值**:
```typescript
{
  id: string;           // 唯一标识（建议格式：作者.类别.名称）
  name: string;         // 显示名称
  version: string;      // 语义化版本
  author?: string;      // 作者
  category: string;     // 漏洞类别（sqli/xss/csrf 等）
  default_severity: "critical" | "high" | "medium" | "low" | "info";
  tags?: string[];      // 标签（用于筛选）
  description?: string; // 描述
}
```

#### `scan_request(ctx: RequestContext): Finding[]`

扫描 HTTP 请求，返回发现的漏洞数组。

**参数**:
- `ctx.id`: 请求 ID（UUID）
- `ctx.method`: HTTP 方法（GET/POST/PUT 等）
- `ctx.url`: 完整 URL
- `ctx.headers`: 请求头（键值对）
- `ctx.body`: 请求体（Uint8Array，最多 2MB）
- `ctx.query_params`: 查询参数
- `ctx.is_https`: 是否 HTTPS

**返回值**: `Finding[]` - 漏洞列表（没有发现返回 `[]`）

#### `scan_response(ctx: CombinedContext): Finding[]`

扫描 HTTP 响应，返回发现的漏洞数组。

**参数**:
- `ctx.request`: 请求上下文（同 `scan_request`）
- `ctx.response`: 响应上下文
  - `request_id`: 关联的请求 ID
  - `status`: HTTP 状态码
  - `headers`: 响应头
  - `body`: 响应体（Uint8Array）

**返回值**: `Finding[]`

### 可选函数

#### `init(config?: Record<string, any>): void`

插件初始化函数，在插件首次启用时调用。

**使用场景**:
- 加载配置文件
- 编译正则表达式缓存
- 建立外部服务连接

### Finding 结构

```typescript
interface Finding {
  vuln_type: string;       // 漏洞类型（与 category 一致）
  severity: "critical" | "high" | "medium" | "low" | "info";
  title: string;           // 标题（简短描述）
  description: string;     // 详细描述
  evidence: string;        // 证据片段（原始数据，不脱敏）
  location: string;        // 位置（param:xxx / header:xxx / body / response:body）
  confidence: "high" | "medium" | "low";
  cwe?: string;            // CWE 标签（如 CWE-89）
  owasp?: string;          // OWASP Top 10 标签（如 A03:2021）
  remediation?: string;    // 修复建议
}
```

## 🛠️ 工具函数

插件引擎提供了常用工具函数（在 `template.ts` 中定义）：

### `decodeBody(body: Uint8Array): string`

将二进制请求/响应体解码为 UTF-8 字符串。

```typescript
const bodyText = decodeBody(ctx.body);
```

### `truncate(str: string, maxLen: number = 200): string`

截断字符串，用于证据展示。

```typescript
evidence: truncate(longString, 100)
```

## 📝 最佳实践

### 1. 性能优化

- ✅ 使用高效的正则表达式
- ✅ 避免在循环中重复创建对象
- ✅ 提前编译正则（在 `init()` 中）
- ❌ 不要在插件中执行阻塞操作（如同步文件 I/O）

### 2. 证据收集

- ✅ 保留原始证据（不脱敏）
- ✅ 截断过长的证据（使用 `truncate()`）
- ✅ 明确标注位置（`param:xxx`、`header:Authorization`）

### 3. 置信度评估

- `high`: 确认的漏洞（如数据库错误+注入符号）
- `medium`: 可疑模式（如仅有注入符号）
- `low`: 弱信号（如敏感路径）

### 4. 严重等级

- `critical`: 确认的严重漏洞（SQL 注入、RCE）
- `high`: 潜在严重漏洞（XSS、敏感信息泄露）
- `medium`: 中等风险（缺失安全头）
- `low`: 低风险（信息泄露）
- `info`: 仅供参考（不算漏洞）

## 🎯 插件示例

### SQL 注入检测

```typescript
function scan_request(ctx: RequestContext): Finding[] {
  const findings: Finding[] = [];
  const sqlPatterns = [
    /['";]/,
    /--/,
    /\/\*/,
    /\bOR\b.*=.*=\b/i,
    /\bUNION\b.*\bSELECT\b/i,
  ];

  for (const [key, value] of Object.entries(ctx.query_params)) {
    for (const pattern of sqlPatterns) {
      if (pattern.test(value)) {
        findings.push({
          vuln_type: "sqli",
          severity: "high",
          title: `SQL injection in ${key}`,
          description: `Parameter contains SQL metacharacters`,
          evidence: truncate(value, 100),
          location: `param:${key}`,
          confidence: "medium",
          cwe: "CWE-89",
        });
        break;
      }
    }
  }
  return findings;
}

function scan_response(ctx: CombinedContext): Finding[] {
  const findings: Finding[] = [];
  const body = decodeBody(ctx.response.body);
  
  const errorPatterns = [
    /mysql_fetch/i,
    /You have an error in your SQL syntax/i,
    /ORA-\d{5}/,
  ];

  for (const pattern of errorPatterns) {
    if (pattern.test(body)) {
      findings.push({
        vuln_type: "sqli",
        severity: "critical",
        title: "SQL error in response",
        description: "Database error disclosed, confirming SQL injection",
        evidence: truncate(body.match(pattern)![0], 200),
        location: "response:body",
        confidence: "high",
        cwe: "CWE-89",
      });
    }
  }
  return findings;
}
```

### XSS 检测（反射型）

```typescript
function scan_response(ctx: CombinedContext): Finding[] {
  const findings: Finding[] = [];
  const responseBody = decodeBody(ctx.response.body);

  // 检测反射点
  for (const [key, value] of Object.entries(ctx.request.query_params)) {
    if (/<script|onerror|onclick/i.test(value) && responseBody.includes(value)) {
      findings.push({
        vuln_type: "xss",
        severity: "high",
        title: `Reflected XSS via ${key}`,
        description: `Parameter value is reflected in response without encoding`,
        evidence: truncate(value, 100),
        location: `param:${key}`,
        confidence: "high",
        cwe: "CWE-79",
        owasp: "A03:2021",
      });
    }
  }
  return findings;
}
```

## 🔒 安全与限制

### 权限

- 插件默认运行在 **全权限** 模式（`--allow-all`）
- 未来版本可能添加权限审批机制

### 限制

- ❌ 不支持 Node.js 特定 API（如 `fs`、`process`）
- ❌ 不支持 `require()` 或 `import`（Deno Core 环境）
- ✅ 支持所有 ES2022 标准特性
- ✅ 支持 `console.log()` 用于调试

### 数据隐私

- ⚠️ 插件可访问完整的请求/响应数据（包括 Cookie、Token）
- ⚠️ 不要将敏感数据记录到日志或外部服务
- ⚠️ 仅在授权测试环境中使用

## 📦 插件发布

### 目录结构

推荐的插件发布结构：

```
my-awesome-plugin/
├── plugin.ts           # 主文件
├── README.md           # 说明文档
├── LICENSE             # 开源协议
└── tests/              # 测试用例（可选）
```

### 命名规范

- 插件 ID: `作者.类别.名称`（如 `john.sqli.advanced`）
- 文件名: 小写、短横线分隔（如 `sql-injection-advanced.ts`）

## 🐛 调试技巧

### 1. 使用 console.log

```typescript
function scan_request(ctx: RequestContext): Finding[] {
  console.log("Scanning URL:", ctx.url);
  console.log("Query params:", ctx.query_params);
  // ...
}
```

### 2. 查看日志

插件输出会记录到 Sentinel AI 日志文件：

```bash
tail -f ~/.sentinel-ai/logs/passive-scan.log
```

### 3. 测试插件元数据

在浏览器控制台测试：

```javascript
const metadata = await invoke('get_plugin_metadata', { 
  pluginId: 'myname.sqli.basic' 
});
console.log(metadata);
```

## 📚 参考资源

- [Deno 标准库](https://deno.land/std)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE List](https://cwe.mitre.org/)
- [Sentinel AI GitHub](https://github.com/sentinel-ai/sentinel-ai)

## 💡 贡献

欢迎提交优秀插件到官方仓库！

1. Fork 项目
2. 创建插件分支
3. 编写插件和测试用例
4. 提交 Pull Request

---

**Happy Hacking! 🚀**
