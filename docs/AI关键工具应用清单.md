# Sentinel-AI 项目 - AI关键工具应用清单

## 一、LLM（大语言模型）服务集成

### 1.1 支持的LLM提供商

项目通过统一的 `AiProvider` 枚举支持多个主流LLM服务商：

| 提供商 | 用途 | 推荐模型 | 特点 |
|--------|------|---------|------|
| **OpenAI** | 插件生成、任务规划、代码修复 | GPT-4, GPT-4-Turbo, GPT-3.5-Turbo | 代码生成能力强，理解准确 |
| **Anthropic** | 插件生成、复杂推理 | Claude-3.5-Sonnet, Claude-3-Opus | 长上下文，安全性好 |
| **Google Gemini** | 多模态任务、快速交互 | Gemini-Pro, Gemini-1.5-Pro | 多模态支持，速度快 |
| **XAI (Grok)** | 实时信息查询 | Grok-1 | 联网能力 |
| **DeepSeek** | 代码生成、低成本推理 | DeepSeek-Chat, DeepSeek-Coder | 高性价比，编程能力强 |
| **Ollama** | 本地部署、隐私保护 | Llama 3, Qwen, CodeLlama | 离线可用，数据安全 |
| **Groq** | 快速推理 | Mixtral, Llama 3 | 推理速度极快 |
| **Cohere** | 文本生成、分类 | Command, Command-R | 企业级API |
| **ModelScope** | 国产模型 | 通义千问等 | 国内访问稳定 |
| **OpenRouter** | 多模型聚合 | 100+ 模型 | 一个API访问多个模型 |
| **Custom** | 自定义提供商 | 自部署模型 | 完全自主可控 |

**实现位置**：
- 模型定义：`src-tauri/src/models/ai.rs`
- 统一客户端：`src-tauri/src/services/ai.rs`

### 1.2 LLM核心应用场景

#### 场景1：AI驱动的插件代码生成
```rust
// 应用：根据网站分析结果生成TypeScript检测插件
let prompt = build_generation_prompt(analysis, vuln_type);
let plugin_code = llm_client.generate(prompt).await?;

// 支持的漏洞类型：
// - SQL注入 (sqli)
// - XSS (xss)
// - IDOR (idor)
// - 认证绕过 (auth_bypass)
// - 信息泄露 (info_leak)
// - 文件上传 (file_upload)
// - 命令注入 (command_injection)
// - 路径遍历 (path_traversal)
// - XXE (xxe)
// - SSRF (ssrf)
```

#### 场景2：任务规划与分解
```rust
// 应用：将用户的复杂安全测试需求分解为可执行步骤
// 使用的引擎：Plan-and-Execute, ReWOO, Orchestrator
let plan = planner.create_plan(user_request).await?;
// 输出：包含顺序步骤和依赖关系的执行计划
```

#### 场景3：代码自动修复
```rust
// 应用：当生成的插件代码有语法或逻辑错误时，自动修复
let fix_prompt = build_fix_prompt(code, error, attempt);
let fixed_code = llm_client.generate(fix_prompt).await?;
// 最多尝试3次修复
```

#### 场景4：结果汇总与报告生成
```rust
// 应用：LLM Compiler引擎汇总并行任务结果
let final_report = joiner.synthesize_results(task_results).await?;
```

---

## 二、Rig框架（Rust LLM工具库）

### 2.1 Rig-Core

**版本**：0.24.0  
**作用**：Rust生态的LLM应用开发框架，提供Agent、Tool、Prompt等抽象

**依赖配置**：
```toml
[dependencies]
rig-core = { version = "0.24.0", features = ["derive"] }
```

**主要功能**：
- **Agent抽象**：构建AI Agent的基础框架
- **Tool调用**：统一的工具调用接口
- **Prompt管理**：模板化的提示词管理
- **流式响应**：支持流式输出（虽然StreamingPromptHook是私有的）

**应用示例**：
```rust
// 创建支持工具调用的AI Agent
let agent = rig_core::Agent::builder()
    .model(model)
    .tools(tools)
    .build()?;

// 执行带工具调用的推理
let response = agent.prompt(&user_query).await?;
```

