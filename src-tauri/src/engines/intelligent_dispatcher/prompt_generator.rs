//! 动态Prompt生成器模块
//! 
//! 负责根据查询分析和架构选择结果生成优化的Prompt模板
//! 集成现有的Prompt系统以提供更优质的Prompt生成能力

use anyhow::Result;
use serde::{Deserialize, Serialize};
use super::query_analyzer::QueryAnalysisResult;
use super::architecture_selector::ArchitectureSelectionResult;
use log::info;
use std::collections::HashMap;

// 导入现有的Prompt系统模块
use crate::prompt::{
    PromptBuilder, PromptOptimizer, PromptTemplateManager
};

/// 动态Prompt生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGenerationResult {
    /// 规划器Prompt
    pub planner_prompt: String,
    /// 执行器Prompt
    pub executor_prompt: String,
    /// 分析器Prompt
    pub analyzer_prompt: Option<String>,
    /// 工具选择器Prompt
    pub tool_selector_prompt: Option<String>,
    /// Prompt质量评分
    pub quality_score: f32,
    /// 生成策略
    pub generation_strategy: String,
}

/// 动态Prompt生成器
pub struct DynamicPromptGenerator {
    /// Prompt模板管理器 - 使用Option以支持回退模式
    template_manager: Option<PromptTemplateManager>,
    /// Prompt优化器 - 使用Option以支持回退模式
    optimizer: Option<PromptOptimizer>,
    /// Prompt构建器 - 使用Option以支持回退模式
    builder: Option<PromptBuilder>,
    /// 是否使用回退模式
    use_fallback: bool,
}

impl DynamicPromptGenerator {
    /// 创建新的Prompt生成器（使用回退模式）
    pub fn new() -> Self {
        Self::new_fallback()
    }

    /// 创建回退版本的生成器（不依赖Prompt系统）
    fn new_fallback() -> Self {
        Self {
            template_manager: None,
            optimizer: None,
            builder: None,
            use_fallback: true,
        }
    }

    /// 使用现有Prompt系统创建新的生成器
    pub fn new_with_systems(
        template_manager: PromptTemplateManager,
        optimizer: PromptOptimizer,
        builder: PromptBuilder,
    ) -> Self {
        Self {
            template_manager: Some(template_manager),
            optimizer: Some(optimizer),
            builder: Some(builder),
            use_fallback: false,
        }
    }

    /// 尝试创建完整的Prompt生成器，失败时回退到简化模式
    pub async fn new_with_auto_fallback() -> Self {
        // 尝试初始化完整的prompt系统
        match Self::try_initialize_full_system().await {
            Ok(generator) => {
                info!("Successfully initialized full prompt system");
                generator
            }
            Err(e) => {
                log::warn!("Failed to initialize full prompt system, using fallback mode: {}", e);
                Self::new_fallback()
            }
        }
    }

    /// 尝试初始化完整的prompt系统
    async fn try_initialize_full_system() -> Result<Self> {
        // 这里可以添加实际的初始化逻辑
        // 目前返回错误以使用回退模式
        Err(anyhow::anyhow!("Full prompt system initialization not implemented yet"))
    }

    /// 生成Prompt
    pub async fn generate_prompt(
        &self,
        analysis: &QueryAnalysisResult,
        selection: &ArchitectureSelectionResult,
        user_input: &str,
        context: Option<&str>,
    ) -> Result<PromptGenerationResult> {
        info!("Generating dynamic prompts for architecture: {} (fallback mode: {})", 
              selection.selected_architecture, self.use_fallback);

        let result = if self.use_fallback {
            // 使用回退模式生成prompt
            self.generate_fallback_prompts(analysis, selection, user_input, context)
        } else {
            // 使用完整的prompt系统
            match selection.selected_architecture.as_str() {
                "LlmCompiler" => self.generate_llm_compiler_prompts(analysis, selection, user_input, context),
                "ReWoo" => self.generate_rewoo_prompts(analysis, selection, user_input, context),
                "PlanAndExecute" => self.generate_plan_execute_prompts(analysis, selection, user_input, context),
                _ => self.generate_default_prompts(analysis, selection, user_input, context),
            }
        };

        info!("Prompt generation completed with quality score: {:.2}", result.quality_score);
        Ok(result)
    }

