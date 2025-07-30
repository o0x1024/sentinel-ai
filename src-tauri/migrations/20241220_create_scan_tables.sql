-- 创建扫描会话表
CREATE TABLE IF NOT EXISTS scan_sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    target TEXT NOT NULL,
    scan_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    config TEXT NOT NULL DEFAULT '{}',
    progress REAL NOT NULL DEFAULT 0.0,
    current_stage TEXT,
    total_stages INTEGER NOT NULL DEFAULT 0,
    completed_stages INTEGER NOT NULL DEFAULT 0,
    results_summary TEXT,
    error_message TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    completed_at DATETIME,
    created_by TEXT NOT NULL
);

-- 创建扫描阶段表
CREATE TABLE IF NOT EXISTS scan_stages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    stage_name TEXT NOT NULL,
    stage_order INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    tool_name TEXT NOT NULL,
    config TEXT NOT NULL DEFAULT '{}',
    results TEXT,
    error_message TEXT,
    started_at DATETIME,
    completed_at DATETIME,
    duration_ms INTEGER,
    FOREIGN KEY (session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- 创建索引以提高查询性能
CREATE INDEX IF NOT EXISTS idx_scan_sessions_status ON scan_sessions(status);
CREATE INDEX IF NOT EXISTS idx_scan_sessions_created_at ON scan_sessions(created_at);
CREATE INDEX IF NOT EXISTS idx_scan_sessions_created_by ON scan_sessions(created_by);
CREATE INDEX IF NOT EXISTS idx_scan_stages_session_id ON scan_stages(session_id);
CREATE INDEX IF NOT EXISTS idx_scan_stages_status ON scan_stages(status);
CREATE INDEX IF NOT EXISTS idx_scan_stages_order ON scan_stages(session_id, stage_order);