**实现位置**：
- `src-tauri/src/services/ai.rs` - AI服务实现
- `src-tauri/src/engines/` - 各个AI引擎实现

### 2.2 Rig-LanceDB

**版本**：0.2.27  
**作用**：Rig与LanceDB的集成，用于RAG（检索增强生成）

**依赖配置**：
```toml
[dependencies]
rig-lancedb = "0.2.27"
```

**主要功能**：
- **向量存储集成**：将Rig Agent与LanceDB向量数据库连接
- **自动Embedding**：文本自动向量化
- **语义检索**：基于向量相似度的检索

**应用场景**：
```rust
// RAG服务初始化
let vector_store = LanceDbManager::new(db_path, embedding_config);
vector_store.initialize().await?;

// 查询相似文档
let results = vector_store.search_similar(collection, query, top_k).await?;
```

**实现位置**：
- `src-tauri/sentinel-rag/src/service.rs` - RAG服务主实现

---

## 三、LanceDB（向量数据库）

### 3.1 基本信息

**版本**：0.22.3  
**类型**：嵌入式向量数据库  
**存储格式**：Apache Arrow + Lance格式

**依赖配置**：
```toml
[dependencies]
lancedb = "0.22.3"
arrow-array = "55.2.0"
arrow-schema = "55.2.0"
```

### 3.2 核心功能

#### 功能1：文档向量化存储
```rust
// 存储文档块及其向量表示
pub async fn insert_chunks(
    &self,
    collection_name: &str,
    chunks: Vec<DocumentChunk>,
) -> Result<usize> {
    // 1. 为每个chunk生成embedding
    // 2. 存储到LanceDB表中
    // 3. 返回插入数量
}
```

#### 功能2：语义相似度搜索
```rust
// 基于向量相似度的语义搜索
pub async fn search_similar(
    &self,
    collection_name: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<QueryResult>> {
    // 1. 将查询文本向量化
    // 2. 在LanceDB中进行向量检索
    // 3. 返回最相似的文档块
}
```

#### 功能3：集合管理
```rust
// 创建向量集合（表）
pub async fn create_collection(
    &self,
    name: &str,
    dimensions: usize,
) -> Result<()> {
    // 指定向量维度（如768维）
}

// 删除集合
pub async fn delete_collection(&self, name: &str) -> Result<()>
```

### 3.3 存储路径与配置

**默认路径**：`{APP_DATA_DIR}/lancedb/`  
**示例**（macOS）：`/Users/username/Library/Application Support/sentinel-ai/lancedb/`

**向量维度配置**：
- OpenAI text-embedding-3-small: 1536维
- OpenAI text-embedding-3-large: 3072维
- Cohere embed-v3: 1024维
- 自定义模型：可配置

### 3.4 应用场景

| 场景 | 描述 | 示例 |
|------|------|------|
| **知识库检索** | 存储安全文档、历史漏洞报告 | 用户查询"如何检测SQL注入"，系统自动检索相关知识 |
| **相似漏洞查找** | 基于漏洞特征查找历史相似案例 | 发现新漏洞时，查找是否有类似的历史记录 |
| **插件推荐** | 根据目标网站特征推荐合适的检测插件 | 分析到网站使用MySQL，推荐MySQL相关的SQL注入插件 |
| **学习增强** | AI从历史成功案例中学习 | 生成插件前，检索类似网站的成功检测案例 |

**实现位置**：
- `src-tauri/sentinel-rag/src/database.rs` - LanceDB管理器
- `src-tauri/sentinel-rag/src/service.rs` - RAG服务

---

## 四、Deno Core（JS/TS运行时）

### 4.1 基本信息

**版本**：0.368.0  
**作用**：嵌入式JavaScript/TypeScript运行时，基于V8引擎

**依赖配置**：
```toml
[dependencies]
deno_core = "0.368.0"
deno_ast = { version = "0.51.0", features = ["transpiling"] }
```

