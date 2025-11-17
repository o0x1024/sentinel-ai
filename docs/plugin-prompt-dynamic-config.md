# AI 插件生成和修复 Prompt 动态配置

## 概述

本次修改实现了 AI 插件生成和修复的 prompt 动态配置功能，允许用户通过 Prompt 管理页面自定义插件生成和修复的提示词模板，同时保留硬编码的回退方案。

## 修改内容

### 1. PromptRepository 增强 (`src-tauri/src/services/prompt_db.rs`)

#### 新增方法

- **`get_active_template_by_type(template_type: TemplateType)`**
  - 根据模板类型获取激活的 prompt 模板
  - 返回第一个 `is_active=true` 的模板

- **`evaluate_content(content: &str, context: &serde_json::Value)`**
  - 直接评估模板内容，替换变量
  - 支持 `{var}` 和 `{{VAR}}` 两种占位符格式

### 2. AdvancedPluginGenerator 支持动态 Prompt (`src-tauri/src/generators/advanced_generator.rs`)

#### 结构修改

```rust
pub struct AdvancedPluginGenerator {
    ai_manager: Arc<AiServiceManager>,
    prompt_repo: Option<Arc<PromptRepository>>,  // 新增
    validator: PluginValidator,
    prompt_builder: PromptTemplateBuilder,
    few_shot_repo: FewShotRepository,
    auto_approval_engine: PluginAutoApprovalEngine,
}
```

#### 新增构造方法

- **`new_with_prompt_repo()`** - 创建带 PromptRepository 的实例
- **`set_prompt_repo()`** - 设置 PromptRepository

#### 修改的方法

- **`call_llm_for_generation()`**
  - 优先从数据库获取 `PluginGeneration` 类型的激活模板
  - 如果没有找到或加载失败，回退到硬编码的简单 prompt
  - 记录日志以便追踪使用的 prompt 来源

### 3. PromptTemplateBuilder 支持异步模板加载 (`src-tauri/src/generators/prompt_templates.rs`)

#### 新增方法

- **`build_fix_prompt_async()`**
  - 异步版本的修复 prompt 构建方法
  - 从数据库加载 `PluginFix` 类型的模板
  - 支持变量替换：`{original_code}`, `{error_message}`, `{error_details}`, `{vuln_type}`, `{attempt}`
  - 回退到原有的 `build_fix_prompt()` 同步方法

#### 修改的方法

- **`fix_plugin_code()`**
  - 优先调用 `build_fix_prompt_async()` 使用数据库模板
  - 失败时回退到 `build_fix_prompt()` 使用硬编码模板
  - 记录日志以便追踪

### 4. PromptManagement.vue 增强模板创建功能

#### 插件生成模板 (`PluginGeneration`)

新增详细的默认内容，包括：

- **环境说明**
  - 可用的 API（`Deno.core.ops.op_emit_finding()`, 日志函数等）
  - 完整的插件接口定义（TypeScript 类型）
  - Body 处理辅助函数
  - 对象迭代的正确方式（`Object.entries()`）
  - Finding 发射示例

- **任务要求**
  - 支持的变量：`{vuln_type}`, `{analysis}`, `{endpoints}`, `{requirements}`

- **输出格式**
  - 明确要求只返回 TypeScript 代码块
  - 不包含解释性文字

- **重要约束**
  - 使用 `Object.entries()` 而非 `.entries()`
  - 使用 `bodyToString()` 辅助函数
  - 空值检查
  - 错误处理
  - 置信度阈值

#### 插件修复模板 (`PluginFix`)

新增详细的默认内容，包括：

- **错误信息展示**
  - 修复尝试次数
  - 错误消息
  - 详细错误信息

- **修复指令**
  - 7 条明确的修复要求
  - 常见问题检查清单
  - Body 处理辅助函数
  - 正确的对象迭代方式

- **输出格式**
  - 只返回修复后的代码
  - 不包含解释

- **重要提醒**
  - 聚焦特定错误
  - 保持现有功能
  - 生产就绪

## 使用方式

### 1. 创建插件生成模板

1. 打开 Prompt 管理页面
2. 选择"应用级"分类
3. 点击"创建插件生成模板(被动扫描)"按钮
4. 编辑模板内容（已预填充详细的默认内容）
5. 勾选"激活此模板"
6. 点击"保存"

### 2. 创建插件修复模板

1. 打开 Prompt 管理页面
2. 选择"应用级"分类
3. 点击"创建插件修复模板(被动扫描)"按钮
4. 编辑模板内容（已预填充详细的默认内容）
5. 勾选"激活此模板"
6. 点击"保存"

### 3. 自动使用

- 当 AI 生成插件时，系统会自动使用激活的 `PluginGeneration` 模板
- 当插件修复失败需要修复时，系统会自动使用激活的 `PluginFix` 模板
- 如果没有激活的模板或加载失败，系统会自动回退到硬编码的默认 prompt

## 变量支持

### 插件生成模板变量

- `{vuln_type}` - 漏洞类型（如 "sqli", "xss", "idor"）
- `{analysis}` - 网站分析数据（技术栈、端点、模式）
- `{endpoints}` - 目标端点列表
- `{requirements}` - 额外的特定要求

### 插件修复模板变量

- `{original_code}` - 原始插件代码
- `{error_message}` - 错误消息
- `{error_details}` - 详细错误信息
- `{vuln_type}` - 漏洞类型
- `{attempt}` - 修复尝试次数

## 回退机制

系统采用多层回退机制确保稳定性：

1. **优先级**：数据库中激活的模板
2. **回退 1**：数据库中未激活的模板
3. **回退 2**：硬编码的默认 prompt

每次回退都会记录日志，便于追踪和调试。

## 日志追踪

系统会记录以下关键日志：

```rust
// 使用动态模板
log::info!("Using dynamic plugin generation template: {}", template.name);
log::info!("Using dynamic fix prompt template from database");

// 回退到硬编码
log::warn!("No active plugin generation template found, using fallback");
log::warn!("Failed to load dynamic fix prompt template: {}, using fallback", e);
log::debug!("No prompt repository configured, using fallback prompt");
```

## 优势

1. **灵活性**：用户可以根据实际需求自定义 prompt
2. **可维护性**：无需修改代码即可调整 prompt
3. **可测试性**：可以快速测试不同的 prompt 效果
4. **稳定性**：保留硬编码回退，确保系统始终可用
5. **可追踪性**：详细的日志记录便于调试

## 注意事项

1. **模板激活**：确保创建模板后勾选"激活此模板"
2. **变量格式**：使用 `{var}` 或 `{{VAR}}` 格式的占位符
3. **模板优先级**：同一类型只有一个模板会被激活（最新保存的）
4. **回退安全**：即使模板配置错误，系统也会回退到默认 prompt

## 未来扩展

- 支持 Agent 工具插件的动态 prompt 配置
- 支持模板版本管理和 A/B 测试
- 支持更复杂的变量替换（条件、循环等）
- 支持模板继承和组合

