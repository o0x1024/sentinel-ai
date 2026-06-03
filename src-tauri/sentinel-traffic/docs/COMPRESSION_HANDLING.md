# HTTP 响应压缩处理

## 概述

sentinel-traffic 现在完全支持 HTTP 响应体的压缩格式检测和自动解压，支持以下压缩算法：

- **gzip** (Content-Encoding: gzip)
- **Brotli** (Content-Encoding: br)
- **Deflate** (Content-Encoding: deflate)

## 工作原理

### 1. 响应拦截与检测

在代理服务（`proxy.rs`）中，当拦截到 HTTP 响应时：

```rust
// 提取 Content-Encoding 响应头
let content_encoding = headers.get("content-encoding")
    .or_else(|| headers.get("Content-Encoding"))
    .map(|s| s.as_str());
```

### 2. 自动解压

如果检测到压缩编码，自动调用解压函数：

```rust
let decompressed_body = if content_encoding.is_some() {
    Self::decompress_body(&compressed_body_vec, content_encoding)
} else {
    compressed_body_vec.clone()
};
```

### 3. 双重保存策略

- **转发给客户端**：保持压缩格式（原样转发，不影响性能）
- **保存到数据库**：存储解压后的内容（便于扫描和显示）

```rust
// 转发：使用压缩后的原始数据
let new_body = Body::from(Full::new(body_bytes.clone()));

// 保存：使用解压后的数据
let resp_ctx = ResponseContext {
    body: body_vec, // 解压后的数据
    // ...
};
```

## 支持的压缩格式

### Gzip

使用 `flate2` crate 处理：

```rust
use flate2::read::GzDecoder;
let mut decoder = GzDecoder::new(body_bytes);
decoder.bytes().collect::<std::io::Result<Vec<u8>>>()
```

压缩率：通常 70-90%（大型 JSON 可达 90%）

### Brotli

使用 `brotli` crate 处理：

```rust
use brotli::Decompressor;
let mut decompressor = Decompressor::new(body_bytes, 4096);
decompressor.read_to_end(&mut decompressed)
```

压缩率：通常比 gzip 高 15-20%

### Deflate

使用 `flate2::read::DeflateDecoder` 处理：

```rust
use flate2::read::DeflateDecoder;
let mut decoder = DeflateDecoder::new(body_bytes);
decoder.bytes().collect::<std::io::Result<Vec<u8>>>()
```

## 前端显示

### 压缩状态指示

ProxyHistory.vue 会在响应详情中显示解压标记：

```vue
<span v-if="isResponseCompressed(selectedRequest)" 
      class="badge badge-xs badge-info">
  <i class="fas fa-file-archive mr-1"></i>Decompressed
</span>
```

### 内容类型智能格式化

根据 Content-Type 自动选择显示方式：

- **JSON**: 自动格式化（Pretty 打印）
- **HTML/XML**: 原样显示
- **Text**: 原样显示
- **Binary**: 显示大小和前 200 字符

```typescript
function formatResponse(request: ProxyRequest, tab: string): string {
  const contentType = getResponseContentType(request);
  
  if (contentType.includes('json')) {
    const json = JSON.parse(request.response_body);
    return JSON.stringify(json, null, 2);
  }
  // ...其他处理
}
```

## 错误处理

### 解压失败

如果解压失败（损坏的压缩数据、不支持的格式等），会：

1. 记录警告日志
2. 返回原始（压缩）数据
3. 不中断代理服务

```rust
Err(e) => {
    warn!("Failed to decompress gzip body: {}, returning original", e);
    body_bytes.to_vec()
}
```

### 大小限制

支持两层大小限制：

1. **压缩数据限制**：`max_response_body_size`（默认 2MB）
2. **解压数据限制**：同样为 `max_response_body_size`

如果超出限制，会截断数据并记录警告：

```rust
if decompressed_body.len() > self.config.max_response_body_size {
    warn!("Decompressed response body too large ({} bytes), truncating...");
    decompressed_body[..self.config.max_response_body_size].to_vec()
}
```

## 性能影响

### 内存使用

- 临时内存占用：压缩数据 + 解压数据
- 实际影响：对于 2MB 限制，峰值约 4MB
- 优化：使用流式解压（未来改进）

### CPU 开销

- Gzip 解压：极低（~1ms for 1MB）
- Brotli 解压：略高（~3ms for 1MB）
- 总体影响：可忽略不计

### 网络性能

- **零影响**：原样转发压缩数据给客户端
- 不影响代理吞吐量
- 不影响连接延迟

## 测试覆盖

参见 `tests/test_compression.rs`：

- ✅ Gzip 压缩/解压
- ✅ Brotli 压缩/解压
- ✅ 大型响应处理（28KB JSON）
- ✅ 压缩率验证（gzip: ~8.3%）

运行测试：

```bash
cd src-tauri/sentinel-traffic
cargo test test_compression -- --nocapture
```

## 配置选项

在 `ProxyConfig` 中可以调整：

```rust
pub struct ProxyConfig {
    pub max_response_body_size: usize, // 默认 2MB
    // ...
}
```

## 日志记录

启用调试日志查看解压详情：

```bash
RUST_LOG=sentinel_traffic=debug
```

示例输出：

```
DEBUG sentinel_traffic::proxy: Detected content encoding: Some("gzip"), attempting decompression
DEBUG sentinel_traffic::proxy: Decompressed gzip body: 1024 -> 8192 bytes
DEBUG sentinel_traffic::proxy: Captured response body: compressed=1024 bytes, decompressed=8192 bytes
```

## 已知限制

1. **不支持压缩链**：如 `gzip, deflate` 组合
2. **不支持其他格式**：如 `compress`, `identity`
3. **非流式处理**：整体读取后解压（受 2MB 限制保护）

## 未来改进

- [ ] 流式解压（降低内存占用）
- [ ] 支持压缩链解析
- [ ] 支持更多压缩格式（zstd, lz4）
- [ ] 配置化压缩检测（可选禁用）
- [ ] 解压统计（成功率、平均压缩率等）

## 相关文件

- `src/proxy.rs` - 解压逻辑实现
- `src/database.rs` - 保存解压后的数据
- `tests/test_compression.rs` - 压缩测试
- `../../src/components/ProxyHistory.vue` - 前端显示

## 参考资料

- [RFC 7230 - HTTP/1.1: Content-Encoding](https://tools.ietf.org/html/rfc7230#section-3.3.1)
- [Brotli Compression](https://github.com/google/brotli)
- [flate2 Documentation](https://docs.rs/flate2/)
