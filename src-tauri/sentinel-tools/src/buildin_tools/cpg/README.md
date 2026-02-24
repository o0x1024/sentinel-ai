# CPG (Code Property Graph) 引擎使用说明

## 概述

CPG 引擎是 Sentinel AI 的代码结构分析核心,基于 tree-sitter AST 解析 + petgraph 图引擎构建,
支持 **11 种编程语言** 的自动化安全审计。

### 支持的语言
Rust, JavaScript, TypeScript, Python, Java, Go, C, C++, C#, PHP, Ruby

### 核心能力
- 🔍 **代码结构分析** — 函数/类/导入/调用关系一目了然
- 🎯 **图遍历污点追踪** — 通过实际调用图(非正则)追踪 source → sink 数据流
- 🛡️ **11 种漏洞规则** — SQL注入、XSS、命令注入、路径穿越等
- 🔗 **跨文件分析** — 通过调用边自动追踪跨文件数据流
- ⚡ **自动上下文注入** — 审计模式自动将项目结构注入 AI 上下文

---

## 使用方式

### 方式一: 审计模式(推荐,全自动)

1. **打开 Sentinel AI 应用,进入 AI Agent 对话界面**
2. **开启审计模式:** 点击对话界面右上方的工具配置(齿轮图标),打开 **Audit Mode** 开关
   - 可选配置: 审计范围(repo/git_diff/paths)、验证级别(low/medium/high)、策略(balanced/strict)
3. **在对话输入框中输入审计指令:**

```
请审计 /path/to/your/project 项目的安全性
```

4. **查看审计结果:** 审计完成后,发现的漏洞会自动保存。前往 **安全中心 > 代码审计** 页面查看所有持久化的审计发现

AI 会自动:
- ✅ 调用 `build_cpg` 构建代码属性图
- ✅ 调用 `cpg_security_scan` 运行全规则扫描
- ✅ 对高风险发现调用 `cpg_taint_analysis` 做精确追踪
- ✅ 读取源码确认漏洞
- ✅ 通过 `audit_finding_upsert` 持久化发现

**首次审计自动执行流程:**
```
build_cpg → cpg_security_scan → cpg_taint_analysis → 读源码 → 报告
```

### 方式二: 手动工具调用(适合开发调试)

在 AI 对话中直接请求使用具体工具:

```
请用 build_cpg 工具分析 /Users/like/code/my-project
```

```
用 cpg_security_scan 扫描 /Users/like/code/my-project 的安全风险
```

```
分析 /Users/like/code/my-project 中的 SQL 注入风险
```

---

## 4 个 AI 工具详解

### 1. `build_cpg` — 构建代码属性图

**功能:** 解析项目所有源文件,构建代码结构图(函数/类/调用关系)

**参数:**
| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `path` | string | 必填 | 项目根目录路径 |
| `max_files` | number | 5000 | 最大解析文件数 |
| `force` | boolean | false | 强制重建(忽略缓存) |

**AI 调用示例:**
```json
{
  "path": "/workspace/target-project",
  "max_files": 5000,
  "force": false
}
```

**返回值:**
```json
{
  "success": true,
  "root": "/workspace/target-project",
  "primary_language": "javascript",
  "languages": ["javascript", "typescript"],
  "total_files": 156,
  "total_functions": 420,
  "total_classes": 89,
  "total_imports": 312,
  "total_call_edges": 201,
  "total_nodes": 1580,
  "total_edges": 892
}
```

**注意事项:**
- CPG 在内存中缓存,同一项目不重复构建
- 自动跳过 `.git`, `node_modules`, `target`, `vendor` 等目录
- 大文件(>1MB)会被跳过

---

### 2. `query_cpg` — 查询代码结构

**功能:** 对已构建的 CPG 执行结构查询

**9 种查询类型:**

#### `summary` — 项目概况
```json
{ "path": "/workspace/project", "query": { "type": "summary" } }
```

#### `functions` — 列出函数(可按文件过滤)
```json
{ "path": "/workspace/project", "query": { "type": "functions", "limit": 50 } }
{ "path": "/workspace/project", "query": { "type": "functions", "file": "src/auth.js", "limit": 50 } }
```

#### `classes` — 列出类/结构体
```json
{ "path": "/workspace/project", "query": { "type": "classes", "limit": 50 } }
```

