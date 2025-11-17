# 方案B: 高级AI插件生成 - 技术架构

## 架构概览

```
┌─────────────────────────────────────────────────────────────────┐
│                        Sentinel AI                               │
│                                                                   │
│  ┌────────────────┐  ┌────────────────┐  ┌──────────────────┐  │
│  │   Frontend     │  │  Tauri Backend │  │  Plugin Engine   │  │
│  │   (Vue.js)     │←→│   (Rust)       │←→│  (Deno Core)     │  │
│  └────────────────┘  └────────────────┘  └──────────────────┘  │
│                              ↓                                    │
│                      ┌───────────────┐                           │
│                      │  MCP Tools    │                           │
│                      └───────┬───────┘                           │
│                              ↓                                    │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Plan B: AI Plugin Generation                 │  │
│  │                                                             │  │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐  │  │
│  │  │  Website    │→│  Prompt      │→│  LLM Service    │  │  │
│  │  │  Analyzer   │  │  Builder     │  │  (OpenAI/etc)   │  │  │
│  │  └─────────────┘  └──────────────┘  └─────────────────┘  │  │
│  │         ↓                                     ↓             │  │
│  │  ┌─────────────┐                    ┌─────────────────┐  │  │
│  │  │  Proxy DB   │                    │  Code Generator │  │  │
│  │  │  (SQLite)   │                    └─────────────────┘  │  │
│  │  └─────────────┘                             ↓             │  │
│  │                                    ┌─────────────────┐  │  │
│  │                                    │  Code Validator │  │  │
│  │                                    └─────────────────┘  │  │
│  │                                             ↓             │  │
│  │                                    ┌─────────────────┐  │  │
│  │                                    │ Quality Scorer  │  │  │
│  │                                    └─────────────────┘  │  │
│  │                                             ↓             │  │
│  │                                    ┌─────────────────┐  │  │
│  │                                    │ Generated Plugin│  │  │
│  │                                    └─────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## 核心模块

### 1. Website Analyzer (网站分析器)

**位置**: `src/analyzers/`

**职责**: 从代理日志中提取和分析网站结构

```rust
pub struct WebsiteAnalyzer {
    db_service: Arc<PassiveDatabaseService>,
}

pub struct WebsiteAnalysis {
    pub domain: String,
    pub total_requests: usize,
    pub unique_endpoints: usize,
    pub api_endpoints: Vec<ApiEndpoint>,
    pub tech_stack: TechStack,
    pub security_observations: Vec<String>,
}
```

**子模块**:
- `ParamExtractor`: 提取和分类HTTP参数
- `TechStackDetector`: 识别服务器、框架、数据库

**数据流**:
```
Proxy DB → list_proxy_requests_by_host()
         ↓
    Parse HTTP headers/body
         ↓
    Extract endpoints & parameters
         ↓
    Detect tech stack
         ↓
    Generate WebsiteAnalysis
```

### 2. Prompt Template Builder (Prompt构建器)

**位置**: `src/generators/prompt_templates.rs`

**职责**: 构建结构化的LLM提示

```rust
pub struct PromptTemplateBuilder;

impl PromptTemplateBuilder {
    pub fn build_generation_prompt(
        analysis: &WebsiteAnalysis,
        vuln_types: &[String],
    ) -> String;
    
    fn build_context_section(analysis: &WebsiteAnalysis) -> String;
    fn build_requirements_section(vuln_type: &str, tech: &TechStack) -> String;
    fn build_examples_section(vuln_type: &str) -> String;
    fn build_constraints_section() -> String;
}
```

**Prompt结构**:
```
1. System Message (角色定义)
2. Context (网站分析结果)
3. Requirements (检测需求)
4. Examples (代码示例)
5. Constraints (约束条件)
6. Output Format (输出格式)
```

### 3. Advanced Plugin Generator (高级插件生成器)

**位置**: `src/generators/advanced_generator.rs`

**职责**: 协调整个插件生成流程

```rust
pub struct AdvancedPluginGenerator {
    ai_manager: Arc<AiServiceManager>,
    validator: PluginValidator,
    prompt_builder: PromptTemplateBuilder,
}

