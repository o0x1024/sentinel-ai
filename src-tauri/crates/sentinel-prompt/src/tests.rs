#[cfg(test)]
mod rag_prompt_tests {
    use super::super::prompt_builder::*;
    use crate::prompt::prompt_config::PromptConfigManager;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_format_rag_context_empty() {
        let config_manager = PromptConfigManager::new();
        let builder = PromptBuilder::new(config_manager);
        
        let rag_context = RagContext {
            enabled: true,
            retrieved_documents: vec![],
            formatted_context: String::new(),
            retrieval_config: RagRetrievalConfig {
                collection_name: Some("test_collection".to_string()),
                top_k: 5,
                use_mmr: false,
                mmr_lambda: 0.7,
                similarity_threshold: 0.5,
            },
            token_budget: None,
        };

        let result = builder.format_rag_context(&rag_context).unwrap();
        assert!(result.contains("无相关上下文"));
    }

    #[tokio::test]
    async fn test_format_rag_context_with_documents() {
        let config_manager = PromptConfigManager::new();
        let builder = PromptBuilder::new(config_manager);
        
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "documentation".to_string());
        
        let rag_context = RagContext {
            enabled: true,
            retrieved_documents: vec![
                RagDocument {
                    id: "doc1".to_string(),
                    content: "这是一个测试文档的内容".to_string(),
                    score: 0.85,
                    metadata: metadata.clone(),
                    source: "test_source.md".to_string(),
                },
                RagDocument {
                    id: "doc2".to_string(),
                    content: "这是另一个测试文档".to_string(),
                    score: 0.72,
                    metadata: HashMap::new(),
                    source: "another_source.txt".to_string(),
                },
            ],
            formatted_context: "formatted context".to_string(),
            retrieval_config: RagRetrievalConfig {
                collection_name: Some("test_collection".to_string()),
                top_k: 5,
                use_mmr: true,
                mmr_lambda: 0.7,
                similarity_threshold: 0.5,
            },
            token_budget: Some(TokenBudget {
                max_tokens: 1000,
                used_tokens: 200,
                priority_strategy: ContextPriorityStrategy::BySimilarity,
            }),
        };

        let result = builder.format_rag_context(&rag_context).unwrap();
        
