# Agent Browser Sidecar Integration

这个文档说明了如何使用新的Agent Browser Sidecar集成方案，替代了之前的本地编译方式。

## 方案概述

Agent Browser现在作为Tauri应用的**边车进程（Sidecar）**集成，而不需要本地编译Rust代码。二进制文件从GitHub releases自动下载。

## 快速开始

### 1. 首次设置

```bash
# 安装依赖和下载agent-browser二进制文件
npm run setup:agent-browser
```

这会自动：
- 检测你的平台（macOS/Linux/Windows）和架构（arm64/x64）
- 从GitHub releases下载预编译的agent-browser二进制文件
- 保存到 `src-tauri/bin/agent-browser`

### 2. 开发模式

```bash
# 启动Tauri开发模式
npm run dev
# 或
tauri dev
```

### 3. 生产构建

```bash
# 构建应用（自动下载agent-browser）
npm run build:release
# 或
tauri build --release
```

## 在Rust代码中使用Agent Browser

在 Tauri 命令中调用 agent-browser：

```rust
use tauri::AppHandle;
use crate::commands::agent_browser_commands;

#[tauri::command]
async fn my_command(app_handle: AppHandle) {
    // 运行agent-browser命令
    let result = agent_browser_commands::run_agent_browser_command(
        app_handle,
        vec![
            "open".to_string(),
            "https://example.com".to_string(),
        ]
    ).await;
    
    match result {
        Ok(output) => {
            println!("stdout: {}", output.stdout);
            println!("stderr: {}", output.stderr);
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

## 在前端JavaScript中使用

```javascript
import { invoke } from '@tauri-apps/api/core';

// 获取agent-browser版本
const version = await invoke('get_agent_browser_version');
console.log('Agent Browser version:', version);

// 运行agent-browser命令
const result = await invoke('run_agent_browser_command', {
    args: ['open', 'https://example.com'],
});

console.log('Success:', result.success);
console.log('Output:', result.stdout);
if (result.stderr) {
    console.error('Error:', result.stderr);
}
```

## 可用的Tauri命令

### 1. `run_agent_browser_command(args: Vec<String>)`

运行任意agent-browser命令。

**参数：**
- `args`: agent-browser的命令行参数

**返回值：**
```json
{
  "success": true,
  "stdout": "output here",
  "stderr": "",
  "exit_code": 0
}
```

**示例：**
```javascript
// 获取页面快照
await invoke('run_agent_browser_command', {
    args: ['snapshot', '-i'],
});

// 点击元素
await invoke('run_agent_browser_command', {
    args: ['click', '@e1'],
});

// 填充表单
await invoke('run_agent_browser_command', {
    args: ['fill', '@e2', 'test@example.com'],
});
```

### 2. `get_agent_browser_version()`

获取agent-browser的版本号。

**返回值：** 版本字符串（如 `"v0.21.2"`）

## 文件结构

```
src-tauri/
  bin/
    agent-browser          # 下载的可执行文件（不在git中）
  src/
    commands/
      agent_browser_commands.rs  # Agent Browser命令实现
  tauri.conf.json         # 更新了externalBin配置
```

## 打包与分发

### macOS

二进制文件会自动包含在应用bundle中：

```
Sentinel AI.app/
  Contents/
    MacOS/
    Frameworks/
    Resources/
      bin/
        agent-browser
```

### Linux

二进制文件会包含在应用的资源目录中。

### Windows

`agent-browser.exe` 会包含在应用的资源目录中。

## 故障排除

### 1. 找不到agent-browser二进制文件

确保你已运行了设置命令：

```bash
npm run setup:agent-browser
```

### 2. 权限被拒绝（macOS/Linux）

二进制文件应该已经具有执行权限，但如果没有：

```bash
chmod +x src-tauri/bin/agent-browser
```

### 3. 平台不支持

检查 `scripts/download-agent-browser.mjs` 中的平台检测逻辑。支持的平台包括：
- macOS (arm64/x64)
- Linux (arm64/x64)
- Windows (x64)

## 更新Agent Browser版本

要更新到新版本的agent-browser：

1. 编辑 `scripts/download-agent-browser.mjs` 中的 `AGENT_BROWSER_VERSION`
2. 运行 `npm run setup:agent-browser`

## 删除旧文件

如果你之前使用了 `prepare-agent-browser-latest.mjs` 脚本，可以删除：

```bash
rm scripts/prepare-agent-browser-latest.mjs
rm -rf src-tauri/agent-browser-bundle
```

## 许可证

Agent Browser 采用 Apache-2.0 许可证。参见：https://github.com/vercel-labs/agent-browser

## 相关资源

- [Agent Browser GitHub](https://github.com/vercel-labs/agent-browser)
- [Agent Browser Documentation](https://agent-browser.dev/)
- [Tauri Sidecar文档](https://tauri.app/v1/api/config/#bundleconfig)