pub struct GeneratedPlugin {
    pub name: String,
    pub description: String,
    pub code: String,
    pub status: PluginStatus,
    pub quality_score: f32,
    pub quality_breakdown: QualityBreakdown,
    // ...
}
```

**生成流程**:
```rust
async fn generate(&self, request: PluginGenerationRequest) 
    -> Result<Vec<GeneratedPlugin>> {
    
    let plugins = Vec::new();
    
    for vuln_type in request.vuln_types {
        // 1. Build prompt
        let prompt = self.prompt_builder.build_generation_prompt(
            &request.analysis,
            &[vuln_type]
        );
        
        // 2. Call LLM
        let (response, model) = self.call_llm_for_generation(&prompt).await?;
        
        // 3. Extract code
        let code = self.extract_and_clean_code(&response)?;
        
        // 4. Validate syntax
        let validation = self.validator.validate_syntax(&code)?;
        
        // 5. Calculate quality
        let quality = self.calculate_quality_score(&request.analysis, &code);
        
        // 6. Create plugin
        plugins.push(GeneratedPlugin { /* ... */ });
    }
    
    Ok(plugins)
}
```

### 4. Plugin Validator (插件验证器)

**位置**: `src/generators/validator.rs`

**职责**: 验证生成的插件代码

```rust
pub struct PluginValidator;

impl PluginValidator {
    // 语法验证
    pub fn validate_syntax(&self, code: &str) -> Result<ValidationResult>;
    
    // 沙箱测试（概念性）
    pub async fn run_sandbox_test(&self, code: &str) -> Result<()>;
    
    // 安全检查
    fn check_dangerous_functions(&self, code: &str) -> Vec<String>;
    
    // 结构验证
    fn validate_structure(&self, code: &str) -> Result<()>;
}
```

**验证规则**:
- ✅ 必须导出`plugin`对象
- ✅ 必须包含`metadata`
- ✅ 至少实现`scan_request`或`scan_response`
- ❌ 不能包含`eval()`, `Function()`
- ❌ 不能访问文件系统
- ❌ 不能发起外部网络请求

### 5. Quality Scorer (质量评分器)

**位置**: `src/generators/advanced_generator.rs`

**职责**: 多维度评估插件质量

```rust
impl AdvancedPluginGenerator {
    fn calculate_quality_score(
        &self,
        analysis: &WebsiteAnalysis,
        code: &str,
    ) -> f32 {
        let logic_score = self.calculate_logic_score(code);
        let security_score = self.calculate_security_score(code);
        let code_quality_score = self.calculate_code_quality_score(code);
        
        // 加权平均
        (logic_score * 0.4 + security_score * 0.3 + code_quality_score * 0.3)
    }
}
```

**评分维度**:

| 维度 | 权重 | 检查项 | 分值 |
|------|------|--------|------|
| Logic | 40% | get_metadata | 20 |
| | | scan_request | 25 |
| | | scan_response | 25 |
| | | op_emit_finding | 20 |
| | | vuln_type | 10 |
| Security | 30% | 无eval() | -30 |
| | | 无Function() | -30 |
| | | 无innerHTML | -15 |
| Code Quality | 30% | 有注释 | +10 |
| | | 有类型定义 | +10 |
| | | 有错误处理 | +10 |
| | | 代码行数<500 | +10 |

## MCP工具接口

### AnalyzerToolProvider

**工具**: `analyze_website`

```typescript
interface AnalyzeWebsiteParams {
  domain: string;      // 目标域名
  limit?: number;      // 分析请求数量限制
}

interface AnalyzeWebsiteResult {
  domain: string;
  total_requests: number;
  unique_endpoints: number;
  api_endpoints: ApiEndpoint[];
  tech_stack: TechStack;
  security_observations: string[];
}
```

### GeneratorToolProvider

**工具**: `generate_advanced_plugin`

```typescript
interface GeneratePluginParams {
  analysis: WebsiteAnalysis;      // 网站分析结果
  vuln_types: string[];            // 漏洞类型列表
  target_endpoints?: string[];     // 可选：目标端点
  requirements?: string;           // 可选：额外需求
}

interface GeneratePluginResult {
  plugins: GeneratedPlugin[];
  summary: string;
  statistics: {
    total: number;
    pending_review: number;
    validation_failed: number;
    average_quality: number;
  };
}
```

## 数据库设计

### proxy_requests 表

```sql
CREATE TABLE IF NOT EXISTS proxy_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id TEXT UNIQUE NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    query TEXT,
    headers TEXT NOT NULL,
    body BLOB,
    response_status INTEGER,
    response_headers TEXT,
    response_body BLOB,
    timestamp INTEGER NOT NULL,
    duration_ms INTEGER,
    
    -- 索引优化
    INDEX idx_host (host),
    INDEX idx_timestamp (timestamp),
    INDEX idx_method (method)
);
```

**查询模式**:
```rust
// 获取特定域名的所有请求
db_service.list_proxy_requests_by_host("example.com", 1000).await?
```

## 性能优化

### 1. 并行处理

```rust
// 并行生成多个插件
use tokio::task::JoinSet;

