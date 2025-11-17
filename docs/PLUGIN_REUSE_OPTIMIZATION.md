# 插件复用优化

## 问题描述

之前的插件生成流程会无条件生成所有请求的插件类型，即使数据库中已经存在高质量的同类型插件。这导致：
1. 浪费 LLM API 调用和时间
2. 生成重复的插件
3. 用户体验不佳（等待时间长）

## 解决方案

### 1. 数据库查询方法

在 `sentinel-passive/src/database.rs` 中新增 `find_reusable_plugins_by_category` 方法：

```rust
pub async fn find_reusable_plugins_by_category(
    &self,
    category: &str,
    min_quality_score: f64,
) -> Result<Vec<serde_json::Value>>
```

**查询条件**：
- 匹配指定的 `category`（漏洞类型）
- 质量分数 >= 最低阈值（默认 70 分）
- 验证状态为 `Approved` 或 `Passed`
- 主类别为 `passiveScan`
- 按质量分数和更新时间排序，返回前 5 个

### 2. 生成前检查逻辑

在 `generator_tools.rs` 的 `execute` 方法中，生成前进行检查：

```rust
// 检查数据库中是否已有可复用的高质量插件
for vuln_type in &vuln_types {
    match db_service.find_reusable_plugins_by_category(vuln_type, min_quality_score).await {
        Ok(existing_plugins) if !existing_plugins.is_empty() => {
            // 找到可复用的插件，记录并跳过生成
            reused_plugins.push(best_plugin);
        }
        _ => {
            // 没有找到合适的插件，需要生成
            types_to_generate.push(vuln_type.clone());
        }
    }
}
```

### 3. 输出优化

生成结果会明确区分：
- **复用的插件**：显示 ♻️ 标记和已有插件信息
- **新生成的插件**：显示 🆕 标记和生成详情

统计信息包括：
- Total Plugins（总数）
- Newly Generated（新生成）
- Reused Existing（复用）

## 使用场景

### 场景 1：全部复用

如果所有请求的插件类型都有高质量插件：
```
✅ All 3 plugin types already have high-quality plugins in database.
Reused existing plugins instead of generating new ones.

♻️  Reused Existing High-Quality Plugins:
1. XSS Detection Plugin (ID: xss_001)
   Type: xss
   Quality Score: 85.0/100
   Status: Already in database

2. SQLi Detection Plugin (ID: sqli_002)
   Type: sqli
   Quality Score: 90.0/100
   Status: Already in database
```

### 场景 2：部分复用

如果部分插件类型有高质量插件：
```
🤖 AI Plugin Generation Complete
Total: 3 plugins (1 generated, 2 reused)

♻️  Reused Existing High-Quality Plugins:
1. XSS Detection Plugin (ID: xss_001)
   Type: xss
   Quality Score: 85.0/100

🆕 Newly Generated Plugins:
1. IDOR Detection Plugin (ID: idor_new_001)
   Type: idor
   Quality Score: 78.0/100
   Status: Approved
```

## 配置参数

- **最低质量分数阈值**：`min_quality_score = 70.0`
  - 只有质量分数 >= 70 的插件才会被复用
  - 可根据需要调整此阈值

- **查询数量限制**：`LIMIT 5`
  - 每个类别最多返回 5 个候选插件
  - 选择质量分数最高的第一个

## 优势

1. **节省成本**：避免重复调用 LLM API
2. **提高速度**：跳过已有高质量插件的生成
3. **保证质量**：优先使用经过验证的高质量插件
4. **更好的用户体验**：明确告知用户哪些插件被复用

## 相关文件

- `src-tauri/sentinel-passive/src/database.rs`
- `src-tauri/src/tools/generator_tools.rs`

## 日志示例

```
[INFO] Checking for reusable plugins for type: xss
[INFO] Found reusable plugin for xss: XSS Detection Plugin (ID: xss_001, Quality: 85.0)
[INFO] Checking for reusable plugins for type: sqli
[INFO] No reusable plugin found for sqli, will generate new one
[INFO] Will generate 1 new plugins: ["sqli"]
```

