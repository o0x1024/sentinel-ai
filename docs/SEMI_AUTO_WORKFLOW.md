# 半自动化工作流程说明

## 📋 概述

方案B已实现**智能半自动化审核流程**，根据插件质量自动决策：
- **高质量插件（>=80分）**：自动批准 ✅
- **中等质量插件（60-80分）**：需要人工审核 ⚠️
- **低质量插件（<60分）**：自动拒绝或重新生成 ❌

## 🔄 完整工作流程

### 用户发起扫描请求

```
用户: "测试 http://testphp.vulnweb.com 是否存在SQL注入和XSS漏洞，
      开启被动扫描，并设置浏览器代理为被动扫描的端口"
```

### Step 1-4: 全自动执行

AI助手会自动执行以下步骤（无需人工介入）：

1. **启动被动扫描代理**
   ```
   Action: start_passive_scan
   Result: 代理启动在 http://127.0.0.1:8080
   ```

2. **配置浏览器代理并访问目标**
   ```
   Action: playwright_navigate
   Input: {
     "url": "http://testphp.vulnweb.com",
     "proxy": {"server": "http://127.0.0.1:8080"}
   }
   ```

3. **自动交互收集流量**
   ```
   - 浏览主要页面
   - 填写表单
   - 点击链接
   - 测试API端点
   ```

4. **分析网站结构**
   ```
   Action: analyze_website
   Input: {
     "domain": "testphp.vulnweb.com",
     "limit": 1000
   }
   Result: {
     endpoints: ["/search", "/login", "/api/user"],
     tech_stack: ["PHP", "MySQL", "Apache"],
     parameters: {
       "query": ["GET", "vulnerable"],
       "username": ["POST", "auth"]
     }
   }
   ```

5. **生成AI插件**
   ```
   Action: generate_advanced_plugin
   Input: {
     "analysis": [Step 4结果],
     "vuln_types": ["sqli", "xss"],
     "target_endpoints": null
   }
   ```

### Step 5: 自动评估和分类 🤖

生成器内部自动执行：

```rust
// 1. 生成插件代码
let code = llm.generate(prompt);

// 2. 语法验证
let validation = validator.validate(code);

// 3. 质量评分
let quality_score = calculate_quality(code, validation);

// 4. 自动批准决策
let decision = auto_approval_engine.evaluate_plugin(
    quality_score,
    validation_status,
    code,
    attempt
);

// 5. 应用决策
match decision {
    AutoApprove => {
        status = Approved;  // 直接批准，跳过Step 6
        log::info!("✅ Auto-approved: score={}", quality_score);
    },
    RequireHumanReview => {
        status = PendingReview;  // 进入Step 6人工审核
        log::info!("⚠️ Requires review: score={}", quality_score);
    },
    AutoReject => {
        status = Rejected;  // 直接拒绝
        log::error!("❌ Auto-rejected: score={}", quality_score);
    },
    Regenerate => {
        // 重新生成（最多2次）
        log::warn!("🔄 Regenerating: score={}", quality_score);
    }
}
```

### Step 6: 人工审核（仅中等质量插件）⚠️

**当且仅当** 质量分数在60-80分时，才需要人工介入：

1. **查看审核队列**
   - 访问: `http://localhost:1420/plugin-review`
   - 看到需要审核的插件列表

2. **审核插件**
   ```
   插件信息：
   - ID: ai_gen_sqli_testphp_vulnweb_com_20251113_143022
   - 类型: SQL Injection
   - 质量分数: 72.5 ⚠️ (需要审核)
     - 语法分: 85/100
     - 逻辑分: 70/100
     - 安全分: 68/100
     - 代码质量: 67/100
   
   操作选项：
   [查看代码] [编辑] [批准] [拒绝]
   ```

3. **做出决策**
   - ✅ 批准：插件立即启用
   - ❌ 拒绝：标记为拒绝
   - ✏️ 编辑：修改后批准

### Step 7-8: 自动执行扫描 ✅

对于**已批准**的插件（自动批准 + 人工批准）：

1. **自动部署插件**
   ```
   System: Loading approved plugin: ai_gen_sqli_...
   Status: ✅ Plugin active
   ```

2. **自动检测漏洞**
   ```
   - 监听代理流量
   - 应用检测规则
   - 发现漏洞时自动记录
   ```

3. **报告结果**
   ```
   Action: list_findings
   Result: Found 3 vulnerabilities:
   - SQL Injection in /search?query=
   - XSS in /comment/submit
   - IDOR in /user/profile?id=
   ```

## 📊 自动化程度

### 默认配置（Balanced - 推荐）

| 质量分数 | 自动决策 | 需要人工 |
|---------|---------|---------|
| >= 80分 | ✅ 自动批准 | ❌ 不需要 |
| 60-80分 | ⚠️ 待审核 | ✅ 需要审核 |
| < 60分 | ❌ 自动拒绝/重生成 | ❌ 不需要 |

**自动化率**: 约 **70-80%**（假设70%的插件质量>=80分）

### 其他可选配置

#### Conservative (保守模式 - 手动为主)
```toml
auto_approve_threshold = 90  # 90分以上才自动批准
require_review_threshold = 0  # 0-90分都需要审核
```
- 自动化率: 约 **40-50%**
- 适用场景: 高安全要求环境

#### Aggressive (激进模式 - 自动为主)
```toml
auto_approve_threshold = 70  # 70分以上自动批准
require_review_threshold = 50  # 50-70分需要审核
```
- 自动化率: 约 **85-90%**
- 适用场景: 快速扫描、低风险目标