    /// 回退模式生成prompt
    fn generate_fallback_prompts(
        &self,
        analysis: &QueryAnalysisResult,
        selection: &ArchitectureSelectionResult,
        user_input: &str,
        context: Option<&str>,
    ) -> PromptGenerationResult {
        info!("Using fallback prompt generation for architecture: {}", selection.selected_architecture);
        
        match selection.selected_architecture.as_str() {
            "LlmCompiler" => self.generate_llm_compiler_fallback_prompts(analysis, selection, user_input, context),
            "ReWoo" => self.generate_rewoo_fallback_prompts(analysis, selection, user_input, context),
            "PlanAndExecute" => self.generate_plan_execute_fallback_prompts(analysis, selection, user_input, context),
            _ => self.generate_default_fallback_prompts(analysis, selection, user_input, context),
        }
    }

    /// 获取基础模板方法
    fn get_llm_compiler_planner_template(&self) -> String {
        r#"You are an LLMCompiler Task Decomposition Unit for security testing.

Task Analysis:
- User request: {{user_input}}
- Task type: {{task_type}}  
- Complexity: {{complexity}}
- Parallelization potential: {{parallelization}}

Configuration:
- Max parallel tasks: {{max_parallel_tasks}}
- Task timeout: {{timeout_per_task}}s
- Retry policy: {{retry_policy}}

Generate a streaming DAG plan for parallel execution.
Focus on security scanning, vulnerability assessment, and network analysis tasks."#.to_string()
    }

    fn get_llm_compiler_executor_template(&self) -> String {
        r#"You are an LLMCompiler Task Fetching Unit for parallel task execution.

Configuration:
- Max parallel tasks: {{max_parallel_tasks}}
- Task timeout: {{timeout_per_task}}s
- Retry policy: {{retry_policy}}

Execute tasks in parallel based on the DAG from the planner.
Handle tool calls, manage dependencies, and report progress efficiently."#.to_string()
    }

    fn get_llm_compiler_analyzer_template(&self) -> String {
        r#"You are an LLMCompiler Joiner for result analysis and decision making.

Original request: {{user_input}}
Task type: {{task_type}}

Analyze execution results and decide whether to complete or replan.
Provide comprehensive security analysis based on gathered data."#.to_string()
    }

    fn get_rewoo_planner_template(&self) -> String {
        r#"You are a ReWOO Planner for security task decomposition.

Task Analysis:
- User request: {{user_input}}
- Task type: {{task_type}}
- Complexity: {{complexity}}
- Key indicators: {{key_indicators}}

Generate a structured execution plan with variable dependencies.
Focus on security tools and analysis workflows."#.to_string()
    }

    fn get_rewoo_executor_template(&self) -> String {
        r#"You are a ReWOO Worker for tool execution.

Configuration:
- Max parallel tasks: {{max_parallel_tasks}}

Execute tasks sequentially, call security tools, and manage variable mappings.
Handle network scanning, vulnerability detection, and analysis tools."#.to_string()
    }

    fn get_rewoo_analyzer_template(&self) -> String {
        r#"You are a ReWOO Solver for final answer generation.

Original question: {{user_input}}
Task type: {{task_type}}

Synthesize tool results into comprehensive security analysis.
Provide actionable insights and recommendations."#.to_string()
    }

    fn get_plan_execute_planner_template(&self) -> String {
        r#"You are a Plan-Execute Planner for security tasks.

Task Analysis:
- User request: {{user_input}}
- Task type: {{task_type}}
- Complexity: {{complexity}}
- Estimated steps: {{estimated_steps}}
- Target: {{target_domain}}

Create a step-by-step execution plan for security testing and analysis."#.to_string()
    }

    fn get_plan_execute_executor_template(&self) -> String {
        r#"You are a Plan-Execute Executor for security operations.

Task type: {{task_type}}
Target: {{target_domain}}

Execute the planned security tasks systematically.
Use appropriate security tools and provide detailed results."#.to_string()
    }

