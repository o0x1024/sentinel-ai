-- Plan-and-Execute架构数据库迁移
-- 创建时间: 2024-12-20
-- 描述: 为Plan-and-Execute架构添加必要的数据表

-- 执行计划表
CREATE TABLE IF NOT EXISTS execution_plans (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    estimated_duration INTEGER, -- 预估执行时间（秒）
    created_at INTEGER NOT NULL, -- Unix时间戳
    metadata TEXT, -- JSON格式的元数据
    UNIQUE(id)
);

-- 计划步骤表
CREATE TABLE IF NOT EXISTS plan_steps (
    id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    step_type TEXT NOT NULL, -- 步骤类型: ToolCall, DataProcessing, Conditional, Parallel, Wait, ManualConfirmation
    estimated_duration INTEGER, -- 预估执行时间（秒）
    tool_config TEXT, -- JSON格式的工具配置
    retry_config TEXT, -- JSON格式的重试配置
    parameters TEXT, -- JSON格式的参数
    created_at INTEGER NOT NULL,
    FOREIGN KEY (plan_id) REFERENCES execution_plans(id) ON DELETE CASCADE
);

-- 步骤依赖关系表
CREATE TABLE IF NOT EXISTS step_dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    step_id TEXT NOT NULL,
    depends_on_step_id TEXT NOT NULL,
    dependency_type TEXT DEFAULT 'sequential', -- 依赖类型: sequential, conditional, resource
    created_at INTEGER NOT NULL,
    FOREIGN KEY (step_id) REFERENCES plan_steps(id) ON DELETE CASCADE,
    FOREIGN KEY (depends_on_step_id) REFERENCES plan_steps(id) ON DELETE CASCADE,
    UNIQUE(step_id, depends_on_step_id)
);

-- 执行会话表
CREATE TABLE IF NOT EXISTS execution_sessions (
    id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL,
    status TEXT NOT NULL, -- 执行状态: Pending, Running, Paused, Completed, Failed, Cancelled
    started_at INTEGER,
    completed_at INTEGER,
    context TEXT, -- JSON格式的执行上下文
    error_message TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (plan_id) REFERENCES execution_plans(id) ON DELETE CASCADE
);

-- 步骤执行结果表
CREATE TABLE IF NOT EXISTS step_execution_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    step_id TEXT NOT NULL,
    status TEXT NOT NULL, -- 执行状态: Pending, Running, Completed, Failed, Skipped, Retrying
    started_at INTEGER,
    completed_at INTEGER,
    result_data TEXT, -- JSON格式的结果数据
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    logs TEXT, -- JSON格式的日志数组
    metrics TEXT, -- JSON格式的执行指标
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES execution_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (step_id) REFERENCES plan_steps(id) ON DELETE CASCADE
);

-- 执行指标表
CREATE TABLE IF NOT EXISTS execution_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    step_id TEXT,
    execution_time_ms INTEGER NOT NULL,
    memory_usage_mb INTEGER,
    cpu_usage_percent REAL,
    network_io_kb INTEGER,
    disk_io_kb INTEGER,
    success_count INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    retry_count INTEGER DEFAULT 0,
    recorded_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES execution_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (step_id) REFERENCES plan_steps(id) ON DELETE CASCADE
);

-- 异常检测表
CREATE TABLE IF NOT EXISTS execution_anomalies (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    step_id TEXT,
    anomaly_type TEXT NOT NULL, -- 异常类型: ExecutionTime, ResourceUsage, ErrorRate, Performance
    severity TEXT NOT NULL, -- 严重程度: Low, Medium, High, Critical
    description TEXT NOT NULL,
    detected_at INTEGER NOT NULL,
    value REAL NOT NULL,
    threshold_value REAL NOT NULL,
    metadata TEXT, -- JSON格式的元数据
    FOREIGN KEY (session_id) REFERENCES execution_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (step_id) REFERENCES plan_steps(id) ON DELETE CASCADE
);

-- 执行反馈表
CREATE TABLE IF NOT EXISTS execution_feedback (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    step_id TEXT,
    feedback_type TEXT NOT NULL, -- 反馈类型: Success, Error, Warning, Performance, User
    content TEXT NOT NULL,
    severity TEXT NOT NULL, -- 严重程度: Low, Medium, High, Critical
    created_at INTEGER NOT NULL,
    metadata TEXT, -- JSON格式的元数据
    FOREIGN KEY (session_id) REFERENCES execution_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (step_id) REFERENCES plan_steps(id) ON DELETE CASCADE
);

-- 计划修订记录表
CREATE TABLE IF NOT EXISTS plan_revisions (
    id TEXT PRIMARY KEY,
    original_plan_id TEXT NOT NULL,
    revised_plan_id TEXT NOT NULL,
    revision_reason TEXT NOT NULL,
    revision_type TEXT NOT NULL, -- 修订类型: StepModification, DependencyChange, ParameterUpdate, ErrorCorrection
    changes TEXT NOT NULL, -- JSON格式的变更详情
    created_at INTEGER NOT NULL,
    FOREIGN KEY (original_plan_id) REFERENCES execution_plans(id) ON DELETE CASCADE,
    FOREIGN KEY (revised_plan_id) REFERENCES execution_plans(id) ON DELETE CASCADE
);