### 4.2 核心应用：插件沙箱执行

#### 架构设计
```
┌─────────────────────────────────────┐
│   Rust 主进程                        │
│   ┌───────────────────────────────┐ │
│   │ Deno Core 运行时              │ │
│   │  ┌─────────────────────────┐ │ │
│   │  │ V8 隔离沙箱             │ │ │
│   │  │  ┌───────────────────┐ │ │ │
│   │  │  │ AI生成的插件代码  │ │ │ │
│   │  │  │ (TypeScript)      │ │ │ │
│   │  │  └───────────────────┘ │ │ │
│   │  │                         │ │ │
│   │  │ 内置API:               │ │ │
│   │  │ - fetch()              │ │ │
│   │  │ - TextDecoder          │ │ │
│   │  │ - op_emit_finding()    │ │ │
│   │  │ - op_plugin_log()      │ │ │
│   │  └─────────────────────────┘ │ │
│   └───────────────────────────────┘ │
└─────────────────────────────────────┘
```

#### 安全隔离特性
- ✅ **内存隔离**：每个插件运行在独立的V8隔离区
- ✅ **权限控制**：仅允许调用预定义的Rust Op函数
- ✅ **资源限制**：可设置CPU、内存限制
- ✅ **无文件系统访问**：插件无法直接访问磁盘
- ✅ **网络受限**：仅通过`fetch` API发起网络请求

#### 插件生命周期
```rust
// 1. 创建Deno运行时
let mut runtime = JsRuntime::new(RuntimeOptions {
    extensions: vec![
        // 注入自定义Op函数
        deno_core::Extension::builder("sentinel_ops")
            .ops(vec![
                op_emit_finding::decl(),
                op_plugin_log::decl(),
            ])
            .build(),
    ],
    ..Default::default()
});

// 2. 加载插件代码（AI生成的TypeScript）
runtime.execute_script("plugin.ts", plugin_code)?;

// 3. 调用插件函数
let result = runtime.call_function("scan_response", ctx)?;

// 4. 收集漏洞发现
let findings = extract_findings(result)?;
```

#### 内置API列表

| API | 描述 | 用途 |
|-----|------|------|
| `fetch(url, options)` | HTTP请求 | 插件可以调用外部API进行辅助检测 |
| `TextDecoder/TextEncoder` | 文本编解码 | 处理HTTP请求/响应体 |
| `URL/URLSearchParams` | URL解析 | 提取和分析URL参数 |
| `Deno.core.ops.op_emit_finding()` | 发出漏洞告警 | 插件发现漏洞时调用 |
| `Deno.core.ops.op_plugin_log()` | 日志输出 | 插件调试和信息记录 |

#### 示例：AI生成的插件代码
```typescript
// 插件元数据
function get_metadata(): PluginMetadata {
    return {
        id: "ai_gen_sqli_example_com",
        name: "SQL Injection Detector for example.com",
        category: "sqli",
        version: "1.0.0",
        author: "AI Generated",
    };
}

// 扫描响应
export function scan_response(ctx: CombinedContext): void {
    const responseText = decodeBody(ctx.response.body);
    
    // MySQL错误特征检测
    const mysqlErrors = [
        /You have an error in your SQL syntax/i,
        /mysql_fetch_array\(\)/i,
    ];
    
    for (const pattern of mysqlErrors) {
        if (pattern.test(responseText)) {
            // 发出漏洞告警
            Deno.core.ops.op_emit_finding({
                vuln_type: "sqli",
                severity: "critical",
                confidence: "high",
                url: ctx.request.url,
                evidence: truncate(responseText, 200),
                description: "Detected MySQL error in response, indicating SQL injection vulnerability",
            });
        }
    }
}

// 导出到全局作用域
globalThis.get_metadata = get_metadata;
globalThis.scan_response = scan_response;
```

**实现位置**：
- `src-tauri/sentinel-plugins/src/engine.rs` - 插件引擎
- `src-tauri/src/generators/` - 插件生成器

---

## 五、Embedding（文本向量化）服务

