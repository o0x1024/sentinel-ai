-- Plan-and-Execute 增强架构数据库迁移
-- 创建时间: 2024-12-23
-- 描述: 为Plan-and-Execute架构添加Replanner（重规划器）和Memory（记忆模块）功能

BEGIN TRANSACTION;

-- ============================================================================
-- Replanner（重规划器）相关表
-- ============================================================================

-- 重规划历史表
CREATE TABLE IF NOT EXISTS replan_history (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    original_plan_id TEXT NOT NULL,
    revised_plan_id TEXT NOT NULL,
    replan_reason TEXT NOT NULL,
    strategy_type TEXT NOT NULL, -- CompleteReplan, PartialModification, ParameterOptimization, StepReordering, ResourceReallocation, AlternativeSwitch
    confidence REAL NOT NULL DEFAULT 0.0,
    success BOOLEAN,
    failure_analysis TEXT, -- JSON格式的失败分析
    modifications TEXT, -- JSON格式的修改详情
    expected_improvements TEXT, -- JSON格式的预期改进
    actual_improvements TEXT, -- JSON格式的实际改进（执行后更新）
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    completed_at INTEGER,
    FOREIGN KEY (session_id) REFERENCES execution_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (original_plan_id) REFERENCES execution_plans(id) ON DELETE CASCADE,
    FOREIGN KEY (revised_plan_id) REFERENCES execution_plans(id) ON DELETE CASCADE
);

-- 重规划触发器表
CREATE TABLE IF NOT EXISTS replan_triggers (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    trigger_type TEXT NOT NULL, -- FailureRate, ExecutionTime, ResourceUsage, ErrorPattern, UserRequest
    trigger_condition TEXT NOT NULL, -- JSON格式的触发条件
    threshold_value REAL,
    actual_value REAL,
    severity TEXT NOT NULL, -- Low, Medium, High, Critical
    triggered_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    resolved_at INTEGER,
    replan_id TEXT, -- 关联的重规划记录
    metadata TEXT, -- JSON格式的元数据
    FOREIGN KEY (session_id) REFERENCES execution_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (replan_id) REFERENCES replan_history(id) ON DELETE SET NULL
);

-- ============================================================================
-- Memory（记忆模块）相关表
-- ============================================================================

-- 执行经验表
CREATE TABLE IF NOT EXISTS execution_experiences (
    id TEXT PRIMARY KEY,
    task_type TEXT NOT NULL,
    target_description TEXT NOT NULL,
    target_hash TEXT NOT NULL, -- 目标的哈希值，用于快速匹配
    target_properties TEXT, -- JSON格式的目标属性
    environment_context TEXT NOT NULL,
    environment_hash TEXT NOT NULL, -- 环境的哈希值
    environment_properties TEXT, -- JSON格式的环境属性
    successful_steps TEXT, -- JSON格式的成功步骤数组
    failure_info TEXT, -- JSON格式的失败信息
    performance_metrics TEXT, -- JSON格式的性能指标
    confidence_score REAL NOT NULL DEFAULT 1.0,
    usage_count INTEGER NOT NULL DEFAULT 0, -- 被引用次数
    last_used_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- 计划模板表
CREATE TABLE IF NOT EXISTS plan_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    domain TEXT NOT NULL, -- 领域分类：security_scan, vulnerability_assessment, penetration_test等
    task_type TEXT NOT NULL, -- 任务类型
    template_steps TEXT NOT NULL, -- JSON格式的模板步骤数组
    success_rate REAL NOT NULL DEFAULT 0.0,
    usage_count INTEGER NOT NULL DEFAULT 0,
    effectiveness_score REAL NOT NULL DEFAULT 0.0, -- 效果评分
    applicability_conditions TEXT, -- JSON格式的适用条件
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_used_at INTEGER
);

