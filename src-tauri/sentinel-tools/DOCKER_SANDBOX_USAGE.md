# Docker Sandbox 使用指南

## 概述

Docker 沙箱为 Sentinel AI 的 shell 工具提供了安全的隔离执行环境，有效降低了在宿主机上执行命令的安全风险。

## 主要特性

### 1. 安全隔离
- ✅ 资源限制（CPU、内存）
- ✅ 网络隔离选项
- ✅ 只读文件系统支持
- ✅ 非 root 用户执行

### 2. 性能优化
- ✅ 容器池复用（避免频繁创建）
- ✅ 自动清理闲置容器
- ✅ 可配置的池大小和复用次数

### 3. 预装工具
- ✅ 网络扫描工具（nmap, netcat, tcpdump）
- ✅ 安全测试工具（sqlmap, nikto, dirb, gobuster）
- ✅ 开发工具（Python, Node.js, gcc）
- ✅ 常用实用工具（curl, wget, jq, git）

## 快速开始

### 步骤 1: 安装 Docker

```bash
# macOS (使用 Docker Desktop)
brew install --cask docker

# Linux (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install docker.io
sudo systemctl start docker
sudo usermod -aG docker $USER  # 添加当前用户到 docker 组

# 验证安装
docker --version
docker run hello-world
```

### 步骤 2: 构建沙箱镜像

```bash
# 在项目根目录执行
cd /Users/a1024/code/ai/sentinel-ai

# Linux/macOS
./scripts/build-docker-sandbox.sh

# Windows PowerShell
.\scripts\build-docker-sandbox.ps1
```

### 步骤 3: 在应用中初始化

在 Tauri 应用启动时调用初始化命令：

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// 检查 Docker 是否可用
const dockerAvailable = await invoke('check_docker_availability');
if (!dockerAvailable) {
  console.warn('Docker is not available, shell tool will use host mode');
  return;
}

// 初始化 Docker 沙箱（会自动构建镜像如果不存在）
try {
  await invoke('initialize_docker_sandbox');
  console.log('Docker sandbox initialized successfully');
} catch (error) {
  console.error('Failed to initialize Docker sandbox:', error);
}
```

## 配置说明

### 获取和更新配置

```typescript
// 获取当前配置
const { config, docker_available } = await invoke('get_shell_configuration');
console.log('Current config:', config);
console.log('Docker available:', docker_available);

// 更新配置
await invoke('update_shell_configuration', {
  config: {
    // 默认策略：RequestReview（需要用户确认）或 AlwaysProceed（自动执行）
    default_policy: 'RequestReview',
    
    // 默认执行模式：Docker（推荐）或 Host
    default_execution_mode: 'Docker',
    
    // 自动允许的命令（前缀匹配）
    allowed_commands: [
      'ls',
      'cat',
      'grep',
      'echo',
      'pwd',
      'whoami'
    ],
    
    // 始终拒绝的命令（前缀匹配，优先级最高）
    denied_commands: [
      'rm -rf',
      'dd',
      'mkfs',
      'format'
    ],
    
    // Docker 配置
    docker_config: {
      image: 'sentinel-sandbox:latest',
      memory_limit: '512m',      // 内存限制
      cpu_limit: '1.0',          // CPU 限制（1.0 = 1 核）
      network_mode: 'bridge',    // 网络模式：none/bridge/host
      read_only_rootfs: false,   // 是否只读根文件系统
      volumes: {},               // 卷挂载（宿主路径 -> 容器路径）
      env_vars: {}               // 环境变量
    }
  }
});
```

### 配置选项详解

#### 执行策略 (default_policy)

- **RequestReview**: 默认需要用户确认（推荐）
- **AlwaysProceed**: 自动执行（除非在拒绝列表中）

#### 执行模式 (default_execution_mode)

- **Docker**: 在 Docker 容器中执行（推荐，更安全）
- **Host**: 在宿主机上执行（性能更好，但安全性较低）

#### 网络模式 (network_mode)

- **none**: 无网络访问（最安全）
- **bridge**: 隔离的桥接网络（默认）
- **host**: 使用宿主机网络（最不安全）

## 使用示例

### 示例 1: 基本命令执行（Docker 模式）

```typescript
// 执行网络扫描
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'nmap -sV -p 80,443 example.com',
    timeout_secs: 60
  }
});