-- 创建索引以提高查询性能

-- 执行计划索引
CREATE INDEX IF NOT EXISTS idx_execution_plans_created_at ON execution_plans(created_at);
CREATE INDEX IF NOT EXISTS idx_execution_plans_name ON execution_plans(name);

-- 计划步骤索引
CREATE INDEX IF NOT EXISTS idx_plan_steps_plan_id ON plan_steps(plan_id);
CREATE INDEX IF NOT EXISTS idx_plan_steps_step_type ON plan_steps(step_type);
CREATE INDEX IF NOT EXISTS idx_plan_steps_created_at ON plan_steps(created_at);

-- 步骤依赖关系索引
CREATE INDEX IF NOT EXISTS idx_step_dependencies_step_id ON step_dependencies(step_id);
CREATE INDEX IF NOT EXISTS idx_step_dependencies_depends_on ON step_dependencies(depends_on_step_id);

-- 执行会话索引
CREATE INDEX IF NOT EXISTS idx_execution_sessions_plan_id ON execution_sessions(plan_id);
CREATE INDEX IF NOT EXISTS idx_execution_sessions_status ON execution_sessions(status);
CREATE INDEX IF NOT EXISTS idx_execution_sessions_started_at ON execution_sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_execution_sessions_completed_at ON execution_sessions(completed_at);

-- 步骤执行结果索引
CREATE INDEX IF NOT EXISTS idx_step_execution_results_session_id ON step_execution_results(session_id);
CREATE INDEX IF NOT EXISTS idx_step_execution_results_step_id ON step_execution_results(step_id);
CREATE INDEX IF NOT EXISTS idx_step_execution_results_status ON step_execution_results(status);
CREATE INDEX IF NOT EXISTS idx_step_execution_results_started_at ON step_execution_results(started_at);

-- 执行指标索引
CREATE INDEX IF NOT EXISTS idx_execution_metrics_session_id ON execution_metrics(session_id);
CREATE INDEX IF NOT EXISTS idx_execution_metrics_step_id ON execution_metrics(step_id);
CREATE INDEX IF NOT EXISTS idx_execution_metrics_recorded_at ON execution_metrics(recorded_at);

-- 异常检测索引
CREATE INDEX IF NOT EXISTS idx_execution_anomalies_session_id ON execution_anomalies(session_id);
CREATE INDEX IF NOT EXISTS idx_execution_anomalies_step_id ON execution_anomalies(step_id);
CREATE INDEX IF NOT EXISTS idx_execution_anomalies_anomaly_type ON execution_anomalies(anomaly_type);
CREATE INDEX IF NOT EXISTS idx_execution_anomalies_severity ON execution_anomalies(severity);
CREATE INDEX IF NOT EXISTS idx_execution_anomalies_detected_at ON execution_anomalies(detected_at);

-- 执行反馈索引
CREATE INDEX IF NOT EXISTS idx_execution_feedback_session_id ON execution_feedback(session_id);
CREATE INDEX IF NOT EXISTS idx_execution_feedback_step_id ON execution_feedback(step_id);
CREATE INDEX IF NOT EXISTS idx_execution_feedback_feedback_type ON execution_feedback(feedback_type);
CREATE INDEX IF NOT EXISTS idx_execution_feedback_severity ON execution_feedback(severity);
CREATE INDEX IF NOT EXISTS idx_execution_feedback_created_at ON execution_feedback(created_at);

-- 计划修订记录索引
CREATE INDEX IF NOT EXISTS idx_plan_revisions_original_plan_id ON plan_revisions(original_plan_id);
CREATE INDEX IF NOT EXISTS idx_plan_revisions_revised_plan_id ON plan_revisions(revised_plan_id);
CREATE INDEX IF NOT EXISTS idx_plan_revisions_revision_type ON plan_revisions(revision_type);
CREATE INDEX IF NOT EXISTS idx_plan_revisions_created_at ON plan_revisions(created_at);

-- 创建视图以简化常用查询

-- 执行会话详情视图
CREATE VIEW IF NOT EXISTS execution_session_details AS
SELECT 
    es.id as session_id,
    es.status as session_status,
    es.started_at,
    es.completed_at,
    ep.id as plan_id,
    ep.name as plan_name,
    ep.description as plan_description,
    COUNT(ps.id) as total_steps,
    COUNT(CASE WHEN ser.status = 'Completed' THEN 1 END) as completed_steps,
    COUNT(CASE WHEN ser.status = 'Failed' THEN 1 END) as failed_steps,
    COUNT(CASE WHEN ser.status = 'Running' THEN 1 END) as running_steps
FROM execution_sessions es
JOIN execution_plans ep ON es.plan_id = ep.id
LEFT JOIN plan_steps ps ON ep.id = ps.plan_id
LEFT JOIN step_execution_results ser ON es.id = ser.session_id AND ps.id = ser.step_id
GROUP BY es.id, ep.id;