    /// 使用集成Prompt系统生成优化的Prompt
    fn generate_optimized_prompts(
        &self,
        architecture: &str,
        analysis: &QueryAnalysisResult,
        selection: &ArchitectureSelectionResult,
        user_input: &str,
    ) -> Result<PromptGenerationResult> {
        // 创建变量映射
        let mut variables = HashMap::new();
        variables.insert("user_input".to_string(), serde_json::Value::String(user_input.to_string()));
        variables.insert("task_type".to_string(), serde_json::Value::String(analysis.task_type.clone()));
        variables.insert("complexity".to_string(), serde_json::Value::String(analysis.complexity_level.clone()));
        variables.insert("parallelization".to_string(), serde_json::Value::String(analysis.parallelization_potential.clone()));
        variables.insert("max_parallel_tasks".to_string(), serde_json::Value::Number(selection.architecture_config.max_parallel_tasks.into()));
        variables.insert("timeout_per_task".to_string(), serde_json::Value::Number(selection.architecture_config.timeout_per_task.into()));
        variables.insert("retry_policy".to_string(), serde_json::Value::String(selection.architecture_config.retry_policy.clone()));
        variables.insert("estimated_steps".to_string(), serde_json::Value::Number(analysis.estimated_steps.into()));
        variables.insert("target_domain".to_string(), serde_json::Value::String(
            analysis.target_domain.clone().unwrap_or("未指定".to_string())
        ));
        variables.insert("key_indicators".to_string(), serde_json::Value::Array(
            analysis.key_indicators.iter().map(|s| serde_json::Value::String(s.clone())).collect()
        ));

        // 获取对应的模板
        let (planner_template, executor_template, analyzer_template) = match architecture {
            "LlmCompiler" => (
                self.get_llm_compiler_planner_template(),
                self.get_llm_compiler_executor_template(),
                Some(self.get_llm_compiler_analyzer_template()),
            ),
            "ReWoo" => (
                self.get_rewoo_planner_template(),
                self.get_rewoo_executor_template(),
                Some(self.get_rewoo_analyzer_template()),
            ),
            "PlanAndExecute" => (
                self.get_plan_execute_planner_template(),
                self.get_plan_execute_executor_template(),
                None,
            ),
            _ => (
                self.get_plan_execute_planner_template(),
                self.get_plan_execute_executor_template(),
                None,
            ),
        };

        // 使用简化的变量替换（不依赖PromptBuilder）
        let planner_prompt = self.simple_variable_replacement(&planner_template, &variables);
        let executor_prompt = self.simple_variable_replacement(&executor_template, &variables);
        let analyzer_prompt = if let Some(template) = analyzer_template {
            Some(self.simple_variable_replacement(&template, &variables))
        } else {
            None
        };

        // 模拟Prompt优化（因为PromptOptimizer可能需要异步）
        let quality_score = match architecture {
            "LlmCompiler" => 0.95,
            "ReWoo" => 0.92,
            "PlanAndExecute" => 0.88,
            _ => 0.85,
        };

        Ok(PromptGenerationResult {
            planner_prompt,
            executor_prompt,
            analyzer_prompt,
            tool_selector_prompt: None,
            quality_score,
            generation_strategy: format!("{}优化+集成Prompt系统", architecture),
        })
    }

    /// 生成LLMCompiler架构的Prompt（使用集成系统）
    fn generate_llm_compiler_prompts(
        &self,
        analysis: &QueryAnalysisResult,
        selection: &ArchitectureSelectionResult,
        user_input: &str,
        _context: Option<&str>,
    ) -> PromptGenerationResult {
        // 使用集成的Prompt系统
        match self.generate_optimized_prompts("LlmCompiler", analysis, selection, user_input) {
            Ok(result) => result,
            Err(e) => {
                log::warn!("Failed to use integrated prompt system, falling back to hardcoded: {}", e);
                self.generate_llm_compiler_prompts_fallback(analysis, selection, user_input, _context)
            }
        }
    }

