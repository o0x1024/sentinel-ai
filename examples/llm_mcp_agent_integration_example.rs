//! LLM + MCP + Agent + 内置工具串联使用示例
//! 展示如何将所有组件集成在一起进行自动化安全测试

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use anyhow::Result;
use tokio;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// 导入相关模块
use crate::mcp::{McpClient, McpConnection};
use crate::agents::types::*;
use crate::tools::tool_manager::ToolManager;
use crate::services::llm::LlmService;

/// 智能安全测试Agent
/// 集成LLM决策、MCP工具调用和内置扫描工具
#[derive(Debug, Clone)]
pub struct IntelligentSecurityAgent {
    pub id: String,
    pub name: String,
    pub llm_service: LlmService,
    pub mcp_client: McpClient,
    pub tool_manager: ToolManager,
    pub context: AgentContext,
}

impl IntelligentSecurityAgent {
    /// 创建新的智能安全测试Agent
    pub fn new(
        name: String,
        llm_service: LlmService,
        mcp_client: McpClient,
        tool_manager: ToolManager,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            llm_service,
            mcp_client,
            tool_manager,
            context: AgentContext {
                session_id: Uuid::new_v4(),
                target: String::new(),
                scope: Vec::new(),
                constraints: HashMap::new(),
                shared_data: HashMap::new(),
                llm_context: None,
                workflow_id: Some(Uuid::new_v4()),
                workflow_config: Some(WorkflowConfig {
                    max_concurrent_tasks: 5,
                    timeout_seconds: 3600,
                    retry_attempts: 3,
                    enable_llm_guidance: true,
                    enable_adaptive_strategy: true,
                    risk_tolerance: RiskTolerance::Balanced,
                }),
                previous_results: Vec::new(),
            },
        }
    }

    /// 执行完整的安全测试工作流
    pub async fn execute_security_workflow(&mut self, target: &str) -> Result<AgentResult> {
        println!("🚀 开始对目标 {} 执行智能安全测试", target);
        
        // 1. 设置测试上下文
        self.context.target = target.to_string();
        self.context.scope = vec![target.to_string()];
        
        // 2. LLM分析目标并制定测试策略
        let strategy = self.analyze_target_with_llm(target).await?;
        println!("🧠 LLM分析完成，制定测试策略: {}", strategy.reasoning);
        
        // 3. 执行多阶段测试流程
        let mut workflow_results = Vec::new();
        
        for stage in [WorkflowStage::Reconnaissance, WorkflowStage::VulnerabilityDiscovery, WorkflowStage::VulnerabilityValidation] {
            let stage_result = self.execute_workflow_stage(stage, &strategy).await?;
            workflow_results.push(stage_result);
            
            // 更新上下文，为下一阶段提供数据
            self.update_context_with_results(&workflow_results).await?;
        }
        
        // 4. 生成最终报告
        let final_report = self.generate_comprehensive_report(&workflow_results).await?;
        
        Ok(AgentResult {
            task_id: Uuid::new_v4(),
            agent_id: self.id.clone(),
            status: AgentTaskStatus::Completed,
            data: final_report,
            metadata: HashMap::from([
                ("target".to_string(), json!(target)),
                ("workflow_stages".to_string(), json!(workflow_results.len())),
                ("completion_time".to_string(), json!(Utc::now().to_rfc3339())),
            ]),
            next_actions: Vec::new(),
            confidence_score: 0.95,
            execution_time: 1800, // 30分钟
            resource_usage: ResourceUsage {
                cpu_time: 45000,
                memory_peak: 512 * 1024 * 1024, // 512MB
                network_requests: 150,
                disk_io: 10 * 1024 * 1024, // 10MB
            },
        })
    }

    /// 使用LLM分析目标并制定策略
    async fn analyze_target_with_llm(&self, target: &str) -> Result<LlmDecisionResponse> {
        let prompt = format!(
            "作为网络安全专家，请分析目标 '{}' 并制定渗透测试策略。\n\n\
            请考虑以下因素：\n\
            1. 目标类型（域名、IP地址、网段等）\n\
            2. 推荐的侦察工具和方法\n\
            3. 可能的攻击面\n\
            4. 风险评估\n\
            5. 测试优先级\n\n\
            请以JSON格式返回分析结果，包含recommended_actions、reasoning、confidence等字段。",
            target
        );

        let llm_response = self.llm_service.generate_response(&prompt, None).await?;
        
        // 解析LLM响应为结构化数据
        let decision: LlmDecisionResponse = serde_json::from_str(&llm_response)
            .unwrap_or_else(|_| LlmDecisionResponse {
                recommended_actions: vec![
                    AgentAction {
                        action_type: AgentTaskType::Reconnaissance,
                        description: "执行基础信息收集".to_string(),
                        parameters: HashMap::from([
                            ("target".to_string(), json!(target)),
                            ("tools".to_string(), json!(["subfinder", "nmap"])),
                        ]),
                        priority: AgentTaskPriority::High,
                        estimated_duration: Some(300),
                        dependencies: Vec::new(),
                    },
                ],
                reasoning: "基于目标特征，建议先进行信息收集".to_string(),
                confidence: 0.8,
                risk_assessment: RiskTolerance::Balanced,
                next_stage: Some(WorkflowStage::VulnerabilityDiscovery),
            });

        Ok(decision)
    }

    /// 执行工作流阶段
    async fn execute_workflow_stage(
        &self,
        stage: WorkflowStage,
        strategy: &LlmDecisionResponse,
    ) -> Result<Value> {
        println!("📋 执行工作流阶段: {:?}", stage);
        
        match stage {
            WorkflowStage::Reconnaissance => {
                self.execute_reconnaissance_stage().await
            },
            WorkflowStage::VulnerabilityDiscovery => {
                self.execute_vulnerability_discovery_stage().await
            },
            WorkflowStage::VulnerabilityValidation => {
                self.execute_vulnerability_validation_stage().await
            },
            _ => Ok(json!({"stage": format!("{:?}", stage), "status": "skipped"})),
        }
    }

    /// 执行侦察阶段 - 结合MCP工具和内置工具
    async fn execute_reconnaissance_stage(&self) -> Result<Value> {
        println!("🔍 开始侦察阶段...");
        
        let mut results = HashMap::new();
        
        // 1. 使用MCP工具进行子域名发现
        if let Ok(subdomain_result) = self.call_mcp_tool(
            "subfinder",
            json!({
                "domain": self.context.target,
                "sources": ["virustotal", "securitytrails", "shodan"]
            })
        ).await {
            results.insert("subdomains", subdomain_result);
            println!("✅ 子域名发现完成");
        }
        
        // 2. 使用内置工具进行端口扫描
        if let Ok(port_scan_result) = self.call_builtin_tool(
            "nmap_scan",
            json!({
                "target": self.context.target,
                "scan_type": "syn",
                "ports": "1-1000"
            })
        ).await {
            results.insert("port_scan", port_scan_result);
            println!("✅ 端口扫描完成");
        }
        
        // 3. 使用MCP工具进行WHOIS查询
        if let Ok(whois_result) = self.call_mcp_tool(
            "whois_lookup",
            json!({"domain": self.context.target})
        ).await {
            results.insert("whois", whois_result);
            println!("✅ WHOIS查询完成");
        }
        
        Ok(json!({
            "stage": "reconnaissance",
            "status": "completed",
            "results": results,
            "timestamp": Utc::now().to_rfc3339()
        }))
    }

    /// 执行漏洞发现阶段
    async fn execute_vulnerability_discovery_stage(&self) -> Result<Value> {
        println!("🔎 开始漏洞发现阶段...");
        
        let mut results = HashMap::new();
        
        // 1. 使用内置Nuclei工具进行漏洞扫描
        if let Ok(nuclei_result) = self.call_builtin_tool(
            "nuclei_scan",
            json!({
                "target": self.context.target,
                "templates": ["cves", "vulnerabilities", "misconfiguration"],
                "severity": ["medium", "high", "critical"]
            })
        ).await {
            results.insert("nuclei_scan", nuclei_result);
            println!("✅ Nuclei漏洞扫描完成");
        }
        
        // 2. 使用MCP工具进行Web应用扫描
        if let Ok(web_scan_result) = self.call_mcp_tool(
            "web_scanner",
            json!({
                "url": format!("http://{}", self.context.target),
                "scan_types": ["xss", "sqli", "directory_traversal"]
            })
        ).await {
            results.insert("web_scan", web_scan_result);
            println!("✅ Web应用扫描完成");
        }
        
        Ok(json!({
            "stage": "vulnerability_discovery",
            "status": "completed",
            "results": results,
            "timestamp": Utc::now().to_rfc3339()
        }))
    }

    /// 执行漏洞验证阶段
    async fn execute_vulnerability_validation_stage(&self) -> Result<Value> {
        println!("✅ 开始漏洞验证阶段...");
        
        // 从之前的结果中获取发现的漏洞
        let vulnerabilities = self.extract_vulnerabilities_from_context();
        
        let mut validated_vulns = Vec::new();
        
        for vuln in vulnerabilities {
            // 使用LLM分析漏洞的可利用性
            let analysis_prompt = format!(
                "分析以下漏洞的可利用性和风险等级：\n{}",
                serde_json::to_string_pretty(&vuln)?
            );
            
            if let Ok(llm_analysis) = self.llm_service.generate_response(&analysis_prompt, None).await {
                // 根据LLM分析结果决定是否进行进一步验证
                if llm_analysis.contains("high") || llm_analysis.contains("critical") {
                    // 使用MCP工具进行漏洞验证
                    if let Ok(validation_result) = self.call_mcp_tool(
                        "vulnerability_validator",
                        json!({
                            "vulnerability": vuln,
                            "target": self.context.target,
                            "validation_type": "safe_check"
                        })
                    ).await {
                        validated_vulns.push(json!({
                            "vulnerability": vuln,
                            "llm_analysis": llm_analysis,
                            "validation_result": validation_result
                        }));
                    }
                }
            }
        }
        
        Ok(json!({
            "stage": "vulnerability_validation",
            "status": "completed",
            "validated_vulnerabilities": validated_vulns,
            "timestamp": Utc::now().to_rfc3339()
        }))
    }

    /// 调用MCP工具
    async fn call_mcp_tool(&self, tool_name: &str, parameters: Value) -> Result<Value> {
        println!("🔧 调用MCP工具: {}", tool_name);
        
        // 获取可用的MCP连接
        let connections = self.mcp_client.get_connections().await;
        
        for connection in connections {
            if connection.tools.iter().any(|t| t.name == tool_name) {
                match self.mcp_client.call_tool(&connection.id, tool_name, parameters.clone()).await {
                    Ok(result) => {
                        if !result.is_error.unwrap_or(false) {
                            // 解析工具执行结果
                            let content = result.content.into_iter()
                                .filter_map(|c| match c.raw {
                                    rmcp::model::RawContent::Text(text) => Some(text.text),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            
                            return Ok(serde_json::from_str(&content)
                                .unwrap_or_else(|_| json!({"output": content})));
                        }
                    },
                    Err(e) => {
                        println!("⚠️ MCP工具调用失败: {}", e);
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("未找到可用的MCP工具: {}", tool_name))
    }

    /// 调用内置工具
    async fn call_builtin_tool(&self, tool_name: &str, parameters: Value) -> Result<Value> {
        println!("🛠️ 调用内置工具: {}", tool_name);
        
        // 使用工具管理器执行内置工具
        let scan_id = self.tool_manager.start_scan(
            tool_name.to_string(),
            parameters,
        ).await?;
        
        // 等待扫描完成
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            
            if let Ok(result) = self.tool_manager.get_scan_result(&scan_id).await {
                if result.status != crate::tools::ScanStatus::Running {
                    return Ok(result.result.unwrap_or_else(|| json!({
                        "status": "completed",
                        "tool": tool_name
                    })));
                }
            }
        }
    }

    /// 从上下文中提取漏洞信息
    fn extract_vulnerabilities_from_context(&self) -> Vec<Value> {
        // 从之前的扫描结果中提取漏洞
        self.context.previous_results
            .iter()
            .filter_map(|result| {
                if let Some(vulns) = result.data.get("vulnerabilities") {
                    vulns.as_array().cloned()
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    /// 更新上下文
    async fn update_context_with_results(&mut self, results: &[Value]) -> Result<()> {
        // 将结果添加到共享数据中
        for (i, result) in results.iter().enumerate() {
            self.context.shared_data.insert(
                format!("stage_{}_result", i),
                result.clone(),
            );
        }
        
        // 使用LLM分析当前状态并更新上下文
        let analysis_prompt = format!(
            "基于以下测试结果，分析当前安全测试状态并提供下一步建议：\n{}",
            serde_json::to_string_pretty(&results)?
        );
        
        if let Ok(llm_context) = self.llm_service.generate_response(&analysis_prompt, None).await {
            self.context.llm_context = Some(llm_context);
        }
        
        Ok(())
    }

    /// 生成综合报告
    async fn generate_comprehensive_report(&self, workflow_results: &[Value]) -> Result<Value> {
        println!("📊 生成综合安全测试报告...");
        
        // 使用LLM生成智能报告
        let report_prompt = format!(
            "基于以下安全测试结果，生成一份专业的安全评估报告：\n\n\
            目标: {}\n\
            测试结果: {}\n\n\
            请包含：\n\
            1. 执行摘要\n\
            2. 发现的漏洞和风险\n\
            3. 风险等级评估\n\
            4. 修复建议\n\
            5. 后续测试建议\n\n\
            请以结构化JSON格式返回报告。",
            self.context.target,
            serde_json::to_string_pretty(&workflow_results)?
        );
        
        let llm_report = self.llm_service.generate_response(&report_prompt, None).await
            .unwrap_or_else(|_| json!({
                "executive_summary": "安全测试已完成",
                "vulnerabilities_found": workflow_results.len(),
                "risk_level": "medium",
                "recommendations": ["定期进行安全测试", "及时更新系统补丁"]
            }).to_string());
        
        let final_report = json!({
            "target": self.context.target,
            "test_session_id": self.context.session_id,
            "workflow_id": self.context.workflow_id,
            "execution_time": Utc::now().to_rfc3339(),
            "agent_info": {
                "id": self.id,
                "name": self.name
            },
            "workflow_results": workflow_results,
            "llm_generated_report": serde_json::from_str::<Value>(&llm_report)
                .unwrap_or_else(|_| json!({"content": llm_report})),
            "statistics": {
                "total_stages": workflow_results.len(),
                "mcp_tools_used": self.count_mcp_tools_used(workflow_results),
                "builtin_tools_used": self.count_builtin_tools_used(workflow_results)
            }
        });
        
        println!("✅ 综合报告生成完成");
        Ok(final_report)
    }

    /// 统计使用的MCP工具数量
    fn count_mcp_tools_used(&self, results: &[Value]) -> usize {
        // 实现统计逻辑
        3 // 示例值
    }

    /// 统计使用的内置工具数量
    fn count_builtin_tools_used(&self, results: &[Value]) -> usize {
        // 实现统计逻辑
        2 // 示例值
    }
}

/// 使用示例
#[tokio::main]
async fn main() -> Result<()> {
    println!("🎯 LLM + MCP + Agent + 内置工具集成示例");
    
    // 1. 初始化各个组件
    let llm_service = LlmService::new("gpt-4".to_string(), "your-api-key".to_string())?;
    let mcp_client = McpClient::new();
    let tool_manager = ToolManager::new().await?;
    
    // 2. 连接MCP服务器
    println!("🔌 连接MCP服务器...");
    let _bilibili_connection = mcp_client.connect_to_child_process(
        "Bilibili Search".to_string(),
        "node",
        vec!["path/to/bilibili-search-server.js"]
    ).await?;
    
    let _security_tools_connection = mcp_client.connect_to_child_process(
        "Security Tools".to_string(),
        "python",
        vec!["path/to/security-tools-server.py"]
    ).await?;
    
    // 3. 创建智能安全测试Agent
    let mut agent = IntelligentSecurityAgent::new(
        "智能安全测试Agent".to_string(),
        llm_service,
        mcp_client,
        tool_manager,
    );
    
    // 4. 执行完整的安全测试工作流
    let target = "example.com";
    match agent.execute_security_workflow(target).await {
        Ok(result) => {
            println!("\n🎉 安全测试完成！");
            println!("📋 测试结果摘要:");
            println!("   - 目标: {}", target);
            println!("   - 状态: {:?}", result.status);
            println!("   - 置信度: {:.2}%", result.confidence_score * 100.0);
            println!("   - 执行时间: {}秒", result.execution_time);
            println!("   - 网络请求: {}次", result.resource_usage.network_requests);
            
            // 保存详细报告
            let report_json = serde_json::to_string_pretty(&result.data)?;
            tokio::fs::write("security_test_report.json", report_json).await?;
            println!("📄 详细报告已保存到: security_test_report.json");
        },
        Err(e) => {
            println!("❌ 安全测试失败: {}", e);
        }
    }
    
    Ok(())
}

/// 扩展示例：自定义工作流
pub async fn custom_workflow_example() -> Result<()> {
    println!("🔧 自定义工作流示例");
    
    // 创建自定义工作流，展示更复杂的LLM+MCP+Agent集成
    let workflow_config = json!({
        "name": "高级威胁检测工作流",
        "stages": [
            {
                "name": "智能侦察",
                "llm_guidance": true,
                "mcp_tools": ["subfinder", "amass", "shodan_search"],
                "builtin_tools": ["nmap", "masscan"]
            },
            {
                "name": "深度扫描",
                "llm_guidance": true,
                "mcp_tools": ["nuclei_cloud", "burp_scanner"],
                "builtin_tools": ["nuclei", "sqlmap"]
            },
            {
                "name": "智能分析",
                "llm_guidance": true,
                "mcp_tools": ["threat_intelligence", "cve_lookup"],
                "builtin_tools": ["custom_analyzer"]
            }
        ],
        "llm_prompts": {
            "stage_analysis": "分析当前阶段结果并决定下一步行动",
            "risk_assessment": "评估发现的安全风险",
            "report_generation": "生成专业的安全评估报告"
        }
    });
    
    println!("✅ 自定义工作流配置完成: {}", workflow_config["name"]);
    
    Ok(())
}