import Foundation
import SystemExtensions
import NetworkExtension
import os.log

/// System Extension 安装状态
@objc public enum ExtensionStatus: Int {
    case unknown = 0
    case notInstalled = 1
    case installing = 2
    case installed = 3
    case needsApproval = 4
    case failed = 5
}

/// Network Extension 管理器
/// 负责安装、激活和管理 System Extension
@objc public class ExtensionManager: NSObject {
    
    // MARK: - Singleton
    
    @objc public static let shared = ExtensionManager()
    
    // MARK: - Properties
    
    private let logger = Logger(subsystem: "com.sentinel-ai", category: "ExtensionManager")
    
    /// Extension Bundle Identifier
    private let extensionBundleIdentifier = "com.sentinel-ai.proxy.extension"
    
    /// 当前安装状态
    @objc public private(set) var status: ExtensionStatus = .unknown
    
    /// 状态变化回调
    public var statusChangedHandler: ((ExtensionStatus) -> Void)?
    
    /// NETunnelProviderManager
    private var providerManager: NETunnelProviderManager?
    
    // MARK: - Initialization
    
    private override init() {
        super.init()
    }
    
    // MARK: - Public Methods
    
    /// 检查 Extension 状态
    @objc public func checkStatus(completion: @escaping (ExtensionStatus) -> Void) {
        NETunnelProviderManager.loadAllFromPreferences { [weak self] managers, error in
            guard let self = self else { return }
            
            if let error = error {
                self.logger.error("Failed to load managers: \(error.localizedDescription)")
                self.updateStatus(.failed)
                completion(.failed)
                return
            }
            
            if let manager = managers?.first(where: { 
                ($0.protocolConfiguration as? NETunnelProviderProtocol)?.providerBundleIdentifier == self.extensionBundleIdentifier 
            }) {
                self.providerManager = manager
                if manager.isEnabled {
                    self.updateStatus(.installed)
                    completion(.installed)
                } else {
                    self.updateStatus(.notInstalled)
                    completion(.notInstalled)
                }
            } else {
                self.updateStatus(.notInstalled)
                completion(.notInstalled)
            }
        }
    }
    
    /// 安装 System Extension
    @objc public func install(completion: @escaping (Bool, String?) -> Void) {
        logger.info("Installing system extension")
        updateStatus(.installing)
        
        let request = OSSystemExtensionRequest.activationRequest(
            forExtensionWithIdentifier: extensionBundleIdentifier,
            queue: .main
        )
        
        let delegate = ExtensionRequestDelegate { [weak self] result in
            switch result {
            case .success:
                self?.logger.info("Extension activation succeeded")
                self?.configureProvider(completion: completion)
                
            case .needsApproval:
                self?.logger.info("Extension needs user approval")
                self?.updateStatus(.needsApproval)
                completion(false, "需要在系统偏好设置中批准扩展")
                
            case .failure(let error):
                self?.logger.error("Extension activation failed: \(error)")
                self?.updateStatus(.failed)
                completion(false, error)
            }
        }
        
        request.delegate = delegate
        OSSystemExtensionManager.shared.submitRequest(request)
        
        // 保持 delegate 引用
        objc_setAssociatedObject(request, "delegate", delegate, .OBJC_ASSOCIATION_RETAIN)
    }
    
    /// 卸载 System Extension
    @objc public func uninstall(completion: @escaping (Bool, String?) -> Void) {
        logger.info("Uninstalling system extension")
        
        let request = OSSystemExtensionRequest.deactivationRequest(
            forExtensionWithIdentifier: extensionBundleIdentifier,
            queue: .main
        )
        
        let delegate = ExtensionRequestDelegate { [weak self] result in
            switch result {
            case .success:
                self?.logger.info("Extension deactivation succeeded")
                self?.removeProvider(completion: completion)
                
            case .needsApproval:
                self?.updateStatus(.needsApproval)
                completion(false, "需要在系统偏好设置中批准")
                
            case .failure(let error):
                self?.logger.error("Extension deactivation failed: \(error)")
                self?.updateStatus(.failed)
                completion(false, error)
            }
        }
        
        request.delegate = delegate
        OSSystemExtensionManager.shared.submitRequest(request)
        
        objc_setAssociatedObject(request, "delegate", delegate, .OBJC_ASSOCIATION_RETAIN)
    }
    