        assert!(result.contains("相关上下文"));
        assert!(result.contains("文档 1"));
        assert!(result.contains("文档 2"));
        assert!(result.contains("0.85"));
        assert!(result.contains("0.72"));
        assert!(result.contains("test_source.md"));
        assert!(result.contains("another_source.txt"));
        assert!(result.contains("这是一个测试文档的内容"));
        assert!(result.contains("这是另一个测试文档"));
        assert!(result.contains("test_collection"));
        assert!(result.contains("Top-K: 5"));
    }

    #[tokio::test]
    async fn test_build_planner_prompt_with_rag() {
        let config_manager = PromptConfigManager::new();
        let builder = PromptBuilder::new(config_manager);
        
        let rag_context = RagContext {
            enabled: true,
            retrieved_documents: vec![
                RagDocument {
                    id: "doc1".to_string(),
                    content: "相关的安全扫描文档内容".to_string(),
                    score: 0.9,
                    metadata: HashMap::new(),
                    source: "security_guide.md".to_string(),
                },
            ],
            formatted_context: "formatted context".to_string(),
            retrieval_config: RagRetrievalConfig {
                collection_name: Some("security_docs".to_string()),
                top_k: 3,
                use_mmr: false,
                mmr_lambda: 0.7,
                similarity_threshold: 0.6,
            },
            token_budget: None,
        };
        
        let context = PromptBuildContext {
            user_query: "执行安全扫描".to_string(),
            target_info: None,
            available_tools: vec![],
            execution_context: None,
            history: vec![],
            custom_variables: HashMap::new(),
            rag_context: Some(rag_context),
        };

        let result = builder.build_planner_prompt(&context).await;
        assert!(result.is_ok());
        
        let prompt_result = result.unwrap();
        assert!(prompt_result.prompt.contains("执行安全扫描"));
        assert!(prompt_result.variable_mapping.contains_key("rag_context"));
    }

    #[tokio::test]
    async fn test_build_executor_prompt_with_rag() {
        let config_manager = PromptConfigManager::new();
        let builder = PromptBuilder::new(config_manager);
        
        let rag_context = RagContext {
            enabled: true,
            retrieved_documents: vec![
                RagDocument {
                    id: "tool_doc".to_string(),
                    content: "工具使用说明和最佳实践".to_string(),
                    score: 0.88,
                    metadata: HashMap::new(),
                    source: "tool_manual.md".to_string(),
                },
            ],
            formatted_context: "formatted context".to_string(),
            retrieval_config: RagRetrievalConfig {
                collection_name: Some("tool_docs".to_string()),
                top_k: 2,
                use_mmr: true,
                mmr_lambda: 0.8,
                similarity_threshold: 0.7,
            },
            token_budget: None,
        };
        
        let context = PromptBuildContext {
            user_query: "使用nmap扫描端口".to_string(),
            target_info: None,
            available_tools: vec![],
            execution_context: None,
            history: vec![],
            custom_variables: HashMap::new(),
            rag_context: Some(rag_context),
        };

        let result = builder.build_executor_prompt(&context, "执行端口扫描").await;
        assert!(result.is_ok());
        
        let prompt_result = result.unwrap();
        assert!(prompt_result.prompt.contains("使用nmap扫描端口"));
        assert!(prompt_result.variable_mapping.contains_key("rag_context"));
    }

    #[tokio::test]
    async fn test_rag_context_disabled() {
        let config_manager = PromptConfigManager::new();
        let builder = PromptBuilder::new(config_manager);
        
        let rag_context = RagContext {
            enabled: false, // 禁用RAG
            retrieved_documents: vec![
                RagDocument {
                    id: "doc1".to_string(),
                    content: "这个内容不应该被包含".to_string(),
                    score: 0.9,
                    metadata: HashMap::new(),
                    source: "test.md".to_string(),
                },
            ],
            formatted_context: "formatted context".to_string(),
            retrieval_config: RagRetrievalConfig {
                collection_name: Some("test_collection".to_string()),
                top_k: 5,
                use_mmr: false,
                mmr_lambda: 0.7,
                similarity_threshold: 0.5,
            },
            token_budget: None,
        };
        
        let context = PromptBuildContext {
            user_query: "测试查询".to_string(),
            target_info: None,
            available_tools: vec![],
            execution_context: None,
            history: vec![],
            custom_variables: HashMap::new(),
            rag_context: Some(rag_context),
        };

        let result = builder.build_planner_prompt(&context).await;
        assert!(result.is_ok());
        
        let prompt_result = result.unwrap();
        // 当RAG被禁用时，不应该包含rag_context变量
        assert!(!prompt_result.variable_mapping.contains_key("rag_context"));
    }

    #[tokio::test]
    async fn test_rag_context_empty_formatted_context() {
        let config_manager = PromptConfigManager::new();
        let builder = PromptBuilder::new(config_manager);
        
        let rag_context = RagContext {
            enabled: true,
            retrieved_documents: vec![],
            formatted_context: String::new(), // 空的格式化上下文
            retrieval_config: RagRetrievalConfig {
                collection_name: Some("test_collection".to_string()),
                top_k: 5,
                use_mmr: false,
                mmr_lambda: 0.7,
                similarity_threshold: 0.5,
            },
            token_budget: None,
        };
        
        let context = PromptBuildContext {
            user_query: "测试查询".to_string(),
            target_info: None,
            available_tools: vec![],
            execution_context: None,
            history: vec![],
            custom_variables: HashMap::new(),
            rag_context: Some(rag_context),
        };

        let result = builder.build_planner_prompt(&context).await;
        assert!(result.is_ok());
        
        let prompt_result = result.unwrap();
        // 当formatted_context为空时，不应该包含rag_context变量
        assert!(!prompt_result.variable_mapping.contains_key("rag_context"));
    }
}