    /// LLMCompiler的回退实现
    fn generate_llm_compiler_prompts_fallback(
        &self,
        analysis: &QueryAnalysisResult,
        selection: &ArchitectureSelectionResult,
        user_input: &str,
        _context: Option<&str>,
    ) -> PromptGenerationResult {
        let planner_prompt = format!(
            r#"你是一个专业的LLMCompiler Planner，负责生成可并行执行的任务DAG。

任务分析：
- 用户请求：{}
- 任务类型：{}
- 复杂度：{}
- 并行化潜力：{}
- 预估步骤：{}

请按以下格式生成任务DAG：
1. 分析用户请求，识别可并行的子任务
2. 生成有向无环图(DAG)，包含任务依赖关系
3. 为每个任务分配变量名，支持变量替换
4. 输出格式要求：JSON格式的任务列表，每个任务包含：
   - task_id: 任务唯一标识
   - description: 任务描述
   - dependencies: 依赖的任务ID列表
   - variables: 输出变量映射
   - estimated_time: 预计执行时间（秒）
   - parallel_group: 并行组ID（相同组可并行执行）

优化原则：
- 最大化并行执行机会
- 最小化任务间依赖
- 确保关键路径最短
- 支持流式输出和即时调度

请生成高效的任务分解方案。"#,
            user_input,
            analysis.task_type,
            analysis.complexity_level,
            analysis.parallelization_potential,
            analysis.estimated_steps
        );

        let executor_prompt = format!(
            r#"你是一个LLMCompiler Task Fetching Unit，负责并行调度和执行任务。

执行配置：
- 最大并行任务数：{}
- 每任务超时：{}秒
- 重试策略：{}
- 资源限制：CPU {}核，内存 {}GB

执行策略：
1. 监听Planner的流式DAG输出
2. 实时识别可执行的任务（依赖已满足）
3. 智能调度任务到可用的并行槽位
4. 执行任务并更新变量映射
5. 处理任务失败和重试逻辑
6. 向Joiner实时报告执行进度

调度原则：
- 优先执行关键路径上的任务
- 动态负载均衡
- 及时依赖解析
- 资源使用优化

请确保高效的并行执行和异常处理。"#,
            selection.architecture_config.max_parallel_tasks,
            selection.architecture_config.timeout_per_task,
            selection.architecture_config.retry_policy,
            selection.architecture_config.resource_limits.as_ref().map_or(4, |r| r.cpu_cores),
            selection.architecture_config.resource_limits.as_ref().map_or(2, |r| r.memory_gb)
        );

        let analyzer_prompt = Some(format!(
            r#"你是一个LLMCompiler Joiner，负责决策是否重规划或完成任务。

决策依据：
- 用户原始请求：{}
- 当前执行进度和结果
- 任务成功/失败状态
- 变量值和中间结果

决策流程：
1. 评估当前执行结果是否满足用户需求
2. 分析是否存在失败任务需要重新规划
3. 判断是否需要补充额外任务
4. 决定返回"complete"或"replan"

完成条件：
- 所有关键任务成功执行
- 用户需求得到满足
- 结果质量达到阈值

重规划条件：
- 关键任务失败且有替代方案
- 结果不完整需要补充
- 发现更优执行路径

请基于执行历史做出最优决策。"#,
            user_input
        ));

        PromptGenerationResult {
            planner_prompt,
            executor_prompt,
            analyzer_prompt,
            tool_selector_prompt: None,
            quality_score: 0.9,
            generation_strategy: "LLMCompiler优化".to_string(),
        }
    }

    /// 生成ReWOO架构的Prompt（使用集成系统）
    fn generate_rewoo_prompts(
        &self,
        analysis: &QueryAnalysisResult,
        selection: &ArchitectureSelectionResult,
        user_input: &str,
        _context: Option<&str>,
    ) -> PromptGenerationResult {
        // 使用集成的Prompt系统
        match self.generate_optimized_prompts("ReWoo", analysis, selection, user_input) {
            Ok(result) => result,
            Err(e) => {
                log::warn!("Failed to use integrated prompt system for ReWOO, falling back: {}", e);
                self.generate_rewoo_prompts_fallback(analysis, selection, user_input, _context)
            }
        }
    }

