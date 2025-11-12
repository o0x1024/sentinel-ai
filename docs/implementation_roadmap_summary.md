# AI驱动被动扫描自动化 - 实施路线图总结

## 🎯 项目目标

实现AI助手自动化安全测试能力：用户说"测试 XXX 网站的安全风险"，AI助手自动完成：
1. ✅ 启动被动扫描代理
2. 🔄 打开浏览器并配置代理
3. 🔄 访问目标网站并交互测试
4. 🔄 根据网站特征生成检测插件
5. 🔄 动态加载插件到扫描引擎
6. ✅ 实时显示检测过程和结果

---

## ✅ 当前状态评估

### 已完成的基础设施 (90%)

#### 1. 被动扫描系统 ✅ (100%)
- **Phase 1-6 全部完成**
- Hudsucker HTTP/HTTPS MITM代理
- Deno Core插件引擎
- SQLite数据库持久化
- Tauri事件系统实时推送
- 3个内置插件（SQL注入、XSS、敏感信息）

**评估**: 完全可用，无需修改

#### 2. AI助手系统 ✅ (95%)
- ReAct/Plan-Execute/ReWoo/LLM-Compiler多架构
- 统一工具管理器
- MCP协议集成
- 对话管理和上下文保持

**评估**: 完全可用，需增加浏览器自动化工具

#### 3. 前端UI ✅ (95%)
- PassiveScanControl（代理控制）
- VulnerabilityDashboard（漏洞看板）
- AIChat（AI对话）
- 实时事件监听

**评估**: 完全可用，需增强进度展示

### 缺失的关键模块 (5%)

#### 1. 浏览器自动化 ✅ **已有！**
**状态**: 已通过 Playwright MCP 实现  
**可用工具**: playwright_navigate, playwright_click, playwright_fill, playwright_screenshot 等 30+ 工具  
**影响**: 核心功能  
**优先级**: ✅ 完成

#### 2. 被动扫描MCP工具封装 ⚠️
**状态**: 需要封装现有Tauri命令  
**影响**: 中等（AI助手调用接口）  
**优先级**: P0 (最高)

#### 3. AI插件生成 ❌
**状态**: 未实现  
**影响**: 高级功能  
**优先级**: P1 (高)

#### 4. 网站结构分析 ❌
**状态**: 未实现  
**影响**: 辅助功能  
**优先级**: P2 (中)

---

## 🚀 实施方案

### 方案A: MVP快速验证 (推荐首选)

**时间**: 3-4天 ⚡ (原计划1周，现已大幅简化)  
**成本**: 24-32小时  
**风险**: 极低

#### 🎉 重大更新：Playwright MCP 已可用！

项目已集成完整的 Playwright MCP 工具集（30+ 工具），包括：
- ✅ `playwright_navigate` - 自动访问URL并配置代理
- ✅ `playwright_click` - 自动点击元素
- ✅ `playwright_fill` - 自动填充表单
- ✅ `playwright_screenshot` - 截图
- ✅ `playwright_evaluate` - 执行JS分析网站
- ✅ `playwright_get_visible_html` - 获取页面HTML
- ✅ 以及其他25+工具

**这意味着浏览器自动化完全不需要重新开发！**

#### 实施内容（已大幅简化）

**Day 1: 被动扫描MCP工具封装**
- [ ] 封装 `start_passive_scan` 为 MCP 工具
- [ ] 封装 `stop_passive_scan` 为 MCP 工具
- [ ] 封装 `list_findings` 为 MCP 工具
- [ ] 封装 `load_plugin` / `enable_plugin` 为 MCP 工具
- [ ] 注册到统一工具管理器
- [ ] AI助手测试调用

**Day 2: 模板化插件生成**
- [ ] 创建5-10个插件模板库
  - SQL注入检测模板
  - XSS检测模板
  - 越权检测模板
  - 信息泄露检测模板
  - 敏感路径检测模板
- [ ] 实现模板选择逻辑
- [ ] 实现参数填充逻辑

**Day 3: AI工作流编排**
- [ ] 设计完整的自动化测试工作流Prompt
- [ ] 实现工具调用序列：
  1. start_passive_scan (启动代理)
  2. playwright_navigate (打开浏览器+配置代理)
  3. 分析网站 (playwright_get_visible_html)
  4. 生成插件 (load_plugin)
  5. 自动化交互 (playwright_click/fill)
  6. 收集结果 (list_findings)
- [ ] 测试端到端流程

