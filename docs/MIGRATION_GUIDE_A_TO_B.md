# 从方案A迁移到方案B指南

## 快速参考

| 方案A (已删除) | 方案B (当前) | 说明 |
|---------------|-------------|------|
| `generate_plugin` | `generate_advanced_plugin` | 工具名变更 |
| `PluginGenerator` | `AdvancedPluginGenerator` | Rust结构体 |
| 模板填充 | AI代码生成 | 生成方式 |
| 固定逻辑 | 上下文感知 | 适应性 |
| 无质量保障 | 多维度验证 | 质量控制 |

## 迁移步骤

### 1. AI Prompt 迁移

#### 旧的Prompt (方案A)
```markdown
步骤4: 生成针对性插件
根据用户需求和网站特征，调用 generate_plugin 工具：

generate_plugin({
  template_type: "sqli",
  target_url: "https://example.com",
  target_params: ["id", "search"],
  sensitivity: "medium",
  auto_enable: true
})
```

#### 新的Prompt (方案B)
```markdown
步骤4: 分析网站结构
先调用 analyze_website 工具获取网站信息：

analyze_website({
  domain: "example.com",
  limit: 1000
})

步骤5: 生成高级AI插件
使用分析结果调用 generate_advanced_plugin：

generate_advanced_plugin({
  analysis: <来自步骤4的分析结果>,
  vuln_types: ["sqli", "xss"],
  target_endpoints: ["/search", "/api/user"],
  requirements: "Focus on authentication and input validation"
})
```

### 2. 代码迁移

#### 2.1 导入语句

**旧代码**:
```rust
use crate::tools::plugin_generator::{
    PluginGenerator,
    PluginGenerationParams,
    GeneratedPlugin
};
```

**新代码**:
```rust
use crate::generators::{
    AdvancedPluginGenerator,
    PluginGenerationRequest,
    GeneratedPlugin
};
use crate::analyzers::WebsiteAnalyzer;
use crate::services::ai::AiServiceManager;
```

#### 2.2 生成器初始化

**旧代码**:
```rust
let generator = PluginGenerator::new();
```

**新代码**:
```rust
let ai_manager = Arc::new(ai_service_manager);
let generator = AdvancedPluginGenerator::new(ai_manager);
```

#### 2.3 插件生成调用

**旧代码**:
```rust
let params = PluginGenerationParams {
    template_type: "sqli".to_string(),
    target_url: "https://example.com".to_string(),
    target_params: vec!["id".to_string()],
    sensitivity: Some("medium".to_string()),
    custom_config: None,
};

let plugin = generator.generate_from_template(params)?;
```

**新代码**:
```rust
// 1. 先进行网站分析
let analyzer = WebsiteAnalyzer::new(db_service);
let analysis = analyzer.analyze_website("example.com", 1000).await?;

// 2. 构建生成请求
let request = PluginGenerationRequest {
    analysis,
    vuln_types: vec!["sqli".to_string(), "xss".to_string()],
    target_endpoints: Some(vec!["/search".to_string()]),
    requirements: Some("Focus on input validation".to_string()),
};

// 3. 生成插件（可能返回多个）
let plugins = generator.generate(request).await?;

// 4. 处理结果
for plugin in plugins {
    println!("Generated: {} (quality: {:.1})", 
        plugin.name, plugin.quality_score);
}
```

### 3. 配置迁移

#### 3.1 环境变量

方案B需要额外的环境变量：

```bash
# LLM服务配置 (必须)
OPENAI_API_KEY=your_api_key_here

# 或者使用其他LLM服务
ANTHROPIC_API_KEY=your_api_key_here

# 可选：本地LLM
LOCAL_LLM_URL=http://localhost:11434
```

#### 3.2 数据库

确保数据库schema已更新：

```sql
-- 检查 plugin_registry 表是否存在
SELECT name FROM sqlite_master 
WHERE type='table' AND name='plugin_registry';

-- 检查是否有新字段
PRAGMA table_info(plugin_registry);
-- 应该包含: quality_score, validation_status
```

### 4. 工作流迁移

#### 4.1 完整工作流对比

**方案A工作流** (3步):
```
1. start_passive_scan()
2. generate_plugin(template_type, url, params)
3. list_findings()
```

**方案B工作流** (5步):
```
1. start_passive_scan()
2. playwright_navigate(url)  # 浏览网站，收集流量
3. analyze_website(domain)   # 分析网站结构
4. generate_advanced_plugin(analysis, vuln_types)  # AI生成
5. list_findings()
```

#### 4.2 时间对比

| 操作 | 方案A | 方案B | 差异 |
|------|------|------|------|
| 生成单个插件 | ~1秒 | 5-15秒 | +4-14秒 |
| 质量评分 | 无 | < 1秒 | +1秒 |
| 代码验证 | 无 | < 1秒 | +1秒 |
| **总计** | ~1秒 | 6-17秒 | +5-16秒 |

**权衡**: 方案B慢一些，但生成质量显著提升（70分 → 85分）。

### 5. 测试迁移

#### 5.1 单元测试

