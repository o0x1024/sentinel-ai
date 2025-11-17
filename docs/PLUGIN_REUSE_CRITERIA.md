# 插件复用条件说明

## 当前复用机制

### 什么样的插件可以被复用？

插件必须**同时满足**以下所有条件才会被复用：

#### 1. **类别匹配** ✅
```sql
WHERE category = ?
```
- 插件的 `category` 字段必须与请求的漏洞类型完全匹配
- 例如：请求生成 `xss` 插件，只会查找 `category = 'xss'` 的插件

**支持的类别**：
- `sqli` - SQL 注入
- `xss` - 跨站脚本
- `idor` - 不安全的直接对象引用
- `info_leak` - 信息泄露
- `auth_bypass` - 认证绕过
- `csrf` - 跨站请求伪造
- 等等...

#### 2. **质量分数达标** ✅
```sql
AND quality_score >= ?
```
- 默认阈值：**70.0 分**（满分 100）
- 只有高质量的插件才会被复用
- 质量分数由以下四个维度计算：
  - 语法正确性（Syntax Score）
  - 逻辑完整性（Logic Score）
  - 安全考虑（Security Score）
  - 代码质量（Code Quality Score）

#### 3. **验证状态通过** ✅
```sql
AND validation_status IN ('Approved', 'Passed')
```
- `Approved` - 自动审批通过（质量分数 ≥ 80）
- `Passed` - 验证通过

**不会复用的状态**：
- `PendingReview` - 待审核
- `Rejected` - 已拒绝
- `ValidationFailed` - 验证失败
- `NULL` - 未设置状态

#### 4. **主类别正确** ✅
```sql
AND main_category = 'passiveScan'
```
- 只复用被动扫描插件
- 不会复用其他类型的插件（如 `agent` 工具插件）

#### 5. **排序和选择** 📊
```sql
ORDER BY quality_score DESC, updated_at DESC
LIMIT 5
```
- 按质量分数**从高到低**排序
- 相同质量分数时，优先选择**最新更新**的
- 最多返回 5 个候选插件
- 实际使用：**选择质量最高的第一个**

## 复用流程

### 步骤 1：检查每个请求的漏洞类型

```rust
for vuln_type in &vuln_types {
    // 查询数据库
    match db_service.find_reusable_plugins_by_category(vuln_type, 70.0).await {
        Ok(existing_plugins) if !existing_plugins.is_empty() => {
            // 找到可复用的插件
            reused_plugins.push(best_plugin);
        }
        _ => {
            // 需要生成新插件
            types_to_generate.push(vuln_type);
        }
    }
}
```

### 步骤 2：决定是否生成

- **全部复用**：所有请求的类型都有高质量插件 → 跳过生成，直接返回
- **部分复用**：部分类型有高质量插件 → 只生成缺失的类型
- **全部生成**：没有任何可复用的插件 → 生成所有请求的类型

## 实际示例

### 示例 1：完全复用 ✅

**数据库中的插件**：
```
ID: ai_gen_xss_20251114_063139
名称: Cross-Site Scripting (XSS) Detector
类别: xss
质量分数: 85.0
验证状态: Approved
主类别: passiveScan
```

**用户请求**：生成 `xss` 插件

**结果**：
```
✅ 找到可复用插件: XSS Detector (质量: 85.0)
✅ 跳过生成，直接使用现有插件
⏱️ 节省时间: ~2-3 分钟
💰 节省成本: 1 次 LLM API 调用
```

### 示例 2：部分复用 ⚡

**数据库中的插件**：
```
1. XSS Detector (xss, 质量: 85.0, Approved)
2. SQLi Detector (sqli, 质量: 90.0, Approved)
```

**用户请求**：生成 `xss`, `sqli`, `idor` 插件

**结果**：
```
✅ xss: 复用现有插件 (质量: 85.0)
✅ sqli: 复用现有插件 (质量: 90.0)
🆕 idor: 无可复用插件，生成新插件
⏱️ 节省时间: ~4-6 分钟（2/3）
💰 节省成本: 2 次 LLM API 调用
```

### 示例 3：无法复用 ❌

**数据库中的插件**：
```
ID: ai_gen_xss_20251114_063139
名称: XSS Detector
类别: xss
质量分数: 65.0  ❌ 低于阈值 70
验证状态: PendingReview  ❌ 未通过审批
```

**用户请求**：生成 `xss` 插件

**结果**：
```
❌ 找到插件但不符合复用条件:
   - 质量分数 65.0 < 70.0 (阈值)
   - 验证状态为 PendingReview (需要 Approved/Passed)
🆕 生成新的 XSS 插件
```

## 配置参数

### 当前配置

```rust
// 在 generator_tools.rs 中
let min_quality_score = 70.0; // 最低质量分数阈值
```

### 可调整的参数

1. **质量分数阈值**（`min_quality_score`）
   - 当前值：70.0
   - 建议范围：60.0 - 85.0
   - 越高越严格，复用率越低

2. **候选插件数量**（`LIMIT`）
   - 当前值：5
   - 只影响查询性能，实际只用第一个

3. **验证状态**（`validation_status`）
   - 当前：`IN ('Approved', 'Passed')`
   - 可扩展：添加其他可信状态

## 如何查看可复用的插件

### 方法 1：查看日志

生成插件时会输出日志：
```
[INFO] Checking for reusable plugins for type: xss
[INFO] Found reusable plugin for xss: XSS Detector (ID: ai_gen_xss_20251114, Quality: 85.0)
```

或：
```
[INFO] Checking for reusable plugins for type: idor
[INFO] No reusable plugin found for idor, will generate new one
```

### 方法 2：查看插件管理界面

在插件管理页面可以看到：
- 插件名称（不包含网站信息的是通用插件）
- 质量分数
- 验证状态
- 标签（`ai-generated`）

### 方法 3：直接查询数据库

```sql
SELECT id, name, category, quality_score, validation_status
FROM plugin_registry
WHERE main_category = 'passiveScan'
  AND quality_score >= 70.0
  AND validation_status IN ('Approved', 'Passed')
ORDER BY category, quality_score DESC;
```

## 复用策略建议

### 推荐做法 ✅

1. **首次生成后保留高质量插件**
   - 质量分数 ≥ 80 的插件会自动审批
   - 这些插件可以被后续扫描复用

2. **定期清理低质量插件**
   - 删除质量分数 < 60 的插件
   - 删除验证失败的插件

3. **手动审核待审插件**
   - 对于 `PendingReview` 状态的插件
   - 如果测试效果好，手动改为 `Approved`

### 不推荐做法 ❌

1. **降低质量阈值到 < 60**
   - 可能复用低质量插件
   - 影响检测准确性

2. **复用未验证的插件**
   - 可能包含语法错误
   - 可能无法正常运行

3. **禁用所有 AI 生成的插件**
   - 无法享受自动化的便利
   - 需要手动编写所有插件

## 总结

**一个插件可以被复用的完整条件**：

```
✅ category = 请求的漏洞类型
✅ quality_score >= 70.0
✅ validation_status IN ('Approved', 'Passed')
✅ main_category = 'passiveScan'
✅ 按质量分数排序，选择最高的
```

**简单记忆**：
> 高质量（≥70分）、已审批、同类型的被动扫描插件会被复用