### 5.1 支持的Embedding模型

| 提供商 | 模型 | 维度 | 价格 | 用途 |
|--------|------|------|------|------|
| **OpenAI** | text-embedding-3-small | 1536 | $0.02/1M tokens | 通用文本向量化 |
| **OpenAI** | text-embedding-3-large | 3072 | $0.13/1M tokens | 高精度向量化 |
| **Cohere** | embed-multilingual-v3 | 1024 | $0.10/1M tokens | 多语言支持 |
| **本地模型** | sentence-transformers | 384-768 | 免费 | 离线使用 |

### 5.2 向量化配置
```rust
// RAG配置
pub struct EmbeddingConfig {
    pub provider: String,       // "openai", "cohere", etc.
    pub model: String,          // "text-embedding-3-small"
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub dimensions: Option<usize>, // 1536, 3072, etc.
}
```

### 5.3 向量化流程
```rust
// 1. 文档导入时自动向量化
pub async fn import_document(&self, file_path: &str) -> Result<()> {
    // 解析文档
    let content = parse_document(file_path)?;
    
    // 分块（500字符，重叠50）
    let chunks = chunk_text(&content, 500, 50);
    
    // 批量生成Embedding
    let embeddings = self.generate_embeddings(&chunks).await?;
    
    // 存储到LanceDB
    for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
        self.vector_store.insert(chunk, embedding).await?;
    }
}

// 2. 查询时向量化
pub async fn query(&self, query: &str) -> Result<Vec<Document>> {
    // 将查询文本向量化
    let query_embedding = self.generate_embedding(query).await?;
    
    // 向量检索
    let results = self.vector_store.search(query_embedding, top_k).await?;
    
    Ok(results)
}
```

### 5.4 相似度计算

**余弦相似度**：
```rust
pub fn cosine_similarity(vec1: &[f32], vec2: &[f32]) -> f64 {
    let dot_product: f32 = vec1.iter()
        .zip(vec2.iter())
        .map(|(a, b)| a * b)
        .sum();
    
    let magnitude1: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude2: f32 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    (dot_product / (magnitude1 * magnitude2)) as f64
}
```

**实现位置**：
- `src-tauri/sentinel-rag/src/embeddings.rs` - Embedding管理器
- `src-tauri/sentinel-db/src/database_service.rs` - 向量相似度搜索

---

## 六、Reranking（重排序）服务

### 6.1 作用

在向量检索后，使用更精确的模型对结果重新排序，提升检索精度。

**工作流程**：
```
用户查询
  ↓
文本向量化 (Embedding)
  ↓
向量相似度搜索 (LanceDB) → 获得Top-100结果
  ↓
Reranking (更精确的模型) → 重新排序
  ↓
返回Top-10最相关结果
```

### 6.2 支持的Reranking模型

| 提供商 | 模型 | 价格 | 特点 |
|--------|------|------|------|
| **Cohere** | rerank-english-v3.0 | $2.00/1M tokens | 英文优化 |
| **Cohere** | rerank-multilingual-v3.0 | $2.00/1M tokens | 多语言支持 |
| **Jina AI** | jina-reranker-v1-base | 免费 | 开源模型 |

### 6.3 配置与使用
```rust
// 创建Reranking管理器
let reranker = create_reranking_provider(
    "cohere",
    "rerank-english-v3.0",
    base_url,
    api_key,
)?;

// 对检索结果重排序
let reranked = reranker.rerank(
    query,           // 用户查询
    &documents,      // 待排序的文档列表
    top_k,           // 返回前K个
).await?;
```

**实现位置**：
- `src-tauri/sentinel-rag/src/embeddings.rs` - Reranking实现

---

## 七、Prompt工程工具

### 7.1 模板管理系统

