//! Agent Team 文档产物与模板引擎

/// 标准产物类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactType {
    Prd,
    Architecture,
    DetailedDesign,
    TestPlan,
    WorkflowTasks,
    // 安全领域
    VulnerabilityReport,
    IncidentReview,
    // 通用
    Custom(String),
}

impl ArtifactType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Prd => "prd",
            Self::Architecture => "architecture",
            Self::DetailedDesign => "detailed_design",
            Self::TestPlan => "test_plan",
            Self::WorkflowTasks => "workflow_tasks",
            Self::VulnerabilityReport => "vulnerability_report",
            Self::IncidentReview => "incident_review",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Prd => "产品需求文档（PRD）",
            Self::Architecture => "架构设计文档",
            Self::DetailedDesign => "详细设计文档",
            Self::TestPlan => "测试计划",
            Self::WorkflowTasks => "工作流任务拆解",
            Self::VulnerabilityReport => "漏洞分析报告",
            Self::IncidentReview => "安全事件复盘",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// 产物生成 Prompt 模板
pub struct ArtifactTemplate {
    pub artifact_type: ArtifactType,
    pub title: String,
}

impl ArtifactTemplate {
    /// 生成产物提示词
    pub fn build_prompt(
        &self,
        session_goal: &str,
        discussion_summary: &str,
        blackboard_summary: &str,
    ) -> String {
        match &self.artifact_type {
            ArtifactType::Prd => {
                self.prd_prompt(session_goal, discussion_summary, blackboard_summary)
            }
            ArtifactType::Architecture => {
                self.architecture_prompt(session_goal, discussion_summary, blackboard_summary)
            }
            ArtifactType::TestPlan => {
                self.test_plan_prompt(session_goal, discussion_summary, blackboard_summary)
            }
            ArtifactType::VulnerabilityReport => self.vulnerability_report_prompt(
                session_goal,
                discussion_summary,
                blackboard_summary,
            ),
            _ => self.generic_prompt(session_goal, discussion_summary, blackboard_summary),
        }
    }

    fn prd_prompt(&self, goal: &str, discussion: &str, blackboard: &str) -> String {
        format!(
            "请基于以下团队讨论，生成一份完整的产品需求文档（PRD）：\n\n\
            **产品目标**: {}\n\n**讨论摘要**: {}\n\n**白板共识**: {}\n\n\
            请严格按以下格式输出 Markdown：\n\
            # 产品需求文档（PRD）\n\
            ## 版本信息\n\
            ## 一、产品概述\n### 背景与目标\n### 用户价值\n\
            ## 二、功能需求\n### 核心功能\n### 边界需求\n\
            ## 三、非功能需求\n### 性能\n### 安全性\n### 可扩展性\n\
            ## 四、接受标准（Acceptance Criteria）\n\
            ## 五、风险与优先级\n\
            ## 六、里程碑",
            goal, discussion, blackboard
        )
    }

    fn architecture_prompt(&self, goal: &str, discussion: &str, blackboard: &str) -> String {
        format!(
            "请基于以下团队讨论，生成一份完整的架构设计文档：\n\n\
            **系统目标**: {}\n\n**讨论摘要**: {}\n\n**白板共识**: {}\n\n\
            请严格按以下格式输出 Markdown：\n\
            # 架构设计文档\n\
            ## 一、概述\n### 系统背景\n### 设计原则\n\
            ## 二、整体架构\n### 架构图（文本描述）\n### 核心组件\n\
            ## 三、模块设计\n\
            ## 四、数据模型\n\
            ## 五、API 设计\n\
            ## 六、安全架构\n\
            ## 七、部署架构\n\
            ## 八、风险与对策",
            goal, discussion, blackboard
        )
    }

    fn test_plan_prompt(&self, goal: &str, discussion: &str, blackboard: &str) -> String {
        format!(
            "请基于以下团队讨论，生成一份完整的测试计划：\n\n\
            **测试目标**: {}\n\n**讨论摘要**: {}\n\n**白板共识**: {}\n\n\
            请严格按以下格式输出 Markdown：\n\
            # 测试计划\n\
            ## 一、测试策略\n\
            ## 二、测试范围\n### 功能测试\n### 性能测试\n### 安全测试\n### 集成测试\n\
            ## 三、测试用例设计\n\
            ## 四、边界条件与异常路径\n\
            ## 五、测试环境\n\
            ## 六、自动化策略\n\
            ## 七、验收标准",
            goal, discussion, blackboard
        )
    }

    fn vulnerability_report_prompt(
        &self,
        goal: &str,
        discussion: &str,
        blackboard: &str,
    ) -> String {
        format!(
            "请基于以下安全团队讨论，生成一份漏洞分析报告：\n\n\
            **分析目标**: {}\n\n**讨论摘要**: {}\n\n**白板共识**: {}\n\n\
            请严格按以下格式输出 Markdown：\n\
            # 漏洞分析报告\n\
            ## 一、执行摘要\n\
            ## 二、漏洞列表\n### 高危漏洞\n### 中危漏洞\n### 低危漏洞\n\
            ## 三、漏洞详情\n\
            ## 四、风险评估（CVSS）\n\
            ## 五、修复建议\n\
            ## 六、验证方法\n\
            ## 七、合规影响",
            goal, discussion, blackboard
        )
    }

    fn generic_prompt(&self, goal: &str, discussion: &str, blackboard: &str) -> String {
        format!(
            "请基于以下团队讨论，生成一份「{}」文档：\n\n\
            **目标**: {}\n\n**讨论摘要**: {}\n\n**白板共识**: {}\n\n\
            请以结构化 Markdown 格式输出，包含摘要、详细内容、行动计划。",
            self.artifact_type.display_name(),
            goal,
            discussion,
            blackboard
        )
    }
}

/// 根据会话 domain 推断应生成的产物类型列表
pub fn get_artifact_types_for_domain(domain: &str) -> Vec<ArtifactType> {
    match domain {
        "product" => vec![ArtifactType::Prd, ArtifactType::Architecture],
        "security" | "audit" => vec![ArtifactType::VulnerabilityReport],
        "ops" => vec![ArtifactType::DetailedDesign],
        _ => vec![ArtifactType::Architecture],
    }
}

/// 格式化产物内容（添加元数据头部）
pub fn format_artifact_with_header(
    content: &str,
    artifact_type: &ArtifactType,
    session_id: &str,
    version: i32,
) -> String {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    format!(
        "---\n\
        artifact_type: {}\n\
        session_id: {}\n\
        version: {}\n\
        generated_at: {}\n\
        ---\n\n\
        {}",
        artifact_type.as_str(),
        session_id,
        version,
        now,
        content
    )
}
