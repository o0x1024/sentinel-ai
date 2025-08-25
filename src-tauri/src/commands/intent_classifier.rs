//! 用户意图分类器
//! 
//! 用于区分用户输入是普通对话还是需要Agent执行的任务

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use log::{info, debug};
use crate::services::ai::AiServiceManager;
use std::sync::Arc;

/// 用户意图类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserIntent {
    /// 普通对话 - 问候、闲聊、简单回答
    Chat,
    /// 知识性问答 - 不需要工具调用的问题
    Question,
    /// 任务执行 - 需要Agent执行的具体任务
    Task,
}

/// 意图分类结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentClassificationResult {
    /// 分类的意图类型
    pub intent: UserIntent,
    /// 置信度 (0.0-1.0)
    pub confidence: f32,
    /// 分类理由
    pub reasoning: String,
    /// 是否需要Agent执行
    pub requires_agent: bool,
    /// 提取的关键信息
    pub extracted_info: HashMap<String, String>,
}

/// 轻量级意图分类器
pub struct IntentClassifier {
    ai_service_manager: Arc<AiServiceManager>,
    /// 任务关键词列表
    task_keywords: Vec<String>,
    /// 对话关键词列表
    chat_keywords: Vec<String>,
}

impl IntentClassifier {
    /// 创建新的意图分类器
    pub fn new(ai_service_manager: Arc<AiServiceManager>) -> Self {
        Self {
            ai_service_manager,
            task_keywords: vec![
                // 安全相关
                "扫描".to_string(), "scan".to_string(), "检测".to_string(), "detect".to_string(),
                "分析".to_string(), "analyze".to_string(), "测试".to_string(), "test".to_string(),
                "漏洞".to_string(), "vulnerability".to_string(), "安全".to_string(), "security".to_string(),
                "渗透".to_string(), "penetration".to_string(), "入侵".to_string(), "intrusion".to_string(),
                
                // 网络相关
                "端口".to_string(), "port".to_string(), "域名".to_string(), "domain".to_string(),
                "子域名".to_string(), "subdomain".to_string(), "IP".to_string(), "网站".to_string(), "website".to_string(),
                
                // 动作词
                "执行".to_string(), "运行".to_string(), "run".to_string(), "启动".to_string(), "start".to_string(),
                "搜索".to_string(), "search".to_string(), "查找".to_string(), "find".to_string(),
                "监控".to_string(), "monitor".to_string(), "跟踪".to_string(), "track".to_string(),
                
                // 工具相关
                "nmap".to_string(), "dirb".to_string(), "gobuster".to_string(), "nikto".to_string(),
                "sqlmap".to_string(), "masscan".to_string(), "ffuf".to_string(),
                
                // 目标指示词
                "目标".to_string(), "target".to_string(), "主机".to_string(), "host".to_string(),
                "服务器".to_string(), "server".to_string(), "应用".to_string(), "application".to_string(),
            ],
            chat_keywords: vec![
                // 问候
                "你好".to_string(), "hello".to_string(), "hi".to_string(), "嗨".to_string(),
                "早上好".to_string(), "晚上好".to_string(), "good morning".to_string(), "good evening".to_string(),
                
                // 感谢
                "谢谢".to_string(), "thanks".to_string(), "thank you".to_string(),
                
                // 状态询问
                "怎么样".to_string(), "如何".to_string(), "how".to_string(), "what".to_string(),
                "是什么".to_string(), "为什么".to_string(), "why".to_string(),
                
                // 功能询问
                "功能".to_string(), "feature".to_string(), "能力".to_string(), "capability".to_string(),
                "帮助".to_string(), "help".to_string(), "支持".to_string(), "support".to_string(),
            ],
        }
    }
    
    /// 快速关键词匹配分类
    fn quick_classify(&self, input: &str) -> Option<UserIntent> {
        let input_lower = input.to_lowercase();
        
        // 检查任务关键词
        let task_matches = self.task_keywords.iter()
            .filter(|keyword| input_lower.contains(&keyword.to_lowercase()))
            .count();
            
        // 检查对话关键词
        let chat_matches = self.chat_keywords.iter()
            .filter(|keyword| input_lower.contains(&keyword.to_lowercase()))
            .count();
        
        // 简单的启发式规则
        if task_matches > 0 && self.has_action_pattern(&input_lower) {
            Some(UserIntent::Task)
        } else if chat_matches > 0 && input.len() < 50 {
            Some(UserIntent::Chat)
        } else if self.is_simple_question(&input_lower) {
            Some(UserIntent::Question)
        } else {
            None // 需要LLM分析
        }
    }
    
