# 专利调研摘要

## 技术概要

**发明名称**：基于大语言模型的网站自动化探索与API接口发现方法及系统

**技术领域**：人工智能 + Web安全测试 + 自动化测试

---

## 核心创新点（调研重点）

### 1. AI驱动的网站自动探索
- 使用VLM（视觉语言模型）分析页面截图做出浏览决策
- 使用LLM分析结构化元素列表做出浏览决策
- AI自主决定点击、输入、滚动、导航等操作
- **调研关键词**: `LLM web crawling`, `AI automated browser`, `VLM website exploration`, `intelligent web testing`

### 2. 元素标注索引系统
- 为页面元素生成唯一索引号
- 元素指纹算法（selector+type+text hash）
- 跨页面元素去重和交互追踪
- **调研关键词**: `element annotation system`, `web element fingerprint`, `DOM element indexing`

### 3. 被动代理集成API发现
- 浏览器通过代理访问网站
- 实时捕获所有HTTP请求
- 自动提取和分类API端点
- **调研关键词**: `passive proxy API discovery`, `traffic interception API extraction`, `automated API endpoint discovery`

### 4. 多维度覆盖率引擎
- 路由覆盖率（已访问/已发现路由）
- 元素覆盖率（已交互/总元素）
- 稳定性检测（连续无新发现轮次）
- **调研关键词**: `web crawling coverage`, `exploration completeness detection`, `website testing coverage metrics`

### 5. 智能人机交互接管（Takeover）
- 自动检测登录页面
- 暂停探索请求用户输入凭据
- 自动填充凭据继续探索
- **调研关键词**: `automated login handling`, `AI crawler authentication`, `human-in-the-loop web testing`

### 6. SPA路由追踪
- History API监听（pushState/replaceState）
- Hash路由变化检测
- 前端路由与后端API关联
- **调研关键词**: `SPA route tracking`, `single page application crawling`, `client-side routing discovery`

---

## 技术组合独特性

本发明的核心创新在于**组合**以下技术形成完整解决方案：

```
VLM/LLM决策引擎
    ↓
元素标注索引系统 → 精确交互
    ↓
被动代理 → API捕获
    ↓
覆盖率引擎 → 完成判断
    ↓
Takeover机制 → 登录处理
```

---

## 主要检索数据库建议

1. **中国专利数据库（CNIPA）**
   - 关键词：大语言模型+网页爬虫、AI自动化测试、API接口发现

2. **USPTO（美国专利商标局）**
   - 关键词：LLM web crawler, automated API discovery, AI browser automation

3. **EPO（欧洲专利局）**
   - 关键词：machine learning web testing, automated endpoint discovery

4. **Google Patents**
   - 综合检索

5. **学术论文数据库**
   - IEEE Xplore、ACM Digital Library
   - 关键词：LLM-based web crawling, AI-driven security testing

---

## 可能相关的现有技术方向

1. **传统网页爬虫**：Scrapy、Crawl4AI等（区别：无AI决策能力）

2. **浏览器自动化**：Selenium、Playwright、Puppeteer（区别：需要预编写脚本）

3. **安全扫描工具**：Burp Suite、OWASP ZAP（区别：被动/主动扫描，非AI驱动探索）

4. **AI辅助测试**：可能有部分重叠，需重点调研

5. **Web应用安全测试**：DAST工具（区别：通常基于规则而非AI）

---

## 建议调研的公司/产品专利

- OpenAI（GPT相关web应用）
- Anthropic（Claude相关应用）
- Google（Bard/Gemini + Chrome自动化）
- Microsoft（Playwright + Copilot）
- PortSwigger（Burp Suite）
- Checkmarx、Veracode等安全厂商

---

## 差异化亮点（答辩准备）

1. **双模态支持**：同时支持VLM视觉模式和LLM文本模式，适应不同场景
2. **端到端集成**：从探索到API发现到覆盖率追踪的完整闭环
3. **智能完成判断**：基于多维度覆盖率和稳定性的完成条件
4. **无脚本自动化**：完全由AI决策，无需预编写测试脚本
5. **人机协作设计**：Takeover机制支持验证码等需人工介入场景

---

*调研摘要生成日期：2025年12月16日*