#### `imports` — 列出导入
```json
{ "path": "/workspace/project", "query": { "type": "imports", "limit": 50 } }
```

#### `call_edges` — 函数调用关系
```json
{ "path": "/workspace/project", "query": { "type": "call_edges", "limit": 50 } }
```

#### `callers_of` — 谁调用了函数 X?
```json
{ "path": "/workspace/project", "query": { "type": "callers_of", "function_name": "executeQuery" } }
```

#### `callees_of` — 函数 X 调用了谁?
```json
{ "path": "/workspace/project", "query": { "type": "callees_of", "function_name": "handleRequest" } }
```

#### `files` — 文件列表(含复杂度指标)
```json
{ "path": "/workspace/project", "query": { "type": "files", "limit": 30 } }
```

#### `search` — 按名称搜索符号
```json
{ "path": "/workspace/project", "query": { "type": "search", "query": "auth", "limit": 20 } }
```

---

### 3. `cpg_taint_analysis` — 精确污点分析

**功能:** 基于图遍历的 source → sink 数据流追踪

**参数:**
| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `path` | string | 必填 | 项目根目录 |
| `rules` | string[] | 全部规则 | 要检查的规则ID列表 |
| `max_depth` | number | 8 | 最大调用链追踪深度 |
| `max_findings_per_rule` | number | 30 | 每条规则最多报告多少发现 |

**可用规则 ID:**
| ID | 漏洞类型 | CWE | 严重级别 |
|----|---------|-----|---------|
| `sql_injection` | SQL 注入 | CWE-89 | Critical |
| `xss` | 跨站脚本 | CWE-79 | High |
| `command_injection` | 命令注入 | CWE-78 | Critical |
| `path_traversal` | 路径穿越 | CWE-22 | High |
| `ssrf` | 服务端请求伪造 | CWE-918 | High |
| `deserialization` | 不安全反序列化 | CWE-502 | Critical |
| `ldap_injection` | LDAP 注入 | CWE-90 | High |
| `xxe` | XML 外部实体 | CWE-611 | High |
| `open_redirect` | 开放重定向 | CWE-601 | Medium |
| `log_injection` | 日志注入 | CWE-117 | Medium |

**AI 调用示例:**
```json
{
  "path": "/workspace/project",
  "rules": ["sql_injection", "command_injection"],
  "max_depth": 10
}
```

**返回值:**
```json
{
  "total_sources": 15,
  "total_sinks": 8,
  "total_findings": 3,
  "unsanitized_findings": 2,
  "findings": [
    {
      "rule_id": "sql_injection",
      "rule_name": "SQL Injection",
      "cwe": "CWE-89",
      "severity": "critical",
      "source": {
        "name": "req.body",
        "file": "src/controllers/user.js",
        "line": 25
      },
      "sink": {
        "name": "db.query",
        "file": "src/models/user.js",
        "line": 42
      },
      "trace_path": [
        { "name": "handleCreate", "file": "src/controllers/user.js", "line": 20 },
        { "name": "saveUser", "file": "src/models/user.js", "line": 38 }
      ],
      "distance": 2,
      "sanitized": false,
      "confidence": 0.90
    }
  ]
}
```

---

### 4. `cpg_security_scan` — 全面安全扫描

**功能:** 运行所有规则的基线安全评估(污点分析 + 模式匹配)

**参数:**
| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `path` | string | 必填 | 项目根目录 |
| `max_depth` | number | 6 | 最大调用链深度 |
| `max_findings` | number | 100 | 最大发现数 |

**AI 调用示例:**
```json
{
  "path": "/workspace/project"
}
```

**返回值:**
```json
{
  "total_rules": 11,
  "total_findings": 12,
  "by_severity": {
    "critical": 3,
    "high": 5,
    "medium": 3,
    "low": 1,
    "info": 0
  },
  "findings": [...],
  "pattern_findings": [
    {
      "rule_id": "hardcoded_secrets",
      "rule_name": "Hardcoded Secrets/Credentials",
      "file": "src/config.js",
      "line": 15,
      "name": "API_SECRET",
      "description": "Variable 'API_SECRET' suggests sensitive data."
    }
  ]
}
```

---

## 典型审计流程