    /// 检查是否包含动作模式
    fn has_action_pattern(&self, input: &str) -> bool {
        let action_patterns = [
            "帮我", "help me", "请", "please", "执行", "运行", "run", 
            "开始", "start", "进行", "perform", "做", "do"
        ];
        
        action_patterns.iter().any(|pattern| input.contains(pattern))
    }
    
    /// 检查是否为简单问题
    fn is_simple_question(&self, input: &str) -> bool {
        let question_patterns = [
            "是什么", "what is", "如何", "how to", "为什么", "why",
            "什么时候", "when", "在哪里", "where", "谁", "who"
        ];
        
        question_patterns.iter().any(|pattern| input.contains(pattern)) && 
        !self.task_keywords.iter().any(|keyword| input.contains(&keyword.to_lowercase()))
    }
    
    /// LLM深度分析意图
    async fn llm_classify(&self, input: &str) -> Result<IntentClassificationResult> {
        // 从prompt管理系统获取意图分析器的系统prompt
        let system_prompt = match self.get_intent_classifier_prompt().await {
            Ok(prompt) => prompt,
            Err(e) => {
                log::warn!("获取意图分析器prompt失败: {}, 使用默认prompt", e);
                self.get_default_intent_classifier_prompt()
            }
        };

        debug!("Intent classification system prompt: {}", system_prompt);
        debug!("User input: {}", input);
        
        // 使用调度策略中配置的意图分析模型，带有完整的降级策略
        let response = match self.ai_service_manager.get_service_for_stage(crate::services::ai::SchedulerStage::IntentAnalysis).await {
            Ok(Some(service)) => {
                info!("使用调度策略配置的意图分析模型: {}", service.get_config().model);
                match self.send_structured_request(&service, &system_prompt, input).await {
                    Ok(response) => response,
                    Err(e) => {
                        log::warn!("调度器配置的意图分析模型调用失败: {}, 降级到默认服务", e);
                        return Err(e);
                    }
                }
            }
            Ok(None) => {
                log::warn!("调度策略中未配置意图分析模型，使用默认服务");
                return Err(anyhow!("调度策略中未配置意图分析模型"));
            }
            Err(e) => {
                log::warn!("获取调度策略意图分析模型失败: {}, 使用默认服务", e);
                return Err(e);
            }
        };
        
        // 尝试解析JSON响应
        match serde_json::from_str::<IntentClassificationResult>(&response) {
            Ok(result) => Ok(result),
            Err(_) => {
                // 如果JSON解析失败，使用简单的文本分析
                let intent = if response.to_lowercase().contains("task") {
                    UserIntent::Task
                } else if response.to_lowercase().contains("question") {
                    UserIntent::Question
                } else {
                    UserIntent::Chat
                };
                
                Ok(IntentClassificationResult {
                    intent: intent.clone(),
                    confidence: 0.7,
                    reasoning: "基于LLM响应文本分析".to_string(),
                    requires_agent: matches!(intent, UserIntent::Task),
                    extracted_info: HashMap::new(),
                })
            }
        }
    }
    
    /// 从prompt管理系统获取意图分析器prompt
    async fn get_intent_classifier_prompt(&self) -> Result<String> {
        // 这里应该连接到prompt管理系统
        // 暂时使用全局实例，实际应该通过依赖注入
        use crate::prompt::prompt_config::*;
        
        let prompt_manager = PromptConfigManager::new();
        
        match prompt_manager.get_system_prompt_by_type(&TemplateType::IntentClassifier).await {
            Ok(Some(prompt)) => Ok(prompt),
            Ok(None) => Ok(self.get_default_intent_classifier_prompt()),
            Err(e) => {
                log::warn!("获取意图分析器prompt失败: {}", e);
                Ok(self.get_default_intent_classifier_prompt())
            }
        }
    }
    
