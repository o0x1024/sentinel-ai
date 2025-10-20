# HTTP请求日志记录

## 概述

本系统为所有LLM HTTP请求提供了完整的日志记录功能，将原始请求和响应数据记录到日志文件中，便于调试、审计和性能分析。

## 功能特性

- **自动记录**: 所有通过AI适配器发送的HTTP请求都会自动记录
- **完整信息**: 记录请求方法、URL、头部、请求体、响应状态、响应体等完整信息
- **提供商标识**: 自动识别并记录LLM提供商名称和模型名称
- **错误记录**: 记录请求失败的错误信息
- **日志轮转**: 按日期自动创建新的日志文件
- **性能优化**: 异步写入，不影响主要业务流程

## 日志文件格式

日志文件保存在 `logs/` 目录下，文件名格式为：`llm-http-requests-YYYY-MM-DD.log`

每个HTTP请求记录为原始HTTP格式，包含请求、响应和元数据信息，各请求之间用分隔线分开：

```
POST /v1/chat/completions HTTP/2
Host: api.openai.com
Content-Type: application/json
Authorization: Bearer sk-***
User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36
Content-Length: 2037

{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Hello"}]}

--- RESPONSE ---
HTTP/2 200 OK
Content-Type: application/json
Content-Length: 150

{"id":"chatcmpl-test","choices":[{"message":{"role":"assistant","content":"Hi there!"}}]}

--- METADATA ---
Timestamp: 2025-10-17 02:06:15.862 UTC
Request ID: req_d70c20ba56dfbda0
Provider: openai
Model: gpt-3.5-turbo
Request Type: chat
Duration: 500ms

===================================================================================================================================
```

## 日志结构说明

每个日志条目包含以下部分：

1. **HTTP请求部分**: 
   - 请求行（方法、路径、协议版本）
   - 请求头部
   - 空行
   - 请求体（如果有）

2. **HTTP响应部分**（如果有响应）:
   - `--- RESPONSE ---` 标记
   - 响应状态行
   - 响应头部
   - 空行
   - 响应体（如果有）

3. **错误信息部分**（如果有错误）:
   - `--- ERROR ---` 标记
   - 错误详细信息

4. **元数据部分**:
   - `--- METADATA ---` 标记
   - 时间戳、请求ID、提供商、模型、请求类型、持续时间等信息

5. **分隔线**: 每个请求记录后都有一行等号分隔线

## 配置

### 启用/禁用日志记录

```rust
use crate::ai_adapter::set_global_logger_enabled;

// 启用日志记录
set_global_logger_enabled(true).unwrap();

// 禁用日志记录
set_global_logger_enabled(false).unwrap();
```

### 清理旧日志

```rust
use crate::ai_adapter::cleanup_old_logs;

// 清理30天前的日志文件
cleanup_old_logs(30).await.unwrap();
```

## 安全考虑

- API密钥在日志中会被记录，请确保日志文件的访问权限得到适当控制
- 建议定期清理旧的日志文件以节省磁盘空间
- 在生产环境中，可以考虑对敏感信息进行脱敏处理

## 性能影响

- 日志记录采用异步方式，对主要业务流程的性能影响最小
- 日志写入失败不会影响HTTP请求的正常执行
- 使用互斥锁确保日志文件写入的线程安全

## 故障排除

如果日志记录不工作，请检查：

1. 日志目录是否存在且有写入权限
2. 全局日志记录器是否已正确初始化
3. 日志记录是否已启用

## 示例用法

查看今天的HTTP请求日志：

```bash
# 查看今天的日志文件
cat logs/llm-http-requests-$(date +%Y-%m-%d).log

# 查看最近的请求（显示最后几个请求）
tail -n 100 logs/llm-http-requests-$(date +%Y-%m-%d).log

# 统计各提供商的请求数量
grep "Provider:" logs/llm-http-requests-*.log | cut -d: -f3 | sort | uniq -c

# 查找错误请求
grep -A 10 "--- ERROR ---" logs/llm-http-requests-*.log

# 查看特定模型的请求
grep -B 20 -A 10 "Model: gpt-3.5-turbo" logs/llm-http-requests-*.log

# 查看特定提供商的请求
grep -B 20 -A 10 "Provider: openai" logs/llm-http-requests-*.log

# 统计请求总数
grep -c "POST\|GET" logs/llm-http-requests-*.log

# 查看响应状态码分布
grep "HTTP/2" logs/llm-http-requests-*.log | grep -v "POST\|GET" | cut -d' ' -f2 | sort | uniq -c
```
