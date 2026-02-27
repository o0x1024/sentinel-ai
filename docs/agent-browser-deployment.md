# Agent Browser 部署指南

## 概述

Agent Browser 功能依赖 Node.js 和 Playwright 浏览器。本文档说明如何在不同环境中部署。

## 架构说明

```
Sentinel AI (Rust/Tauri)
    ↓
agent-browser daemon (Node.js)
    ↓
Playwright (浏览器自动化)
    ↓
Chromium 浏览器
```

## 开发环境

### 前置要求
- Node.js >= 18
- npm 或 yarn

### 设置步骤

1. **编译 agent-browser TypeScript 代码**
```bash
cd src-tauri/agent-browser
npm install
npm run build
```

2. **安装 Playwright 浏览器**
```bash
cd src-tauri/agent-browser
npx playwright install chromium
```

3. **运行应用**
```bash
cd ../..
cargo run
```

## Release 版本部署

### 打包配置

应用会自动将 `agent-browser/dist` 目录打包到应用资源中。

**macOS**: `Sentinel AI.app/Contents/Resources/agent-browser/`
**Windows**: `Sentinel AI/agent-browser/`
**Linux**: `sentinel-ai/agent-browser/`

### 用户机器要求

#### 必需
- **Node.js** >= 18
  - macOS: `brew install node`
  - Windows: 从 https://nodejs.org/ 下载安装
  - Linux: `sudo apt install nodejs npm` 或 `sudo yum install nodejs`

#### 自动安装
- **Playwright 浏览器**: 应用首次使用浏览器功能时会自动安装
- 如果自动安装失败，用户需要手动运行：
  ```bash
  npx playwright install chromium
  ```

### 磁盘空间要求

- agent-browser 代码: ~5MB
- Playwright 浏览器 (Chromium): ~300MB
- 总计: ~305MB

### 网络要求

首次使用时需要网络连接以下载 Playwright 浏览器：
- 下载源: https://playwright.azureedge.net/
- 大小: ~300MB
- 如果网络受限，可以预先下载并手动安装

## 故障排查

### 1. "Node.js is required" 错误

**原因**: 系统未安装 Node.js

**解决方案**:
```bash
# macOS
brew install node

# Windows
# 从 https://nodejs.org/ 下载安装

# Linux (Ubuntu/Debian)
sudo apt update
sudo apt install nodejs npm

# Linux (CentOS/RHEL)
sudo yum install nodejs
```

### 2. "Browser not launched" 错误

**原因**: Playwright 浏览器未安装

**解决方案**:
```bash
npx playwright install chromium
```

### 3. Daemon 启动失败

**检查步骤**:

1. 检查 daemon 是否在运行
```bash
ps aux | grep daemon.js
```

2. 检查 daemon 文件是否存在
```bash
# macOS
ls -la "/Applications/Sentinel AI.app/Contents/Resources/agent-browser/dist/daemon.js"

# Linux
ls -la "/opt/sentinel-ai/agent-browser/dist/daemon.js"
```

3. 查看应用日志
```bash
# macOS
tail -f ~/Library/Application\ Support/sentinel-ai/logs/sentinel-ai.log

# Linux
tail -f ~/.local/share/sentinel-ai/logs/sentinel-ai.log
```

### 4. 网络代理问题

如果在企业网络环境中，可能需要配置代理：

```bash
# 设置 npm 代理
npm config set proxy http://proxy.company.com:8080
npm config set https-proxy http://proxy.company.com:8080

# 然后安装浏览器
npx playwright install chromium
```

## 离线部署

对于无法访问互联网的环境：

### 1. 准备离线包

在有网络的机器上：

```bash
# 安装 Playwright 浏览器
npx playwright install chromium

# 打包浏览器文件
cd ~/Library/Caches/ms-playwright  # macOS
# 或 ~/.cache/ms-playwright  # Linux
# 或 %USERPROFILE%\AppData\Local\ms-playwright  # Windows

tar -czf playwright-browsers.tar.gz chromium-*
```

### 2. 在目标机器上安装

```bash
# 解压到 Playwright 缓存目录
tar -xzf playwright-browsers.tar.gz -C ~/Library/Caches/ms-playwright/
```

## 性能优化

### 1. 使用 headless 模式

headless 模式消耗更少资源：
```json
{
  "headless": true
}
```

### 2. 限制并发浏览器实例

建议同时最多运行 2-3 个浏览器实例。

### 3. 定期清理缓存

Playwright 会缓存浏览器数据，定期清理可以释放空间：
```bash
rm -rf ~/Library/Caches/ms-playwright/  # macOS
```

## 安全考虑

1. **浏览器沙箱**: Playwright 在沙箱环境中运行浏览器
2. **网络隔离**: 可以配置浏览器使用代理
3. **数据隔离**: 每个会话使用独立的用户数据目录

## 更新和维护

### 更新 Playwright 浏览器

```bash
npx playwright install chromium --force
```

### 更新 agent-browser

应用更新时会自动包含最新的 agent-browser 代码。

## 技术支持

如遇问题，请提供以下信息：
1. 操作系统版本
2. Node.js 版本 (`node --version`)
3. 应用日志
4. 错误截图

---

**注意**: 本功能需要 Node.js 运行时。如果目标环境无法安装 Node.js，请考虑使用其他浏览器自动化方案。