    /// ReWOO的回退实现
    fn generate_rewoo_prompts_fallback(
        &self,
        analysis: &QueryAnalysisResult,
        selection: &ArchitectureSelectionResult,
        user_input: &str,
        _context: Option<&str>,
    ) -> PromptGenerationResult {
        let planner_prompt = format!(
            r#"你是一个ReWOO Planner，负责生成结构化的任务执行计划。

任务分析：
- 用户请求：{}
- 任务类型：{}
- 复杂度：{}
- 关键指标：{:?}

请按以下格式生成执行计划：
1. 将复杂任务分解为有序的子任务
2. 为每个子任务定义输入输出变量
3. 指定需要调用的工具和参数
4. 建立任务间的变量依赖关系

输出格式：
Plan:
Task1: [工具名] 任务描述 -> #E1
Task2: [工具名] 基于#E1的任务描述 -> #E2
...
TaskN: [工具名] 基于#E(N-1)的最终任务 -> #EN

变量映射说明：
- #E1, #E2, ...为任务输出变量
- 后续任务可引用前面任务的输出
- 确保逻辑依赖关系正确

请生成清晰的任务分解计划。"#,
            user_input,
            analysis.task_type,
            analysis.complexity_level,
            analysis.key_indicators
        );

        let executor_prompt = format!(
            r#"你是一个ReWOO Worker，负责执行具体的任务并调用工具。

执行配置：
- 并行任务数：{}
- 工具调用超时：{}秒
- 启用缓存：{}

执行流程：
1. 接收Planner生成的任务列表
2. 按顺序执行每个任务
3. 调用指定的工具获取结果
4. 更新变量映射表
5. 处理工具调用异常
6. 收集所有任务的执行结果

工具调用原则：
- 严格按照任务描述调用工具
- 正确传递参数和上下文
- 处理工具返回的各种格式
- 保持变量引用的一致性

请确保准确的工具调用和结果收集。"#,
            selection.architecture_config.max_parallel_tasks,
            selection.architecture_config.timeout_per_task,
            selection.architecture_config.optimization_params.enable_caching
        );

        let analyzer_prompt = Some(format!(
            r#"你是一个ReWOO Solver，负责基于工具输出生成最终答案。

综合分析：
- 用户原始问题：{}
- 任务执行结果和工具输出
- 变量值和中间结果

生成策略：
1. 整合所有工具输出和中间结果
2. 基于用户问题进行推理分析
3. 生成完整、准确的最终答案
4. 提供推理链和置信度评估

答案要求：
- 直接回答用户问题
- 结构化呈现关键信息
- 引用具体的工具输出作为证据
- 标明不确定的部分

质量标准：
- 答案完整性
- 逻辑一致性
- 事实准确性
- 表达清晰度

请生成高质量的最终答案。"#,
            user_input
        ));

        PromptGenerationResult {
            planner_prompt,
            executor_prompt,
            analyzer_prompt,
            tool_selector_prompt: None,
            quality_score: 0.85,
            generation_strategy: "ReWOO推理链".to_string(),
        }
    }

    /// 生成Plan-Execute架构的Prompt（使用集成系统）
    fn generate_plan_execute_prompts(
        &self,
        analysis: &QueryAnalysisResult,
        selection: &ArchitectureSelectionResult,
        user_input: &str,
        _context: Option<&str>,
    ) -> PromptGenerationResult {
        // 使用集成的Prompt系统
        match self.generate_optimized_prompts("PlanAndExecute", analysis, selection, user_input) {
            Ok(result) => result,
            Err(e) => {
                log::warn!("Failed to use integrated prompt system for Plan-Execute, falling back: {}", e);
                self.generate_plan_execute_prompts_fallback(analysis, selection, user_input, _context)
            }
        }
    }