**数据库存储**：
```sql
CREATE TABLE prompts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    template_type TEXT NOT NULL, -- PluginGeneration, PluginFix, etc.
    content TEXT NOT NULL,
    priority INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT true,
    is_default BOOLEAN DEFAULT false,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

**模板类型**：
```rust
pub enum TemplateType {
    PluginGeneration,   // AI插件生成主模板
    PluginFix,          // 插件代码修复模板
    PluginInterface,    // 插件接口说明模板
    PluginOutputFormat, // 插件输出格式模板
    ReactPrompt,        // ReAct引擎提示词
    PlanExecutePrompt,  // Plan-Execute引擎提示词
    LlmCompilerPrompt,  // LLM Compiler引擎提示词
    OrchestratorPrompt, // Orchestrator引擎提示词
}
```

### 7.2 Prompt构建器

**核心类**：`PromptTemplateBuilder`

**功能**：
```rust
pub struct PromptTemplateBuilder {
    db: Option<Arc<DatabaseService>>,
}

impl PromptTemplateBuilder {
    // 构建插件生成Prompt（支持Few-shot）
    pub async fn build_generation_prompt_async(
        &self,
        analysis: &WebsiteAnalysis,      // 网站分析结果
        vuln_type: &str,                 // 漏洞类型
        target_endpoints: Option<&[String]>, // 目标端点
        requirements: Option<&str>,      // 自定义需求
    ) -> Result<String>;
    
    // 构建代码修复Prompt
    pub async fn build_fix_prompt_async(
        &self,
        original_code: &str,    // 原始代码
        error_message: &str,    // 错误信息
        vuln_type: &str,        // 漏洞类型
        attempt: u32,           // 第几次尝试
    ) -> Result<String>;
}
```

### 7.3 Few-shot示例管理

**作用**：为LLM提供优秀的插件代码示例，提升生成质量

**示例结构**：
```rust
pub struct FewShotExample {
    pub id: String,
    pub vuln_type: String,           // "sqli", "xss", etc.
    pub target_domain: String,       // "example.com"
    pub website_analysis: WebsiteAnalysis,
    pub plugin_code: String,         // 完整的TypeScript代码
    pub quality_score: f32,          // 0-100分
    pub success_rate: f32,           // 实际检测成功率
}
```

**选择策略**：
1. 优先选择相同漏洞类型的示例
2. 优先选择相似技术栈的示例（如都是MySQL）
3. 优先选择高质量分数的示例
4. 通常提供3-5个示例

**实现位置**：
- `src-tauri/src/generators/prompt_templates.rs` - Prompt构建器
- `src-tauri/src/generators/few_shot_examples.rs` - Few-shot管理

---

## 八、MCP（Model Context Protocol）集成

### 8.1 MCP协议介绍

**版本**：rmcp 0.8.3  
**作用**：标准化的AI模型上下文传递协议，用于扩展AI Agent能力

**依赖配置**：
```toml
[dependencies]
rmcp = { version = "0.8.3", features = [
    "client",           # MCP客户端
    "server",           # MCP服务端
    "macros",           # 宏支持
    "transport-async-rw",
    "transport-child-process",
    "transport-sse-client",
    "transport-sse-server",
    "reqwest",
    "auth",
    "uuid",
    "base64"
] }
```

### 8.2 集成的MCP工具

#### 8.2.1 Playwright MCP（浏览器自动化）

**功能**：
- 启动浏览器（Chromium/Firefox/WebKit）
- 页面导航
- 元素交互（点击、填充、选择）
- JavaScript执行
- 截图
- 网络流量捕获

**工具列表**：
```typescript
// 导航
playwright_navigate({ url: string, browserType?: string, headless?: boolean })

// 交互
playwright_click({ selector: string })
playwright_fill({ selector: string, value: string })
playwright_select_option({ selector: string, value: string })

// 信息获取
playwright_get_visible_text() -> string
playwright_get_visible_html() -> string
playwright_evaluate({ script: string }) -> any

// 截图
playwright_screenshot({ name: string })