### 快速扫描(5 分钟)
```
用户: 扫描 /path/to/project 的安全风险
AI:
  1. build_cpg → 解析 200 文件, 500 函数
  2. cpg_security_scan → 发现 8 个风险 (Critical: 2, High: 3, Medium: 3)
  3. 输出摘要报告
```

### 深度审计(30+ 分钟)
```
用户: 对 /path/to/project 进行全面安全审计
AI:
  1. build_cpg → 构建代码结构图
  2. query_cpg(files) → 识别重要文件
  3. cpg_security_scan → 基线扫描
  4. cpg_taint_analysis(sql_injection) → 精确追踪 SQL 注入      ← 循环
  5. read_file → 阅读源码确认                                   ← 每个发现
  6. audit_finding_upsert → 持久化确认的漏洞                     ←
  7. cpg_taint_analysis(xss) → 追踪 XSS
  8. cpg_taint_analysis(command_injection) → 追踪命令注入
  9. dependency_audit → 依赖项审计
  10. audit_report → 生成审计报告
```

### 定向分析
```
用户: 检查 /path/to/project 中是否存在 SQL 注入漏洞
AI:
  1. build_cpg
  2. cpg_taint_analysis(rules: ["sql_injection"], max_depth: 12)
  3. 对每个发现读源码确认
  4. 输出 SQL 注入专项报告
```

---

## 上下文自动注入(Phase 3)

当处于审计模式时,如果 CPG 已缓存,AI 的系统提示词会自动包含:

```xml
<code_structure>
Project: my-app (javascript, 156 files, 420 functions, 89 classes)
Languages: javascript, typescript
Graph: 1580 nodes, 892 edges, 201 call edges

Key Files (by complexity):
  src/services/UserService.js (javascript, 450L, 15 fn, 2 cls)
  src/controllers/AuthController.js (javascript, 320L, 12 fn, 1 cls) ⚠ complex
  ...

Most-Called Functions (high fan-in → critical):
  executeQuery (called 23 times)
  validateToken (called 18 times)
  ...

⚠ Auto-detected High-Risk Data Flows:
  SQL Injection (CWE-89, critical) — 3 flow(s), 2 unsanitized
    req.body (src/controllers/user.js:25) → db.query (src/models/user.js:42)
  ...
</code_structure>
```

这使 AI 从第一轮对话就掌握项目全局,无需额外探索。

---

## 工作流技能(Phase 4)

CPG 审计工作流会自动安装为技能(`cpg-code-audit`),引导 AI 按 5 阶段进行审计:

1. **🔍 侦察** — build_cpg + 项目概视
2. **📊 基线扫描** — cpg_security_scan + 创建审计计划
3. **🎯 深度分析** — cpg_taint_analysis + 读源码 + 确认漏洞
4. **🔗 交叉分析** — 依赖审计 + 全局模式检查
5. **📝 报告** — tenth_man_review + audit_report

---

## 与现有工具对比

| 特性 | cross_file_taint | **cpg_taint_analysis** |
|------|:---:|:---:|
| 分析方式 | 正则+函数名启发 | **AST图遍历** |
| 跨文件追踪 | ✅ (启发式) | ✅ (**实际调用图**) |
| 净化器检测 | ❌ | ✅ |
| 调用链追踪 | 部分 | ✅ (完整路径) |
| 置信度评分 | 字符串匹配 | **图距离+净化器** |
| 语言感知 | ❌ | ✅ (per-language rules) |
| 需要外部工具 | rg | **无(内置)** |
| 速度 | 快 | 中 | 构建慢/查询快 |

---

## Docker 环境

在 Docker 审计环境中,使用 `/workspace` 作为项目路径:

```
build_cpg({ path: "/workspace" })
cpg_security_scan({ path: "/workspace" })
```

---

## 常见问题

**Q: CPG 构建太慢?**
A: 使用 `max_files` 参数限制文件数: `build_cpg({ path: "...", max_files: 500 })`

**Q: 找不到漏洞?**
A: 先检查 `query_cpg(summary)` 确认文件是否被解析。如果是不支持的语言则无法分析。

**Q: 如何只检查特定规则?**
A: 使用 `cpg_taint_analysis` 的 `rules` 参数: `{ rules: ["sql_injection", "xss"] }`

**Q: CPG 缓存何时过期?**
A: 当前实现中,CPG 在应用运行期间缓存,重启后丢失。可以用 `force: true` 强制重建。