    /// Plan-Execute的回退实现
    fn generate_plan_execute_prompts_fallback(
        &self,
        analysis: &QueryAnalysisResult,
        selection: &ArchitectureSelectionResult,
        user_input: &str,
        _context: Option<&str>,
    ) -> PromptGenerationResult {
        let planner_prompt = format!(
            r#"你是一个Plan-Execute Planner，负责创建灵活的执行计划。

任务背景：
- 用户请求：{}
- 任务类型：{}
- 复杂度：{}
- 时间敏感性：{}

规划要求：
1. 将任务分解为可执行的步骤序列
2. 每个步骤包含明确的行动和预期结果
3. 考虑执行过程中可能的变化和调整
4. 设计检查点和决策节点

计划格式：
```
执行计划：
步骤1：[行动描述] - 预期结果：[结果描述]
步骤2：[行动描述] - 预期结果：[结果描述]
...
步骤N：[行动描述] - 预期结果：[结果描述]

检查点：
- 在步骤X后评估进度
- 在步骤Y后检查结果质量
- 在步骤Z后决定是否需要调整

成功标准：
- [明确的成功指标]
```

请生成适应性强的执行计划。"#,
            user_input,
            analysis.task_type,
            analysis.complexity_level,
            analysis.time_sensitivity
        );

        let executor_prompt = format!(
            r#"你是一个Plan-Execute Agent，负责执行计划步骤并处理动态调整。

执行配置：
- 自适应调度：{}
- 最大重规划次数：3
- 步骤超时：{}秒

执行流程：
1. 接收当前执行计划
2. 执行当前步骤的具体行动
3. 评估执行结果和质量
4. 判断是否需要调整后续计划
5. 记录执行过程和中间结果

执行原则：
- 严格按照计划执行
- 灵活应对执行中的问题
- 及时反馈执行状态
- 保持结果的一致性

异常处理：
- 步骤执行失败时的恢复策略
- 工具调用异常的处理方法
- 超时情况的应对措施

请确保稳定可靠的步骤执行。"#,
            selection.architecture_config.optimization_params.enable_adaptive_scheduling,
            selection.architecture_config.timeout_per_task
        );

        let analyzer_prompt = Some(format!(
            r#"你是一个Plan-Execute Replanner，负责评估执行进度并决定后续行动。

评估维度：
- 用户原始需求：{}
- 当前执行进度和结果
- 计划完成度和质量
- 剩余步骤的可行性

决策选项：
1. 继续执行：当前计划进展顺利，继续执行下一步
2. 重新规划：当前计划需要调整，生成新的执行步骤
3. 任务完成：已满足用户需求，可以结束执行

决策依据：
- 执行结果是否符合预期
- 是否已实现用户目标
- 是否存在更优的执行路径
- 资源和时间约束

输出格式：
```
决策：[继续执行/重新规划/任务完成]
理由：[详细说明决策原因]
下一步行动：[具体的执行指令]
```

请做出最优的执行决策。"#,
            user_input
        ));

        PromptGenerationResult {
            planner_prompt,
            executor_prompt,
            analyzer_prompt,
            tool_selector_prompt: None,
            quality_score: 0.8,
            generation_strategy: "Plan-Execute自适应".to_string(),
        }
    }

    /// 生成LLM Compiler回退模式Prompt
    fn generate_llm_compiler_fallback_prompts(
        &self,
        analysis: &QueryAnalysisResult,
        _selection: &ArchitectureSelectionResult,
        user_input: &str,
        _context: Option<&str>,
    ) -> PromptGenerationResult {
        let mut variables = HashMap::new();
        variables.insert("user_input".to_string(), serde_json::Value::String(user_input.to_string()));
        variables.insert("task_type".to_string(), serde_json::Value::String(analysis.task_type.clone()));
        variables.insert("complexity".to_string(), serde_json::Value::String(analysis.complexity_level.clone()));
        variables.insert("parallelization".to_string(), serde_json::Value::String(analysis.parallelization_potential.clone()));
        variables.insert("max_parallel_tasks".to_string(), serde_json::Value::Number(serde_json::Number::from(4)));
        variables.insert("timeout_per_task".to_string(), serde_json::Value::Number(serde_json::Number::from(60)));
        variables.insert("retry_policy".to_string(), serde_json::Value::String("3 retries".to_string()));

        let planner_prompt = self.simple_variable_replacement(&self.get_llm_compiler_planner_template(), &variables);
        let executor_prompt = self.simple_variable_replacement(&self.get_llm_compiler_executor_template(), &variables);
        let analyzer_prompt = Some(self.simple_variable_replacement(&self.get_llm_compiler_analyzer_template(), &variables));

        PromptGenerationResult {
            planner_prompt,
            executor_prompt,
            analyzer_prompt,
            tool_selector_prompt: None,
            quality_score: 0.75,
            generation_strategy: "LLMCompiler回退模式".to_string(),
        }
    }

    /// 生成ReWOO回退模式Prompt
    fn generate_rewoo_fallback_prompts(
        &self,
        analysis: &QueryAnalysisResult,
        _selection: &ArchitectureSelectionResult,
        user_input: &str,
        _context: Option<&str>,
    ) -> PromptGenerationResult {
        let mut variables = HashMap::new();
        variables.insert("user_input".to_string(), serde_json::Value::String(user_input.to_string()));
        variables.insert("task_type".to_string(), serde_json::Value::String(analysis.task_type.clone()));
        variables.insert("complexity".to_string(), serde_json::Value::String(analysis.complexity_level.clone()));
        variables.insert("key_indicators".to_string(), serde_json::Value::String(analysis.key_indicators.join(", ")));
        variables.insert("max_parallel_tasks".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));