    /// 获取默认的意图分析器prompt
    fn get_default_intent_classifier_prompt(&self) -> String {
        r#"作为一个AI意图分类器，请分析用户输入并判断意图类型。

请判断用户输入属于以下哪种类型：
1. Chat - 普通对话（问候、闲聊、简单交流）
2. Question - 知识性问答（询问概念、原理等，不需要实际执行）  
3. Task - 任务执行（需要AI助手执行具体的安全扫描、分析等操作）

判断标准：
- Chat: 问候语、感谢、简单交流等
- Question: 以"什么是"、"如何理解"等开头的概念性问题
- Task: 包含"扫描"、"检测"、"分析"、"帮我执行"等行动指令

请以JSON格式回复：
{
    "intent": "Chat|Question|Task",
    "confidence": 0.0-1.0,
    "reasoning": "分类理由",
    "requires_agent": true/false,
    "extracted_info": {"key": "value"}
}"#.to_string()
    }
    
    /// 使用结构化的system/user角色发送请求
    async fn send_structured_request(
        &self, 
        service: &crate::services::ai::AiService, 
        system_prompt: &str, 
        user_input: &str
    ) -> Result<String> {

        service.send_message_stream(user_input, Some(system_prompt), None, false, None).await
            .map_err(|e| anyhow!("AI服务请求失败: {}", e))
    }


    /// 分类用户意图（主入口）
    pub async fn classify_intent(&self, input: &str) -> Result<IntentClassificationResult> {
        info!("开始分类用户意图: {}", input);
        
        // 1. 首先尝试快速分类
        if let Some(quick_intent) = self.quick_classify(input) {
            info!("快速分类结果: {:?}", quick_intent);
            return Ok(IntentClassificationResult {
                intent: quick_intent.clone(),
                confidence: 0.85,
                reasoning: "基于关键词快速匹配".to_string(),
                requires_agent: matches!(quick_intent, UserIntent::Task),
                extracted_info: self.extract_basic_info(input),
            });
        }
        
        // 2. 快速分类失败，使用LLM深度分析
        info!("快速分类无结果，启用LLM深度分析");
        match self.llm_classify(input).await {
            Ok(result) => {
                info!("LLM分类结果: {:?}, 置信度: {}", result.intent, result.confidence);
                Ok(result)
            }
            Err(e) => {
                log::warn!("LLM分类失败: {}, 使用默认分类", e);
                // 降级到默认策略
                Ok(IntentClassificationResult {
                    intent: UserIntent::Question, // 默认当作问题处理
                    confidence: 0.5,
                    reasoning: format!("LLM分类失败，使用默认策略: {}", e),
                    requires_agent: false,
                    extracted_info: HashMap::new(),
                })
            }
        }
    }
    
    /// 提取基本信息
    fn extract_basic_info(&self, input: &str) -> HashMap<String, String> {
        let mut info = HashMap::new();
        
        // 提取可能的目标域名或IP
        if let Some(target) = self.extract_target(input) {
            info.insert("target".to_string(), target);
        }
        
        // 提取任务类型
        if let Some(task_type) = self.extract_task_type(input) {
            info.insert("task_type".to_string(), task_type);
        }
        
        info
    }
    
    /// 提取目标
    fn extract_target(&self, input: &str) -> Option<String> {
        // 简单的域名/IP提取逻辑
        let words: Vec<&str> = input.split_whitespace().collect();
        for word in words {
            if word.contains('.') && (word.contains("com") || word.contains("cn") || word.contains("org")) {
                return Some(word.to_string());
            }
        }
        None
    }
    
    /// 提取任务类型
    fn extract_task_type(&self, input: &str) -> Option<String> {
        let input_lower = input.to_lowercase();
        if input_lower.contains("扫描") || input_lower.contains("scan") {
            Some("scan".to_string())
        } else if input_lower.contains("分析") || input_lower.contains("analyze") {
            Some("analyze".to_string())
        } else if input_lower.contains("检测") || input_lower.contains("detect") {
            Some("detect".to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    
    #[test]
    fn test_quick_classify_task() {
        // 这里可以添加单元测试
        // 由于依赖AiServiceManager，实际测试需要mock
    }
    
    #[test]
    fn test_extract_target() {
        // 由于需要 AiServiceManager，这里暂时跳过实际测试
        // let classifier = IntentClassifier::new(Arc::new(todo!()));
        // assert_eq!(
        //     classifier.extract_target("扫描 example.com 的端口"),
        //     Some("example.com".to_string())
        // );
    }
}
