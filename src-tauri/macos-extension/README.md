# Sentinel AI Network Extension

macOS System Extension 用于实现应用级透明代理，类似 Proxifier。

## 功能

- 使用 `NETransparentProxyProvider` 拦截所有应用的网络流量
- 支持按应用程序过滤
- 支持按目标主机/端口过滤
- 将流量转发到本地代理服务器

## 开发要求

1. Apple Developer Program 会员资格
2. Xcode 14+
3. macOS 12+

## Entitlements

需要以下 entitlements：
- `com.apple.developer.networking.networkextension`
- `com.apple.developer.system-extension.install`

## 构建步骤

1. 在 Apple Developer Portal 创建 App ID 并启用 Network Extension capability
2. 创建 Provisioning Profile
3. 使用 Xcode 打开项目并配置签名
4. 构建 System Extension

## 项目结构

```
macos-extension/
├── SentinelProxy/                    # System Extension
│   ├── Info.plist
│   ├── SentinelProxy.entitlements
│   ├── TransparentProxyProvider.swift
│   ├── ProxyConnection.swift
│   └── main.swift
├── SentinelProxyManager/             # 管理库 (供 Tauri 调用)
│   ├── ExtensionManager.swift
│   └── XPCService.swift
└── Package.swift
```