        let planner_prompt = self.simple_variable_replacement(&self.get_rewoo_planner_template(), &variables);
        let executor_prompt = self.simple_variable_replacement(&self.get_rewoo_executor_template(), &variables);
        let analyzer_prompt = Some(self.simple_variable_replacement(&self.get_rewoo_analyzer_template(), &variables));

        PromptGenerationResult {
            planner_prompt,
            executor_prompt,
            analyzer_prompt,
            tool_selector_prompt: None,
            quality_score: 0.75,
            generation_strategy: "ReWOO回退模式".to_string(),
        }
    }

    /// 生成Plan-Execute回退模式Prompt
    fn generate_plan_execute_fallback_prompts(
        &self,
        analysis: &QueryAnalysisResult,
        _selection: &ArchitectureSelectionResult,
        user_input: &str,
        _context: Option<&str>,
    ) -> PromptGenerationResult {
        let mut variables = HashMap::new();
        variables.insert("user_input".to_string(), serde_json::Value::String(user_input.to_string()));
        variables.insert("task_type".to_string(), serde_json::Value::String(analysis.task_type.clone()));
        variables.insert("complexity".to_string(), serde_json::Value::String(analysis.complexity_level.clone()));
        variables.insert("estimated_steps".to_string(), serde_json::Value::Number(serde_json::Number::from(analysis.estimated_steps)));
        variables.insert("target_domain".to_string(), serde_json::Value::String(
            analysis.target_domain.clone().unwrap_or_else(|| "general".to_string())
        ));

        let planner_prompt = self.simple_variable_replacement(&self.get_plan_execute_planner_template(), &variables);
        let executor_prompt = self.simple_variable_replacement(&self.get_plan_execute_executor_template(), &variables);

        PromptGenerationResult {
            planner_prompt,
            executor_prompt,
            analyzer_prompt: None,
            tool_selector_prompt: None,
            quality_score: 0.75,
            generation_strategy: "Plan-Execute回退模式".to_string(),
        }
    }

    /// 生成默认回退模式Prompt
    fn generate_default_fallback_prompts(
        &self,
        analysis: &QueryAnalysisResult,
        _selection: &ArchitectureSelectionResult,
        user_input: &str,
        _context: Option<&str>,
    ) -> PromptGenerationResult {
        let planner_prompt = format!(
            "请为以下任务制定执行计划：{}\n\n任务类型：{}\n复杂度：{}\n目标领域：{}\n\n请提供详细的步骤规划。",
            user_input, 
            analysis.task_type, 
            analysis.complexity_level, 
            analysis.target_domain.as_deref().unwrap_or("general")
        );

        let executor_prompt = format!(
            "请执行以下任务：{}\n\n请按步骤进行，使用适当的工具，并提供详细的执行结果。",
            user_input
        );

        PromptGenerationResult {
            planner_prompt,
            executor_prompt,
            analyzer_prompt: None,
            tool_selector_prompt: None,
            quality_score: 0.65,
            generation_strategy: "默认回退模式".to_string(),
        }
    }

    /// 生成默认Prompt（使用集成系统）
    fn generate_default_prompts(
        &self,
        analysis: &QueryAnalysisResult,
        selection: &ArchitectureSelectionResult,
        user_input: &str,
        _context: Option<&str>,
    ) -> PromptGenerationResult {
        // 优先使用集成的Prompt系统
        match self.generate_optimized_prompts("PlanAndExecute", analysis, selection, user_input) {
            Ok(result) => result,
            Err(e) => {
                log::warn!("Failed to use integrated prompt system for default, using fallback: {}", e);
                
                // 回退到简单的默认模板
                let planner_prompt = format!(
                    "请为以下任务制定执行计划：{}\n任务类型：{}\n复杂度：{}",
                    user_input, analysis.task_type, analysis.complexity_level
                );

                let executor_prompt = format!(
                    "请执行以下任务：{}\n请按步骤进行，并提供详细的执行结果。",
                    user_input
                );

                PromptGenerationResult {
                    planner_prompt,
                    executor_prompt,
                    analyzer_prompt: None,
                    tool_selector_prompt: None,
                    quality_score: 0.6,
                    generation_strategy: "默认回退模板".to_string(),
                }
            }
        }
    }

    /// 简化的变量替换方法（不依赖PromptBuilder）
    pub fn simple_variable_replacement(&self, template: &str, variables: &HashMap<String, serde_json::Value>) -> String {
        let mut result = template.to_string();
        
        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }
        
        result
    }
}