-- 知识实体表
CREATE TABLE IF NOT EXISTS knowledge_entities (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL, -- Tool, Target, Environment, Technique, Vulnerability, Asset
    name TEXT NOT NULL,
    properties TEXT, -- JSON格式的属性
    confidence REAL NOT NULL DEFAULT 1.0,
    usage_count INTEGER NOT NULL DEFAULT 0,
    effectiveness_score REAL NOT NULL DEFAULT 0.0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- 知识关系表
CREATE TABLE IF NOT EXISTS knowledge_relationships (
    id TEXT PRIMARY KEY,
    from_entity TEXT NOT NULL,
    to_entity TEXT NOT NULL,
    relationship_type TEXT NOT NULL, -- EffectiveAgainst, Precedes, Requires, Conflicts, Enhances, Substitutes
    strength REAL NOT NULL DEFAULT 1.0, -- 关系强度
    context TEXT, -- JSON格式的上下文信息
    confidence REAL NOT NULL DEFAULT 1.0,
    usage_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (from_entity) REFERENCES knowledge_entities(id) ON DELETE CASCADE,
    FOREIGN KEY (to_entity) REFERENCES knowledge_entities(id) ON DELETE CASCADE,
    UNIQUE(from_entity, to_entity, relationship_type)
);

-- 学习反馈表
CREATE TABLE IF NOT EXISTS learning_feedback (
    id TEXT PRIMARY KEY,
    experience_id TEXT,
    template_id TEXT,
    entity_id TEXT,
    relationship_id TEXT,
    feedback_type TEXT NOT NULL, -- Success, Failure, Improvement, UserCorrection
    feedback_content TEXT NOT NULL, -- JSON格式的反馈内容
    improvements TEXT, -- JSON格式的改进建议
    confidence_adjustments TEXT, -- JSON格式的置信度调整
    user_rating REAL, -- 用户评分 1-5
    automated_score REAL, -- 自动评分
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    processed_at INTEGER,
    FOREIGN KEY (experience_id) REFERENCES execution_experiences(id) ON DELETE CASCADE,
    FOREIGN KEY (template_id) REFERENCES plan_templates(id) ON DELETE CASCADE,
    FOREIGN KEY (entity_id) REFERENCES knowledge_entities(id) ON DELETE CASCADE,
    FOREIGN KEY (relationship_id) REFERENCES knowledge_relationships(id) ON DELETE CASCADE
);

-- 向量嵌入表（用于相似度搜索）
CREATE TABLE IF NOT EXISTS vector_embeddings (
    id TEXT PRIMARY KEY,
    content_type TEXT NOT NULL, -- experience, template, entity, relationship
    content_id TEXT NOT NULL,
    embedding BLOB NOT NULL, -- 向量数据（二进制存储）
    dimensions INTEGER NOT NULL,
    model_name TEXT NOT NULL, -- 使用的嵌入模型名称
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- 记忆查询历史表
CREATE TABLE IF NOT EXISTS memory_query_history (
    id TEXT PRIMARY KEY,
    query_type TEXT NOT NULL, -- SimilarFailures, SuccessfulPatterns, ToolEffectiveness, EnvironmentSpecific
    query_content TEXT NOT NULL, -- JSON格式的查询内容
    results_count INTEGER NOT NULL DEFAULT 0,
    execution_time_ms INTEGER NOT NULL,
    similarity_threshold REAL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- 索引创建
-- ============================================================================

-- 重规划历史索引
CREATE INDEX IF NOT EXISTS idx_replan_history_session_id ON replan_history(session_id);
CREATE INDEX IF NOT EXISTS idx_replan_history_strategy_type ON replan_history(strategy_type);
CREATE INDEX IF NOT EXISTS idx_replan_history_created_at ON replan_history(created_at);
CREATE INDEX IF NOT EXISTS idx_replan_history_success ON replan_history(success);
CREATE INDEX IF NOT EXISTS idx_replan_history_confidence ON replan_history(confidence);

-- 重规划触发器索引
CREATE INDEX IF NOT EXISTS idx_replan_triggers_session_id ON replan_triggers(session_id);
CREATE INDEX IF NOT EXISTS idx_replan_triggers_type ON replan_triggers(trigger_type);
CREATE INDEX IF NOT EXISTS idx_replan_triggers_severity ON replan_triggers(severity);
CREATE INDEX IF NOT EXISTS idx_replan_triggers_triggered_at ON replan_triggers(triggered_at);

-- 执行经验索引
CREATE INDEX IF NOT EXISTS idx_execution_experiences_task_type ON execution_experiences(task_type);
CREATE INDEX IF NOT EXISTS idx_execution_experiences_target_hash ON execution_experiences(target_hash);
CREATE INDEX IF NOT EXISTS idx_execution_experiences_environment_hash ON execution_experiences(environment_hash);
CREATE INDEX IF NOT EXISTS idx_execution_experiences_confidence_score ON execution_experiences(confidence_score);
CREATE INDEX IF NOT EXISTS idx_execution_experiences_usage_count ON execution_experiences(usage_count);
CREATE INDEX IF NOT EXISTS idx_execution_experiences_created_at ON execution_experiences(created_at);
CREATE INDEX IF NOT EXISTS idx_execution_experiences_last_used_at ON execution_experiences(last_used_at);

-- 计划模板索引
CREATE INDEX IF NOT EXISTS idx_plan_templates_domain ON plan_templates(domain);
CREATE INDEX IF NOT EXISTS idx_plan_templates_task_type ON plan_templates(task_type);
CREATE INDEX IF NOT EXISTS idx_plan_templates_success_rate ON plan_templates(success_rate);
CREATE INDEX IF NOT EXISTS idx_plan_templates_usage_count ON plan_templates(usage_count);
CREATE INDEX IF NOT EXISTS idx_plan_templates_effectiveness_score ON plan_templates(effectiveness_score);
CREATE INDEX IF NOT EXISTS idx_plan_templates_created_at ON plan_templates(created_at);

-- 知识实体索引
CREATE INDEX IF NOT EXISTS idx_knowledge_entities_type ON knowledge_entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_knowledge_entities_name ON knowledge_entities(name);
CREATE INDEX IF NOT EXISTS idx_knowledge_entities_confidence ON knowledge_entities(confidence);
CREATE INDEX IF NOT EXISTS idx_knowledge_entities_effectiveness ON knowledge_entities(effectiveness_score);
CREATE INDEX IF NOT EXISTS idx_knowledge_entities_usage_count ON knowledge_entities(usage_count);

-- 知识关系索引
CREATE INDEX IF NOT EXISTS idx_knowledge_relationships_from ON knowledge_relationships(from_entity);
CREATE INDEX IF NOT EXISTS idx_knowledge_relationships_to ON knowledge_relationships(to_entity);
CREATE INDEX IF NOT EXISTS idx_knowledge_relationships_type ON knowledge_relationships(relationship_type);
CREATE INDEX IF NOT EXISTS idx_knowledge_relationships_strength ON knowledge_relationships(strength);
CREATE INDEX IF NOT EXISTS idx_knowledge_relationships_confidence ON knowledge_relationships(confidence);

-- 学习反馈索引
CREATE INDEX IF NOT EXISTS idx_learning_feedback_experience_id ON learning_feedback(experience_id);
CREATE INDEX IF NOT EXISTS idx_learning_feedback_template_id ON learning_feedback(template_id);
CREATE INDEX IF NOT EXISTS idx_learning_feedback_entity_id ON learning_feedback(entity_id);
CREATE INDEX IF NOT EXISTS idx_learning_feedback_relationship_id ON learning_feedback(relationship_id);
CREATE INDEX IF NOT EXISTS idx_learning_feedback_type ON learning_feedback(feedback_type);
CREATE INDEX IF NOT EXISTS idx_learning_feedback_created_at ON learning_feedback(created_at);
CREATE INDEX IF NOT EXISTS idx_learning_feedback_user_rating ON learning_feedback(user_rating);

-- 向量嵌入索引
CREATE INDEX IF NOT EXISTS idx_vector_embeddings_content_type ON vector_embeddings(content_type);
CREATE INDEX IF NOT EXISTS idx_vector_embeddings_content_id ON vector_embeddings(content_id);
CREATE INDEX IF NOT EXISTS idx_vector_embeddings_model_name ON vector_embeddings(model_name);
CREATE INDEX IF NOT EXISTS idx_vector_embeddings_dimensions ON vector_embeddings(dimensions);
CREATE INDEX IF NOT EXISTS idx_vector_embeddings_created_at ON vector_embeddings(created_at);

-- 记忆查询历史索引
CREATE INDEX IF NOT EXISTS idx_memory_query_history_type ON memory_query_history(query_type);
CREATE INDEX IF NOT EXISTS idx_memory_query_history_created_at ON memory_query_history(created_at);
CREATE INDEX IF NOT EXISTS idx_memory_query_history_execution_time ON memory_query_history(execution_time_ms);

-- ============================================================================
-- 视图创建
-- ============================================================================

-- 重规划统计视图
CREATE VIEW IF NOT EXISTS replan_statistics AS
SELECT 
    strategy_type,
    COUNT(*) as total_replans,
    COUNT(CASE WHEN success = 1 THEN 1 END) as successful_replans,
    COUNT(CASE WHEN success = 0 THEN 1 END) as failed_replans,
    AVG(confidence) as avg_confidence,
    AVG(CASE WHEN completed_at IS NOT NULL THEN 
        (completed_at - created_at) END) as avg_replan_time,
    DATE(created_at, 'unixepoch') as replan_date
FROM replan_history
GROUP BY strategy_type, DATE(created_at, 'unixepoch')
ORDER BY replan_date DESC;

-- 经验使用统计视图
CREATE VIEW IF NOT EXISTS experience_usage_stats AS
SELECT 
    task_type,
    environment_context,
    COUNT(*) as experience_count,
    AVG(confidence_score) as avg_confidence,
    AVG(usage_count) as avg_usage,
    MAX(last_used_at) as last_used,
    DATE(created_at, 'unixepoch') as creation_date
FROM execution_experiences
GROUP BY task_type, environment_context, DATE(created_at, 'unixepoch')
ORDER BY creation_date DESC, avg_usage DESC;

-- 模板效果统计视图
CREATE VIEW IF NOT EXISTS template_effectiveness AS
SELECT 
    pt.id,
    pt.name,
    pt.domain,
    pt.task_type,
    pt.success_rate,
    pt.usage_count,
    pt.effectiveness_score,
    AVG(lf.user_rating) as avg_user_rating,
    COUNT(lf.id) as feedback_count,
    pt.last_used_at
FROM plan_templates pt
LEFT JOIN learning_feedback lf ON pt.id = lf.template_id
GROUP BY pt.id
ORDER BY pt.effectiveness_score DESC, pt.usage_count DESC;

-- 知识图谱统计视图
CREATE VIEW IF NOT EXISTS knowledge_graph_stats AS
SELECT 
    ke.entity_type,
    COUNT(DISTINCT ke.id) as entity_count,
    COUNT(DISTINCT kr.id) as relationship_count,
    AVG(ke.confidence) as avg_entity_confidence,
    AVG(kr.strength) as avg_relationship_strength,
    AVG(ke.effectiveness_score) as avg_effectiveness
FROM knowledge_entities ke
LEFT JOIN knowledge_relationships kr ON ke.id = kr.from_entity OR ke.id = kr.to_entity
GROUP BY ke.entity_type
ORDER BY entity_count DESC;

-- 学习进度视图
CREATE VIEW IF NOT EXISTS learning_progress AS
SELECT 
    DATE(created_at, 'unixepoch') as learning_date,
    feedback_type,
    COUNT(*) as feedback_count,
    AVG(user_rating) as avg_user_rating,
    AVG(automated_score) as avg_automated_score,
    COUNT(CASE WHEN processed_at IS NOT NULL THEN 1 END) as processed_count
FROM learning_feedback
GROUP BY DATE(created_at, 'unixepoch'), feedback_type
ORDER BY learning_date DESC;

-- ============================================================================
-- 示例数据插入
-- ============================================================================

-- 插入示例知识实体
INSERT OR IGNORE INTO knowledge_entities (id, entity_type, name, properties, confidence, effectiveness_score)
VALUES 
('tool_nmap', 'Tool', 'Nmap', '{"category": "port_scanner", "capabilities": ["tcp_scan", "udp_scan", "service_detection"], "platforms": ["linux", "windows", "macos"]}', 0.95, 0.9),
('tool_nikto', 'Tool', 'Nikto', '{"category": "web_scanner", "capabilities": ["web_vulnerability_scan", "cgi_scan"], "platforms": ["linux", "windows"]}', 0.85, 0.8),
('target_web_server', 'Target', 'Web Server', '{"type": "web_application", "common_ports": [80, 443, 8080, 8443], "technologies": ["apache", "nginx", "iis"]}', 0.9, 0.85),
('env_internal_network', 'Environment', 'Internal Network', '{"network_type": "internal", "security_level": "medium", "monitoring": true}', 0.9, 0.8);

-- 插入示例知识关系
INSERT OR IGNORE INTO knowledge_relationships (id, from_entity, to_entity, relationship_type, strength, context, confidence)
VALUES 
('rel_nmap_web', 'tool_nmap', 'target_web_server', 'EffectiveAgainst', 0.9, '{"scan_types": ["tcp_connect", "service_detection"], "typical_ports": [80, 443, 8080]}', 0.9),
('rel_nikto_web', 'tool_nikto', 'target_web_server', 'EffectiveAgainst', 0.85, '{"scan_types": ["web_vulnerability"], "requires_http": true}', 0.85),
('rel_nmap_nikto', 'tool_nmap', 'tool_nikto', 'Precedes', 0.8, '{"reason": "port_discovery_before_web_scan", "dependency": "open_web_ports"}', 0.8);

-- 插入示例计划模板
INSERT OR IGNORE INTO plan_templates (id, name, description, domain, task_type, template_steps, success_rate, effectiveness_score, applicability_conditions)
VALUES (
    'template_web_scan',
    'Web应用安全扫描模板',
    '针对Web应用的标准安全扫描流程',
    'web_security',
    'vulnerability_assessment',
    '[
        {
            "name": "端口发现",
            "tool_name": "nmap",
            "parameter_template": {
                "target": {"type": "string", "required": true},
                "ports": {"type": "string", "default": "80,443,8080,8443"},
                "scan_type": {"type": "string", "default": "tcp"}
            },
            "conditions": ["target_is_web_server"],
            "alternatives": ["masscan"]
        },
        {
            "name": "Web漏洞扫描",
            "tool_name": "nikto",
            "parameter_template": {
                "target": {"type": "string", "required": true},
                "port": {"type": "integer", "default": 80}
            },
            "conditions": ["web_port_open"],
            "alternatives": ["dirb", "gobuster"]
        }
    ]',
    0.85,
    0.8,
    '{"target_type": "web_application", "environment": ["internal", "external"], "min_confidence": 0.7}'
);

-- 插入示例执行经验
INSERT OR IGNORE INTO execution_experiences (id, task_type, target_description, target_hash, target_properties, environment_context, environment_hash, environment_properties, successful_steps, performance_metrics, confidence_score)
VALUES (
    'exp_web_scan_001',
    'web_vulnerability_scan',
    'Apache Web Server on port 80',
    'hash_apache_80',
    '{"server_type": "apache", "version": "2.4.41", "port": 80, "ssl": false}',
    'Internal corporate network',
    'hash_internal_corp',
    '{"network_type": "internal", "security_level": "medium", "bandwidth": "high"}',
    '[
        {
            "id": "step_port_scan",
            "name": "端口扫描",
            "tool_name": "nmap",
            "parameters": {"target": "192.168.1.100", "ports": "80,443", "scan_type": "tcp"},
            "success_rate": 1.0,
            "execution_time_ms": 15000,
            "context": {"discovered_ports": [80], "services": ["http"]}
        },
        {
            "id": "step_web_scan",
            "name": "Web扫描",
            "tool_name": "nikto",
            "parameters": {"target": "192.168.1.100", "port": 80},
            "success_rate": 0.9,
            "execution_time_ms": 45000,
            "context": {"vulnerabilities_found": 3, "scan_coverage": "high"}
        }
    ]',
    '{"total_execution_time_ms": 60000, "memory_usage_mb": 128, "cpu_usage_percent": 25, "success_rate": 0.95}',
    0.9
);

COMMIT;

-- 创建触发器以自动更新时间戳
CREATE TRIGGER IF NOT EXISTS update_execution_experiences_timestamp 
AFTER UPDATE ON execution_experiences
BEGIN
    UPDATE execution_experiences SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_plan_templates_timestamp 
AFTER UPDATE ON plan_templates
BEGIN
    UPDATE plan_templates SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_knowledge_entities_timestamp 
AFTER UPDATE ON knowledge_entities
BEGIN
    UPDATE knowledge_entities SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_knowledge_relationships_timestamp 
AFTER UPDATE ON knowledge_relationships
BEGIN
    UPDATE knowledge_relationships SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_vector_embeddings_timestamp 
AFTER UPDATE ON vector_embeddings
BEGIN
    UPDATE vector_embeddings SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
END;