let mut join_set = JoinSet::new();

for vuln_type in vuln_types {
    let generator = self.clone();
    let analysis = analysis.clone();
    
    join_set.spawn(async move {
        generator.generate_single(&analysis, &vuln_type).await
    });
}

while let Some(result) = join_set.join_next().await {
    plugins.push(result??);
}
```

### 2. 缓存策略

```rust
// Prompt模板缓存
lazy_static! {
    static ref PROMPT_CACHE: RwLock<HashMap<String, String>> = 
        RwLock::new(HashMap::new());
}

// 技术栈识别缓存
lazy_static! {
    static ref TECH_STACK_CACHE: RwLock<HashMap<String, TechStack>> = 
        RwLock::new(HashMap::new());
}
```

### 3. 数据库优化

```rust
// 批量查询
let requests = sqlx::query_as::<_, ProxyRequestRecord>(
    "SELECT * FROM proxy_requests 
     WHERE host LIKE ? 
     ORDER BY timestamp DESC 
     LIMIT ?"
)
.bind(format!("%{}%", domain))
.bind(limit)
.fetch_all(&pool)
.await?;
```

## 错误处理

### 错误类型层次

```rust
use anyhow::{Result, Context, anyhow};

// 模块级错误
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("LLM service error: {0}")]
    LlmError(String),
    
    #[error("Code validation failed: {0}")]
    ValidationError(String),
    
    #[error("No AI service available")]
    NoAiService,
}

// 使用示例
let response = service.send_message_stream(...)
    .await
    .context("Failed to call LLM service")?;
```

### 错误恢复策略

```rust
// 1. 重试机制
for attempt in 1..=3 {
    match self.call_llm_for_generation(prompt).await {
        Ok(result) => return Ok(result),
        Err(e) if attempt < 3 => {
            log::warn!("LLM call failed (attempt {}): {}", attempt, e);
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }
        Err(e) => return Err(e),
    }
}

// 2. 降级策略
let code = match self.extract_and_clean_code(&response) {
    Ok(code) => code,
    Err(_) => {
        log::warn!("Failed to extract code, using full response");
        response.clone()
    }
};
```

## 日志系统

### 日志级别

```rust
// DEBUG: 详细的调试信息
log::debug!("Calling LLM for code generation");

// INFO: 关键操作信息
log::info!("Generated {} plugins for domain: {}", plugins.len(), domain);

// WARN: 警告但不影响功能
log::warn!("Security issue detected: {}", pattern);

// ERROR: 错误需要关注
log::error!("Failed to validate plugin: {}", e);
```

### 结构化日志

```rust
tracing::info!(
    domain = %analysis.domain,
    vuln_types = ?vuln_types,
    endpoints = analysis.api_endpoints.len(),
    "Starting plugin generation"
);
```

## 安全考虑

### 1. 代码沙箱

```rust
// Deno Core 沙箱配置
JsRuntime::new(RuntimeOptions {
    module_loader: Some(Rc::new(FsModuleLoader)),
    // 禁用文件系统访问
    create_params: Some(CreateParams::default()
        .heap_limits(0, 50 * 1024 * 1024)), // 50MB堆限制
    ..Default::default()
})
```

### 2. 输入验证

```rust
// 验证域名格式
fn validate_domain(domain: &str) -> Result<()> {
    if domain.is_empty() || domain.len() > 255 {
        return Err(anyhow!("Invalid domain length"));
    }
    
    // 简单的域名格式检查
    if !domain.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err(anyhow!("Invalid domain format"));
    }
    
    Ok(())
}
```

### 3. 资源限制

```rust
// LLM调用超时
tokio::time::timeout(
    Duration::from_secs(60),
    service.send_message_stream(...)
).await??;

// 代码长度限制
if code.len() > 100_000 {
    return Err(anyhow!("Generated code too large"));
}
```

## 扩展点

### 1. 自定义Prompt模板

```rust
pub trait PromptTemplate {
    fn build(&self, analysis: &WebsiteAnalysis, vuln_type: &str) -> String;
}

// 用户可以实现自定义模板
pub struct CustomPromptTemplate {
    // ...
}

impl PromptTemplate for CustomPromptTemplate {
    fn build(&self, analysis: &WebsiteAnalysis, vuln_type: &str) -> String {
        // 自定义逻辑
    }
}
```

### 2. 自定义验证器

```rust
pub trait CodeValidator {
    fn validate(&self, code: &str) -> Result<ValidationResult>;
}

// 可以添加自定义验证规则
pub struct CustomValidator;

