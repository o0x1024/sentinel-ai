-- 创建资产类型枚举表
CREATE TABLE IF NOT EXISTS asset_types (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 插入预定义的资产类型
INSERT OR IGNORE INTO asset_types (name, description) VALUES
('domain', '域名资产'),
('subdomain', '子域名资产'),
('ip', 'IP地址资产'),
('port', '端口资产'),
('service', '服务资产'),
('website', '网站资产'),
('api', 'API接口资产'),
('certificate', '证书资产'),
('fingerprint', '指纹资产'),
('vulnerability', '漏洞资产'),
('technology', '技术栈资产'),
('email', '邮箱资产'),
('phone', '电话资产'),
('file', '文件资产'),
('directory', '目录资产');

-- 创建资产表
CREATE TABLE IF NOT EXISTS assets (
    id TEXT PRIMARY KEY,
    asset_type TEXT NOT NULL,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    description TEXT,
    confidence REAL NOT NULL DEFAULT 1.0, -- 置信度 0.0-1.0
    status TEXT NOT NULL DEFAULT 'active', -- active, inactive, verified, unverified
    source TEXT, -- 来源：扫描工具名称或手动添加
    source_scan_id TEXT, -- 来源扫描ID
    metadata TEXT NOT NULL DEFAULT '{}', -- JSON格式的元数据
    tags TEXT NOT NULL DEFAULT '[]', -- JSON格式的标签数组
    risk_level TEXT NOT NULL DEFAULT 'unknown', -- low, medium, high, critical, unknown
    last_seen DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    first_seen DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT NOT NULL,
    FOREIGN KEY (asset_type) REFERENCES asset_types(name)
);

-- 创建资产关系表（用于表示资产之间的关联）
CREATE TABLE IF NOT EXISTS asset_relationships (
    id TEXT PRIMARY KEY,
    source_asset_id TEXT NOT NULL,
    target_asset_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL, -- belongs_to, contains, connects_to, depends_on, etc.
    description TEXT,
    confidence REAL NOT NULL DEFAULT 1.0,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT NOT NULL,
    FOREIGN KEY (source_asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    FOREIGN KEY (target_asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    UNIQUE(source_asset_id, target_asset_id, relationship_type)
);

-- 创建资产历史记录表
CREATE TABLE IF NOT EXISTS asset_history (
    id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL,
    action TEXT NOT NULL, -- created, updated, deleted, verified, etc.
    old_value TEXT,
    new_value TEXT,
    changed_fields TEXT, -- JSON格式的变更字段
    reason TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT NOT NULL,
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE
);

-- 创建资产标签表
CREATE TABLE IF NOT EXISTS asset_tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL DEFAULT '#6B7280',
    description TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT NOT NULL
);

-- 插入预定义标签
INSERT OR IGNORE INTO asset_tags (name, color, description, created_by) VALUES
('high-value', '#EF4444', '高价值资产', 'system'),
('external', '#F59E0B', '外部资产', 'system'),
('internal', '#10B981', '内部资产', 'system'),
('production', '#DC2626', '生产环境', 'system'),
('staging', '#F59E0B', '测试环境', 'system'),
('development', '#3B82F6', '开发环境', 'system'),
('deprecated', '#6B7280', '已废弃', 'system'),
('monitored', '#8B5CF6', '已监控', 'system'),
('unmonitored', '#EF4444', '未监控', 'system');

-- 创建资产搜索视图
CREATE VIEW IF NOT EXISTS asset_search_view AS
SELECT 
    a.id,
    a.asset_type,
    a.name,
    a.value,
    a.description,
    a.confidence,
    a.status,
    a.source,
    a.risk_level,
    a.last_seen,
    a.first_seen,
    a.created_at,
    a.updated_at,
    at.description as type_description,
    COUNT(ar1.target_asset_id) as outgoing_relationships,
    COUNT(ar2.source_asset_id) as incoming_relationships
FROM assets a
LEFT JOIN asset_types at ON a.asset_type = at.name
LEFT JOIN asset_relationships ar1 ON a.id = ar1.source_asset_id
LEFT JOIN asset_relationships ar2 ON a.id = ar2.target_asset_id
GROUP BY a.id;

-- 创建索引以提高查询性能
CREATE INDEX IF NOT EXISTS idx_assets_type ON assets(asset_type);
CREATE INDEX IF NOT EXISTS idx_assets_name ON assets(name);
CREATE INDEX IF NOT EXISTS idx_assets_value ON assets(value);
CREATE INDEX IF NOT EXISTS idx_assets_status ON assets(status);
CREATE INDEX IF NOT EXISTS idx_assets_source ON assets(source);
CREATE INDEX IF NOT EXISTS idx_assets_risk_level ON assets(risk_level);
CREATE INDEX IF NOT EXISTS idx_assets_last_seen ON assets(last_seen);
CREATE INDEX IF NOT EXISTS idx_assets_created_at ON assets(created_at);
CREATE INDEX IF NOT EXISTS idx_assets_created_by ON assets(created_by);
CREATE INDEX IF NOT EXISTS idx_assets_source_scan_id ON assets(source_scan_id);

CREATE INDEX IF NOT EXISTS idx_asset_relationships_source ON asset_relationships(source_asset_id);
CREATE INDEX IF NOT EXISTS idx_asset_relationships_target ON asset_relationships(target_asset_id);
CREATE INDEX IF NOT EXISTS idx_asset_relationships_type ON asset_relationships(relationship_type);

CREATE INDEX IF NOT EXISTS idx_asset_history_asset_id ON asset_history(asset_id);
CREATE INDEX IF NOT EXISTS idx_asset_history_action ON asset_history(action);
CREATE INDEX IF NOT EXISTS idx_asset_history_created_at ON asset_history(created_at);

-- 创建触发器来自动更新 updated_at 字段
CREATE TRIGGER IF NOT EXISTS update_assets_updated_at
    AFTER UPDATE ON assets
    FOR EACH ROW
BEGIN
    UPDATE assets SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- 创建触发器来记录资产变更历史
CREATE TRIGGER IF NOT EXISTS asset_history_trigger
    AFTER UPDATE ON assets
    FOR EACH ROW
    WHEN OLD.value != NEW.value OR OLD.status != NEW.status OR OLD.risk_level != NEW.risk_level
BEGIN
    INSERT INTO asset_history (
        id, asset_id, action, old_value, new_value, 
        changed_fields, created_by
    ) VALUES (
        lower(hex(randomblob(16))),
        NEW.id,
        'updated',
        OLD.value,
        NEW.value,
        json_object(
            'value', json_array(OLD.value, NEW.value),
            'status', json_array(OLD.status, NEW.status),
            'risk_level', json_array(OLD.risk_level, NEW.risk_level)
        ),
        NEW.created_by
    );
END;