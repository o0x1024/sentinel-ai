# 全局代理协议支持说明

## 概述

Sentinel AI 全局代理配置现已完整支持以下四种代理协议：

- **HTTP** - 标准HTTP代理协议
- **HTTPS** - HTTPS代理协议
- **SOCKS5** - SOCKS5代理（本地DNS解析）
- **SOCKS5H** - SOCKS5代理（远程DNS解析，更安全）

## 协议说明

### HTTP 代理
最常见的代理协议，适用于大多数HTTP/HTTPS流量。

**使用场景：**
- 普通网页访问
- API请求
- 常规网络代理

### HTTPS 代理
通过HTTPS连接到代理服务器，提供额外的加密层。

**使用场景：**
- 需要加密代理连接
- 企业级代理服务器

### SOCKS5 代理
更底层的代理协议，支持TCP和UDP流量。DNS解析在本地进行。

**使用场景：**
- 需要代理非HTTP流量
- 高级网络配置

**注意：** DNS请求不经过代理，可能泄露访问意图

### SOCKS5H 代理（推荐）
SOCKS5的变体，DNS解析在远程代理服务器进行。

**使用场景：**
- 需要完全隐私的网络访问
- 防止DNS泄露
- 访问受限网络资源

**优势：**
- 更高的隐私保护
- DNS请求也通过代理
- 防止DNS污染

## 配置说明

### 前端配置

在设置界面中：
1. 选择协议类型
2. 输入代理服务器地址和端口
3. （可选）输入认证信息
4. （可选）配置不走代理的地址列表

### 后端实现

#### 环境变量设置

不同协议设置的环境变量：

**HTTP/HTTPS 代理：**
- `HTTP_PROXY`
- `HTTPS_PROXY`

**SOCKS5/SOCKS5H 代理：**
- `ALL_PROXY` / `all_proxy`
- `HTTP_PROXY` / `HTTPS_PROXY` (为了兼容性)

**通用：**
- `NO_PROXY` / `no_proxy` (不走代理的地址列表)

#### 代理URL格式

```
{scheme}://{username}:{password}@{host}:{port}
```

示例：
- HTTP: `http://127.0.0.1:7890`
- HTTPS: `https://proxy.example.com:8080`
- SOCKS5: `socks5://192.168.1.1:1080`
- SOCKS5H: `socks5h://user:pass@proxy.example.com:1080`

## 技术实现

### 依赖更新

在 `Cargo.toml` 中添加了 `socks` feature：

```toml
# 主 crate
reqwest = { version = "0.12.0", features = ["json", "stream", "socks"] }

# sentinel-tools crate
reqwest = { version = "0.12.0", features = ["json", "socks"] }
```

### 代码结构

1. **主 crate** (`src-tauri/src/utils/global_proxy.rs`)
   - 全局代理配置管理
   - 环境变量设置
   - reqwest 客户端代理应用

2. **sentinel-tools** (`src-tauri/sentinel-tools/src/global_proxy.rs`)
   - 独立的代理配置存储
   - 工具crate专用的代理应用

3. **前端** (`src/components/Settings/NetworkSettings.vue`)
   - 用户界面配置
   - 协议选择和参数输入

## 使用建议

1. **日常使用** - 选择 HTTP 代理，简单高效
2. **隐私优先** - 选择 SOCKS5H，DNS也走代理
3. **企业环境** - 根据企业代理服务器类型选择
4. **特殊场景** - SOCKS5支持更多协议类型

## 测试验证

可以使用界面上的测试功能验证代理配置：
- 动态更新测试
- 持久化测试
- 客户端更新测试

## 注意事项

1. 修改代理配置后会立即生效，无需重启
2. 代理配置保存到数据库，重启后自动加载
3. 密码字段不会在日志中明文显示
4. 不走代理列表支持逗号分隔的多个地址

## 故障排除

如果代理不工作：

1. 检查代理服务器地址和端口是否正确
2. 验证代理服务器是否支持所选协议
3. 确认认证信息（如果需要）是否正确
4. 查看应用日志获取详细错误信息
