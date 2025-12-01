//! 查询分析器模块
//! 
//! 负责分析用户输入的特征，包括：
//! - 任务类型识别
//! - 复杂度评估
//! - 并行化潜力分析
//! - 资源需求评估

use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::services::ai::AiServiceManager;
use crate::ai_adapter::types::AiProvider;
use log::info;

/// 查询分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysisResult {
    /// 任务类型
    pub task_type: String,
    /// 子类别
    pub sub_category: String,
    /// 复杂度级别 (simple|medium|complex)
    pub complexity_level: String,
    /// 并行化潜力 (high|medium|low)
    pub parallelization_potential: String,
    /// 时间敏感性 (high|medium|low)
    pub time_sensitivity: String,
    /// 依赖复杂度 (simple|medium|complex)
    pub dependency_complexity: String,
    /// 预估步骤数
    pub estimated_steps: u32,
    /// 资源需求 (light|medium|heavy)
    pub resource_requirements: String,
    /// 关键指标
    pub key_indicators: Vec<String>,
    /// 目标域名或IP
    pub target_domain: Option<String>,
    /// 置信度分数
    pub confidence: f32,
}

/// 查询分析器
pub struct QueryAnalyzer {
    /// AI提供者
    ai_provider: Arc<dyn AiProvider>,
    /// AI服务管理器
    ai_service_manager: Arc<AiServiceManager>,
}

impl QueryAnalyzer {
    /// 创建带有AI服务管理器的查询分析器
    pub fn new_with_service_manager(
        ai_provider: Arc<dyn AiProvider>,
        ai_service_manager: Arc<AiServiceManager>,
    ) -> Self {
        Self {
            ai_provider,
            ai_service_manager,
        }
    }

