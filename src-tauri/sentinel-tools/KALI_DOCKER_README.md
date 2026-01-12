# Sentinel AI - Kali Linux Docker Sandbox

## 概述

Sentinel AI 现在使用 **Kali Linux** 作为 Docker 沙箱的基础镜像，提供专业的安全测试环境。

## 为什么选择 Kali Linux？

### ✅ 优势

1. **预装安全工具**: Kali Linux 是专为渗透测试和安全审计设计的发行版
2. **工具齐全**: 包含 600+ 安全测试工具
3. **持续更新**: Rolling release 模式，工具始终保持最新
4. **社区支持**: 庞大的安全社区和丰富的文档
5. **兼容性好**: 基于 Debian，软件包管理成熟稳定

### 🆚 对比 Ubuntu

| 特性 | Kali Linux | Ubuntu |
|------|-----------|--------|
| 预装安全工具 | ✅ 600+ | ❌ 需手动安装 |
| 镜像大小（最小化） | 370MB | 255MB |
| 工具可用性 | ✅ 开箱即用 | ⚠️ 部分需编译 |
| 更新频率 | Rolling | LTS/定期 |
| 适用场景 | 安全测试 | 通用开发 |

## 可用版本

### 1. Minimal（推荐日常使用）

**镜像大小**: ~370MB  
**构建时间**: 2-3 分钟  
**适用场景**: 日常安全测试、脚本执行

```bash
./scripts/build-docker-sandbox.sh minimal
```

**包含工具**:
- 基础工具: curl, wget, git, vim
- 网络工具: nmap, netcat, dnsutils
- 开发工具: python3, pip, jq
- 压缩工具: zip, unzip

### 2. Standard（默认推荐）

**镜像大小**: ~1.5GB  
**构建时间**: 5-10 分钟  
**适用场景**: 专业安全测试、漏洞扫描

```bash
./scripts/build-docker-sandbox.sh kali
# 或
./scripts/build-docker-sandbox.sh
```

**包含工具**:
- **Kali Top 10**: 最流行的 10 个工具
  - nmap, burpsuite, wireshark, metasploit, sqlmap 等
- 额外工具: gobuster, ffuf, wfuzz
- 开发环境: Python3, Node.js, Go
- Python 库: requests, beautifulsoup4, pwntools

### 3. Kali Full（完整版）

**镜像大小**: ~3-4GB  
**构建时间**: 15-30 分钟  
**适用场景**: 全面安全评估、CTF 竞赛

```bash
./scripts/build-docker-sandbox.sh kali-full
```

**包含工具包**:
- kali-tools-top10: 最流行工具
- kali-tools-web: Web 应用测试
- kali-tools-information-gathering: 信息收集
- kali-tools-vulnerability: 漏洞分析
- kali-tools-passwords: 密码攻击
- kali-tools-wireless: 无线网络攻击
- kali-tools-exploitation: 漏洞利用

**额外 Go 工具**:
- ffuf: Web fuzzer
- nuclei: 漏洞扫描器
- subfinder: 子域名发现
- httpx: HTTP 探测

## 快速开始

### 1. 构建镜像

```bash
# 推荐：最小化版本（快速）
./scripts/build-docker-sandbox.sh minimal

# 标准版本（平衡）
./scripts/build-docker-sandbox.sh kali

# 完整版本（专业）
./scripts/build-docker-sandbox.sh kali-full
```

### 2. 验证安装

```bash
# 查看镜像
docker images | grep sentinel-sandbox

# 测试运行
docker run --rm sentinel-sandbox:latest bash -c "cat /etc/os-release"

# 检查工具
docker run --rm sentinel-sandbox:latest nmap --version
```

### 3. 在应用中使用

镜像构建完成后，shell 工具会自动使用 Docker 模式执行命令：

```typescript
// 自动在 Kali Docker 容器中执行
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'nmap -sV localhost',
    timeout_secs: 60
  }
});
```

## 预装工具列表

### Minimal 版本

```bash
# 网络工具
nmap, netcat, dig, ping

# 基础工具
curl, wget, git, python3, jq

# 文本处理
grep, sed, awk
```

### Standard 版本（Kali Top 10）

```bash
# 网络扫描
nmap, masscan, zmap

# Web 测试
burpsuite, nikto, dirb, gobuster, ffuf, wfuzz

# 漏洞利用
metasploit-framework, sqlmap

# 密码攻击
hydra, john, hashcat

# 嗅探分析
wireshark, tcpdump

# 信息收集
theharvester, recon-ng, maltego
```

### Kali Full 版本

包含 Standard 版本的所有工具，另外还有：

```bash
# Web 应用
- wpscan (WordPress 扫描)
- joomscan (Joomla 扫描)
- commix (命令注入)
- xsser (XSS 测试)

# 无线攻击
- aircrack-ng
- reaver
- wifite

# 社会工程
- set (Social Engineering Toolkit)

# 漏洞分析
- openvas
- nikto
- skipfish

# 逆向工程
- radare2
- ghidra
- binwalk

# 取证分析
- autopsy
- volatility
```

