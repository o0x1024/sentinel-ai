// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "SentinelProxy",
    platforms: [
        .macOS(.v12)
    ],
    products: [
        // System Extension
        .executable(
            name: "SentinelProxy",
            targets: ["SentinelProxy"]
        ),
        // 管理库（供 Tauri 调用）
        .library(
            name: "SentinelProxyManager",
            type: .dynamic,
            targets: ["SentinelProxyManager"]
        )
    ],
    dependencies: [],
    targets: [
        // System Extension Target
        .executableTarget(
            name: "SentinelProxy",
            dependencies: [],
            path: "SentinelProxy",
            exclude: ["Info.plist", "SentinelProxy.entitlements"],
            linkerSettings: [
                .linkedFramework("NetworkExtension"),
                .linkedFramework("SystemExtensions"),
            ]
        ),
        // 管理库 Target
        .target(
            name: "SentinelProxyManager",
            dependencies: [],
            path: "SentinelProxyManager",
            linkerSettings: [
                .linkedFramework("NetworkExtension"),
                .linkedFramework("SystemExtensions"),
            ]
        )
    ]
)

