# 真实工具调用功能实现总结

## 概述

Sentinel AI现在已经实现了真实的安全工具调用功能，不再返回模拟结果。系统会检查工具安装状态，执行真实的命令行工具，并提供智能的回退方案。

## 实现的功能

### 1. 工具执行器 (`tool_executor.rs`)

**核心功能**:
- 自动检测工具安装状态
- 执行真实的命令行工具
- 解析工具输出结果
- 提供内置替代方案

**支持的工具**:
- **Nmap**: 网络扫描和端口发现
- **Subfinder**: 子域名发现
- **Nuclei**: 漏洞扫描
- **HTTPx**: HTTP服务探测
- **内置端口扫描器**: TCP连接测试
- **内置HTTP检查**: 基本HTTP请求分析

### 2. 智能工具选择

**工作流程**:
1. **工具检查**: 使用`which`/`where`命令检测工具是否安装
2. **优先执行**: 如果外部工具已安装，优先使用专业工具
3. **回退方案**: 如果工具未安装，使用内置替代或提供安装建议
4. **结果解析**: 解析工具输出为结构化JSON数据

### 3. 真实工具执行示例

#### Nmap端口扫描（AI智能参数选择）
```bash
# 基于用户需求的智能参数选择
# 例如，用户要求"全面扫描"时：
nmap -v -T4 -p- -sS -sV -O --script=vuln -oX - 127.0.0.1

# 或者用户要求"轻量扫描"时：
nmap -v -T3 -F -sS -oX - 127.0.0.1

# 返回的结构化结果
{
  "tool": "nmap",
  "scan_results": {
    "host_status": "up",
    "open_ports": [
      {"port": 22, "service": "ssh", "state": "open"},
      {"port": 80, "service": "http", "state": "open"}
    ],
    "total_open_ports": 2
  }
}
```

#### Subfinder子域名发现
```bash
# 实际执行的命令
subfinder -d example.com -silent

# 返回的结构化结果
{
  "tool": "subfinder",
  "results": {
    "domain": "example.com",
    "subdomains": ["www.example.com", "api.example.com"],
    "total_found": 2
  }
}
```

#### 内置端口扫描器
```rust
// 使用TCP连接测试端口
TcpStream::connect("127.0.0.1:80").await

// 返回结果
{
  "tool": "port_scanner",
  "scan_results": {
    "target": "127.0.0.1",
    "scanned_range": "1-1000",
    "open_ports": [
      {"port": 80, "service": "http", "state": "open"}
    ],
    "total_open_ports": 1
  }
}
```

## 安全特性

### 1. 参数安全
- 限制扫描范围（端口扫描最多100个端口）
- 超时控制（防止长时间阻塞）
- 输入验证（防止命令注入）

### 2. 权限控制
- 只扫描指定目标
- 不执行危险命令参数
- 记录所有工具执行日志

### 3. 错误处理
- 优雅处理工具执行失败
- 提供详细的错误信息
- 自动回退到替代方案

## 工具安装指南

### 必需工具（推荐安装）

**Nmap** - 网络扫描工具
```bash
# Windows
# 下载: https://nmap.org/download.html

# macOS
brew install nmap

# Linux
sudo apt-get install nmap
```

**Go环境** - ProjectDiscovery工具依赖
```bash
# 下载安装: https://golang.org/dl/
```

**ProjectDiscovery工具套件**
```bash
# Subfinder - 子域名发现
go install -v github.com/projectdiscovery/subfinder/v2/cmd/subfinder@latest

# Nuclei - 漏洞扫描
go install -v github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest

# HTTPx - HTTP探测
go install -v github.com/projectdiscovery/httpx/cmd/httpx@latest
```

### 内置工具（无需安装）

**端口扫描器**
- 使用Rust的tokio库进行TCP连接测试
- 支持超时控制和并发扫描
- 自动识别常见服务端口

**HTTP检查器**
- 使用reqwest库进行HTTP请求
- 获取状态码、标题、头部信息
- 支持HTTP和HTTPS协议

## 使用示例

### 1. 自然语言交互

**用户输入**:
```
帮我扫描本地开放的端口
```

**AI响应**:
```
我来帮您扫描本地开放的端口。检测到nmap工具已安装，将使用nmap进行详细扫描...

🔧 执行nmap扫描...
✅ 扫描完成！发现以下开放端口：

- 端口22 (SSH): 远程访问服务
- 端口80 (HTTP): Web服务器
- 端口3306 (MySQL): 数据库服务

⚠️ 安全建议：
1. MySQL端口不应对外开放，建议配置防火墙
2. 确保SSH使用密钥认证
3. 考虑将HTTP重定向到HTTPS
```

### 2. 工具未安装的处理

**用户输入**:
```
使用nuclei扫描网站漏洞
```

**AI响应**:
```
检测到nuclei工具未安装。

📥 安装nuclei:
go install -v github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest

安装完成后，您就可以使用nuclei进行专业的漏洞扫描了。

💡 临时方案：我可以使用内置的HTTP检查功能进行基本的安全检查，是否需要？
```

## 技术实现细节

### 1. 异步工具执行
```rust
// 使用tokio异步执行命令
let output = TokioCommand::new("nmap")
    .arg("-T4")
    .arg("-v")
    .arg(target)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .await?;
```

### 2. 结果解析
```rust
// XML解析nmap输出
fn parse_nmap_output(xml_output: &str) -> Result<Value> {
    // 解析XML，提取端口信息
    // 返回结构化JSON数据
}
```

### 3. 错误处理
```rust
// 优雅处理工具执行失败
match ToolExecutor::execute_nmap(target, &options).await {
    Ok(result) => Ok(vec![ToolContent { text: result.to_string() }]),
    Err(e) => Ok(vec![ToolContent {
        text: json!({
            "error": e.to_string(),
            "suggestion": "请检查nmap安装或使用内置扫描器"
        }).to_string()
    }])
}
```

## 性能优化

### 1. 并发执行
- 端口扫描使用并发连接测试
- HTTP检查支持批量请求
- 合理的超时设置避免阻塞

### 2. 资源限制
- 限制扫描范围防止资源耗尽
- 实现请求速率限制
- 自动清理临时文件

### 3. 缓存机制
- 工具安装状态缓存
- 常见端口服务名称缓存
- 减少重复的系统调用

## 未来扩展

### 1. 更多工具支持
- Masscan - 大规模端口扫描
- Gobuster - 目录爆破
- SQLMap - SQL注入测试
- Nikto - Web漏洞扫描

### 2. 高级功能
- 扫描任务队列
- 结果导出功能
- 自定义扫描模板
- 分布式扫描支持

### 3. 可视化增强
- 扫描进度显示
- 结果图表展示
- 交互式报告
- 实时日志查看

## 总结

Sentinel AI现在具备了真正的安全工具调用能力：

✅ **真实工具执行** - 不再是模拟结果，而是真实的工具输出
✅ **智能回退方案** - 工具未安装时提供替代方案
✅ **安全参数控制** - 防止危险操作和资源滥用
✅ **结构化结果** - 统一的JSON格式便于AI分析
✅ **用户友好** - 自然语言交互，自动安装建议
✅ **可扩展架构** - 易于添加新的安全工具支持

这使得Sentinel AI从一个"咨询型"AI助手升级为真正的"实操型"安全工具，能够执行实际的安全扫描任务并提供专业的分析建议。 