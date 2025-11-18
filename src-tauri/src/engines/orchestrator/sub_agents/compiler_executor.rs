use crate::agents::orchestrator::sub_agent_interface::*;
use crate::models::security_testing::*;
use crate::services::AiServiceManager;
use crate::services::database::DatabaseService;
use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;

/// LLM-Compiler sub-agent executor
pub struct CompilerSubAgentExecutor {
    _ai_service_manager: Arc<AiServiceManager>,
    _db_service: Arc<DatabaseService>,
}

impl CompilerSubAgentExecutor {
    pub fn new(
        ai_service_manager: Arc<AiServiceManager>,
        db_service: Arc<DatabaseService>,
    ) -> Self {
        Self {
            _ai_service_manager: ai_service_manager,
            _db_service: db_service,
        }
    }
}

// Removed Default implementation - requires dependencies

impl CompilerSubAgentExecutor {
    /// Build security-focused prompt for LLM-Compiler
    /// Checks for Orchestrator's prompt configuration first, falls back to default if not found
    fn build_security_prompt(&self, context: &SubAgentContext, language: Option<&str>) -> String {
        let lang = language.unwrap_or("python");
        
        // Try to get evaluator prompt from Orchestrator's configuration
        if let Some(prompts) = context.task_parameters.get("prompts") {
            if let Some(evaluator_prompt) = prompts.get("evaluator").and_then(|v| v.as_str()) {
                if !evaluator_prompt.trim().is_empty() {
                    log::info!("LLM-Compiler sub-agent: Using evaluator prompt from Orchestrator configuration");
                    let mut context_vars = self.build_context_variables(context);
                    context_vars.insert("language".to_string(), lang.to_string());
                    return self.replace_prompt_variables(evaluator_prompt, &context_vars);
                }
            }
        }
        
        // Fallback to default hardcoded prompt
        log::info!("LLM-Compiler sub-agent: Using default hardcoded prompt");
        self.build_default_security_prompt(context, lang)
    }
    
    /// Build default security prompt (original hardcoded logic)
    fn build_default_security_prompt(&self, context: &SubAgentContext, lang: &str) -> String {
        
        let mut prompt = format!(
            "Generate {} code for the following security testing objective:\n\n\
            Objective: {}\n\
            Target: {}\n\
            Task type: {:?}\n",
            lang,
            context.objective,
            context.primary_target,
            context.task_kind
        );
        
        // Add task-specific guidance
        match context.task_kind {
            SecurityTaskKind::WebPentest | SecurityTaskKind::APIPentest => {
                prompt.push_str(
                    "\nGenerate code that:\n\
                    - Handles HTTP requests properly\n\
                    - Includes authentication if needed\n\
                    - Has proper error handling\n\
                    - Outputs results in a structured format\n\
                    - Includes comments explaining the approach\n"
                );
                
                if context.auth_context.is_authenticated() {
                    prompt.push_str("\nAuthentication context is available. Include it in requests.\n");
                }
            }
            SecurityTaskKind::Forensics => {
                prompt.push_str(
                    "\nGenerate code that:\n\
                    - Preserves evidence integrity\n\
                    - Handles various log formats\n\
                    - Extracts relevant indicators\n\
                    - Maintains chain of custody\n"
                );
            }
            SecurityTaskKind::CTF => {
                prompt.push_str(
                    "\nGenerate code that:\n\
                    - Exploits the identified vulnerability\n\
                    - Handles edge cases\n\
                    - Extracts the flag\n\
                    - Is well-commented for writeup\n"
                );
            }
            SecurityTaskKind::ReverseEngineering => {
                prompt.push_str(
                    "\nGenerate code that:\n\
                    - Automates analysis tasks\n\
                    - Handles binary formats\n\
                    - Extracts relevant information\n\
                    - Integrates with analysis tools\n"
                );
            }
            SecurityTaskKind::OtherSecurity => {
                prompt.push_str(
                    "\nGenerate well-structured, secure code with proper error handling.\n"
                );
            }
        }
        
        // Add context from previous steps
        if !context.previous_steps.is_empty() {
            prompt.push_str("\nContext from previous steps:\n");
            for step in context.previous_steps.iter().rev().take(3) {
                if let Some(output) = &step.output {
                    prompt.push_str(&format!("- {}: {}\n", step.summary, output));
                }
            }
        }
        
        // Add findings context
        if !context.findings.is_empty() {
            prompt.push_str("\nRelevant findings:\n");
            for finding in context.findings.iter().rev().take(3) {
                prompt.push_str(&format!("- {}: {}\n", finding.title, finding.location));
            }
        }
        
        prompt.push_str(
            "\nProvide:\n\
            1. Complete, working code\n\
            2. Clear explanation of how it works\n\
            3. Usage instructions\n\
            4. Any dependencies required\n"
        );
        
        prompt
    }
    
