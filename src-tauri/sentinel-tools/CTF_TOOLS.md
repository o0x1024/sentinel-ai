# CTF Tools in Docker Sandbox

本文档列出了为 CTF 竞赛添加到 Docker Sandbox 中的工具和库。

## 工具分类

### 1. 二进制分析与逆向工程 (Binary Analysis & Reverse Engineering)

#### 标准版和完整版
- **gdb** - GNU 调试器
- **radare2** - 逆向工程框架
- **ltrace** - 库调用跟踪
- **strace** - 系统调用跟踪
- **objdump** - 目标文件分析
- **strings** - 提取文件中的字符串
- **file** - 识别文件类型
- **xxd** - 十六进制查看器
- **hexedit** - 十六进制编辑器

#### 完整版额外工具
- **gdb-multiarch** - 多架构 GDB
- **gdbserver** - GDB 远程调试服务器
- **valgrind** - 内存调试和性能分析
- **qemu-user** - QEMU 用户模式模拟器
- **qemu-user-static** - 静态编译的 QEMU

#### Python 库
- **pwntools** - CTF pwn 工具集
- **angr** - 二进制分析框架
- **capstone** - 反汇编引擎
- **keystone-engine** - 汇编引擎
- **unicorn** - CPU 模拟器
- **ropgadget** - ROP gadget 搜索工具
- **ropper** - ROP gadget 搜索和链构建
- **r2pipe** (完整版) - radare2 Python 接口

### 2. 密码学 (Cryptography)

#### 系统工具
- **openssl** - 加密工具库
- **hashcat** - 密码破解工具
- **john** - John the Ripper 密码破解

#### Python 库
- **pycryptodome** - 密码学库
- **gmpy2** - 多精度算术库 (RSA 攻击)
- **z3-solver** - 约束求解器
- **sympy** - 符号数学库
- **sage** - 数学软件系统 (标准版尝试安装)

#### Node.js 库
- **jwt-cli** - JWT 工具
- **crypto-js** - JavaScript 加密库
- **node-forge** - TLS 和加密工具
- **jwt-cracker** (完整版) - JWT 破解工具

### 3. 隐写术 (Steganography)

#### 标准版和完整版
- **steghide** - 隐写工具
- **stegseek** - Steghide 暴力破解
- **zsteg** - PNG/BMP 隐写检测
- **binwalk** - 固件分析和提取
- **foremost** - 文件恢复工具
- **exiftool** - 元数据查看和编辑

#### 最小版
- **steghide** - 基本隐写工具
- **binwalk** - 基本文件分析
- **exiftool** - 元数据查看

#### Node.js 工具
- **stegcloak** (完整版) - 文本隐写工具

### 4. 网络分析 (Network Analysis)

#### 标准版和完整版
- **wireshark-common/wireshark** - 网络协议分析器
- **tshark** - Wireshark 命令行版本
- **tcpdump** - 网络流量捕获
- **netcat-traditional** - 网络工具
- **socat** - 高级 netcat
- **nmap** - 网络扫描器

#### 完整版额外工具
- **aircrack-ng** - 无线网络安全工具

### 5. Web 安全 (Web Security)

#### 扫描和模糊测试工具
- **gobuster** - 目录/DNS 暴力破解
- **ffuf** - Web 模糊测试工具
- **wfuzz** - Web 应用模糊测试
- **hydra** - 登录暴力破解

#### Go 工具 (完整版)
- **nuclei** - 漏洞扫描器
- **subfinder** - 子域名发现
- **httpx** - HTTP 探测工具
- **katana** - Web 爬虫
- **naabu** - 端口扫描
- **waybackurls** - Wayback Machine URL 提取
- **gf** - Grep wrapper for pentesting
- **httprobe** - HTTP/HTTPS 探测
- **gau** - URL 收集工具

#### Python 库
- **requests** - HTTP 客户端库
- **beautifulsoup4** - HTML 解析
- **paramiko** - SSH 客户端
- **scapy** - 网络包构造和分析
- **impacket** (完整版) - 网络协议实现

### 6. 取证分析 (Forensics)

#### 完整版工具
- **volatility3** - 内存取证框架
- **autopsy** - 数字取证平台
- **sleuthkit** - 文件系统取证工具

#### Python 库
- **yara-python** (完整版) - 恶意软件识别

### 7. 动态分析 (Dynamic Analysis)

#### 完整版 Python 库
- **frida** - 动态插桩工具包
- **frida-tools** - Frida 命令行工具

### 8. PHP 开发和安全 (PHP Development & Security)

#### 标准版和最小版
- **php** - PHP 解释器
- **php-cli** - PHP 命令行接口
- **php-curl** - cURL 扩展
- **php-json** - JSON 扩展
- **php-mbstring** - 多字节字符串扩展

