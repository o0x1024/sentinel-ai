# AI工具调用使用指南

## 概述

Sentinel AI安全助手现在支持智能工具调用功能。当您与AI对话时，AI可以自动识别您的需求并调用相应的安全工具来执行实际的安全扫描和分析任务。

## 支持的工具

目前支持以下安全工具的自动调用：

### 1. Nmap - 网络扫描工具
**功能**: 端口扫描、服务发现、网络探测

**触发关键词**:
- "nmap"
- "扫描端口"、"端口扫描"
- "网络扫描"
- "本地端口"、"开放端口"

**示例对话**:
```
用户: 使用nmap扫描本地开放了哪些端口
AI: 我将使用nmap工具扫描本地开放的端口...
🔧 工具执行结果:
✅ nmap 执行成功:
{
  "scan_results": [
    {"port": 22, "state": "open", "service": "ssh"},
    {"port": 80, "state": "open", "service": "http"},
    {"port": 443, "state": "open", "service": "https"}
  ]
}
```

### 2. Subfinder - 子域名发现工具
**功能**: 子域名枚举和发现

**触发关键词**:
- "subfinder"
- "子域名"、"域名发现"

**示例对话**:
```
用户: 帮我发现example.com的子域名
AI: 我将使用subfinder工具发现example.com的子域名...
```

### 3. Nuclei - 漏洞扫描工具
**功能**: 基于模板的漏洞扫描

**触发关键词**:
- "nuclei"
- "漏洞扫描"、"安全扫描"

### 4. HTTPx - HTTP服务探测工具
**功能**: HTTP服务探测和指纹识别

**触发关键词**:
- "httpx"
- "HTTP探测"、"服务探测"

### 5. Port Scanner - 内置端口扫描器
**功能**: 简单的端口扫描

**触发关键词**:
- "端口扫描"（不包含nmap时）

## 使用方法

### 基本使用

1. **直接请求**: 直接告诉AI您想要做什么
   ```
   "扫描本地开放的端口"
   "检查example.com有哪些子域名"
   "对192.168.1.1进行漏洞扫描"
   ```

2. **指定工具**: 明确指定要使用的工具
   ```
   "使用nmap扫描本地端口"
   "用subfinder发现子域名"
   "执行nuclei漏洞扫描"
   ```

### 高级用法

1. **组合扫描**: 可以要求AI执行多个步骤的扫描
   ```
   "先扫描目标的端口，然后进行漏洞扫描"
   ```

2. **参数定制**: AI会根据上下文自动选择合适的参数
   - 本地扫描 vs 远程扫描
   - 快速扫描 vs 全面扫描
   - 特定端口范围

## 工作流程

1. **用户输入**: 用户描述安全扫描需求
2. **AI解析**: AI分析用户意图，识别需要调用的工具
3. **工具检查**: 系统检查所需工具是否已安装
4. **工具调用**: 
   - 如果工具已安装：执行真实的命令行工具
   - 如果工具未安装：提供安装建议或使用内置替代方案
5. **结果展示**: 将工具执行结果格式化显示给用户
6. **智能分析**: AI基于工具结果提供专业的安全分析建议

## 工具安装状态

系统会自动检查以下工具的安装状态：

### 外部工具（需要单独安装）
- **Nmap**: 网络扫描工具
  - 安装链接: https://nmap.org/download.html
  - 如未安装：使用内置端口扫描器
  
- **Subfinder**: 子域名发现工具
  - 安装命令: `go install -v github.com/projectdiscovery/subfinder/v2/cmd/subfinder@latest`
  - 如未安装：提供安装建议
  
- **Nuclei**: 漏洞扫描工具
  - 安装命令: `go install -v github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest`
  - 如未安装：提供安装建议
  
- **HTTPx**: HTTP探测工具
  - 安装命令: `go install -v github.com/projectdiscovery/httpx/cmd/httpx@latest`
  - 如未安装：使用内置HTTP检查

### 内置工具（无需安装）
- **Port Scanner**: 内置端口扫描器
- **Basic HTTP Check**: 基本HTTP安全检查

## 示例对话场景

