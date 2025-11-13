# 自动化安全测试工作流 - 测试指南

## 🎯 测试目标

验证AI助手能够自动完成完整的安全测试流程，从启动代理到生成报告。

## 📝 测试前准备

### 1. 启动应用
```bash
cd /Users/a1024/code/ai/sentinel-ai
npm run tauri dev
```

### 2. 确认服务状态
- ✅ 应用启动成功
- ✅ AI助手界面可用
- ✅ 工具系统已初始化

## 🧪 测试场景

### 场景 1: 快速测试（使用公开测试靶场）

**测试站点**: http://testphp.vulnweb.com/

这是一个公开的漏洞测试环境，包含已知的SQL注入和XSS漏洞。

**测试命令**:
```
测试 http://testphp.vulnweb.com 是否存在SQL注入和XSS漏洞
```

**预期结果**:
1. ✅ 被动扫描代理启动（端口4201）
2. ✅ 浏览器自动打开并访问目标站点
3. ✅ 生成2个插件（sqli_detector, xss_detector）
4. ✅ 执行自动化测试（搜索框、表单等）
5. ✅ 检测到SQL注入漏洞（Critical）
6. ✅ 检测到XSS漏洞（High）
7. ✅ 生成详细报告
8. ✅ 浏览器关闭

**预计时长**: 2-3分钟

---

### 场景 2: 完整测试（自定义目标）

**测试站点**: https://zeus.imgo.tv （示例）

**测试命令**:
```
打开被动扫描，测试 https://zeus.imgo.tv 的以下漏洞：
1. SQL注入
2. XSS跨站脚本
3. 越权访问
4. 信息泄露
```

**预期行为**:
1. 启动代理
2. 打开浏览器
3. 分析网站结构
4. 生成4个插件
5. 执行多场景测试
6. 收集结果
7. 生成报告

---

### 场景 3: 单一漏洞类型测试

**测试命令**:
```
仅测试 http://testphp.vulnweb.com 的SQL注入漏洞
```

**预期结果**:
- 只生成1个 sqli 插件
- 只针对SQL注入进行测试

---

## 📊 验证检查清单

### 工具可用性检查
- [ ] `start_passive_scan` - 启动代理
- [ ] `get_passive_scan_status` - 查询状态
- [ ] `generate_plugin` - 生成插件
- [ ] `list_plugins` - 列出插件
- [ ] `enable_plugin` - 启用插件
- [ ] `list_findings` - 列出漏洞
- [ ] Playwright工具（navigate, click, fill等）

### 功能验证
- [ ] 代理成功启动并监听端口
- [ ] 浏览器能够通过代理访问网站
- [ ] 插件根据网站特征自动生成
- [ ] 插件自动启用并加载
- [ ] 自动化交互正常工作
- [ ] 被动扫描检测到漏洞
- [ ] 报告包含详细信息

### 数据完整性
- [ ] 插件保存到数据库
- [ ] 漏洞记录保存到数据库
- [ ] 证据链完整
- [ ] CWE/OWASP映射正确

---

## 🐛 常见问题排查

### 问题1: 代理启动失败
**现象**: "Failed to start proxy"

**排查**:
```bash
# 检查端口占用
lsof -i :4201
# 如果被占用，终止进程或换端口
```

### 问题2: 浏览器无法访问网站
**现象**: Playwright导航超时

**排查**:
- 检查目标网站是否可访问
- 检查代理配置是否正确
- 查看浏览器控制台错误

### 问题3: 插件未检测到漏洞
**现象**: 测试完成但无漏洞报告

**排查**:
```
# 检查插件是否启用
list_plugins()

# 检查代理是否接收到流量
get_passive_scan_status()

# 手动查看数据库
list_findings({ limit: 10 })
```

### 问题4: 生成插件失败
**现象**: "Template file not found"

**排查**:
```bash
# 检查模板文件是否存在
ls -la /Users/a1024/code/ai/sentinel-ai/src-tauri/sentinel-plugins/templates/

# 应该看到:
# sqli_template.ts
# xss_template.ts
# auth_bypass_template.ts
# info_leak_template.ts
# csrf_template.ts
```

---

## 📈 性能基准

### 预期性能指标
- 代理启动: < 1秒
- 浏览器启动: < 3秒
- 插件生成: < 2秒/个
- 单次测试场景: 2-5秒
- 完整测试流程: 2-5分钟

### 资源使用
- 内存: < 500MB
- CPU: 中等使用率
- 网络: 取决于目标站点

---

## 🎓 测试技巧

### 1. 逐步测试
不要一次性测试所有功能，按步骤来：
```
# 步骤1: 测试代理
get_passive_scan_status()

# 步骤2: 测试浏览器
playwright_navigate({ url: "http://testphp.vulnweb.com" })

# 步骤3: 测试插件生成
generate_plugin({ 
  template_type: "sqli", 
  target_url: "http://testphp.vulnweb.com",
  target_params: ["searchFor"]
})

# 步骤4: 测试完整流程
测试 http://testphp.vulnweb.com 的SQL注入
```

### 2. 查看详细日志
```bash
# 查看应用日志
tail -f logs/sentinel-ai.log

# 查看插件引擎日志
# 在前端控制台查看
```

### 3. 使用测试靶场
推荐的公开测试环境：
- http://testphp.vulnweb.com - PHP漏洞
- http://testaspnet.vulnweb.com - ASP.NET漏洞
- http://testhtml5.vulnweb.com - HTML5漏洞
- http://testasp.vulnweb.com - Classic ASP漏洞

**警告**: 仅在授权的测试环境中使用！

---

## ✅ 测试成功标准

测试被认为成功，当：

1. **自动化完整性**
   - 所有8个步骤自动执行
   - 无需手动干预
   - 无严重错误

2. **功能准确性**
   - 正确检测已知漏洞
   - 误报率低（<10%）
   - 漏报率低（核心漏洞必须检测到）

3. **报告质量**
   - 包含详细证据
   - 提供修复建议
   - CWE/OWASP映射正确

4. **性能可接受**
   - 完整流程 < 5分钟
   - 无资源泄漏
   - 响应流畅

---

## 📝 测试记录模板

```markdown
## 测试记录

**日期**: 2025-11-XX
**测试人**: XXX
**版本**: v0.1.0

### 测试场景
- 目标: http://testphp.vulnweb.com
- 类型: SQL注入 + XSS

### 测试结果
- 代理启动: ✅ 成功（端口4201）
- 浏览器启动: ✅ 成功
- 插件生成: ✅ 2个插件
- 漏洞检测: ✅ 发现3个漏洞
  - SQL注入 (Critical): 1个
  - XSS (High): 2个
- 测试时长: 2分30秒

### 问题记录
- 无

### 改进建议
- 可以增加更多测试场景
- 报告格式可以更友好
```

---

## 🚀 下一步

测试完成后：
1. 记录测试结果
2. 提交issue（如有bug）
3. 优化工作流
4. 增加更多模板

祝测试顺利！🎉

