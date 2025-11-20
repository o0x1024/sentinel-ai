use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Security task type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SecurityTaskKind {
    WebPentest,
    APIPentest,
    Forensics,
    CTF,
    ReverseEngineering,
    OtherSecurity,
}

impl Default for SecurityTaskKind {
    fn default() -> Self {
        Self::WebPentest
    }
}

/// Test session stage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TestStage {
    // Web/API Pentest stages
    Recon,
    Login,
    APIMapping,
    VulnScan,
    Exploit,
    
    // Forensics stages
    LogCollection,
    TimelineReconstruction,
    IOCExtraction,
    BehaviorAnalysis,
    
    // CTF stages
    ChallengeAnalysis,
    VulnIdentification,
    PayloadCrafting,
    FlagExtraction,
    Writeup,
    
    // Reverse Engineering stages
    BinaryLoading,
    StaticAnalysis,
    DynamicAnalysis,
    Deobfuscation,
    BehaviorSummary,
    
    // Common stages
    Report,
    Completed,
}

/// Test step type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TestStepType {
    // Web/API Pentest steps
    PlanSecurityTest,
    ExecuteLoginFlow,
    ScanAPIVulns,
    EnumerateEndpoints,
    TestAuthentication,
    TestAuthorization,
    TestInputValidation,
    CraftExploit,
    
    // Forensics steps
    CollectLogs,
    ParseTimeline,
    ExtractIOC,
    AnalyzeBehavior,
    CorrelateEvents,
    
    // CTF steps
    AnalyzeChallenge,
    IdentifyVulnerability,
    DevelopExploit,
    ExtractFlag,
    WriteReport,
    
    // Reverse Engineering steps
    LoadBinary,
    PerformStaticAnalysis,
    PerformDynamicAnalysis,
    DeobfuscateCode,
    SummarizeBehavior,
    
    // Common steps
    GenerateScript,
    GeneratePayload,
    UpdatePlan,
    SummarizeFindings,
}

/// Sub-agent type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SubAgentKind {
    Travel,
    ReWOO,
    PlanAndExecute,
    LLMCompiler,
    Copilot,
    Other,
}

/// Risk impact level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum RiskImpact {
    None,
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Default for RiskImpact {
    fn default() -> Self {
        Self::None
    }
}

/// Step status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl Default for StepStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Authentication context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthContext {
    pub cookies: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub tokens: HashMap<String, String>,
    pub credentials: Option<Credentials>,
}

impl AuthContext {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_credentials(username: String, password: String) -> Self {
        Self {
            credentials: Some(Credentials { username, password }),
            ..Default::default()
        }
    }
    
    pub fn add_cookie(&mut self, name: String, value: String) {
        self.cookies.insert(name, value);
    }
    
    pub fn add_header(&mut self, name: String, value: String) {
        self.headers.insert(name, value);
    }
    
    pub fn add_token(&mut self, token_type: String, token: String) {
        self.tokens.insert(token_type, token);
    }
    
    pub fn is_authenticated(&self) -> bool {
        !self.cookies.is_empty() || !self.tokens.is_empty() || self.credentials.is_some()
    }
}

/// User credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// Security finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub location: String,
    pub method: Option<String>,
    pub risk_level: RiskImpact,
    pub title: String,
    pub description: String,
    pub evidence: String,
    pub reproduction_steps: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl Finding {
    pub fn new(
        location: String,
        risk_level: RiskImpact,
        title: String,
        description: String,
        evidence: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            location,
            method: None,
            risk_level,
            title,
            description,
            evidence,
            reproduction_steps: Vec::new(),
            created_at: Utc::now(),
        }
    }
    
    pub fn with_method(mut self, method: String) -> Self {
        self.method = Some(method);
        self
    }
    
    pub fn with_reproduction_steps(mut self, steps: Vec<String>) -> Self {
        self.reproduction_steps = steps;
        self
    }
}

/// Test step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStep {
    pub id: String,
    pub index: usize,
    pub sub_agent_kind: SubAgentKind,
    pub step_type: TestStepType,
    pub short_summary: String,
    pub risk_impact: RiskImpact,
    pub status: StepStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub detail_refs: Vec<String>,
    pub output: Option<String>,
}

