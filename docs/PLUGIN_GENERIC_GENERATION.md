# 插件通用化生成优化

## 问题描述

之前生成的插件包含特定网站信息，导致无法跨站复用：

### 旧的命名方式
```
插件ID: ai_gen_xss_testphp_vulnweb_com_20251114_063139
插件名: Cross-Site Scripting (XSS) Detector for testphp.vulnweb.com
描述: AI-generated plugin for detecting xss vulnerabilities in testphp.vulnweb.com
```

### 问题
1. **无法复用**：为 `example.com` 生成的插件不会被 `another-site.com` 复用
2. **重复生成**：每个网站都会生成自己的插件，即使检测逻辑完全相同
3. **数据库冗余**：大量功能相同但名称不同的插件
4. **资源浪费**：重复调用 LLM API 生成相似的插件

## 解决方案

### 1. 通用化插件元数据

修改 `advanced_generator.rs` 中的插件生成逻辑：

**插件ID**（去除域名）：
```rust
// 旧: ai_gen_{vuln_type}_{domain}_{timestamp}
let plugin_id = format!("ai_gen_{}_{}", vuln_type, timestamp);
```

**插件名称**（去除网站信息）：
```rust
// 旧: "{VulnType} Detector for {domain}"
let plugin_name = format!("{} Detector", self.get_vuln_display_name(vuln_type));
```

**描述**（通用化）：
```rust
// 旧: "AI-generated plugin for detecting {vuln_type} vulnerabilities in {domain}"
description: format!("AI-generated plugin for detecting {} vulnerabilities", vuln_type)
```

### 2. 优化提示词模板

修改 `prompt_templates.rs`，强调生成通用的检测逻辑：

**在 Header 中添加说明**：
```rust
**IMPORTANT**: Generate GENERIC detection logic that can work across different websites, 
not just the analyzed target. Use the website analysis as reference for common patterns, 
but make the detection rules broadly applicable.
```

**在 Few-shot 示例中强调**：
```rust
**Important**: Use these examples as inspiration. Generate GENERIC detection patterns 
that work across different websites, not just the current target.
```

**在分析上下文中说明**：
```rust
The following analysis is from a sample website. Use it as REFERENCE for common patterns, 
but generate GENERIC detection logic that works across different websites.
```

## 新的命名方式

### 示例
```
插件ID: ai_gen_xss_20251114_063139
插件名: Cross-Site Scripting (XSS) Detector
描述: AI-generated plugin for detecting xss vulnerabilities
```

### 优势
1. ✅ **可复用**：同一类型的高质量插件可以跨站使用
2. ✅ **减少重复**：避免为每个网站生成相同的插件
3. ✅ **节省资源**：减少 LLM API 调用次数
4. ✅ **数据库优化**：减少冗余数据

## 复用机制

### 复用检查逻辑
在 `generator_tools.rs` 中：

```rust
// 检查数据库中是否已有可复用的高质量插件
for vuln_type in &vuln_types {
    match db_service.find_reusable_plugins_by_category(vuln_type, min_quality_score).await {
        Ok(existing_plugins) if !existing_plugins.is_empty() => {
            // 找到可复用的插件，跳过生成
            reused_plugins.push(best_plugin);
        }
        _ => {
            // 需要生成新插件
            types_to_generate.push(vuln_type.clone());
        }
    }
}
```

### 复用条件
- **类别匹配**：`category = vuln_type`（如 xss, sqli, idor）
- **质量达标**：`quality_score >= 70.0`
- **验证通过**：`validation_status IN ('Approved', 'Passed')`
- **主类别正确**：`main_category = 'passiveScan'`

## 权衡考虑

### 通用性 vs 精准性

**完全通用的插件**：
- ✅ 可以跨站复用
- ❌ 可能检测精度不够高

**网站特定的插件**：
- ✅ 检测更精准（基于网站分析）
- ❌ 无法复用

**当前方案（半通用）**：
- ✅ 插件名称和ID通用化，便于复用
- ✅ 检测逻辑基于网站分析生成，保持精准度
- ✅ 通过质量分数筛选，确保复用的插件质量高
- ✅ LLM 被指导生成通用的检测模式

## 使用场景

### 场景 1：首次扫描网站
```
用户: 扫描 example.com，生成 XSS 和 SQLi 插件
系统: 
  - 检查数据库：无可复用插件
  - 生成 2 个新插件
  - 保存到数据库
  - 质量分数: XSS=85, SQLi=90
```

### 场景 2：扫描相似网站
```
用户: 扫描 another-site.com，需要 XSS 和 SQLi 插件
系统:
  - 检查数据库：找到高质量插件（XSS=85, SQLi=90）
  - 复用现有插件
  - 跳过生成，节省时间和成本
```

### 场景 3：部分复用
```
用户: 扫描 third-site.com，需要 XSS、SQLi 和 IDOR 插件
系统:
  - XSS: 复用现有插件（质量=85）
  - SQLi: 复用现有插件（质量=90）
  - IDOR: 无高质量插件，生成新插件
```

## 效果评估

### 预期收益
- **API 调用减少**：50-80%（取决于网站相似度）
- **生成时间减少**：60-90%（跳过已有类型）
- **数据库空间**：减少冗余插件数量
- **用户体验**：更快的响应速度

### 质量保证
- 只复用质量分数 >= 70 的插件
- 只复用验证通过的插件
- 用户可以手动禁用或删除不合适的插件
- 可以根据需要重新生成特定类型的插件

## 相关文件

- `src-tauri/src/generators/advanced_generator.rs` - 插件元数据生成
- `src-tauri/src/generators/prompt_templates.rs` - 提示词模板
- `src-tauri/src/tools/generator_tools.rs` - 复用检查逻辑
- `src-tauri/sentinel-passive/src/database.rs` - 数据库查询

## 未来优化方向

1. **智能复用评分**：基于网站相似度评估插件适用性
2. **插件版本管理**：支持插件更新和版本控制
3. **用户反馈机制**：根据检测效果调整复用策略
4. **自动优化**：基于使用数据优化插件质量