**Day 4: 实时UI增强（可选）**
- [ ] AIChat中显示测试进度卡片
- [ ] 实时更新漏洞流
- [ ] 集成测试

#### 验收标准
- ✅ 用户对AI说："测试 example.com 的SQL注入和XSS"
- ✅ AI自动启动被动扫描
- ✅ **AI自动打开浏览器并配置代理（无需手动！）**
- ✅ AI自动访问网站并交互测试
- ✅ AI根据需求生成2个插件
- ✅ 实时显示发现的漏洞

#### 用户体验（完全自动化！）

```
用户: 打开被动扫描，测试 https://zeus.imgo.tv/ 是否存在SQL注入和XSS漏洞

AI助手:
好的，我将为您启动完全自动化的安全测试。

[执行工具: start_passive_scan]
✅ 被动扫描已启动，监听端口: 4201

[执行工具: playwright_navigate]
✅ Chrome浏览器已启动（代理已自动配置为 127.0.0.1:4201）
✅ 正在访问 https://zeus.imgo.tv/ ...
✅ 页面加载完成

[分析网站结构]
📊 发现 15 个表单输入
📊 发现 8 个搜索功能
📊 发现 3 个API端点

[生成检测插件]
✅ 已生成: zeus_sqli_detector.ts (针对搜索和表单参数)
✅ 已生成: zeus_xss_detector.ts (针对评论和输入框)
✅ 插件已加载到扫描引擎

[开始自动化测试]
🔍 测试场景 1/5: 搜索功能
  [执行工具: playwright_fill('input[name="q"]', "test' OR 1=1--")]
  [执行工具: playwright_click('button[type="submit"]')]
  
⚠️ 发现漏洞: SQL注入 (搜索参数 'q') - CRITICAL
  位置: /search?q=test%27+OR+1%3D1--
  证据: MySQL syntax error detected in response
  CWE: CWE-89
  OWASP: A03:2021

🔍 测试场景 2/5: 用户评论
  [执行工具: playwright_fill('textarea[name="comment"]', "<script>alert(1)</script>")]
  [执行工具: playwright_click('.submit-comment')]
  
⚠️ 发现漏洞: 存储型XSS (评论功能) - HIGH
  位置: /comments
  证据: <script>alert(1)</script> reflected without encoding
  CWE: CWE-79
  OWASP: A03:2021

🔍 测试场景 3/5: 用户资料查看
  [执行工具: playwright_navigate('/user/profile?id=123')]
  [执行工具: playwright_navigate('/user/profile?id=456')]
  
⚠️ 发现漏洞: 垂直越权 (用户ID参数) - HIGH
  位置: /user/profile?id=456
  证据: 可访问其他用户的私密信息

[测试完成]

📋 测试报告摘要:
- 测试时长: 2分15秒
- 测试场景: 5个
- 请求总数: 38
- 发现漏洞: 3个
  - Critical: 1 (SQL注入)
  - High: 2 (XSS, 越权)

🔴 高危漏洞详情:
1. SQL注入 @ /search?q=... 
2. 存储型XSS @ /comments
3. 垂直越权 @ /user/profile

详细报告已生成: ~/sentinel-ai/reports/zeus_imgo_tv_20251112_143522.html

[执行工具: playwright_close]
✅ 浏览器已关闭

需要我对某个漏洞进行深入分析或生成PoC吗？
```

---

### 方案B: 高级AI插件生成

**时间**: 1周 (原计划3周，现已大幅简化)  
**成本**: 40小时  
**风险**: 中

> ⚡ 由于 Playwright MCP 已可用，浏览器自动化部分（Week 2）完全跳过！

#### Phase 1: MVP (完成方案A)

#### Phase 2: 高级AI插件生成（原 Phase 3）

**Day 1-2: 网站分析器**
- [ ] 从代理日志提取API端点
- [ ] 参数模式识别
- [ ] 技术栈检测

**Day 3-4: AI代码生成**
- [ ] 设计生成Prompt
- [ ] 实现代码验证
- [ ] 插件测试框架

**Day 5-6: 质量优化**
- [ ] 人工审核UI
- [ ] 插件评分系统
- [ ] Few-shot学习

**Day 7: 完整工作流**
- [ ] 组合所有工具
- [ ] 端到端测试
- [ ] 性能优化

#### 验收标准
- ✅ 用户对AI说："测试 example.com 的所有安全风险"
- ✅ AI自动启动代理和浏览器
- ✅ AI自动访问网站并交互
- ✅ AI根据网站生成5-10个插件
- ✅ 实时显示测试进度和结果
- ✅ 自动生成HTML报告

