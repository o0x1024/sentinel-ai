# 高性能端口扫描器

## 概述

本项目实现了一个高性能的TCP端口扫描工具，基于用户提供的代码进行了完整的集成和优化。该扫描器支持并发扫描、服务识别、灵活的端口配置等功能。

## 功能特性

### 🚀 高性能扫描
- **并发扫描**: 支持1-1000个并发线程
- **异步处理**: 基于Tokio异步运行时
- **信号量控制**: 精确控制并发数量，避免资源耗尽
- **超时控制**: 可配置的连接超时时间

### 🔍 服务识别
- **内置服务库**: 识别常见服务（HTTP、HTTPS、SSH、FTP等）
- **响应时间**: 记录每个端口的响应时间
- **端口状态**: 区分开放、关闭、过滤、超时状态

### ⚙️ 灵活配置
- **端口范围**: 支持范围扫描（如 `1-1000`）
- **端口列表**: 支持指定端口列表（如 `80,443,8080`）
- **常用端口**: 内置常用端口预设（`common`）
- **自定义参数**: 可调整线程数、超时时间等

## 使用方法

### 基本用法

```rust
use sentinel_ai::tools::builtin::PortScanTool;
use sentinel_ai::tools::{ToolExecutionParams, UnifiedTool};
use serde_json::json;
use std::collections::HashMap;

// 创建扫描工具
let port_scanner = PortScanTool::new();

// 配置扫描参数
let mut inputs = HashMap::new();
inputs.insert("target".to_string(), json!("192.168.1.1"));
inputs.insert("ports".to_string(), json!("common"));
inputs.insert("threads".to_string(), json!(100));
inputs.insert("timeout".to_string(), json!(3));

let params = ToolExecutionParams {
    inputs,
    context: HashMap::new(),
    timeout: None,
    conversation_id: Some(Uuid::new_v4()),
};

// 执行扫描
let result = port_scanner.execute(params).await?;
```

### 参数说明

| 参数名 | 类型 | 必需 | 默认值 | 说明 |
|--------|------|------|--------|---------|
| `target` | String | ✅ | - | 目标IP地址 |
| `ports` | String | ❌ | `"common"` | 端口配置 |
| `threads` | Number | ❌ | `100` | 并发线程数 (1-1000) |
| `timeout` | Number | ❌ | `3` | 连接超时时间（秒） |

### 端口配置格式

1. **常用端口**: `"common"`
   - 扫描预定义的常用端口列表
   - 包含: 21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 443, 993, 995, 1723, 3306, 3389, 5432, 5900, 8080

2. **端口范围**: `"1-1000"`
   - 扫描指定范围内的所有端口
   - 支持任意有效的端口范围

3. **端口列表**: `"80,443,8080,9000"`
   - 扫描指定的端口列表
   - 用逗号分隔多个端口

4. **混合配置**: `"80,443,8000-8100"`
   - 支持端口和范围的混合配置

## 输出格式

扫描完成后，工具返回详细的JSON格式结果：

```json
{
  "target": "192.168.1.1",
  "ports_scanned": [21, 22, 23, 25, 53, 80, ...],
  "open_ports": [
    {
      "port": 22,
      "status": "Open",
      "service": "SSH",
      "banner": null,
      "response_time": 15
    },
    {
      "port": 80,
      "status": "Open",
      "service": "HTTP",
      "banner": null,
      "response_time": 8
    }
  ],
  "open_count": 2,
  "total_ports": 20,
  "scan_duration": 5,
  "scan_summary": {
    "target_ip": "192.168.1.1",
    "threads_used": 100,
    "timeout_seconds": 3,
    "ports_config": "common"
  }
}
```

### 字段说明

- `target`: 扫描的目标IP地址
- `ports_scanned`: 实际扫描的端口列表
- `open_ports`: 开放端口的详细信息数组
  - `port`: 端口号
  - `status`: 端口状态（Open/Closed/Filtered/Timeout）
  - `service`: 识别的服务名称
  - `banner`: 服务横幅信息（待实现）
  - `response_time`: 响应时间（毫秒）
- `open_count`: 开放端口数量
- `total_ports`: 总扫描端口数量
- `scan_duration`: 扫描耗时（秒）
- `scan_summary`: 扫描摘要信息

## 性能优化

### 并发控制
- 使用信号量（Semaphore）精确控制并发数量
- 避免创建过多连接导致系统资源耗尽
- 支持1-1000个并发线程的配置范围

### 超时机制
- 每个端口连接都有独立的超时控制
- 避免因网络延迟导致的长时间等待
- 可根据网络环境调整超时时间

### 内存优化
- 异步任务设计，避免阻塞主线程
- 及时释放连接资源
- 结果数据结构优化

## 测试示例

项目提供了完整的测试示例，位于 `examples/port_scanner_test.rs`：

```bash
# 运行测试示例
cargo run --example port_scanner_test
```

测试包含：
1. 扫描本地常用端口
2. 扫描指定端口范围
3. 扫描特定端口列表

## 注意事项

### 安全考虑
- 仅用于授权的网络安全测试
- 遵守相关法律法规和网络使用政策
- 避免对生产环境进行大规模扫描

### 性能建议
- 根据目标网络环境调整并发线程数
- 合理设置超时时间，平衡速度和准确性
- 对于大范围扫描，建议分批进行

### 错误处理
- 工具内置完整的错误处理机制
- 参数验证确保输入的有效性
- 网络异常时提供详细的错误信息

## 技术实现

### 核心组件

1. **PortScanner**: 核心扫描引擎
   - 管理常用端口列表
   - 实现单端口扫描逻辑
   - 提供服务识别功能
   - 支持并发批量扫描

2. **PortScanTool**: 工具接口实现
   - 实现UnifiedTool trait
   - 参数解析和验证
   - 结果格式化和返回

3. **数据结构**:
   - `PortResult`: 单端口扫描结果
   - `PortScanResults`: 完整扫描结果
   - `ScanConfig`: 扫描配置
   - `PortStatus`: 端口状态枚举

### 依赖库
- `tokio`: 异步运行时和网络库
- `serde`: 序列化和反序列化
- `anyhow`: 错误处理
- `uuid`: 执行ID生成
- `tracing`: 日志记录

## 更新日志

### v2.0.0 (当前版本)
- ✨ 重构为高性能异步扫描器
- ✨ 添加服务识别功能
- ✨ 支持灵活的端口配置
- ✨ 实现并发控制和超时机制
- ✨ 优化输出格式和错误处理
- 📝 完善文档和测试示例

### v1.0.0 (旧版本)
- 基础TCP端口扫描功能
- 简单的并发实现
- 基本的结果输出

## 贡献指南

欢迎提交Issue和Pull Request来改进这个工具！

### 开发环境
```bash
# 克隆项目
git clone <repository-url>
cd sentinel-ai/src-tauri

# 安装依赖
cargo build

# 运行测试
cargo test

# 检查代码
cargo check
```

### 代码规范
- 遵循Rust官方代码风格
- 添加适当的文档注释
- 编写单元测试
- 确保代码通过cargo check