### 场景1: 本地安全检查（Nmap已安装 - 智能参数）
```
用户: 我想全面检查我的电脑开放了哪些端口，有没有安全风险？

AI: 我来帮您全面检查本地开放的端口情况。由于您要求全面检查，我将使用nmap进行深度扫描，包括服务版本检测和操作系统识别...

🔧 工具执行结果:
✅ nmap 执行成功 (使用参数: -v -T4 -p- -sS -sV -O):
{
  "tool": "nmap",
  "scan_results": {
    "host_status": "up",
    "open_ports": [
      {"port": 22, "service": "ssh", "state": "open"},
      {"port": 80, "service": "http", "state": "open"},
      {"port": 443, "service": "https", "state": "open"},
      {"port": 3306, "service": "mysql", "state": "open"}
    ],
    "total_open_ports": 4
  }
}

根据nmap扫描结果，您的系统开放了以下端口：
- 端口22 (SSH): 远程访问端口，确保使用强密码和密钥认证
- 端口80 (HTTP): Web服务端口，建议重定向到HTTPS
- 端口443 (HTTPS): 安全Web服务端口，正常
- 端口3306 (MySQL): 数据库端口，存在安全风险

⚠️ 安全建议：
1. MySQL端口不应对外开放，建议配置防火墙限制访问
2. 确保SSH使用密钥认证而非密码认证
3. 检查HTTP服务是否有必要，考虑只保留HTTPS
4. 定期更新系统和服务的安全补丁
```

### 场景1.1: 本地安全检查（Nmap未安装）
```
用户: 我想检查我的电脑开放了哪些端口，有没有安全风险？

AI: 我来帮您检查本地开放的端口情况。检测到nmap工具未安装，将使用内置端口扫描器...

🔧 工具执行结果:
✅ port_scanner 执行成功:
{
  "tool": "port_scanner",
  "scan_results": {
    "target": "127.0.0.1",
    "scanned_range": "1-1000",
    "open_ports": [
      {"port": 22, "service": "ssh", "state": "open"},
      {"port": 80, "service": "http", "state": "open"},
      {"port": 3306, "service": "mysql", "state": "open"}
    ],
    "total_open_ports": 3,
    "scan_time_ms": 5000
  }
}

使用内置扫描器发现了3个开放端口。为了获得更详细的扫描结果，建议安装nmap工具：

📥 安装nmap:
- Windows: 访问 https://nmap.org/download.html 下载安装包
- macOS: `brew install nmap`
- Linux: `sudo apt-get install nmap` 或 `sudo yum install nmap`

当前发现的端口分析：
- 端口22 (SSH): 远程访问端口
- 端口80 (HTTP): Web服务端口  
- 端口3306 (MySQL): 数据库端口，建议限制访问
```

### 场景1.2: 根据需求智能选择扫描参数
```
用户: 帮我快速扫描一下本地主机的常见端口

AI: 我将为您执行一次轻量级的端口扫描，只检查常见端口以提高速度...

🔧 工具执行结果:
✅ nmap 执行成功 (使用参数: -v -T3 -F -sS):
{
  "tool": "nmap",
  "scan_results": {
    "host_status": "up",
    "open_ports": [
      {"port": 80, "service": "http", "state": "open"},
      {"port": 443, "service": "https", "state": "open"}
    ],
    "total_open_ports": 2
  }
}

用户: 现在帮我做一次更深入的扫描，包括服务版本检测

AI: 好的，我将执行一次更全面的扫描，包括服务版本检测。这可能需要几分钟时间...

🔧 工具执行结果:
✅ nmap 执行成功 (使用参数: -v -T4 -p 1-10000 -sS -sV --script=default):
{
  "tool": "nmap",
  "scan_results": {
    "host_status": "up",
    "open_ports": [
      {"port": 22, "service": "ssh", "state": "open", "version": "OpenSSH 8.2p1"},
      {"port": 80, "service": "http", "state": "open", "version": "nginx 1.18.0"},
      {"port": 443, "service": "https", "state": "open", "version": "nginx 1.18.0"},
      {"port": 3306, "service": "mysql", "state": "open", "version": "MySQL 8.0.27"}
    ],
    "total_open_ports": 4
  }
}

根据深度扫描结果，我发现以下安全建议：
1. SSH服务(端口22)使用的是OpenSSH 8.2p1，这是一个较新的版本，但建议定期更新
2. Web服务器使用的是nginx 1.18.0，应确保已应用最新的安全补丁
3. MySQL 8.0.27在公网暴露可能存在风险，建议限制访问或使用防火墙规则
```

### 场景2: 目标侦察
```