import Foundation
import NetworkExtension
import os.log

/// Sentinel AI Transparent Proxy Provider
/// 实现 macOS Network Extension，拦截应用网络流量并转发到本地代理
class TransparentProxyProvider: NETransparentProxyProvider {
    
    // MARK: - Properties
    
    private let logger = Logger(subsystem: "com.sentinel-ai.proxy", category: "TransparentProxy")
    
    /// 本地代理服务器地址
    private var proxyHost: String = "127.0.0.1"
    
    /// 本地代理服务器端口
    private var proxyPort: UInt16 = 8080
    
    /// 要代理的应用列表（Bundle ID），空表示所有应用
    private var targetApps: [String] = []
    
    /// 排除的应用列表
    private var excludedApps: [String] = [
        "com.apple.finder",
        "com.apple.systempreferences",
        "com.apple.Spotlight",
        "com.apple.Safari.SafeBrowsing",
    ]
    
    /// 要代理的端口列表，空表示所有端口
    private var targetPorts: [UInt16] = []
    
    /// 排除的主机列表
    private var excludedHosts: [String] = [
        "localhost",
        "127.0.0.1",
        "::1",
    ]
    
    // MARK: - Lifecycle
    
    override func startProxy(options: [String : Any]?, completionHandler: @escaping (Error?) -> Void) {
        logger.info("Starting Sentinel Transparent Proxy")
        
        // 从配置中读取代理设置
        if let config = options {
            if let host = config["proxyHost"] as? String {
                proxyHost = host
            }
            if let port = config["proxyPort"] as? UInt16 {
                proxyPort = port
            }
            if let apps = config["targetApps"] as? [String] {
                targetApps = apps
            }
            if let excluded = config["excludedApps"] as? [String] {
                excludedApps.append(contentsOf: excluded)
            }
            if let ports = config["targetPorts"] as? [UInt16] {
                targetPorts = ports
            }
            if let hosts = config["excludedHosts"] as? [String] {
                excludedHosts.append(contentsOf: hosts)
            }
        }
        
        logger.info("Proxy configured: \(self.proxyHost):\(self.proxyPort)")
        logger.info("Target apps: \(self.targetApps.isEmpty ? "All" : self.targetApps.joined(separator: ", "))")
        
        completionHandler(nil)
    }
    
    override func stopProxy(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        logger.info("Stopping Sentinel Transparent Proxy, reason: \(String(describing: reason))")
        completionHandler()
    }
    
    // MARK: - Flow Handling
    
    override func handleNewFlow(_ flow: NEAppProxyFlow) -> Bool {
        guard let tcpFlow = flow as? NEAppProxyTCPFlow else {
            logger.debug("Ignoring non-TCP flow")
            return false
        }
        
        // 获取流信息
        let appIdentifier = flow.metaData.sourceAppSigningIdentifier
        let remoteEndpoint = tcpFlow.remoteEndpoint as? NWHostEndpoint
        
        logger.debug("New flow from app: \(appIdentifier)")
        
        // 检查是否应该代理此流
        guard shouldProxyFlow(appIdentifier: appIdentifier, endpoint: remoteEndpoint) else {
            logger.debug("Flow bypassed for app: \(appIdentifier)")
            return false
        }
        
        logger.info("Proxying flow from \(appIdentifier) to \(remoteEndpoint?.hostname ?? "unknown"):\(remoteEndpoint?.port ?? "unknown")")
        
        // 创建到本地代理的连接
        let proxyEndpoint = NWHostEndpoint(hostname: proxyHost, port: String(proxyPort))
        
        tcpFlow.open(withLocalEndpoint: nil) { [weak self] error in
            if let error = error {
                self?.logger.error("Failed to open flow: \(error.localizedDescription)")
                tcpFlow.closeReadWithError(error)
                tcpFlow.closeWriteWithError(error)
                return
            }
            
            self?.handleTCPFlow(tcpFlow, proxyEndpoint: proxyEndpoint)
        }
        
        return true
    }
    
    // MARK: - Private Methods
    