#### Manual Only (全手动)
```toml
enabled = false  # 关闭自动批准
```
- 自动化率: **0%**（所有插件都需要人工审核）
- 适用场景: 学习阶段、极端安全环境

## 🛠️ 配置管理

### 1. 获取当前配置

```typescript
// 前端调用
const config = await invoke('get_auto_approval_config');
console.log(config);
/*
{
  enabled: true,
  auto_approve_threshold: 80.0,
  require_review_threshold: 60.0,
  auto_reject_threshold: 60.0,
  auto_regenerate_on_low_quality: true,
  max_regeneration_attempts: 2,
  check_dangerous_patterns: true,
  dangerous_patterns: ["eval(", "fetch(", ...]
}
*/
```

### 2. 应用预设配置

```typescript
// 获取预设列表
const presets = await invoke('get_config_presets');

// 应用Balanced配置
const balancedConfig = presets.find(p => p.name.includes('Balanced')).config;
await invoke('update_auto_approval_config', { config: balancedConfig });
```

### 3. 自定义配置

```typescript
await invoke('update_auto_approval_config', {
  config: {
    enabled: true,
    auto_approve_threshold: 85.0,  // 自定义阈值
    require_review_threshold: 65.0,
    auto_reject_threshold: 65.0,
    auto_regenerate_on_low_quality: true,
    max_regeneration_attempts: 3,
    check_dangerous_patterns: true,
    dangerous_patterns: ["eval(", "Function("]
  }
});
```

### 4. 测试配置效果

```typescript
// 用历史插件质量分数测试新配置的影响
const testResult = await invoke('test_config_impact', {
  config: newConfig,
  testScores: [45, 55, 65, 75, 85, 95] // 测试数据
});

console.log(testResult);
/*
{
  total_plugins: 6,
  auto_approved: 2,    // 85, 95
  require_review: 2,   // 65, 75
  auto_rejected: 2,    // 45, 55
  automation_rate: 66.7%,  // (2+2)/6
  approval_rate: 33.3%     // 2/6
}
*/
```

## 🔐 安全保障

### 自动检测危险代码模式

即使质量分数很高，以下模式会**强制人工审核**：

```typescript
// ❌ 这些模式会触发人工审核
eval("some code");              // 代码执行
new Function("return x")();     // 动态函数
fetch("http://evil.com");       // 网络请求
XMLHttpRequest();               // HTTP请求
require("fs");                  // 文件系统访问
import("./module");             // 动态导入
Deno.readFile("/etc/passwd");   // 文件读取
Deno.writeFile("/tmp/x", data); // 文件写入
```

示例日志：
```
[INFO] Plugin ai_gen_xss_... quality_score=92.5
[WARN] Contains dangerous pattern: 'fetch('
[INFO] Decision: RequireHumanReview (security risk)
[INFO] Status changed: Approved -> PendingReview
```

## 💡 最佳实践

### 1. 首次使用建议

```
阶段1 (学习): Manual Only模式
  ↓ 审核50+插件，了解质量标准
阶段2 (验证): Conservative模式
  ↓ 验证自动批准的准确性
阶段3 (生产): Balanced模式
  ↓ 平衡效率和质量
阶段4 (高频): Aggressive模式（可选）
```

### 2. 质量提升循环

```
生成插件 → 自动评分 → 人工审核（中等质量）
    ↑                           ↓
    ←──── Few-shot学习 ← 高质量样本积累
```

### 3. 监控关键指标

```typescript
// 定期查看统计
const stats = await invoke('get_plugin_statistics');

// 关注指标：
- 自动批准率 > 70% ✅
- 人工审核率 < 25% ✅
- 自动拒绝率 < 10% ✅
- 平均质量分 > 75分 ✅
```

### 4. 调整阈值建议

| 问题 | 调整方案 |
|------|---------|
| 自动批准的插件质量不稳定 | 提高 `auto_approve_threshold` (80 → 85) |
| 人工审核工作量过大 | 降低 `require_review_threshold` (60 → 55) |
| 误拒高质量插件 | 降低 `auto_approve_threshold` (80 → 75) |
| 生成失败率过高 | 增加 `max_regeneration_attempts` (2 → 3) |

## 🎯 总结

### ✅ 全自动化部分（70-80%）
- 代理启动
- 浏览器控制
- 流量收集
- 网站分析
- 插件生成
- 质量评分
- **高质量插件批准** ⭐
- **低质量插件拒绝** ⭐
- 插件部署
- 漏洞检测

### ⚠️ 需要人工介入（20-30%）
- **中等质量插件审核** （60-80分）
- 危险代码模式审核
- 特殊场景判断

### 🎉 优势
1. **效率**: 70-80%的插件无需人工干预
2. **质量**: 自动拒绝低质量插件，保证基线
3. **安全**: 危险代码强制人工审核
4. **灵活**: 可根据场景调整自动化程度
5. **学习**: Few-shot机制持续提升生成质量

---

## 📞 常见问题

**Q: 如何完全关闭自动批准？**
```typescript
await invoke('update_auto_approval_config', {
  config: { enabled: false, ...其他配置 }
});
```

**Q: 可以手动批准被自动拒绝的插件吗？**  
A: 可以。在Plugin Review UI中可以查看所有插件（包括已拒绝的），可以手动批准。

**Q: 自动批准的插件有问题怎么办？**  
A: 可以在Plugin Review UI中禁用或删除插件，同时调高`auto_approve_threshold`阈值。

**Q: 如何查看自动批准的历史记录？**  
A: 查看日志文件 `logs/sentinel-ai.log`，搜索 "auto-approved" 关键词。