    /// Build context variables for prompt replacement
    fn build_context_variables(&self, context: &SubAgentContext) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("primary_target".to_string(), context.primary_target.clone());
        vars.insert("objective".to_string(), context.objective.clone());
        vars.insert("task_kind".to_string(), format!("{:?}", context.task_kind));
        vars.insert("current_stage".to_string(), format!("{:?}", context.current_stage));
        vars.insert("is_authenticated".to_string(), context.auth_context.is_authenticated().to_string());
        vars.insert("findings_count".to_string(), context.findings.len().to_string());
        vars.insert("steps_count".to_string(), context.previous_steps.len().to_string());
        
        // Build previous steps summary
        if !context.previous_steps.is_empty() {
            let steps_summary = context.previous_steps.iter().rev().take(3)
                .filter_map(|s| s.output.as_ref().map(|o| format!("- {}: {}", s.summary, o)))
                .collect::<Vec<_>>()
                .join("\n");
            vars.insert("previous_steps".to_string(), if steps_summary.is_empty() { "None".to_string() } else { steps_summary });
        } else {
            vars.insert("previous_steps".to_string(), "None".to_string());
        }
        
        // Build findings summary
        if !context.findings.is_empty() {
            let findings_summary = context.findings.iter().rev().take(3)
                .map(|f| format!("- {}: {}", f.title, f.location))
                .collect::<Vec<_>>()
                .join("\n");
            vars.insert("findings".to_string(), findings_summary);
        } else {
            vars.insert("findings".to_string(), "None".to_string());
        }
        
        vars
    }
    
    /// Replace variables in prompt template
    fn replace_prompt_variables(&self, template: &str, vars: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            let placeholder_curly = format!("{{{}}}", key);
            let placeholder_double = format!("{{{{{}}}}}", key.to_uppercase());
            result = result.replace(&placeholder_curly, value);
            result = result.replace(&placeholder_double, value);
        }
        result
    }
}

#[async_trait::async_trait]
impl SubAgentExecutor for CompilerSubAgentExecutor {
    async fn execute(&self, request: SubAgentRequest) -> Result<SubAgentResponse> {
        log::info!(
            "LLM-Compiler sub-agent executing: session={}, objective={}",
            request.session_id,
            request.context.objective
        );
        
        // Extract language from objective or use default
        let language = if request.context.objective.to_lowercase().contains("python") {
            "python"
        } else if request.context.objective.to_lowercase().contains("javascript") {
            "javascript"
        } else if request.context.objective.to_lowercase().contains("bash") {
            "bash"
        } else {
            "python"
        };
        
        let prompt = self.build_security_prompt(&request.context, Some(language));
        
        // TODO: Call actual LLM-Compiler engine with the prompt
        // For now, return a mock response
        let mock_code = match request.context.task_kind {
            SecurityTaskKind::WebPentest | SecurityTaskKind::APIPentest => {
                format!(
                    "import requests\n\
                    \n\
                    def test_endpoint(url, auth_token=None):\n\
                        \"\"\"Test security of an API endpoint\"\"\"\n\
                        headers = {{}}\n\
                        if auth_token:\n\
                            headers['Authorization'] = f'Bearer {{auth_token}}'\n\
                        \n\
                        response = requests.get(url, headers=headers)\n\
                        return {{\n\
                            'status': response.status_code,\n\
                            'headers': dict(response.headers),\n\
                            'body': response.text\n\
                        }}\n\
                    \n\
                    # Usage:\n\
                    # result = test_endpoint('{}')\n\
                    # print(result)",
                    request.context.primary_target
                )
            }
            _ => {
                format!(
                    "# Security testing script for: {}\n\
                    # Objective: {}\n\
                    \n\
                    def main():\n\
                        # Implementation here\n\
                        pass\n\
                    \n\
                    if __name__ == '__main__':\n\
                        main()",
                    request.context.primary_target,
                    request.context.objective
                )
            }
        };
        
        let explanation = format!(
            "This {} script implements: {}\n\n\
            Key features:\n\
            - Handles authentication\n\
            - Proper error handling\n\
            - Structured output\n\
            - Well-commented code",
            language,
            request.context.objective
        );
        
        let usage = format!(
            "To use this script:\n\
            1. Install dependencies: pip install requests\n\
            2. Run: python script.py\n\
            3. Review output for findings"
        );
        
        Ok(SubAgentResponse::success(
            SubAgentKind::LLMCompiler,
            SubAgentOutput::Code {
                language: language.to_string(),
                code: mock_code,
                explanation,
                usage,
            },
        ))
    }
}