    /// 启动代理
    @objc public func startProxy(
        proxyHost: String,
        proxyPort: UInt16,
        targetApps: [String],
        completion: @escaping (Bool, String?) -> Void
    ) {
        guard let manager = providerManager else {
            checkStatus { [weak self] status in
                if status == .installed {
                    self?.startProxy(proxyHost: proxyHost, proxyPort: proxyPort, targetApps: targetApps, completion: completion)
                } else {
                    completion(false, "Extension not installed")
                }
            }
            return
        }
        
        // 配置代理选项
        let options: [String: Any] = [
            "proxyHost": proxyHost,
            "proxyPort": proxyPort,
            "targetApps": targetApps
        ]
        
        do {
            try manager.connection.startVPNTunnel(options: options)
            logger.info("Proxy started successfully")
            completion(true, nil)
        } catch {
            logger.error("Failed to start proxy: \(error.localizedDescription)")
            completion(false, error.localizedDescription)
        }
    }
    
    /// 停止代理
    @objc public func stopProxy(completion: @escaping (Bool, String?) -> Void) {
        guard let manager = providerManager else {
            completion(false, "Extension not configured")
            return
        }
        
        manager.connection.stopVPNTunnel()
        logger.info("Proxy stopped")
        completion(true, nil)
    }
    
    /// 获取代理状态
    @objc public func getProxyStatus() -> NEVPNStatus {
        return providerManager?.connection.status ?? .invalid
    }
    
    // MARK: - Private Methods
    
    private func updateStatus(_ newStatus: ExtensionStatus) {
        status = newStatus
        statusChangedHandler?(newStatus)
    }
    
    private func configureProvider(completion: @escaping (Bool, String?) -> Void) {
        NETunnelProviderManager.loadAllFromPreferences { [weak self] managers, error in
            guard let self = self else { return }
            
            if let error = error {
                self.logger.error("Failed to load managers: \(error.localizedDescription)")
                completion(false, error.localizedDescription)
                return
            }
            
            let manager = managers?.first ?? NETunnelProviderManager()
            
            // 配置协议
            let proto = NETunnelProviderProtocol()
            proto.providerBundleIdentifier = self.extensionBundleIdentifier
            proto.serverAddress = "Sentinel AI Proxy"
            
            manager.protocolConfiguration = proto
            manager.localizedDescription = "Sentinel AI Transparent Proxy"
            manager.isEnabled = true
            
            manager.saveToPreferences { error in
                if let error = error {
                    self.logger.error("Failed to save preferences: \(error.localizedDescription)")
                    self.updateStatus(.failed)
                    completion(false, error.localizedDescription)
                    return
                }
                
                self.providerManager = manager
                self.updateStatus(.installed)
                self.logger.info("Provider configured successfully")
                completion(true, nil)
            }
        }
    }
    
    private func removeProvider(completion: @escaping (Bool, String?) -> Void) {
        providerManager?.removeFromPreferences { [weak self] error in
            if let error = error {
                self?.logger.error("Failed to remove preferences: \(error.localizedDescription)")
                completion(false, error.localizedDescription)
                return
            }
            
            self?.providerManager = nil
            self?.updateStatus(.notInstalled)
            self?.logger.info("Provider removed successfully")
            completion(true, nil)
        }
    }
}

// MARK: - Extension Request Delegate

private enum ExtensionRequestResult {
    case success
    case needsApproval
    case failure(String)
}

private class ExtensionRequestDelegate: NSObject, OSSystemExtensionRequestDelegate {
    
    private let completion: (ExtensionRequestResult) -> Void
    
    init(completion: @escaping (ExtensionRequestResult) -> Void) {
        self.completion = completion
    }
    
    func request(_ request: OSSystemExtensionRequest, actionForReplacingExtension existing: OSSystemExtensionProperties, withExtension ext: OSSystemExtensionProperties) -> OSSystemExtensionRequest.ReplacementAction {
        return .replace
    }
    
    func requestNeedsUserApproval(_ request: OSSystemExtensionRequest) {
        completion(.needsApproval)
    }
    
    func request(_ request: OSSystemExtensionRequest, didFinishWithResult result: OSSystemExtensionRequest.Result) {
        switch result {
        case .completed:
            completion(.success)
        case .willCompleteAfterReboot:
            completion(.failure("需要重启后完成安装"))
        @unknown default:
            completion(.failure("未知结果"))
        }
    }
    
    func request(_ request: OSSystemExtensionRequest, didFailWithError error: Error) {
        completion(.failure(error.localizedDescription))
    }
}