console.log('Scan result:', result);
```

### 示例 2: 强制使用宿主机模式

```typescript
// 需要访问本地文件系统时
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'ls -la /Users/username/Documents',
    execution_mode: 'Host',
    cwd: '/Users/username',
    timeout_secs: 30
  }
});
```

### 示例 3: SQL 注入测试

```typescript
// 使用预装的 sqlmap
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'sqlmap -u "http://target.com/page?id=1" --batch --level=1',
    timeout_secs: 300  // 5 分钟超时
  }
});
```

### 示例 4: 目录扫描

```typescript
// 使用 gobuster 进行目录扫描
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'gobuster dir -u http://target.com -w /usr/share/wordlists/common.txt',
    timeout_secs: 180
  }
});
```

## 容器管理

### 手动清理容器

```typescript
// 清理所有容器（释放资源）
await invoke('cleanup_docker_containers');
```

### 重新构建镜像

```typescript
// 当需要更新工具或修改 Dockerfile 后
await invoke('build_docker_sandbox_image');
```

### 查看容器状态

```bash
# 在终端中查看运行中的容器
docker ps | grep sentinel-sandbox

# 查看所有容器（包括已停止的）
docker ps -a | grep sentinel-sandbox

# 查看镜像
docker images | grep sentinel-sandbox
```

## 性能对比

| 操作 | Docker 模式 | Host 模式 |
|------|------------|----------|
| 首次执行 | ~2-3秒 | ~100毫秒 |
| 后续执行（容器池） | ~200-300毫秒 | ~100毫秒 |
| 安全性 | ⭐⭐⭐⭐⭐ | ⭐⭐ |
| 资源隔离 | ✅ | ❌ |
| 网络隔离 | ✅ | ❌ |

## 故障排查

### 问题 1: Docker 不可用

```bash
# 检查 Docker 服务状态
docker ps

# macOS: 启动 Docker Desktop
open -a Docker

# Linux: 启动 Docker 服务
sudo systemctl start docker
sudo systemctl enable docker
```

### 问题 2: 权限被拒绝

```bash
# Linux: 将用户添加到 docker 组
sudo usermod -aG docker $USER

# 然后重新登录或执行
newgrp docker
```

### 问题 3: 镜像构建失败

```bash
# 清理 Docker 缓存
docker system prune -a

# 重新构建（无缓存）
cd /Users/a1024/code/ai/sentinel-ai/src-tauri/sentinel-tools
docker build --no-cache -t sentinel-sandbox:latest -f Dockerfile.sandbox .
```

### 问题 4: 容器无法清理

```bash
# 手动强制删除所有相关容器
docker ps -a | grep sentinel-sandbox | awk '{print $1}' | xargs docker rm -f

# 或使用应用命令
await invoke('cleanup_docker_containers');
```

## 安全建议

1. **默认使用 Docker 模式**: 除非必要，否则不要使用 Host 模式
2. **限制网络访问**: 对于不需要网络的命令，使用 `network_mode: 'none'`
3. **设置资源限制**: 防止恶意命令耗尽系统资源
4. **启用权限审查**: 使用 `RequestReview` 策略
5. **定期更新镜像**: 保持工具和依赖的最新版本
6. **监控容器使用**: 定期检查是否有异常容器

## 自定义工具

如需添加自定义工具，修改 `Dockerfile.sandbox`:

```dockerfile
# 添加系统工具
RUN apt-get update && apt-get install -y your-tool

# 添加 Python 包
RUN pip3 install your-python-package

# 添加 Node.js 包
RUN npm install -g your-node-package
```

然后重新构建镜像：

```bash
./scripts/build-docker-sandbox.sh
```

## 最佳实践

1. **命令超时设置**: 根据命令复杂度设置合理的超时时间
2. **批量操作**: 对于多个命令，考虑在一个 shell 脚本中执行
3. **日志记录**: 保存重要命令的执行结果
4. **错误处理**: 妥善处理命令执行失败的情况
5. **资源监控**: 定期检查容器资源使用情况

## 相关文档

- [Sentinel Tools README](./README.md)
- [Shell Tool API 文档](../../docs/shell-tool-api.md)
- [Docker 官方文档](https://docs.docker.com/)