#### 标准版额外扩展
- **php-gd** - 图像处理
- **php-xml** - XML 处理
- **php-zip** - ZIP 压缩
- **php-mysql** - MySQL 数据库支持
- **php-pgsql** - PostgreSQL 数据库支持
- **php-sqlite3** - SQLite3 数据库支持
- **composer** - PHP 包管理器

#### 完整版额外扩展
- **php-bcmath** - 高精度数学计算
- **php-gmp** - GMP 大数运算
- **php-soap** - SOAP 协议支持
- **php-xdebug** - PHP 调试器

#### Composer 包

**标准版**：
- **phpunit/phpunit** - PHP 单元测试框架
- **squizlabs/php_codesniffer** - PHP 代码规范检查
- **vimeo/psalm** - PHP 静态分析工具

**完整版额外包**：
- **phpstan/phpstan** - PHP 静态分析
- **friendsofphp/php-cs-fixer** - PHP 代码格式化

### 9. 数学和数据处理 (Math & Data Processing)

#### Python 库
- **numpy** - 数值计算
- **scipy** - 科学计算 (标准版尝试安装)
- **sympy** - 符号数学
- **pillow** - 图像处理

### 10. Payload 和字典 (Payloads & Wordlists)

#### 完整版 GitHub 仓库
- **/opt/SecLists** - 综合安全测试字典
- **/opt/payloads/sqli** - SQL 注入 Payload
- **/opt/payloads/xss** - XSS Payload
- **/opt/ysoserial** - Java 反序列化利用

### 10. 学习资源 (Learning Resources)

#### GitHub 仓库 (标准版和完整版)
- **/opt/pwntools-tutorial** - Pwntools 教程
- **/opt/scapy** - Scapy 源码和示例

## 镜像版本对比

### Minimal (最小版) - 适合快速启动
- 基础二进制工具 (file, strings, xxd)
- 基础隐写工具 (steghide, binwalk, exiftool)
- Python: pwntools, pycryptodome
- 体积最小，启动最快

### Standard (标准版) - 推荐日常使用
- Kali Top 10 工具包
- 完整的二进制分析工具链
- 密码学工具和库
- 隐写术工具套件
- 网络分析工具
- Web 安全扫描器
- Python CTF 库（包含 angr, z3）
- Node.js 安全工具

### Kali Full (完整版) - 适合专业 CTF 竞赛
- 包含标准版所有工具
- Kali 多个工具包（web, exploitation, passwords, wireless 等）
- 多架构调试支持
- 内存取证工具
- 动态插桩工具 (Frida)
- 完整的 Go 安全工具链
- 大型 Payload 和字典库
- 体积最大，功能最全

## 构建镜像

```bash
# 标准版（推荐）
./scripts/build-docker-sandbox.sh kali

# 最小版（快速）
./scripts/build-docker-sandbox.sh minimal

# 完整版（专业 CTF）
./scripts/build-docker-sandbox.sh kali-full
```

## 使用示例

### PWN 题目
```bash
# 分析二进制文件
file challenge
strings challenge
radare2 challenge

# 使用 pwntools 编写 exploit
python3 exploit.py
```

### Crypto 题目
```bash
# 使用 Python 进行密码学分析
python3 -c "from Crypto.Util.number import *; print(inverse(e, phi))"

# RSA 攻击
python3 -c "import gmpy2; print(gmpy2.iroot(c, 3))"
```

### Steganography 题目
```bash
# 检查图片元数据
exiftool image.png

# 提取隐藏文件
binwalk -e image.png
steghide extract -sf image.jpg

# PNG 隐写检测
zsteg image.png
```

### Web 题目
```bash
# 目录扫描
gobuster dir -u http://target.com -w /opt/SecLists/Discovery/Web-Content/common.txt

# 漏洞扫描
nuclei -u http://target.com

# SQL 注入测试
sqlmap -u "http://target.com?id=1" --batch
```

### Forensics 题目
```bash
# 网络流量分析
tshark -r capture.pcap

# 内存取证
volatility3 -f memory.dmp windows.info
```

## 工具安装容错

所有工具安装都添加了容错处理：
- Python 包优先使用 `--break-system-packages`，失败后尝试基础安装
- Node.js 包使用 `|| true` 避免构建失败
- Go 工具使用 `|| true` 避免网络问题导致构建失败
- GitHub 克隆使用 `|| true` 避免网络问题

这确保即使某些工具安装失败，镜像仍能成功构建。

## 注意事项

1. **镜像大小**：
   - Minimal: ~2-3 GB
   - Standard: ~5-7 GB
   - Kali Full: ~10-15 GB

2. **构建时间**：
   - Minimal: ~10-15 分钟
   - Standard: ~20-30 分钟
   - Kali Full: ~40-60 分钟

3. **网络要求**：
   - 需要稳定的网络连接下载 Kali 包
   - Go 工具需要访问 GitHub
   - 建议使用国内镜像或代理加速

4. **资源限制**：
   - 默认内存限制：512MB
   - 可通过配置调整资源限制