    /// 分析用户输入
    pub async fn analyze_query(&self, user_input: &str) -> Result<QueryAnalysisResult> {
        info!("Starting query analysis for: {}", user_input);

        // 构建分析Prompt
        let analysis_prompt = self.build_analysis_prompt(user_input);
        
        // 动态选择模型和提供商：优先使用调度器配置，其次使用默认模型
        let (model, provider) = match self.ai_service_manager.get_scheduler_config().await {
            Ok(cfg) if !cfg.intent_analysis_model.trim().is_empty() => {
                let model = cfg.intent_analysis_model.clone();
                let provider_name = if !cfg.intent_analysis_provider.trim().is_empty() {
                    cfg.intent_analysis_provider.clone()
                } else {
                    // 如果没有配置提供商，尝试从模型名推断
                    self.infer_provider_from_model(&model)
                };
                (model, provider_name)
            },
            _ => {
                // 从AI服务获取一个默认模型（如果实现了）
                match self.ai_service_manager.get_default_model("chat").await {
                    Ok(Some(m)) => (m.name, m.provider),
                    _ => {
                        // 最后回退：使用空模型，让当前provider自行决定
                        (String::new(), String::new())
                    }
                }
            }
        };

        // 根据提供商获取对应的AI适配器
        let ai_provider = if !provider.is_empty() {
            use crate::ai_adapter::core::AiAdapterManager;
            match AiAdapterManager::global().get_provider_or_default(&provider) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Failed to get provider '{}': {}, falling back to default", provider, e);
                    self.ai_provider.clone()
                }
            }
        } else {
            self.ai_provider.clone()
        };

        let chat_request = crate::ai_adapter::types::ChatRequest {
            model,
            messages: vec![crate::ai_adapter::types::Message::system(&analysis_prompt)],
            tools: None,
            tool_choice: None,
            user: None,
            extra_params: None,
            options: None,
        };
        
        // 使用动态选择的提供商发送请求
        let mut stream = ai_provider.send_chat_stream(&chat_request).await
            .map_err(|e| anyhow::anyhow!("AI analysis failed: {}", e))?;

        let mut content = String::new();

        // 收集流式响应
        use futures::StreamExt;
        while let Some(chunk_result) = stream.stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    //需要处理报错 EOF while parsing a value at line 1 column 0    
                    if !chunk.content.is_empty() {
                        let choice = serde_json::from_str::<serde_json::Value>(&chunk.content)?;
                        // 取content的值  INFO sentinel_ai_lib::engines::intelligent_dispatcher::query_analyzer: 107: choice: Object {"choices": Array [Object {"delta": Object {"content": String(""), "role": String("assistant")}, "finish_reason": Null, "index": Number(0)}], "created": Number(1756283183), "id": String("chatcmpl-68aec12f1e91f778fae1cc59"), "model": String("moonshot-v1-8k"), "object": String("chat.completion.chunk"), "system_fingerprint": String("fpv0_ff52a3ef")}
                        let content_str = choice["choices"][0]["delta"]["content"].as_str().unwrap_or("");
                        content.push_str(content_str);
                    }
                }
                Err(e) => return Err(anyhow::anyhow!("Stream error: {}", e)),
            }
        }
            
        let response = content;
        // tracing::info!("response: {}", response);

        // 解析AI响应
        let analysis_result = self.parse_analysis_response(&response, user_input)?;
        
        info!("Query analysis completed: task_type={}, complexity={}, parallelization={}", 
              analysis_result.task_type, 
              analysis_result.complexity_level,
              analysis_result.parallelization_potential);

        Ok(analysis_result)
    }

    /// 根据模型名推断提供商
    fn infer_provider_from_model(&self, model: &str) -> String {
        if model.contains("moonshot") || model.contains("kimi") {
            "moonshot".to_string()
        } else if model.contains("gpt") || model.contains("openai") {
            "openai".to_string()
        } else if model.contains("claude") || model.contains("anthropic") {
            "anthropic".to_string()
        } else if model.contains("deepseek") {
            "deepseek".to_string()
        } else if model.contains("gemini") || model.contains("google") {
            "gemini".to_string()
        } else if model.contains("qwen") || model.contains("baichuan") {
            "modelscope".to_string()
        } else if model.contains("groq") {
            "groq".to_string()
        } else if model.contains("openrouter") {
            "openrouter".to_string()
        } else {
            // 默认回退
            String::new()
        }
    }

    /// 构建分析Prompt
    fn build_analysis_prompt(&self, user_input: &str) -> String {
        format!(
            r#"
你是一个专业的任务分析专家，请分析以下用户输入并以JSON格式返回分析结果：

用户输入: "{}"

请按以下格式分析：
{{
    "task_type": "扫描任务|分析任务|查询任务|配置任务|监控任务|其他",
    "sub_category": "具体子类别",
    "complexity_level": "simple|medium|complex",
    "parallelization_potential": "high|medium|low",
    "time_sensitivity": "high|medium|low", 
    "dependency_complexity": "simple|medium|complex",
    "estimated_steps": 数字,
    "resource_requirements": "light|medium|heavy",
    "key_indicators": ["关键词1", "关键词2"],
    "target_domain": "目标域名或IP（如果有）",
    "confidence": 0.0-1.0
}}

分析要点：
1. 任务类型：根据用户意图判断主要任务类型
2. 复杂度：simple(1-3步)、medium(4-10步)、complex(10+步)
3. 并行化潜力：评估任务是否可以并行执行
4. 时间敏感性：任务的紧急程度
5. 依赖复杂度：任务间的依赖关系复杂程度
6. 预估步骤数：完成任务所需的大致步骤数
7. 资源需求：CPU、内存、网络等资源消耗水平
8. 关键指标：提取查询中的关键词
9. 目标域名：如果涉及特定目标，提取出来
10. 置信度：对分析结果的信心程度

请仅返回JSON，不要添加其他解释。
"#,
            user_input
        )
    }

    /// 解析AI响应
    fn parse_analysis_response(&self, response: &str, user_input: &str) -> Result<QueryAnalysisResult> {
        // 尝试从响应中提取JSON
        let json_start = response.find('{');
        let json_end = response.rfind('}');
        
        let json_str = if let (Some(start), Some(end)) = (json_start, json_end) {
            &response[start..=end]
        } else {
            response
        };

        // 尝试解析JSON
        match serde_json::from_str::<QueryAnalysisResult>(json_str) {
            Ok(result) => Ok(result),
            Err(_) => {
                // 如果解析失败，返回基于规则的分析结果
                info!("AI response parsing failed, using rule-based analysis");
                Ok(self.rule_based_analysis(user_input))
            }
        }
    }

    /// 基于规则的分析（作为AI分析的回退）
    fn rule_based_analysis(&self, user_input: &str) -> QueryAnalysisResult {
        let input_lower = user_input.to_lowercase();
        
        // 任务类型识别
        let task_type = if input_lower.contains("扫描") || input_lower.contains("scan") {
            "扫描任务"
        } else if input_lower.contains("分析") || input_lower.contains("analyze") {
            "分析任务"
        } else if input_lower.contains("查询") || input_lower.contains("search") || input_lower.contains("find") {
            "查询任务"
        } else if input_lower.contains("配置") || input_lower.contains("config") || input_lower.contains("设置") {
            "配置任务"
        } else if input_lower.contains("监控") || input_lower.contains("monitor") {
            "监控任务"
        } else {
            "其他"
        }.to_string();

        // 复杂度评估
        let complexity_level = if input_lower.len() > 100 || 
                                 input_lower.matches("并且").count() > 2 ||
                                 input_lower.matches("然后").count() > 2 {
            "complex"
        } else if input_lower.len() > 50 || 
                  input_lower.matches("并且").count() > 0 ||
                  input_lower.matches("然后").count() > 0 {
            "medium"
        } else {
            "simple"
        }.to_string();

        // 并行化潜力
        let parallelization_potential = if input_lower.contains("同时") || 
                                          input_lower.contains("并行") ||
                                          input_lower.contains("批量") {
            "high"
        } else if task_type == "扫描任务" || task_type == "查询任务" {
            "medium"
        } else {
            "low"
        }.to_string();

        // 预估步骤数
        let estimated_steps = match complexity_level.as_str() {
            "simple" => 2,
            "medium" => 6,
            "complex" => 12,
            _ => 4,
        };

        // 提取关键指标
        let key_indicators = self.extract_keywords(&input_lower);

        // 提取目标域名
        let target_domain = self.extract_domain(&input_lower);

        QueryAnalysisResult {
            task_type,
            sub_category: "通用".to_string(),
            complexity_level: complexity_level.clone(),
            parallelization_potential,
            time_sensitivity: if input_lower.contains("紧急") || input_lower.contains("立即") {
                "high"
            } else {
                "medium"
            }.to_string(),
            dependency_complexity: if complexity_level == "complex" {
                "complex"
            } else {
                "simple"
            }.to_string(),
            estimated_steps,
            resource_requirements: match complexity_level.as_str() {
                "simple" => "light",
                "medium" => "medium", 
                "complex" => "heavy",
                _ => "medium",
            }.to_string(),
            key_indicators,
            target_domain,
            confidence: 0.7, // 规则基础分析的置信度设为0.7
        }
    }

    /// 提取关键词
    fn extract_keywords(&self, input: &str) -> Vec<String> {
        let keywords = vec![
            "扫描", "scan", "端口", "port", "漏洞", "vulnerability",
            "分析", "analyze", "检测", "detect", "监控", "monitor",
            "查询", "query", "搜索", "search", "配置", "config"
        ];
        
        keywords.into_iter()
            .filter(|&keyword| input.contains(keyword))
            .map(|s| s.to_string())
            .collect()
    }

    /// 提取域名或IP
    fn extract_domain(&self, input: &str) -> Option<String> {
        // 简单的域名和IP提取逻辑
        let words: Vec<&str> = input.split_whitespace().collect();
        
        for word in words {
            // 检查IP地址格式
            if self.is_ip_address(word) {
                return Some(word.to_string());
            }
            
            // 检查域名格式
            if self.is_domain_name(word) {
                return Some(word.to_string());
            }
        }
        
        None
    }

    /// 检查是否为IP地址
    fn is_ip_address(&self, s: &str) -> bool {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        
        parts.iter().all(|part| {
            part.parse::<u8>().is_ok()
        })
    }

    /// 检查是否为域名
    fn is_domain_name(&self, s: &str) -> bool {
        s.contains('.') && 
        s.len() > 3 && 
        !s.starts_with('.') && 
        !s.ends_with('.') &&
        s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    }
}