impl TestStep {
    pub fn new(
        index: usize,
        sub_agent_kind: SubAgentKind,
        step_type: TestStepType,
        short_summary: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            index,
            sub_agent_kind,
            step_type,
            short_summary,
            risk_impact: RiskImpact::None,
            status: StepStatus::Pending,
            started_at: None,
            finished_at: None,
            detail_refs: Vec::new(),
            output: None,
        }
    }
    
    pub fn start(&mut self) {
        self.status = StepStatus::Running;
        self.started_at = Some(Utc::now());
    }
    
    pub fn complete(&mut self, output: Option<String>) {
        self.status = StepStatus::Completed;
        self.finished_at = Some(Utc::now());
        self.output = output;
    }
    
    pub fn fail(&mut self, error: String) {
        self.status = StepStatus::Failed;
        self.finished_at = Some(Utc::now());
        self.output = Some(error);
    }
    
    pub fn with_risk_impact(mut self, risk: RiskImpact) -> Self {
        self.risk_impact = risk;
        self
    }
}

/// Test session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSession {
    pub id: String,
    pub task_kind: SecurityTaskKind,
    pub primary_target: String,
    pub stage: TestStage,
    pub summary: String,
    pub auth_context: AuthContext,
    pub steps: Vec<TestStep>,
    pub findings: Vec<Finding>,
    pub coverage: TestCoverage,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Task parameters from original AgentTask (includes tools_allow, etc.)
    #[serde(default)]
    pub task_parameters: HashMap<String, serde_json::Value>,
}

impl TestSession {
    pub fn new(task_kind: SecurityTaskKind, primary_target: String, summary: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_kind,
            primary_target,
            stage: TestStage::Recon,
            summary,
            auth_context: AuthContext::new(),
            steps: Vec::new(),
            findings: Vec::new(),
            coverage: TestCoverage::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            task_parameters: HashMap::new(),
        }
    }
    
    pub fn add_step(&mut self, step: TestStep) {
        self.steps.push(step);
        self.updated_at = Utc::now();
    }
    
    pub fn add_finding(&mut self, finding: Finding) {
        self.findings.push(finding);
        self.updated_at = Utc::now();
    }
    
    pub fn update_stage(&mut self, stage: TestStage) {
        self.stage = stage;
        self.updated_at = Utc::now();
    }
    
    pub fn update_auth_context(&mut self, auth_context: AuthContext) {
        self.auth_context = auth_context;
        self.updated_at = Utc::now();
    }
    
    pub fn get_high_risk_findings(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.risk_level >= RiskImpact::High)
            .collect()
    }
    
    pub fn get_step_by_id(&self, id: &str) -> Option<&TestStep> {
        self.steps.iter().find(|s| s.id == id)
    }
    
    pub fn get_step_by_id_mut(&mut self, id: &str) -> Option<&mut TestStep> {
        self.steps.iter_mut().find(|s| s.id == id)
    }
}

/// Test coverage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCoverage {
    pub tested_urls: Vec<String>,
    pub tested_endpoints: Vec<String>,
    pub tested_parameters: Vec<String>,
    pub tested_vuln_types: Vec<String>,
    pub progress_percentage: f32,
}

impl TestCoverage {
    pub fn new() -> Self {
        Self {
            tested_urls: Vec::new(),
            tested_endpoints: Vec::new(),
            tested_parameters: Vec::new(),
            tested_vuln_types: Vec::new(),
            progress_percentage: 0.0,
        }
    }
    
    pub fn add_tested_url(&mut self, url: String) {
        if !self.tested_urls.contains(&url) {
            self.tested_urls.push(url);
        }
    }
    
    pub fn add_tested_endpoint(&mut self, endpoint: String) {
        if !self.tested_endpoints.contains(&endpoint) {
            self.tested_endpoints.push(endpoint);
        }
    }
    
    pub fn add_tested_parameter(&mut self, param: String) {
        if !self.tested_parameters.contains(&param) {
            self.tested_parameters.push(param);
        }
    }
    
    pub fn add_tested_vuln_type(&mut self, vuln_type: String) {
        if !self.tested_vuln_types.contains(&vuln_type) {
            self.tested_vuln_types.push(vuln_type);
        }
    }
    
    pub fn update_progress(&mut self, percentage: f32) {
        self.progress_percentage = percentage.clamp(0.0, 100.0);
    }
}

impl Default for TestCoverage {
    fn default() -> Self {
        Self::new()
    }
}