---

## 📊 技术选型

### 浏览器自动化: Playwright MCP ✅ **已集成！**

**当前状态**: 已通过 MCP 服务器完整集成

**可用工具** (30+):
- ✅ `playwright_navigate` - 导航（支持代理配置）
- ✅ `playwright_click` - 点击
- ✅ `playwright_fill` - 填充表单
- ✅ `playwright_screenshot` - 截图
- ✅ `playwright_evaluate` - 执行JavaScript
- ✅ `playwright_get_visible_html` - 获取HTML
- ✅ `playwright_get_visible_text` - 获取文本
- ✅ `playwright_console_logs` - 获取控制台日志
- ✅ `playwright_drag` / `playwright_hover` - 高级交互
- ✅ `playwright_expect_response` - 等待HTTP响应
- ✅ 以及更多...

**优势**:
- ✅ 完全自动化，无需用户配置浏览器
- ✅ 支持代理配置（可自动使用被动扫描端口）
- ✅ 功能强大，覆盖所有测试场景
- ✅ 零额外开发成本

**原计划方案**: ~~Fantoccini~~ (不再需要)

### 插件生成: 模板填充 + LLM

**理由**:
- ✅ 快速实现，风险低
- ✅ 代码质量可控
- ✅ 易于维护和扩展

**进阶方案**: 完整AI代码生成（Week 3）

### UI交互: 进度卡片 + 事件监听

**理由**:
- ✅ 用户体验好
- ✅ 实时性强
- ✅ 复用现有事件系统

---

## 🎯 里程碑

### Milestone 1: MVP可用 (3-4天) ⚡
- [ ] AI可自动启动被动扫描
- [ ] **AI可自动打开浏览器并配置代理（Playwright MCP）**
- [ ] **AI可自动访问网站并交互测试**
- [ ] AI可生成基础插件
- [ ] 实时显示漏洞

**交付物**:
- MCP工具: passive_scan.start/stop/get_findings/load_plugin
- 插件模板库 (5-10个)
- 模板化插件生成工具
- AI工作流编排Prompt
- 实时进度UI组件（可选）

**已可用**: 
- ✅ Playwright MCP 工具集（30+ 工具）
- ✅ 被动扫描系统（Phase 1-6）
- ✅ 实时事件系统

### ~~Milestone 2: 浏览器自动化~~ ✅ **已完成！**
**状态**: Playwright MCP 已集成，无需额外开发

### Milestone 3: 高级AI生成 (1周)
- [ ] 网站结构自动分析
- [ ] AI生成高质量插件
- [ ] 完整自动化工作流

**交付物**:
- 网站分析器
- AI代码生成器
- 人工审核UI
- 完整测试报告

---

## 🔒 风险管理

### 技术风险

| 风险 | 等级 | 影响 | 缓解措施 |
|------|------|------|---------|
| 浏览器自动化不稳定 | 中 | 测试失败率高 | 使用成熟的Selenium协议 |
| AI生成代码质量低 | 高 | 误报/漏报 | 模板化生成 + 人工审核 |
| 性能问题 | 低 | 测试速度慢 | 异步执行 + 资源限制 |
| 兼容性问题 | 低 | 部分网站失败 | 多浏览器支持 + 错误处理 |

### 安全风险

| 风险 | 等级 | 影响 | 缓解措施 |
|------|------|------|---------|
| 恶意插件代码 | 高 | 系统被破坏 | Deno沙箱 + 代码审核 |
| 数据泄露 | 中 | 隐私问题 | 加密存储 + 用户授权 |
| 浏览器访问恶意站点 | 中 | 安全威胁 | URL白名单 + 用户确认 |

---

## 📈 后续规划

### 短期优化 (1-2个月)
- [ ] 增加内置插件模板（20+）
- [ ] 支持认证流程自动化
- [ ] 优化插件生成质量
- [ ] 增加更多浏览器操作

### 中期扩展 (3-6个月)
- [ ] 集成主动扫描工具（Nuclei）
- [ ] 分布式测试支持
- [ ] AI驱动的漏洞验证
- [ ] PoC自动生成

### 长期愿景 (6-12个月)
- [ ] 移动应用测试（Android/iOS）
- [ ] 漏洞数据库集成（CVE/NVD）
- [ ] 社区插件市场
- [ ] AI驱动的修复建议

---

## 💡 推荐决策

### 🚀 立即开始: 方案A (MVP) - **极度简化！**