-- 步骤执行统计视图
CREATE VIEW IF NOT EXISTS step_execution_stats AS
SELECT 
    ps.id as step_id,
    ps.name as step_name,
    ps.step_type,
    COUNT(ser.id) as execution_count,
    COUNT(CASE WHEN ser.status = 'Completed' THEN 1 END) as success_count,
    COUNT(CASE WHEN ser.status = 'Failed' THEN 1 END) as failure_count,
    AVG(CASE WHEN ser.status = 'Completed' THEN 
        (ser.completed_at - ser.started_at) END) as avg_execution_time,
    AVG(ser.retry_count) as avg_retry_count
FROM plan_steps ps
LEFT JOIN step_execution_results ser ON ps.id = ser.step_id
GROUP BY ps.id;

-- 异常统计视图
CREATE VIEW IF NOT EXISTS anomaly_stats AS
SELECT 
    anomaly_type,
    severity,
    COUNT(*) as occurrence_count,
    AVG(value) as avg_value,
    MAX(value) as max_value,
    MIN(value) as min_value,
    DATE(detected_at, 'unixepoch') as detection_date
FROM execution_anomalies
GROUP BY anomaly_type, severity, DATE(detected_at, 'unixepoch')
ORDER BY detection_date DESC, occurrence_count DESC;

-- 性能趋势视图
CREATE VIEW IF NOT EXISTS performance_trends AS
SELECT 
    DATE(recorded_at, 'unixepoch') as date,
    AVG(execution_time_ms) as avg_execution_time,
    AVG(memory_usage_mb) as avg_memory_usage,
    AVG(cpu_usage_percent) as avg_cpu_usage,
    SUM(success_count) as total_success,
    SUM(error_count) as total_errors,
    COUNT(DISTINCT session_id) as session_count
FROM execution_metrics
GROUP BY DATE(recorded_at, 'unixepoch')
ORDER BY date DESC;

-- 插入一些示例配置数据（可选）

-- 示例执行计划
INSERT OR IGNORE INTO execution_plans (id, name, description, estimated_duration, created_at, metadata)
VALUES (
    'example-plan-001',
    '示例安全扫描计划',
    '这是一个示例的安全扫描执行计划，包含端口扫描、漏洞检测等步骤',
    1800, -- 30分钟
    strftime('%s', 'now'),
    '{
        "priority": "high",
        "category": "security_scan",
        "tags": ["example", "security", "scan"],
        "created_by": "system",
        "version": "1.0"
    }'
);

-- 示例计划步骤
INSERT OR IGNORE INTO plan_steps (id, plan_id, name, description, step_type, estimated_duration, tool_config, retry_config, parameters, created_at)
VALUES 
(
    'step-001',
    'example-plan-001',
    '端口扫描',
    '对目标主机进行端口扫描',
    'ToolCall',
    300, -- 5分钟
    '{
        "tool_name": "nmap",
        "tool_args": {
            "target": "${target_host}",
            "ports": "1-1000",
            "scan_type": "tcp"
        },
        "timeout": 300
    }',
    '{
        "max_retries": 3,
        "retry_interval": 10,
        "backoff_strategy": "exponential",
        "retry_conditions": ["NetworkError", "Timeout"]
    }',
    '{
        "target_host": "127.0.0.1",
        "scan_intensity": "normal"
    }',
    strftime('%s', 'now')
),
(
    'step-002',
    'example-plan-001',
    '漏洞扫描',
    '基于端口扫描结果进行漏洞检测',
    'ToolCall',
    600, -- 10分钟
    '{
        "tool_name": "vulnerability_scanner",
        "tool_args": {
            "target": "${target_host}",
            "ports": "${open_ports}",
            "scan_depth": "medium"
        },
        "timeout": 600
    }',
    '{
        "max_retries": 2,
        "retry_interval": 30,
        "backoff_strategy": "linear",
        "retry_conditions": ["NetworkError"]
    }',
    '{
        "scan_depth": "medium",
        "check_cves": true
    }',
    strftime('%s', 'now')
),
(
    'step-003',
    'example-plan-001',
    '报告生成',
    '生成扫描结果报告',
    'DataProcessing',
    120, -- 2分钟
    '{
        "tool_name": "report_generator",
        "tool_args": {
            "format": "html",
            "include_details": true
        },
        "timeout": 120
    }',
    '{
        "max_retries": 1,
        "retry_interval": 5,
        "backoff_strategy": "fixed",
        "retry_conditions": []
    }',
    '{
        "output_format": "html",
        "include_charts": true
    }',
    strftime('%s', 'now')
);

-- 示例步骤依赖关系
INSERT OR IGNORE INTO step_dependencies (step_id, depends_on_step_id, dependency_type, created_at)
VALUES 
('step-002', 'step-001', 'sequential', strftime('%s', 'now')),
('step-003', 'step-002', 'sequential', strftime('%s', 'now'));

-- 提交事务
COMMIT;