# 🚀 快速测试指南

## ✅ 应用已启动

前端地址: http://localhost:1420

## 📋 快速测试步骤

### 测试 1: 使用测试靶场（推荐）

1. **打开AI助手界面**
   - 访问 http://localhost:1420
   - 找到AI对话界面

2. **输入测试命令**（复制粘贴即可）：

```
测试 http://testphp.vulnweb.com 是否存在SQL注入和XSS漏洞
```

3. **观察执行过程**：
   - ✅ 被动扫描代理启动（端口4201）
   - ✅ 浏览器自动打开
   - ✅ 生成2个插件
   - ✅ 执行自动化测试
   - ✅ 显示发现的漏洞

4. **预期结果**：
   - 应该检测到 SQL注入漏洞（Critical）
   - 应该检测到 XSS漏洞（High）
   - 完整测试报告

---

### 测试 2: 简单功能测试

#### 步骤 2.1: 检查工具可用性

在AI助手中输入：
```
列出所有可用的被动扫描工具
```

预期应该看到：
- start_passive_scan
- stop_passive_scan
- get_passive_scan_status
- generate_plugin
- list_findings
- 等等

#### 步骤 2.2: 启动代理

```
启动被动扫描代理
```

预期输出：
```
✅ 被动扫描代理已启动，监听端口: 4201
```

#### 步骤 2.3: 生成插件

```
为 http://testphp.vulnweb.com 生成一个SQL注入检测插件
```

预期输出：
```
✅ 插件已生成: auto_gen_sqli_testphp_vulnweb_com_20251112_XXXXXX
✅ 插件已自动启用
```

#### 步骤 2.4: 查看插件

```
列出所有被动扫描插件
```

应该能看到刚才生成的插件。

---

### 测试 3: Playwright集成测试

#### 步骤 3.1: 手动测试浏览器

```
打开浏览器访问 http://testphp.vulnweb.com
```

#### 步骤 3.2: 网站分析

```
分析 http://testphp.vulnweb.com 的网站结构
```

应该显示：
- 表单数量
- 输入框数量
- 链接数量
- 等等

---

## 🔍 调试技巧

### 查看日志

如果遇到问题，查看日志：

```bash
# 应用主日志
tail -f /tmp/tauri-dev.log

# 被动扫描日志
tail -f logs/sentinel-ai.log
```

### 检查端口

```bash
# 检查前端端口
lsof -i :1420

# 检查代理端口
lsof -i :4201
```

### 重启应用

如果需要重启：

```bash
# 停止应用
pkill -f "sentinel-ai"

# 重新启动
cd /Users/a1024/code/ai/sentinel-ai
npm run tauri dev
```

---

## 📊 测试检查清单

- [ ] 应用成功启动
- [ ] AI助手界面可访问
- [ ] 被动扫描代理可启动
- [ ] 插件生成功能正常
- [ ] Playwright浏览器可打开
- [ ] 网站分析功能正常
- [ ] 漏洞检测功能正常
- [ ] 报告生成正常

---

## 🎯 测试成功标准

测试通过，当：

1. ✅ 完整工作流无报错
2. ✅ 检测到已知漏洞（testphp.vulnweb.com）
3. ✅ 生成的报告包含详细信息
4. ✅ 浏览器自动化正常工作

---

## ❓ 常见问题

**Q: 浏览器没有打开？**
A: Playwright MCP可能需要配置。检查MCP服务器状态。

**Q: 没有检测到漏洞？**
A: 
1. 确认插件已启用：`列出所有插件`
2. 确认代理正在运行：`查询代理状态`
3. 检查浏览器是否通过代理访问

**Q: 插件生成失败？**
A: 检查模板文件是否存在：
```bash
ls -la src-tauri/sentinel-plugins/templates/
```

---

## 📞 需要帮助？

如有问题，请查看：
1. 完整测试指南: `TESTING_GUIDE.md`
2. 应用日志: `/tmp/tauri-dev.log`
3. 工作流文档: `src-tauri/src/prompts/automated_security_testing.md`

祝测试顺利！🎉

