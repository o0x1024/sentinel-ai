# 代理配置持久化实现

## 概述

实现了代理配置的数据库持久化功能，使得用户修改的配置能够保存到数据库并在重启后恢复。

## 实现内容

### 1. 数据库表结构

在 `sentinel-passive/src/database.rs` 中添加了 `proxy_config` 表：

```sql
CREATE TABLE IF NOT EXISTS proxy_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

**表结构说明：**
- `key`: 配置项的键（主键）
- `value`: 配置项的值（JSON 格式）
- `updated_at`: 最后更新时间

### 2. 数据库服务方法

在 `PassiveDatabaseService` 中添加了三个配置管理方法：

#### `save_config`
保存或更新配置项：

```rust
pub async fn save_config(&self, key: &str, value: &str) -> Result<()>
```

- 使用 `INSERT ... ON CONFLICT DO UPDATE` 实现 upsert 操作
- 自动更新 `updated_at` 时间戳

#### `load_config`
加载配置项：

```rust
pub async fn load_config(&self, key: &str) -> Result<Option<String>>
```

- 返回 `Option<String>`，如果配置不存在则返回 `None`

#### `delete_config`
删除配置项：

```rust
pub async fn delete_config(&self, key: &str) -> Result<()>
```

### 3. Tauri 命令实现

#### `save_proxy_config`

**位置**: `src-tauri/src/commands/passive_scan_commands.rs`

**功能**: 保存代理配置到数据库

**实现流程**:
1. 获取数据库服务
2. 将 `ProxyConfig` 序列化为 JSON
3. 调用 `db.save_config("proxy_config", &config_json)` 保存到数据库
4. 返回成功响应

**代码**:
```rust
#[tauri::command]
pub async fn save_proxy_config(
    state: State<'_, PassiveScanState>,
    config: ProxyConfig,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Saving proxy configuration: {:?}", config);
    
    let db = state.get_db_service().await.map_err(|e| {
        tracing::error!("Failed to get database service: {}", e);
        format!("Failed to get database service: {}", e)
    })?;
    
    let config_json = serde_json::to_string(&config).map_err(|e| {
        tracing::error!("Failed to serialize config: {}", e);
        format!("Failed to serialize config: {}", e)
    })?;
    
    db.save_config("proxy_config", &config_json).await.map_err(|e| {
        tracing::error!("Failed to save config to database: {}", e);
        format!("Failed to save config: {}", e)
    })?;
    
    tracing::info!("Proxy configuration saved successfully");
    Ok(CommandResponse::ok(()))
}
```

#### `get_proxy_config`

**位置**: `src-tauri/src/commands/passive_scan_commands.rs`

**功能**: 从数据库加载代理配置

**实现流程**:
1. 获取数据库服务
2. 调用 `db.load_config("proxy_config")` 从数据库加载
3. 如果配置存在，反序列化 JSON 为 `ProxyConfig`
4. 如果配置不存在或反序列化失败，使用默认配置
5. 返回配置

**代码**:
```rust
#[tauri::command]
pub async fn get_proxy_config(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<ProxyConfig>, String> {
    tracing::info!("Getting proxy configuration");
    
    let db = state.get_db_service().await.map_err(|e| {
        tracing::error!("Failed to get database service: {}", e);
        format!("Failed to get database service: {}", e)
    })?;
    
    let config = match db.load_config("proxy_config").await {
        Ok(Some(config_json)) => {
            match serde_json::from_str::<ProxyConfig>(&config_json) {
                Ok(config) => {
                    tracing::info!("Loaded proxy configuration from database: {:?}", config);
                    config
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize config, using default: {}", e);
                    ProxyConfig::default()
                }
            }
        }
        Ok(None) => {
            tracing::info!("No saved configuration found, using default");
            ProxyConfig::default()
        }
        Err(e) => {
            tracing::warn!("Failed to load config from database, using default: {}", e);
            ProxyConfig::default()
        }
    };
    
    Ok(CommandResponse::ok(config))
}
```

## 配置结构

`ProxyConfig` 结构定义在 `sentinel-passive/src/proxy.rs`：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 起始端口（默认 4201）
    pub start_port: u16,
    /// 最大端口尝试次数（默认 10）
    pub max_port_attempts: u16,
    /// HTTPS MITM 是否启用（默认 true）
    pub mitm_enabled: bool,
    /// 请求体大小限制（字节，默认 2MB）
    pub max_request_body_size: usize,
    /// 响应体大小限制（字节，默认 2MB）
    pub max_response_body_size: usize,
    /// 对同一域名发生握手/证书错误的次数阈值，超过后自动绕过 MITM
    pub mitm_bypass_fail_threshold: u32,
}
```

**默认值**:
```rust
impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            start_port: 4201,
            max_port_attempts: 10,
            mitm_enabled: true,
            max_request_body_size: 2 * 1024 * 1024,
            max_response_body_size: 2 * 1024 * 1024,
            mitm_bypass_fail_threshold: 3,
        }
    }
}
```

## 工作流程

### 保存配置流程

1. **前端**: 用户修改配置（如端口从 8080 改为 8081）
2. **前端**: 自动保存触发（1秒防抖）
3. **前端**: 调用 `invoke('save_proxy_config', { config })`
4. **后端**: `save_proxy_config` 命令接收配置
5. **后端**: 序列化为 JSON
6. **后端**: 保存到数据库 `proxy_config` 表
7. **后端**: 返回成功响应

### 加载配置流程

1. **前端**: 页面加载或刷新
2. **前端**: 调用 `invoke('get_proxy_config')`
3. **后端**: `get_proxy_config` 命令执行
4. **后端**: 从数据库加载配置
5. **后端**: 反序列化 JSON 为 `ProxyConfig`
6. **后端**: 返回配置给前端
7. **前端**: 更新界面显示

## 错误处理

### 保存失败
- 数据库连接失败：返回错误信息
- 序列化失败：返回错误信息
- 保存失败：返回错误信息

### 加载失败
- 数据库连接失败：使用默认配置
- 配置不存在：使用默认配置
- 反序列化失败：使用默认配置

**优点**: 即使数据库出现问题，系统仍然可以使用默认配置正常运行

## 数据库位置

配置保存在被动扫描数据库中：
```
/Users/a1024/Library/Application Support/sentinel-ai/sentinel-passive.db
```

可以使用 SQLite 工具查看：
```bash
sqlite3 "/Users/a1024/Library/Application Support/sentinel-ai/sentinel-passive.db"
```

查询配置：
```sql
SELECT * FROM proxy_config;
```

## 测试建议

### 1. 基本保存和加载
```bash
# 1. 修改端口从 8080 到 8081
# 2. 等待 1 秒自动保存
# 3. 刷新页面
# 4. 验证端口仍然是 8081
```

### 2. 数据库验证
```bash
# 查看数据库中的配置
sqlite3 "/Users/a1024/Library/Application Support/sentinel-ai/sentinel-passive.db" \
  "SELECT key, value FROM proxy_config WHERE key = 'proxy_config';"
```

### 3. 默认配置测试
```bash
# 1. 删除数据库中的配置
sqlite3 "/Users/a1024/Library/Application Support/sentinel-ai/sentinel-passive.db" \
  "DELETE FROM proxy_config WHERE key = 'proxy_config';"

# 2. 刷新页面
# 3. 验证显示默认配置（4201 端口）
```

### 4. 多字段修改测试
```bash
# 1. 修改多个配置项：
#    - 端口: 8080 -> 8081
#    - MITM: true -> false
#    - 请求体大小: 2MB -> 5MB
# 2. 等待自动保存
# 3. 刷新页面
# 4. 验证所有修改都已保存
```

## 日志追踪

保存配置时的日志：
```
[INFO] Saving proxy configuration: ProxyConfig { start_port: 8081, ... }
[INFO] Saved config: proxy_config = {"start_port":8081,...}
[INFO] Proxy configuration saved successfully
```

加载配置时的日志：
```
[INFO] Getting proxy configuration
[INFO] Loaded proxy configuration from database: ProxyConfig { start_port: 8081, ... }
```

## 相关文件

- `src-tauri/sentinel-passive/src/database.rs`: 数据库服务和配置管理方法
- `src-tauri/sentinel-passive/src/proxy.rs`: `ProxyConfig` 结构定义
- `src-tauri/src/commands/passive_scan_commands.rs`: Tauri 命令实现
- `src/components/ProxyConfiguration.vue`: 前端配置界面

## 后续优化建议

1. **配置版本控制**: 添加配置版本号，支持配置迁移
2. **配置验证**: 在保存前验证配置的有效性
3. **配置备份**: 定期备份配置到文件
4. **配置导入导出**: 支持配置的导入和导出功能
5. **配置历史**: 记录配置修改历史

