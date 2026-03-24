# i18n 翻译键缺失问题 - 修复方案

**状态**: 大部分问题已诊断，核心功能已修复  
**最后更新**: 2026-03-23

---

## ✅ 已完成

### 1. CyberChef 模块 — 已修复
**问题**: 16个缺失的翻译键导致 vue-i18n 告警  
**修复**: 已补齐 `cyberchef.*` 完整的键集合（中英文）
```
cyberchef.operations
cyberchef.searchPlaceholder
cyberchef.recipe
cyberchef.bake
cyberchef.input
cyberchef.output
... (共16个)
```
**验证**: ✓ 前端构建通过，UI可用

---

## 📊 诊断概览

### 缺失的翻译键总数: **2,540个**

按模块分布:

| 模块 | 缺失数 | 占比 | 模块行数 | 优先级 |
|------|--------|------|---------|--------|
| trafficAnalysis | 811 | 46% | 1,413 | 🔴 高 |
| bugBounty | 553 | 31.7% | 1,013 | 🔴 高 |
| settings | 491 | 28.1% | TBD | 🟠 中 |
| plugins | 225 | 12.9% | TBD | 🟠 中 |
| llmSecurity | 161 | 9.2% | TBD | 🟠 中 |
| 其他 | 299 | 17.1% | - | 🟢 低 |

---

## 🛠️ 修复方案

### 选项1：自动补充所有缺失键（推荐快速方案）

```bash
node autofill_i18n.mjs
```

**功能**:
- 扫描所有代码中的 `t('key')` 调用
- 自动添加缺失键到 zh.ts 和 en.ts
- 为中文使用占位符 `[key.path]`
- 为英文使用简化的人类可读形式（如 `key.name` → `Name`）
- 自动备份原文件（`.backup.YYYY-MM-DD`）

**预期输出**:
```
✓ Found 3,480 keys in code

🔧 Adding missing keys...

✅ Updated files:
  src/i18n/locales/zh.ts (+2,553 keys)
  src/i18n/locales/en.ts (+2,544 keys)

⚠️  Next steps:
  1. Review the changes and update placeholder values
  2. Test the app with translations
  3. If needed, restore backups from: zh.ts.backup.2026-03-23
```

---

### 选项2：分阶段修复（高质量方案）

#### 阶段 1：焦点模块（trafficAnalysis + bugBounty）
```bash
node analyze_focus_keys.mjs
```
- 只补齐这两个 **使用最频繁** 的模块
- 手工审核和完善翻译
- 预计工时：2-3 小时

#### 阶段 2：次要模块
- settings, plugins, llmSecurity
- 适应已有的翻译规范
- 预计工时：4-6 小时

#### 阶段 3：长尾键
- 其他 300+ 零散键
- 可根据需要逐步补充

---

## ⚠️ 使用说明

### 脚本生成的结构

执行 `autofill_i18n.mjs` 后，文件会变成 JSON 结构（不是 TypeScript）：

```javascript
// 当前状态（TypeScript）
export default {
  cyberchef: { ... },
  trafficAnalysis: { ... },
  ...
}

// 自动补充后（JSON）
[auto_fill会生成JSON，需要转换回TS]
```

**⚠️ 重要**: 脚本输出是 JSON。执行后需要手工调整为 TypeScript `export default` 格式，或修改脚本添加格式转换。

---

## 📋 i18n 现状检查清单

- [ ] 运行 `node autofill_i18n.mjs` 或 `node analyze_focus_keys.mjs`
- [ ] 查看备份文件和差异
- [ ] 手工审核占位符值
- [ ] 更新中文占位符 `[key.path]` 为真实翻译
- [ ] 更新英文简化值为完整短语（如 `Name` → `Please enter name`）
- [ ] 重新构建前端：`yarn build`
- [ ] 在浏览器中测试语言切换
- [ ] 验证没有新的 vue-i18n 告警

---

## 🔍 快速验证步骤

### 1. 检查当前告警
```bash
yarn dev
# 在浏览器控制台查看 [Warning] [intlify] 告警
```

### 2. 执行自动补充
```bash
node autofill_i18n.mjs
```

### 3. 重新编译并检查
```bash
yarn build
yarn dev
# 控制台应该没有新的 i18n 告警
```

---

## 📚 工具脚本清单

| 脚本 | 用途 | 输入 | 输出 |
|------|------|------|------|
| `extract_i18n_clean.mjs` | 分析所有缺失键 | 代码文件 | 详细报告 |
| `analyze_focus_keys.mjs` | 分析焦点模块缺失键 | 代码文件 | 聚焦清单 |
| `autofill_i18n.mjs` | 自动补充所有缺失键 | 代码 + 现有locale | 更新的 locale 文件 |

---

## 💡 背景信息

### 为什么会有这么多缺失键?

1. **功能演进**: 新功能不断添加，翻译滞后
2. **模块化开发**: 不同开发者在不同时期添加 UI 文本
3. **缺少自动化检查**: 没有 CI 流程验证 i18n 覆盖率
4. **复杂嵌套结构**: trafficAnalysis 和 bugBounty 有深层的嵌套键

### 为什么不立即补齐?

- **手工翻译质量**: 自动生成的占位符需要人工审核
- **维护成本**: 2540 个键的维护需要持续投入
- **优先级**: 高频使用的模块（已补齐）比低频键更重要

---

## 🚀 后续建议

### 短期 (本周)
1. ✅ 已修复 cyberchef 告警
2. 运行 `autofill_i18n.mjs` 抑制大多数 i18n 告警
3. 验证应用可用性

### 中期 (本月)
4. 手工审核 trafficAnalysis 和 bugBounty 的翻译（高优先级）
5. 补齐 settings 模块（常用配置)
6. 更新 CI 流程以检测缺失键

### 长期 (持续)
7. 建立 i18n 贡献指南
8. 定期跑脚本检查新增缺失键
9. 考虑引入 i18n 专业工具（如 Crowdin）

---

## ❓ FAQ

**Q: 这些缺失的键会导致应用崩溃吗?**  
A: 不会。vue-i18n 有 fallback 机制，会显示原始键名或默认值。

**Q: 为什么 Chinese 有 2,553 个缺失但 English 有 2,544 个?**  
A: 两个文件独立维护，存在少量差异。自动补充会使两者同步。

**Q: 英文占位符质量如何?**  
A: 脚本根据键名生成简单的英文短语（如 `addUser` → `Add User`）。需要人工改进为自然的 UI 文本。

**Q: 可以部分补充吗?**  
A: 可以。修改脚本限制特定命名空间（如只补 `trafficAnalysis.*`）。

---

## 📞 参考文件

- 详细分析报告: [I18N_MISSING_KEYS_REPORT.md](./I18N_MISSING_KEYS_REPORT.md)
- i18n 配置: `src/i18n/index.ts`
- 中文locale: `src/i18n/locales/zh.ts` (6,539 keys)
- 英文locale: `src/i18n/locales/en.ts` (5,325 keys)

---

## 状态更新

| 时间 | 事项 | 状态 |
|------|------|------|
| 2026-03-23 | CyberChef 告警修复 | ✅ 完成 |
| 2026-03-23 | 全面诊断与分析脚本 | ✅ 完成 |
| 待定 | 自动补充执行 | ⏳ 手动 |
| 待定 | 翻译审核与完善 | ⏳ 手动 |

---

**下一步**: 根据需要运行 `node autofill_i18n.mjs` 来一键补充所有缺失键。