**旧测试**:
```rust
#[tokio::test]
async fn test_plugin_generation() {
    let generator = PluginGenerator::new();
    let params = PluginGenerationParams {
        template_type: "sqli".to_string(),
        target_url: "https://example.com".to_string(),
        target_params: vec![],
        sensitivity: None,
        custom_config: None,
    };
    
    let result = generator.generate_from_template(params);
    assert!(result.is_ok());
}
```

**新测试**:
```rust
#[tokio::test]
async fn test_advanced_plugin_generation() {
    let ai_manager = create_test_ai_manager().await;
    let generator = AdvancedPluginGenerator::new(ai_manager);
    
    let analysis = create_test_analysis();
    let request = PluginGenerationRequest {
        analysis,
        vuln_types: vec!["sqli".to_string()],
        target_endpoints: None,
        requirements: None,
    };
    
    let plugins = generator.generate(request).await;
    assert!(plugins.is_ok());
    
    let plugins = plugins.unwrap();
    assert!(!plugins.is_empty());
    assert!(plugins[0].quality_score > 0.0);
}
```

## 常见迁移问题

### Q1: "找不到 plugin_generator 模块"

**错误**:
```
error[E0433]: failed to resolve: could not find `plugin_generator` in `tools`
```

**解决**:
```rust
// 旧代码
use crate::tools::plugin_generator::PluginGenerator;

// 新代码
use crate::generators::AdvancedPluginGenerator;
```

### Q2: "generate_plugin 工具不存在"

**错误**:
```
Tool not found: generate_plugin
```

**解决**:
更新AI prompt，使用 `generate_advanced_plugin` 替代。

### Q3: "缺少 analysis 参数"

**错误**:
```
Missing required parameter: analysis
```

**解决**:
方案B需要先进行网站分析：

```rust
// 1. 先分析
let analysis = analyzer.analyze_website(domain, limit).await?;

// 2. 再生成
let request = PluginGenerationRequest {
    analysis,  // 必须提供
    vuln_types: vec!["sqli".to_string()],
    target_endpoints: None,
    requirements: None,
};
```

### Q4: "LLM调用失败"

**错误**:
```
Failed to call LLM service: API key not found
```

**解决**:
1. 确保环境变量设置正确：
```bash
export OPENAI_API_KEY=your_key_here
```

2. 或者在数据库中配置AI服务：
```rust
ai_manager.add_service(service_config).await?;
```

### Q5: "质量评分太低"

**问题**: 生成的插件质量分 < 70

**解决**:
1. 提供更详细的 `requirements`:
```rust
requirements: Some("Focus on SQLi in search parameters. Use time-based detection.")
```

2. 指定 `target_endpoints`:
```rust
target_endpoints: Some(vec!["/search", "/api/query"])
```

3. 确保网站分析数据充足（至少100个请求）

## 验证迁移成功

### 1. 编译检查

```bash
cd src-tauri
cargo build
```

应该没有关于 `plugin_generator` 的错误。

### 2. 工具列表检查

```bash
# 启动应用
cargo run

# 在前端调用
unified_list_tools({ provider: "passive" })
```

应该看到：
- ✅ `start_passive_scan`
- ✅ `stop_passive_scan`
- ✅ `list_findings`
- ✅ `enable_plugin`
- ❌ `generate_plugin` (不应该存在)

```bash
unified_list_tools({ provider: "generator" })
```

应该看到：
- ✅ `generate_advanced_plugin`

### 3. 功能测试

完整测试方案B工作流：

```typescript
// 1. 启动被动扫描
await invoke("start_passive_scan");

// 2. 浏览网站（手动或自动）
await invoke("playwright_navigate", { 
  url: "https://example.com" 
});

// 3. 分析网站
const analysis = await invoke("analyze_website", {
  domain: "example.com",
  limit: 1000
});

// 4. 生成插件
const plugins = await invoke("generate_advanced_plugin", {
  analysis,
  vuln_types: ["sqli", "xss"]
});

// 5. 检查结果
console.log(`Generated ${plugins.length} plugins`);
plugins.forEach(p => {
  console.log(`- ${p.name}: quality=${p.quality_score.toFixed(1)}`);
});
```

## 回滚计划

如果迁移出现问题，可以临时回滚：

### 1. 恢复文件

```bash
git checkout HEAD~1 -- src-tauri/src/tools/plugin_generator.rs
git checkout HEAD~1 -- src-tauri/src/tools/mod.rs
git checkout HEAD~1 -- src-tauri/src/tools/passive_provider.rs
```

### 2. 重新编译

```bash
cargo build
```

### 3. 报告问题

在回滚前，请记录：
- 错误信息
- 复现步骤
- 环境信息

然后提交issue。

## 获取帮助

如果遇到问题：

1. **查看文档**:
   - `PLAN_B_USAGE_GUIDE.md` - 使用指南
   - `PLAN_B_ARCHITECTURE.md` - 架构文档
   - `PLAN_A_REMOVAL_NOTES.md` - 删除说明

2. **检查示例**:
   - `prompts/automated_security_testing.md` - 工作流示例
   - `docs/examples/` - 代码示例

3. **调试日志**:
```bash
RUST_LOG=debug cargo run
```

4. **社区支持**:
   - GitHub Issues
   - Discord频道

---

**最后更新**: 2025-11-13  
**适用版本**: Plan B Day 5+  
**文档版本**: 1.0.0

