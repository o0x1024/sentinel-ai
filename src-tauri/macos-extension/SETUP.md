# Network Extension 配置指南

## 前提条件

1. **Apple Developer Program 会员资格** - 需要付费的开发者账号
2. **Xcode 14+** - 用于构建和签名
3. **macOS 12+** - 最低系统要求

## 步骤 1: 在 Apple Developer Portal 配置

### 1.1 创建 App ID

1. 登录 [Apple Developer Portal](https://developer.apple.com/account)
2. 进入 Certificates, Identifiers & Profiles → Identifiers
3. 点击 "+" 创建新的 App ID

**主应用 App ID:**
- Bundle ID: `com.sentinel-ai.app`
- Capabilities:
  - ✅ App Groups
  - ✅ Network Extensions

**System Extension App ID:**
- Bundle ID: `com.sentinel-ai.proxy.extension`
- Capabilities:
  - ✅ App Groups
  - ✅ Network Extensions

### 1.2 配置 App Group

1. 进入 Identifiers → App Groups
2. 创建新的 App Group: `group.com.sentinel-ai`

### 1.3 申请 Network Extension 权限

> ⚠️ **重要**: Network Extension 需要 Apple 的特殊批准

1. 访问 [Network Extension Entitlement Request](https://developer.apple.com/contact/request/network-extension/)
2. 填写申请表单：
   - App Name: Sentinel AI
   - App Type: Security Tool
   - Network Extension Types: 
     - ✅ Transparent Proxy Provider
     - ✅ App Proxy Provider
   - 描述: Security analysis tool for intercepting and analyzing network traffic

3. 等待 Apple 批准（通常需要几天到几周）

### 1.4 创建 Provisioning Profile

批准后，创建开发和分发 Profile：

1. 进入 Profiles
2. 创建新 Profile:
   - 类型: macOS App Development / macOS App Store Distribution
   - App ID: 选择对应的 App ID
   - 证书: 选择你的开发/分发证书
   - 下载并安装 Profile

## 步骤 2: Xcode 项目配置

### 2.1 创建 Xcode 项目

```bash
cd src-tauri/macos-extension
```

1. 打开 Xcode
2. File → New → Project
3. 选择 macOS → System Extension
4. 配置项目：
   - Product Name: SentinelProxy
   - Team: 选择你的开发者团队
   - Bundle Identifier: com.sentinel-ai.proxy.extension
   - Extension Type: Network Extension

### 2.2 配置主应用 Target

1. 添加 Capability:
   - App Groups: group.com.sentinel-ai
   - Network Extensions

2. 配置 Entitlements (MainApp.entitlements):
```xml
<key>com.apple.developer.system-extension.install</key>
<true/>
<key>com.apple.developer.networking.networkextension</key>
<array>
    <string>transparent-proxy-provider</string>
    <string>app-proxy-provider</string>
</array>
<key>com.apple.security.application-groups</key>
<array>
    <string>group.com.sentinel-ai</string>
</array>
```

### 2.3 配置 System Extension Target

1. 添加 Capability:
   - App Groups: group.com.sentinel-ai
   - Network Extensions

2. 配置 Entitlements (SentinelProxy.entitlements):
```xml
<key>com.apple.developer.networking.networkextension</key>
<array>
    <string>transparent-proxy-provider</string>
    <string>app-proxy-provider</string>
</array>
<key>com.apple.security.application-groups</key>
<array>
    <string>group.com.sentinel-ai</string>
</array>
<key>com.apple.security.network.client</key>
<true/>
<key>com.apple.security.network.server</key>
<true/>
```

3. 配置 Info.plist:
```xml
<key>NetworkExtension</key>
<dict>
    <key>NEProviderClasses</key>
    <dict>
        <key>com.apple.networkextension.transparent-proxy</key>
        <string>$(PRODUCT_MODULE_NAME).TransparentProxyProvider</string>
    </dict>
</dict>
```

### 2.4 复制源代码

将以下文件添加到 System Extension target:
- `SentinelProxy/TransparentProxyProvider.swift`
- `SentinelProxy/main.swift`

将以下文件添加到主应用或 Framework target:
- `SentinelProxyManager/ExtensionManager.swift`
- `SentinelProxyManager/CInterface.swift`

### 2.5 构建设置

1. System Extension Target:
   - Build Settings → Skip Install: No
   - Build Settings → Code Sign Style: Manual
   - Build Settings → Development Team: 你的 Team ID

2. 主应用 Target:
   - 添加 System Extension 作为 Embed App Extension

## 步骤 3: 集成到 Tauri

### 3.1 构建 System Extension

```bash
xcodebuild -project SentinelProxy.xcodeproj \
    -scheme SentinelProxy \
    -configuration Release \
    -archivePath build/SentinelProxy.xcarchive \
    archive
```

### 3.2 复制到 Tauri 应用

System Extension 需要放在主应用的以下位置:
```
Sentinel AI.app/
  Contents/
    Library/
      SystemExtensions/
        com.sentinel-ai.proxy.extension.systemextension/
```

### 3.3 配置 Tauri

在 `tauri.conf.json` 中添加:
```json
{
  "bundle": {
    "macOS": {
      "entitlements": "macos-extension/MainApp.entitlements"
    }
  }
}
```

## 步骤 4: 测试

### 4.1 开发环境测试

1. 启用 SIP 的 System Extension 开发者模式:
```bash
# 进入恢复模式后执行
csrutil enable --without kext
```

或者使用命令行工具:
```bash
systemextensionsctl developer on
```

2. 运行应用并安装 Extension

### 4.2 用户批准

首次运行时，用户需要在以下位置批准:
- System Preferences → Privacy & Security → Security

## 常见问题

### Q: Extension 安装失败
A: 检查:
1. 开发者账号是否有 Network Extension 权限
2. Provisioning Profile 是否正确
3. 代码签名是否有效

### Q: 代理不工作
A: 检查:
1. Extension 是否已安装并启用
2. 防火墙设置是否阻止
3. 查看系统日志: `log stream --predicate 'subsystem == "com.sentinel-ai.proxy"'`

### Q: 系统提示不受信任
A: 确保使用 Developer ID 证书签名，或在系统偏好设置中批准

## 调试

查看 Extension 日志:
```bash
log stream --predicate 'subsystem == "com.sentinel-ai.proxy"' --level debug
```

查看系统 Extension 状态:
```bash
systemextensionsctl list
```