## 使用示例

### 端口扫描

```typescript
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'nmap -sV -p 1-1000 target.com',
    timeout_secs: 300
  }
});
```

### 目录扫描

```typescript
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'gobuster dir -u http://target.com -w /usr/share/wordlists/dirb/common.txt',
    timeout_secs: 180
  }
});
```

### SQL 注入测试

```typescript
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'sqlmap -u "http://target.com/page?id=1" --batch --level=2',
    timeout_secs: 600
  }
});
```

### 子域名发现

```typescript
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'subfinder -d target.com',
    timeout_secs: 120
  }
});
```

### Web Fuzzing

```typescript
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'ffuf -u http://target.com/FUZZ -w /usr/share/wordlists/dirb/common.txt',
    timeout_secs: 180
  }
});
```

## 性能对比

| 版本 | 镜像大小 | 构建时间 | 工具数量 | 适用场景 |
|------|---------|---------|---------|---------|
| Minimal | 370MB | 2-3分钟 | 20+ | 日常使用 ⭐⭐⭐⭐⭐ |
| Standard | 1.5GB | 5-10分钟 | 100+ | 专业测试 ⭐⭐⭐⭐ |
| Kali Full | 3-4GB | 15-30分钟 | 600+ | 全面评估 ⭐⭐⭐ |

## 常见工具路径

```bash
# 字典文件
/usr/share/wordlists/

# 常用字典
/usr/share/wordlists/dirb/common.txt
/usr/share/wordlists/rockyou.txt.gz

# Metasploit
/usr/share/metasploit-framework/

# Nmap 脚本
/usr/share/nmap/scripts/
```

## 自定义工具

### 添加额外工具

编辑 `Dockerfile.sandbox` 并重新构建：

```dockerfile
# 添加自定义工具
RUN apt-get update && apt-get install -y \
    your-custom-tool \
    && rm -rf /var/lib/apt/lists/*
```

### 安装 Python 包

```dockerfile
RUN pip3 install --no-cache-dir --break-system-packages \
    your-python-package
```

### 安装 Go 工具

```dockerfile
RUN export GOPATH=/tmp/go && \
    go install github.com/user/tool@latest && \
    mv /tmp/go/bin/tool /usr/local/bin/ && \
    rm -rf /tmp/go
```

## 故障排查

### 构建失败

如果完整版本构建失败，尝试最小化版本：

```bash
./scripts/build-docker-sandbox.sh minimal
```

### 网络问题

Kali 镜像较大，下载可能需要时间。如果超时：

```bash
# 增加 Docker 超时
export DOCKER_CLIENT_TIMEOUT=300
export COMPOSE_HTTP_TIMEOUT=300

# 重新构建
./scripts/build-docker-sandbox.sh minimal
```

### 镜像源配置（中国大陆）

如果下载很慢，可以配置 Kali 镜像源：

```dockerfile
# 在 Dockerfile 开头添加
RUN echo "deb http://mirrors.aliyun.com/kali kali-rolling main non-free contrib" > /etc/apt/sources.list && \
    echo "deb-src http://mirrors.aliyun.com/kali kali-rolling main non-free contrib" >> /etc/apt/sources.list
```

## 安全建议

1. **默认使用 Docker 模式**: 已在代码中设置为默认
2. **定期更新镜像**: Kali Rolling 持续更新
   ```bash
   ./scripts/build-docker-sandbox.sh minimal --no-cache
   ```
3. **限制资源使用**: 在配置中设置内存和 CPU 限制
4. **监控容器**: 定期检查运行中的容器
   ```bash
   docker ps | grep sentinel-sandbox
   ```

## 更新镜像

```bash
# 拉取最新的 Kali 基础镜像
docker pull kalilinux/kali-rolling

# 重新构建（无缓存）
cd /Users/a1024/code/ai/sentinel-ai/src-tauri/sentinel-tools
docker build --no-cache -t sentinel-sandbox:latest -f Dockerfile.sandbox.minimal .
```

## 相关资源

- [Kali Linux 官方文档](https://www.kali.org/docs/)
- [Kali Tools 列表](https://www.kali.org/tools/)
- [Kali Docker Hub](https://hub.docker.com/r/kalilinux/kali-rolling)
- [Sentinel Tools README](./README.md)
- [故障排查指南](./DOCKER_TROUBLESHOOTING.md)

## 最佳实践

1. **日常使用 Minimal**: 快速、轻量、满足大部分需求
2. **专业测试用 Standard**: 包含主流工具，性能平衡
3. **全面评估用 Full**: CTF、红队、深度测试
4. **定期清理容器**: 避免资源浪费
   ```bash
   docker ps -a | grep sentinel-sandbox | awk '{print $1}' | xargs docker rm -f
   ```
5. **监控镜像大小**: 定期检查并清理不需要的镜像
   ```bash
   docker images | grep sentinel-sandbox
   docker system df
   ```