// 清理
playwright_close()
```

**应用场景**：
- 自动化安全测试
- XSS触发测试
- 登录流程自动化
- 动态内容抓取

#### 8.2.2 File System MCP

**功能**：
- 文件读写
- 目录遍历
- 文件搜索
- 路径操作

**工具列表**：
```typescript
fs_read_file({ path: string }) -> string
fs_write_file({ path: string, content: string })
fs_list_directory({ path: string }) -> string[]
fs_search_files({ pattern: string, path?: string }) -> string[]
```

**应用场景**：
- 导入测试用例
- 导出漏洞报告
- 读取配置文件
- 管理插件代码

### 8.3 MCP工具注册与调用

**注册MCP工具**：
```rust
// 初始化MCP客户端
let mcp_client = MCPClient::new(config)?;

// 连接到Playwright MCP服务器
mcp_client.connect_stdio(
    "playwright-mcp",
    "npx",
    &["@playwright/mcp"],
).await?;

// 列出可用工具
let tools = mcp_client.list_tools().await?;
```

**AI调用MCP工具**：
```rust
// AI引擎生成工具调用
let tool_call = ToolCall {
    name: "playwright_navigate",
    arguments: json!({
        "url": "https://example.com",
        "headless": false
    }),
};

// 执行工具调用
let result = mcp_client.call_tool(tool_call).await?;
```

**实现位置**：
- `src-tauri/src/mcp/` - MCP客户端实现
- `src-tauri/src/tools/mcp_tools.rs` - MCP工具封装

---

## 九、AI模型能力要求

### 9.1 插件生成任务

**推荐模型**：GPT-4, Claude-3.5-Sonnet, DeepSeek-Coder

**能力要求**：
- ✅ 代码生成能力（TypeScript）
- ✅ 理解安全漏洞原理
- ✅ 上下文理解（网站分析结果）
- ✅ Few-shot学习能力
- ✅ 输出格式控制

**上下文窗口**：≥ 16K tokens（推荐32K+）

**温度参数**：0.7-0.9（需要一定创造性）

### 9.2 任务规划

**推荐模型**：GPT-4-Turbo, Claude-3-Opus

**能力要求**：
- ✅ 复杂推理能力
- ✅ 任务分解能力
- ✅ 依赖关系理解
- ✅ JSON输出控制

**上下文窗口**：≥ 32K tokens

**温度参数**：0.3-0.5（需要稳定性和准确性）

### 9.3 快速交互

**推荐模型**：GPT-3.5-Turbo, DeepSeek-Chat, Groq

**能力要求**：
- ✅ 快速响应（<3秒）
- ✅ 基本推理能力
- ✅ 工具调用支持

**上下文窗口**：≥ 4K tokens

**温度参数**：0.5-0.7

### 9.4 RAG检索增强

**推荐模型**：任何支持长上下文的模型

**能力要求**：
- ✅ 长上下文处理（注入检索结果）
- ✅ 信息提取能力
- ✅ 知识综合能力

**上下文窗口**：≥ 32K tokens（注入Top-10检索结果）

---

## 十、性能与成本优化

### 10.1 Token使用优化

| 优化策略 | 描述 | 节省比例 |
|---------|------|---------|
| **Prompt缓存** | 相同的系统提示词缓存 | 40-60% |
| **结果复用** | 相似任务复用之前的结果 | 30-50% |
| **模型降级** | 简单任务使用小模型 | 90% |
| **流式输出** | 提前终止不需要的生成 | 20-40% |

### 10.2 Embedding优化

| 优化策略 | 描述 | 效果 |
|---------|------|------|
| **批量处理** | 一次请求处理多个文本 | 提速5-10x |
| **维度选择** | 根据精度需求选择维度 | 降低50%成本 |
| **本地模型** | 使用sentence-transformers | 免费 |

### 10.3 缓存策略

```rust
// 插件代码缓存
if let Some(cached) = plugin_cache.get(cache_key) {
    return Ok(cached);
}

// RAG检索缓存
if let Some(cached_results) = query_cache.get(query_hash) {
    return Ok(cached_results);
}