    /// 判断是否应该代理此流
    private func shouldProxyFlow(appIdentifier: String, endpoint: NWHostEndpoint?) -> Bool {
        // 检查排除的应用
        if excludedApps.contains(where: { appIdentifier.contains($0) }) {
            return false
        }
        
        // 检查目标应用（如果配置了）
        if !targetApps.isEmpty {
            if !targetApps.contains(where: { appIdentifier.contains($0) }) {
                return false
            }
        }
        
        // 检查排除的主机
        if let hostname = endpoint?.hostname {
            if excludedHosts.contains(where: { hostname.contains($0) }) {
                return false
            }
        }
        
        // 检查目标端口（如果配置了）
        if !targetPorts.isEmpty {
            if let portStr = endpoint?.port, let port = UInt16(portStr) {
                if !targetPorts.contains(port) {
                    return false
                }
            }
        }
        
        return true
    }
    
    /// 处理 TCP 流，转发到代理服务器
    private func handleTCPFlow(_ flow: NEAppProxyTCPFlow, proxyEndpoint: NWHostEndpoint) {
        // 创建代理连接
        let connection = ProxyConnection(flow: flow, proxyEndpoint: proxyEndpoint, logger: logger)
        connection.start()
    }
}

// MARK: - ProxyConnection

/// 管理单个代理连接
class ProxyConnection {
    
    private let flow: NEAppProxyTCPFlow
    private let proxyEndpoint: NWHostEndpoint
    private let logger: Logger
    
    private var proxyConnection: NWTCPConnection?
    private var isConnected = false
    
    init(flow: NEAppProxyTCPFlow, proxyEndpoint: NWHostEndpoint, logger: Logger) {
        self.flow = flow
        self.proxyEndpoint = proxyEndpoint
        self.logger = logger
    }
    
    func start() {
        // 连接到代理服务器
        proxyConnection = NWTCPConnection(upgradeFor: flow)
        
        proxyConnection?.observeStateUpdate { [weak self] state in
            self?.handleConnectionStateChange(state)
        }
        
        // 开始读取客户端数据
        readFromClient()
    }
    
    private func handleConnectionStateChange(_ state: NWTCPConnectionState) {
        switch state {
        case .connected:
            logger.debug("Connected to proxy server")
            isConnected = true
            // 开始读取代理服务器响应
            readFromProxy()
            
        case .disconnected:
            logger.debug("Disconnected from proxy server")
            close()
            
        case .cancelled:
            logger.debug("Connection cancelled")
            close()
            
        case .failed(let error):
            logger.error("Connection failed: \(error.localizedDescription)")
            close()
            
        default:
            break
        }
    }
    
    /// 从客户端读取数据并转发到代理
    private func readFromClient() {
        flow.readData { [weak self] data, error in
            guard let self = self else { return }
            
            if let error = error {
                self.logger.error("Error reading from client: \(error.localizedDescription)")
                self.close()
                return
            }
            
            guard let data = data, !data.isEmpty else {
                // 客户端关闭了写入
                self.proxyConnection?.writeClose()
                return
            }
            
            // 转发数据到代理服务器
            self.proxyConnection?.write(data) { error in
                if let error = error {
                    self.logger.error("Error writing to proxy: \(error.localizedDescription)")
                    self.close()
                    return
                }
                
                // 继续读取
                self.readFromClient()
            }
        }
    }
    
    /// 从代理服务器读取响应并转发回客户端
    private func readFromProxy() {
        proxyConnection?.readMinimumLength(1, maximumLength: 65536) { [weak self] data, error in
            guard let self = self else { return }
            
            if let error = error {
                self.logger.error("Error reading from proxy: \(error.localizedDescription)")
                self.close()
                return
            }
            
            guard let data = data, !data.isEmpty else {
                // 代理服务器关闭了连接
                self.flow.closeWriteWithError(nil)
                return
            }
            
            // 转发响应回客户端
            self.flow.write(data) { error in
                if let error = error {
                    self.logger.error("Error writing to client: \(error.localizedDescription)")
                    self.close()
                    return
                }
                
                // 继续读取
                self.readFromProxy()
            }
        }
    }
    
    private func close() {
        flow.closeReadWithError(nil)
        flow.closeWriteWithError(nil)
        proxyConnection?.cancel()
    }
}

