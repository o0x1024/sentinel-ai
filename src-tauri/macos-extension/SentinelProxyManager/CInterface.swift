import Foundation

/// C 接口层，供 Rust 调用
/// 使用 @_cdecl 导出 C 函数

// MARK: - Extension Status

/// 检查 Extension 状态
/// 返回: ExtensionStatus 的 raw value
@_cdecl("sentinel_extension_check_status")
public func sentinelExtensionCheckStatus() -> Int32 {
    var result: Int32 = 0
    let semaphore = DispatchSemaphore(value: 0)
    
    ExtensionManager.shared.checkStatus { status in
        result = Int32(status.rawValue)
        semaphore.signal()
    }
    
    semaphore.wait()
    return result
}

/// 获取当前状态
@_cdecl("sentinel_extension_get_status")
public func sentinelExtensionGetStatus() -> Int32 {
    return Int32(ExtensionManager.shared.status.rawValue)
}

// MARK: - Extension Install/Uninstall

/// 安装 Extension
/// 返回: 0 成功, 负数 失败
@_cdecl("sentinel_extension_install")
public func sentinelExtensionInstall(errorBuffer: UnsafeMutablePointer<CChar>?, bufferSize: Int32) -> Int32 {
    var result: Int32 = 0
    var errorMessage: String?
    let semaphore = DispatchSemaphore(value: 0)
    
    ExtensionManager.shared.install { success, error in
        result = success ? 0 : -1
        errorMessage = error
        semaphore.signal()
    }
    
    semaphore.wait()
    
    if let error = errorMessage, let buffer = errorBuffer {
        let cString = error.utf8CString
        let copyLength = min(Int(bufferSize) - 1, cString.count - 1)
        for i in 0..<copyLength {
            buffer[i] = cString[i]
        }
        buffer[copyLength] = 0
    }
    
    return result
}

/// 卸载 Extension
@_cdecl("sentinel_extension_uninstall")
public func sentinelExtensionUninstall(errorBuffer: UnsafeMutablePointer<CChar>?, bufferSize: Int32) -> Int32 {
    var result: Int32 = 0
    var errorMessage: String?
    let semaphore = DispatchSemaphore(value: 0)
    
    ExtensionManager.shared.uninstall { success, error in
        result = success ? 0 : -1
        errorMessage = error
        semaphore.signal()
    }
    
    semaphore.wait()
    
    if let error = errorMessage, let buffer = errorBuffer {
        let cString = error.utf8CString
        let copyLength = min(Int(bufferSize) - 1, cString.count - 1)
        for i in 0..<copyLength {
            buffer[i] = cString[i]
        }
        buffer[copyLength] = 0
    }
    
    return result
}

// MARK: - Proxy Control

/// 启动代理
@_cdecl("sentinel_proxy_start")
public func sentinelProxyStart(
    proxyHost: UnsafePointer<CChar>,
    proxyPort: UInt16,
    targetAppsJson: UnsafePointer<CChar>?,
    errorBuffer: UnsafeMutablePointer<CChar>?,
    bufferSize: Int32
) -> Int32 {
    let host = String(cString: proxyHost)
    var apps: [String] = []
    
    if let appsPtr = targetAppsJson {
        let appsString = String(cString: appsPtr)
        if let data = appsString.data(using: .utf8),
           let decoded = try? JSONDecoder().decode([String].self, from: data) {
            apps = decoded
        }
    }
    
    var result: Int32 = 0
    var errorMessage: String?
    let semaphore = DispatchSemaphore(value: 0)
    
    ExtensionManager.shared.startProxy(proxyHost: host, proxyPort: proxyPort, targetApps: apps) { success, error in
        result = success ? 0 : -1
        errorMessage = error
        semaphore.signal()
    }
    
    semaphore.wait()
    
    if let error = errorMessage, let buffer = errorBuffer {
        let cString = error.utf8CString
        let copyLength = min(Int(bufferSize) - 1, cString.count - 1)
        for i in 0..<copyLength {
            buffer[i] = cString[i]
        }
        buffer[copyLength] = 0
    }
    
    return result
}

/// 停止代理
@_cdecl("sentinel_proxy_stop")
public func sentinelProxyStop(errorBuffer: UnsafeMutablePointer<CChar>?, bufferSize: Int32) -> Int32 {
    var result: Int32 = 0
    var errorMessage: String?
    let semaphore = DispatchSemaphore(value: 0)
    
    ExtensionManager.shared.stopProxy { success, error in
        result = success ? 0 : -1
        errorMessage = error
        semaphore.signal()
    }
    
    semaphore.wait()
    
    if let error = errorMessage, let buffer = errorBuffer {
        let cString = error.utf8CString
        let copyLength = min(Int(bufferSize) - 1, cString.count - 1)
        for i in 0..<copyLength {
            buffer[i] = cString[i]
        }
        buffer[copyLength] = 0
    }
    
    return result
}

/// 获取代理状态
/// 返回: NEVPNStatus raw value
@_cdecl("sentinel_proxy_get_status")
public func sentinelProxyGetStatus() -> Int32 {
    return Int32(ExtensionManager.shared.getProxyStatus().rawValue)
}