// LLM响应缓存（相似查询）
if similarity > 0.95 {
    return cached_response;
}
```

### 10.4 成本估算

**示例场景**：生成1个SQL注入检测插件

| 步骤 | 模型 | Token消耗 | 成本 |
|------|------|-----------|------|
| 网站分析 | GPT-4 | 2K input + 1K output | $0.08 |
| 插件生成 | GPT-4 | 8K input + 2K output | $0.34 |
| 代码验证 | GPT-4 | 3K input + 1K output | $0.13 |
| **总计** | - | **16K tokens** | **$0.55** |

**优化后**（使用DeepSeek-Coder）：**$0.05**（降低91%）

---

## 十一、开发与调试工具

### 11.1 LLM请求日志

**日志路径**：`logs/llm-http-requests-{date}.log`

**记录内容**：
- 请求时间戳
- 模型名称
- 输入Token数
- 输出Token数
- 响应时间
- 成本估算

**示例日志**：
```
[2025-11-19 10:23:45] REQUEST
Model: gpt-4
Prompt Length: 8,523 chars
Temperature: 0.7

[2025-11-19 10:23:52] RESPONSE
Duration: 7.2s
Input Tokens: 2,341
Output Tokens: 856
Cost: $0.09
```

### 11.2 Prompt调试工具

**功能**：
- 查看完整的Prompt内容
- 对比不同Prompt版本
- A/B测试不同模板
- 导出Prompt用于外部测试

**使用**：
```rust
// 开启调试模式
export SENTINEL_DEBUG_PROMPTS=1

// Prompt会保存到
logs/prompts/plugin_generation_{timestamp}.txt
```

### 11.3 插件测试工具

**功能**：
- 验证生成的插件语法
- 执行插件单元测试
- 模拟HTTP流量测试

**命令**：
```bash
# 验证插件语法
cargo run --bin validate_plugin -- plugins/ai_gen_sqli.ts

# 测试插件
cargo run --bin test_plugin -- plugins/ai_gen_sqli.ts test_data/sqli_traffic.json
```

---

## 十二、未来计划

### 12.1 短期（3个月内）

- [ ] 支持更多Embedding模型（BGE、M3E）
- [ ] 实现Prompt版本管理和A/B测试
- [ ] 添加插件质量自动评分系统
- [ ] 集成更多MCP工具（GitHub、Slack）

### 12.2 中期（6个月内）

- [ ] 支持多模态模型（图像识别）
- [ ] 实现分布式向量检索（集群部署）
- [ ] 添加Agent自学习能力（强化学习）
- [ ] 支持私有化LLM部署（vLLM、TGI）

### 12.3 长期（1年内）

- [ ] 开发专用的安全检测微调模型
- [ ] 构建安全知识图谱
- [ ] 实现多Agent协同学习
- [ ] 建立插件市场生态

---

## 附录：相关文件位置

### AI相关源码
- `src-tauri/src/services/ai.rs` - AI服务统一接口
- `src-tauri/src/models/ai.rs` - AI模型定义
- `src-tauri/src/engines/` - 5种AI引擎实现

### RAG相关源码
- `src-tauri/sentinel-rag/src/service.rs` - RAG服务主实现
- `src-tauri/sentinel-rag/src/database.rs` - LanceDB管理器
- `src-tauri/sentinel-rag/src/embeddings.rs` - Embedding和Reranking

### 插件生成相关
- `src-tauri/src/generators/prompt_templates.rs` - Prompt构建器
- `src-tauri/src/generators/plugin_generator.rs` - 插件生成器
- `src-tauri/src/generators/few_shot_examples.rs` - Few-shot管理

### 插件执行相关
- `src-tauri/sentinel-plugins/src/engine.rs` - Deno插件引擎
- `src-tauri/sentinel-plugins/plugins/` - 通用插件库

### MCP集成
- `src-tauri/src/mcp/` - MCP客户端
- `src-tauri/src/tools/mcp_tools.rs` - MCP工具封装

### 配置文件
- `src-tauri/Cargo.toml` - Rust依赖配置
- `src-tauri/tauri.conf.json` - Tauri配置

---

**文档版本**：v1.0  
**创建日期**：2025-11-19  
**维护者**：Sentinel-AI 团队

