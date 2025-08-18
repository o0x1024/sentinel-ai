-- 创建MCP服务器状态表
CREATE TABLE IF NOT EXISTS mcp_server_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_name TEXT UNIQUE NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 0,
    last_started_at TIMESTAMP,
    last_stopped_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_mcp_server_state_name ON mcp_server_state(server_name);
CREATE INDEX IF NOT EXISTS idx_mcp_server_state_enabled ON mcp_server_state(enabled);

-- 插入默认的内置服务器状态
INSERT OR IGNORE INTO mcp_server_state (server_name, enabled) VALUES ('builtin_security_tools', 0);