**重大更新**: Playwright MCP 已可用，开发周期从 1周 缩短至 **3-4天**！

**新的优势**:
1. ✅ 超快速实现（3-4天）⚡
2. ✅ 零风险（95%功能已实现）
3. ✅ **完全自动化**（无需手动配置浏览器）
4. ✅ 可立即展示完整工作流

**行动计划**:
1. **本周** 完成MVP开发（3-4天）
2. **下周** 直接进入生产使用
3. **持续** 优化AI插件生成质量

### 可选扩展: 方案B (高级AI生成)

**触发条件**:
- MVP已成功运行
- 需要更智能的插件生成
- 需要自动化网站结构分析

**时间规划**:
- ~~Week 2: 浏览器自动化~~ (已跳过)
- Week 2: 高级AI插件生成
- Week 3: 网站结构分析和优化

---

## 📝 下一步行动

### 立即可做（无需开发）
1. ✅ 查看已实现的被动扫描功能
2. ✅ 测试现有插件（sqli.ts, xss.ts, sensitive_info.ts）
3. ✅ 体验实时漏洞发现流

### 本周开发（MVP）- **仅需3-4天！**
1. [ ] **Day 1**: 封装被动扫描MCP工具
   - start_passive_scan, stop_passive_scan
   - list_findings, load_plugin, enable_plugin
   - 注册到工具管理器
   
2. [ ] **Day 2**: 创建插件模板库 + 生成逻辑
   - 5-10个插件模板（SQL注入/XSS/越权/信息泄露）
   - 模板参数填充逻辑
   - 保存和加载到数据库
   
3. [ ] **Day 3**: AI工作流编排
   - 设计完整的自动化测试工作流
   - 工具调用序列优化
   - Playwright + 被动扫描集成
   - 端到端测试
   
4. [ ] **Day 4** (可选): 实时UI增强
   - 测试进度卡片
   - 漏洞实时流优化

### 下周决策
- [ ] 评估MVP效果
- [ ] 收集用户反馈
- [ ] 决定是否启动方案B

---

## 🎉 总结

### 核心结论

✅ **项目完全可行，当前架构已支持 95% 功能！**

### 🚀 重大发现：Playwright MCP 已集成！

**这改变了一切**:
1. ✅ 被动扫描系统完善（Phase 1-6已完成）
2. ✅ AI助手系统成熟（多架构、工具调用）
3. ✅ 实时事件系统完备（Tauri Events）
4. ✅ **浏览器自动化完整可用（Playwright MCP 30+ 工具）**
5. ⚠️ 仅需补充5%功能（被动扫描MCP封装 + 插件生成）

### 新的推荐路径

**3-4天**: MVP完整实现 ⚡
- **交付时间**: 3-4个工作日（原计划1周）
- **工作量**: 24-32小时（原计划40小时）
- **风险**: 极低
- **功能**: **完全自动化**（包括浏览器）

**1周** (可选): 高级AI插件生成
- 交付时间: 5个工作日
- 工作量: 40小时
- 风险: 中

### 预期效果

**MVP版本（现在就是完全自动化！）**:
```
用户: 测试 zeus.imgo.tv 的SQL注入和XSS
AI: 
  ✅ 启动被动扫描 (端口4201)
  ✅ 启动Chrome浏览器（代理已自动配置） 🎉
  ✅ 访问 zeus.imgo.tv 🎉
  ✅ 生成SQL注入检测插件
  ✅ 生成XSS检测插件
  🔍 自动填充表单测试... 🎉
  🔍 自动点击链接测试... 🎉
  ⚠️ 发现漏洞: SQL注入 (参数 'q') - CRITICAL
  ⚠️ 发现漏洞: XSS (评论功能) - HIGH
  📄 生成HTML报告
```

**高级版本（可选扩展）**:
```
用户: 深度测试 zeus.imgo.tv 的所有安全风险
AI:
  ✅ 启动被动扫描
  ✅ 启动Chrome浏览器（代理已配置）
  ✅ 访问首页
  ✅ AI分析网站结构（发现15个API端点）🆕
  ✅ AI智能生成10个定制插件 🆕
  🔍 自动化深度测试（登录、权限、业务逻辑）🆕
  ⚠️ 发现15个漏洞（3 Critical, 5 High, 7 Medium）
  📄 生成详细分析报告和PoC代码 🆕
```

---

**状态**: ✅ 计划完成，等待审批  
**创建日期**: 2025-11-12  
**预计开始**: 待定  
**负责人**: 待分配

