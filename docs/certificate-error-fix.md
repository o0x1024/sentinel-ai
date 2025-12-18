# Certificate Error Fix - 无法抓包证书无效网站

## 问题描述

根据日志 `sentinel-ai.log.2025-12-17`，代理在遇到证书错误时会自动禁用MITM并切换到隧道模式：

```
WARN sentinel_passive::proxy: 632: MITM disabled for host 172.31.57.200 after 3 failures; future CONNECT will be tunneled
WARN sentinel_passive::proxy: 1485: Forward request failed (network): host=Some("172.31.57.200") conn_key=127.0.0.1:54156 error=client error (Connect) | invalid peer certificate: Other(OtherError(UnsupportedCertVersion))
```

**问题根因**：
1. 代理使用的HTTP连接器配置了`with_native_roots()`，会验证上游服务器证书
2. 遇到`UnsupportedCertVersion`等证书错误时，会计数并达到阈值后自动绕过MITM
3. 一旦绕过MITM，就无法抓取请求和响应内容

## 解决方案

### 1. 修改HTTP连接器配置

**文件**: `src-tauri/sentinel-passive/src/proxy.rs:1949-1958`

**修改前**:
```rust
let https_connector = HttpsConnectorBuilder::new()
    .with_native_roots()  // ❌ 这会验证证书
    .expect("Failed to load native roots")
    .https_or_http()
    .enable_http1()
    .enable_http2()
    .build();
```

**修改后**:
```rust
// 创建忽略证书验证的 rustls ClientConfig
let rustls_config = create_insecure_rustls_config();

let https_connector = HttpsConnectorBuilder::new()
    .with_tls_config(rustls_config)  // ✅ 使用自定义配置，忽略证书验证
    .https_or_http()
    .enable_http1()
    .enable_http2()
    .build();
```

### 2. 禁用自动绕过MITM逻辑

**文件**: `src-tauri/sentinel-passive/src/proxy.rs:1435-1463`

**修改前**:
```rust
if is_tls_error {
    self_clone.note_fail_and_maybe_bypass(host).await;  // ❌ 会禁用MITM
    // ...发送失败记录...
}
```

**修改后**:
```rust
if is_tls_error {
    // 发送失败连接记录到扫描器（用于统计和展示）
    // ...发送失败记录...
    
    // ✅ 不再自动绕过MITM，因为我们已经配置为忽略证书错误
    warn!(
        "TLS error detected for host {}, but continuing with MITM (certificate validation disabled)",
        host
    );
}
```

### 3. `create_insecure_rustls_config()` 函数说明

该函数在文件开头已定义（行271-290），配置：

```rust
fn create_insecure_rustls_config() -> rustls::ClientConfig {
    let mut config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureServerCertVerifier))  // 自定义验证器
        .with_no_client_auth();
    
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    config.enable_sni = true;
    config.enable_secret_extraction = false;
    
    config
}
```

`InsecureServerCertVerifier`的实现（行34-82）：
- 忽略所有证书验证错误
- 支持所有签名算法
- 始终返回`ServerCertVerified::assertion()`

## 测试验证

### 测试场景

1. **标准HTTPS网站** ✅
   - Google, GitHub等
   - 正常证书，应该能抓包

2. **证书版本不支持** ✅
   - 如日志中的`UnsupportedCertVersion`
   - 应该能抓包而不是绕过

3. **自签名证书** ✅
   - 内网服务器
   - 应该能抓包

4. **过期证书** ✅
   - 测试站点
   - 应该能抓包

5. **主机名不匹配** ✅
   - 证书CN与访问域名不符
   - 应该能抓包

### 验证方法

1. 启动代理服务
2. 配置浏览器使用代理（如127.0.0.1:4201）
3. 访问有证书问题的网站
4. 检查：
   - ✅ 请求和响应都能在历史记录中看到
   - ✅ 日志中没有"MITM disabled"警告
   - ✅ 状态码不是0（TLS ERR）而是实际HTTP状态码

## 技术要点

### 为什么这样修复有效？

1. **客户端到代理**：浏览器信任我们的Root CA，使用我们签发的证书
2. **代理到服务器**：使用`InsecureServerCertVerifier`，忽略所有证书错误
3. **MITM完整性**：即使服务器证书有问题，我们仍然能解密和重新加密流量

### 安全考虑

- ⚠️ 这是**测试/调试工具**，不应在生产环境使用
- ⚠️ 忽略证书验证会带来中间人攻击风险
- ✅ 仅用于安全测试和渗透测试场景
- ✅ 用户需要主动安装并信任Root CA

### 与之前的改进配合

此修复与之前的证书生成改进配合：
1. **证书生成**（certificate_authority.rs）: 生成兼容性更好的证书
2. **连接器配置**（此次修复）: 忽略上游服务器证书错误
3. **UI提示**（ProxyHistory.vue）: 友好显示证书错误信息

## 相关文件

- `src-tauri/sentinel-passive/src/proxy.rs` - 主要修改
- `src-tauri/sentinel-passive/src/certificate_authority.rs` - 证书生成
- `src/components/passive/ProxyHistory.vue` - UI显示
- `docs/certificate-error-handling.md` - 完整文档

## 编译验证

```bash
cd /Users/a1024/code/ai/sentinel-ai
cargo check --manifest-path src-tauri/Cargo.toml --package sentinel-passive
```

✅ 编译成功，仅有无关的警告（unused imports等）