impl CodeValidator for CustomValidator {
    fn validate(&self, code: &str) -> Result<ValidationResult> {
        // 自定义验证逻辑
    }
}
```

### 3. 自定义评分器

```rust
pub trait QualityScorer {
    fn score(&self, code: &str, analysis: &WebsiteAnalysis) -> f32;
}

// 可以实现基于机器学习的评分
pub struct MLScorer {
    model: TorchModel,
}

impl QualityScorer for MLScorer {
    fn score(&self, code: &str, analysis: &WebsiteAnalysis) -> f32 {
        self.model.predict(code, analysis)
    }
}
```

## 测试策略

### 1. 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_from_markdown() {
        let generator = AdvancedPluginGenerator::new(mock_ai_manager());
        let response = "```typescript\nexport const plugin = {};\n```";
        let code = generator.extract_from_markdown(response).unwrap();
        assert!(code.contains("export const plugin"));
    }
    
    #[tokio::test]
    async fn test_quality_score() {
        let code = include_str!("../../test_fixtures/good_plugin.ts");
        let score = generator.calculate_quality_score(&mock_analysis(), code);
        assert!(score > 70.0);
    }
}
```

### 2. 集成测试

```rust
#[tokio::test]
async fn test_full_generation_flow() {
    // 1. Setup
    let db_service = setup_test_db().await;
    let ai_manager = setup_test_ai_manager().await;
    let generator = AdvancedPluginGenerator::new(ai_manager);
    
    // 2. Generate analysis
    let analyzer = WebsiteAnalyzer::new(db_service.clone());
    let analysis = analyzer.analyze_website("example.com", 100).await?;
    
    // 3. Generate plugin
    let request = PluginGenerationRequest {
        analysis,
        vuln_types: vec!["sqli".to_string()],
        target_endpoints: None,
        requirements: None,
    };
    
    let plugins = generator.generate(request).await?;
    
    // 4. Assertions
    assert_eq!(plugins.len(), 1);
    assert!(plugins[0].quality_score > 0.0);
}
```

### 3. 端到端测试

```rust
#[tokio::test]
async fn test_e2e_plugin_generation_and_execution() {
    // 1. Start proxy
    start_passive_scan(8080).await?;
    
    // 2. Generate traffic
    generate_test_traffic("http://testapp.local").await?;
    
    // 3. Analyze
    let analysis = analyze_website("testapp.local", 100).await?;
    
    // 4. Generate plugin
    let plugins = generate_advanced_plugin(analysis, vec!["sqli"]).await?;
    
    // 5. Load and enable
    load_plugin(&plugins[0].code, true).await?;
    
    // 6. Trigger detection
    generate_vuln_traffic("http://testapp.local").await?;
    
    // 7. Check findings
    let findings = list_findings(None, Some("sqli"), 10).await?;
    assert!(!findings.is_empty());
}
```

## 部署考虑

### 1. 依赖管理

```toml
[dependencies]
# AI服务
anyhow = "1.0"
tokio = { version = "1.40", features = ["full"] }

# 数据库
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite"] }

# 插件引擎
deno_core = "0.365.0"
deno_ast = { version = "0.51.0", features = ["transpiling"] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 日志
tracing = "0.1"
log = "0.4"
```

### 2. 配置管理

```rust
// config.toml
[plan_b]
enabled = true
max_concurrent_generations = 3
generation_timeout_secs = 60
cache_ttl_secs = 3600

[plan_b.quality_thresholds]
excellent = 80
good = 60
fair = 40

[plan_b.ai_service]
prefer_provider = "openai"
fallback_providers = ["anthropic", "local"]
```

### 3. 监控指标

```rust
// Prometheus metrics
lazy_static! {
    static ref GENERATION_COUNTER: IntCounter = 
        register_int_counter!("plugin_generation_total", "Total plugins generated").unwrap();
        
    static ref GENERATION_DURATION: Histogram = 
        register_histogram!("plugin_generation_duration_seconds", "Generation duration").unwrap();
        
    static ref QUALITY_SCORE_GAUGE: Gauge = 
        register_gauge!("plugin_quality_score", "Average quality score").unwrap();
}
```

## 总结

方案B的技术架构设计遵循以下原则：

✅ **模块化**: 清晰的模块划分和职责分离
✅ **可扩展**: 丰富的扩展点和接口设计
✅ **高性能**: 并行处理和缓存优化
✅ **安全性**: 沙箱执行和输入验证
✅ **可测试**: 完整的测试策略
✅ **可维护**: 详细的日志和错误处理

这个架构为AI驱动的安全插件生成提供了坚实的技术基础。🎯

