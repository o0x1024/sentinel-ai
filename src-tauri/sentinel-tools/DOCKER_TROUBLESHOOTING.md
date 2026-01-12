# Docker Sandbox 故障排查指南

## 构建问题

### 问题 1: apt-get 安装失败 (exit code 100)

**症状**:
```
ERROR: failed to solve: process "/bin/sh -c apt-get update && apt-get install -y ..." 
did not complete successfully: exit code: 100
```

**原因**:
1. 网络连接问题（无法访问 Ubuntu 软件源）
2. 某些软件包在当前 Ubuntu 版本中不可用
3. 软件包名称错误

**解决方案**:

#### 方案 A: 使用最小化版本（推荐）
```bash
# 使用轻量级版本，只包含核心工具
./scripts/build-docker-sandbox.sh minimal

# 或直接构建
cd src-tauri/sentinel-tools
docker build -t sentinel-sandbox:latest -f Dockerfile.sandbox.minimal .
```

最小化版本特点：
- ✅ 镜像大小：~255MB（vs 完整版 ~1GB+）
- ✅ 构建时间：~2-3 分钟（vs 完整版 ~10-15 分钟）
- ✅ 包含核心工具：curl, wget, nmap, python3, jq
- ✅ 网络问题风险低

#### 方案 B: 配置 Docker 代理（中国大陆用户）

如果在中国大陆，可能需要配置 Docker 使用镜像源：

**Linux**:
```bash
# 编辑 daemon.json
sudo mkdir -p /etc/docker
sudo tee /etc/docker/daemon.json <<-'EOF'
{
  "registry-mirrors": [
    "https://docker.mirrors.ustc.edu.cn",
    "https://hub-mirror.c.163.com"
  ]
}
EOF

# 重启 Docker
sudo systemctl daemon-reload
sudo systemctl restart docker
```

**macOS (Docker Desktop)**:
1. 打开 Docker Desktop
2. Settings → Docker Engine
3. 添加配置：
```json
{
  "registry-mirrors": [
    "https://docker.mirrors.ustc.edu.cn"
  ]
}
```
4. Apply & Restart

#### 方案 C: 分阶段构建

修改 Dockerfile，分阶段安装，失败时继续：

```dockerfile
# 安装基础工具（必须成功）
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl wget && \
    rm -rf /var/lib/apt/lists/*

# 安装可选工具（失败不中断）
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    nmap sqlmap nikto || true && \
    rm -rf /var/lib/apt/lists/*
```

#### 方案 D: 使用 --no-cache 重新构建

```bash
docker build --no-cache -t sentinel-sandbox:latest -f Dockerfile.sandbox.minimal .
```

### 问题 2: 网络超时

**症状**:
```
failed to fetch https://...
Could not resolve 'archive.ubuntu.com'
```

**解决方案**:

1. **检查网络连接**:
```bash
ping archive.ubuntu.com
```

2. **使用国内镜像源**:

创建 `Dockerfile.sandbox.cn`:
```dockerfile
FROM ubuntu:22.04

# 使用阿里云镜像源
RUN sed -i 's/archive.ubuntu.com/mirrors.aliyun.com/g' /etc/apt/sources.list && \
    sed -i 's/security.ubuntu.com/mirrors.aliyun.com/g' /etc/apt/sources.list

# 继续其他安装...
```

3. **增加构建超时**:
```bash
docker build --network=host -t sentinel-sandbox:latest -f Dockerfile.sandbox.minimal .
```

### 问题 3: 磁盘空间不足

**症状**:
```
no space left on device
```

**解决方案**:

1. **清理 Docker 缓存**:
```bash
# 清理未使用的镜像和容器
docker system prune -a

# 查看磁盘使用
docker system df
```

2. **删除旧镜像**:
```bash
docker images | grep sentinel-sandbox | awk '{print $3}' | xargs docker rmi -f
```

## 运行时问题

### 问题 4: 容器无法启动

**症状**:
```
Error response from daemon: failed to create shim task
```

**解决方案**:

1. **检查 Docker 服务状态**:
```bash
# macOS
open -a Docker

# Linux
sudo systemctl status docker
sudo systemctl start docker
```

2. **重启 Docker**:
```bash
# macOS: 重启 Docker Desktop

# Linux
sudo systemctl restart docker
```

### 问题 5: 权限被拒绝

**症状**:
```
permission denied while trying to connect to the Docker daemon socket
```

**解决方案**:

**Linux**:
```bash
# 添加用户到 docker 组
sudo usermod -aG docker $USER

# 重新登录或执行
newgrp docker

# 验证
docker ps
```

**macOS**:
- 确保 Docker Desktop 正在运行
- 检查 Docker Desktop 设置中的权限

### 问题 6: 容器执行命令超时

**症状**:
```
Command timeout after 30 seconds
```

**解决方案**:

1. **增加超时时间**:
```typescript
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'nmap -sV target.com',
    timeout_secs: 300  // 增加到 5 分钟
  }
});
```

2. **检查容器资源限制**:
```typescript
await invoke('update_shell_configuration', {
  config: {
    docker_config: {
      memory_limit: '1g',    // 增加内存
      cpu_limit: '2.0',      // 增加 CPU
    }
  }
});
```

### 问题 7: 容器池耗尽

**症状**:
```
Failed to create container: pool is full
```

**解决方案**:

1. **手动清理容器**:
```typescript
await invoke('cleanup_docker_containers');
```

2. **或使用命令行**:
```bash
docker ps -a | grep sentinel-sandbox | awk '{print $1}' | xargs docker rm -f
```

## 性能问题

### 问题 8: 首次执行很慢

**原因**: 需要创建新容器

**解决方案**: 这是正常的，后续执行会使用容器池，速度会快很多

**优化**:
```typescript
// 预热容器池
for (let i = 0; i < 3; i++) {
  await invoke('unified_execute_tool', {
    toolName: 'shell',
    args: { command: 'echo "warmup"', timeout_secs: 10 }
  });
}
```

### 问题 9: 镜像太大

**当前大小**:
- 最小化版本：~255MB ✅
- 完整版本：~1GB+

**减小镜像大小**:

1. **使用 Alpine Linux**:
```dockerfile
FROM alpine:3.18
RUN apk add --no-cache bash curl wget nmap python3
```

2. **多阶段构建**:
```dockerfile
FROM ubuntu:22.04 as builder
RUN apt-get update && apt-get install -y build-tools
RUN compile-something

FROM ubuntu:22.04
COPY --from=builder /output /usr/local/bin/
```

## 网络问题

### 问题 10: 容器无法访问网络

**症状**:
```
Could not resolve host
Network is unreachable
```

**解决方案**:

1. **检查网络模式**:
```typescript
await invoke('update_shell_configuration', {
  config: {
    docker_config: {
      network_mode: 'bridge'  // 或 'host'
    }
  }
});
```

2. **测试网络连接**:
```bash
docker run --rm sentinel-sandbox:latest ping -c 3 8.8.8.8
```

3. **使用 host 网络模式**（不推荐，安全性较低）:
```typescript
docker_config: {
  network_mode: 'host'
}
```

## 调试技巧

### 查看容器日志

```bash
# 查看运行中的容器
docker ps | grep sentinel-sandbox

# 查看容器日志
docker logs <container_id>

# 进入容器调试
docker exec -it <container_id> bash
```

### 测试镜像

```bash
# 交互式测试
docker run --rm -it sentinel-sandbox:latest bash

# 测试特定命令
docker run --rm sentinel-sandbox:latest nmap --version
docker run --rm sentinel-sandbox:latest python3 --version
docker run --rm sentinel-sandbox:latest curl --version
```

### 检查镜像层

```bash
# 查看镜像构建历史
docker history sentinel-sandbox:latest

# 查看镜像详细信息
docker inspect sentinel-sandbox:latest
```

## 常见错误代码

| 错误码 | 含义 | 解决方案 |
|--------|------|----------|
| 100 | apt-get 安装失败 | 使用最小化版本或配置镜像源 |
| 125 | Docker daemon 错误 | 重启 Docker 服务 |
| 126 | 命令无法执行 | 检查权限和路径 |
| 127 | 命令未找到 | 确认工具已安装在镜像中 |
| 137 | 容器被 OOM kill | 增加内存限制 |
| 139 | 段错误 | 检查命令参数 |

## 获取帮助

如果以上方案都无法解决问题：

1. **查看完整日志**:
```bash
docker build -t sentinel-sandbox:latest -f Dockerfile.sandbox.minimal . 2>&1 | tee build.log
```

2. **检查 Docker 版本**:
```bash
docker --version
docker info
```

3. **提供以下信息**:
   - 操作系统和版本
   - Docker 版本
   - 完整的错误日志
   - 使用的 Dockerfile 版本（minimal/full）
   - 网络环境（是否在中国大陆）

## 快速修复清单

遇到问题时，按顺序尝试：

- [ ] 使用最小化版本构建
- [ ] 清理 Docker 缓存 (`docker system prune -a`)
- [ ] 重启 Docker 服务
- [ ] 检查磁盘空间
- [ ] 配置 Docker 镜像源（中国大陆用户）
- [ ] 使用 `--no-cache` 重新构建
- [ ] 检查网络连接
- [ ] 查看 Docker daemon 日志
