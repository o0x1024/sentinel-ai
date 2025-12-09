//! Agent 任务规划器
//!
//! 分析任务复杂度，决定是否创建 Todos，生成预规划步骤。
//! 与现有 ReAct 引擎协作：Planner 生成计划 → Todos 追踪 → ReAct 执行

use super::todo_manager::{Todo, TodoManager, TodoMetadata, TodoStatus};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 规划步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub tool: Option<String>,
    pub args: Option<serde_json::Value>,
    pub depends_on: Vec<String>,
}

/// 任务计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub task: String,
    pub description: String,
    pub steps: Vec<PlanStep>,
    pub expected_outcome: String,
    pub complexity: TaskComplexity,
}

/// 任务复杂度
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TaskComplexity {
    Simple,   // 1-2 步，不需要 Todos
    Medium,   // 3-5 步，建议 Todos
    Complex,  // 5+ 步，必须 Todos
}

impl TaskComplexity {
    /// 是否需要创建 Todos
    pub fn should_create_todos(&self) -> bool {
        matches!(self, TaskComplexity::Medium | TaskComplexity::Complex)
    }
}

/// Planner 配置
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// 简单任务最大步骤数（超过则升级为 Medium）
    pub simple_max_steps: usize,
    /// 中等任务最大步骤数（超过则升级为 Complex）
    pub medium_max_steps: usize,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            simple_max_steps: 2,
            medium_max_steps: 5,
        }
    }
}

/// 任务规划器
pub struct TaskPlanner {
    config: PlannerConfig,
}

impl TaskPlanner {
    pub fn new(config: PlannerConfig) -> Self {
        Self { config }
    }

    /// 分析任务复杂度
    pub fn analyze_complexity(&self, task: &str) -> TaskComplexity {
        // 关键词检测
        let complex_keywords = [
            "渗透测试", "penetration", "pentest",
            "内网渗透", "lateral movement",
            "完整", "全面", "comprehensive",
            "多个", "multiple", "所有",
            "审计", "audit", "review",
        ];
        
        let medium_keywords = [
            "扫描", "scan", "探测", "detect",
            "分析", "analyze", "检查", "check",
            "修复", "fix", "remediate",
            "测试", "test",
        ];

        let task_lower = task.to_lowercase();
        
        // 检查复杂关键词
        if complex_keywords.iter().any(|k| task_lower.contains(k)) {
            return TaskComplexity::Complex;
        }
        
        // 检查中等关键词
        if medium_keywords.iter().any(|k| task_lower.contains(k)) {
            return TaskComplexity::Medium;
        }
        
        TaskComplexity::Simple
    }

    /// 从 LLM 响应解析计划
    pub fn parse_plan_from_response(&self, task: &str, response: &str) -> Result<TaskPlan> {
        // 尝试解析 JSON 格式的计划
        if let Some(json_start) = response.find('{') {
            if let Some(json_end) = response.rfind('}') {
                let json_str = &response[json_start..=json_end];
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(plan_obj) = parsed.get("plan") {
                        return self.parse_plan_object(task, plan_obj);
                    }
                    // 直接是 plan 对象
                    if parsed.get("steps").is_some() {
                        return self.parse_plan_object(task, &parsed);
                    }
                }
            }
        }
        
        // 回退：从文本提取步骤
        self.extract_steps_from_text(task, response)
    }

    /// 从 JSON 对象解析计划
    fn parse_plan_object(&self, task: &str, obj: &serde_json::Value) -> Result<TaskPlan> {
        let steps: Vec<PlanStep> = if let Some(steps_arr) = obj.get("steps").and_then(|v| v.as_array()) {
            steps_arr.iter().enumerate().map(|(i, s)| {
                PlanStep {
                    id: s.get("id").and_then(|v| v.as_str()).unwrap_or(&format!("{}", i + 1)).to_string(),
                    description: s.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    tool: s.get("tool").and_then(|t| {
                        t.get("name").and_then(|n| n.as_str()).map(|s| s.to_string())
                            .or_else(|| t.as_str().map(|s| s.to_string()))
                    }),
                    args: s.get("tool").and_then(|t| t.get("args").cloned())
                        .or_else(|| s.get("args").cloned()),
                    depends_on: s.get("depends_on")
                        .and_then(|d| d.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                }
            }).collect()
        } else {
            vec![]
        };

        let complexity = self.determine_complexity_from_steps(steps.len());

        Ok(TaskPlan {
            task: task.to_string(),
            description: obj.get("description").and_then(|v| v.as_str()).unwrap_or(task).to_string(),
            steps,
            expected_outcome: obj.get("expected_outcome").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            complexity,
        })
    }

    /// 从文本提取步骤（回退方案）
    fn extract_steps_from_text(&self, task: &str, text: &str) -> Result<TaskPlan> {
        let mut steps = Vec::new();
        let mut step_id = 1;

        // 查找编号列表（1. xxx, 2. xxx）
        for line in text.lines() {
            let trimmed = line.trim();
            
            // 匹配 "1. xxx" 或 "- xxx" 或 "* xxx"
            let is_numbered = trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                && trimmed.contains('.');
            let is_bullet = trimmed.starts_with("- ") || trimmed.starts_with("* ");
            
            if is_numbered || is_bullet {
                let content = if is_numbered {
                    trimmed.split('.').skip(1).collect::<Vec<_>>().join(".").trim().to_string()
                } else {
                    trimmed[2..].trim().to_string()
                };
                
                if !content.is_empty() && content.len() > 3 {
                    steps.push(PlanStep {
                        id: step_id.to_string(),
                        description: content,
                        tool: None,
                        args: None,
                        depends_on: vec![],
                    });
                    step_id += 1;
                }
            }
        }

        // 如果没有找到步骤，创建单一步骤
        if steps.is_empty() {
            steps.push(PlanStep {
                id: "1".to_string(),
                description: task.to_string(),
                tool: None,
                args: None,
                depends_on: vec![],
            });
        }

        let complexity = self.determine_complexity_from_steps(steps.len());

        Ok(TaskPlan {
            task: task.to_string(),
            description: task.to_string(),
            steps,
            expected_outcome: String::new(),
            complexity,
        })
    }

    /// 根据步骤数量确定复杂度
    fn determine_complexity_from_steps(&self, step_count: usize) -> TaskComplexity {
        if step_count <= self.config.simple_max_steps {
            TaskComplexity::Simple
        } else if step_count <= self.config.medium_max_steps {
            TaskComplexity::Medium
        } else {
            TaskComplexity::Complex
        }
    }

    /// 将计划转换为 Todos
    pub fn plan_to_todos(&self, plan: &TaskPlan) -> Vec<Todo> {
        plan.steps.iter().enumerate().map(|(i, step)| {
            let status = if i == 0 { TodoStatus::InProgress } else { TodoStatus::Pending };
            
            let mut todo = Todo::new(&step.id, &step.description)
                .with_status(status)
                .with_step_index(i);
            
            if let Some(ref tool) = step.tool {
                todo = todo.with_tool(tool);
            }
            
            todo
        }).collect()
    }
}

/// 快捷函数：创建默认配置的 Planner
pub fn create_planner() -> TaskPlanner {
    TaskPlanner::new(PlannerConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_analysis() {
        let planner = create_planner();
        
        assert_eq!(planner.analyze_complexity("查询天气"), TaskComplexity::Simple);
        assert_eq!(planner.analyze_complexity("扫描端口"), TaskComplexity::Medium);
        assert_eq!(planner.analyze_complexity("对 example.com 进行渗透测试"), TaskComplexity::Complex);
    }

    #[test]
    fn test_parse_json_plan() {
        let planner = create_planner();
        let response = r#"
        {
            "plan": {
                "description": "渗透测试计划",
                "steps": [
                    {"id": "1", "description": "信息收集", "tool": {"name": "whois"}},
                    {"id": "2", "description": "端口扫描", "tool": {"name": "port_scan"}},
                    {"id": "3", "description": "漏洞探测"}
                ],
                "expected_outcome": "发现潜在漏洞"
            }
        }
        "#;
        
        let plan = planner.parse_plan_from_response("测试任务", response).unwrap();
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.complexity, TaskComplexity::Medium);
    }

    #[test]
    fn test_extract_steps_from_text() {
        let planner = create_planner();
        let text = r#"
        执行计划：
        1. 首先进行信息收集
        2. 然后扫描端口
        3. 分析结果
        "#;
        
        let plan = planner.parse_plan_from_response("测试", text).unwrap();
        assert_eq!(plan.steps.len(), 3);
    }

    #[test]
    fn test_plan_to_todos() {
        let planner = create_planner();
        let plan = TaskPlan {
            task: "测试".to_string(),
            description: "测试计划".to_string(),
            steps: vec![
                PlanStep { id: "1".to_string(), description: "步骤1".to_string(), tool: Some("tool1".to_string()), args: None, depends_on: vec![] },
                PlanStep { id: "2".to_string(), description: "步骤2".to_string(), tool: None, args: None, depends_on: vec![] },
            ],
            expected_outcome: "完成".to_string(),
            complexity: TaskComplexity::Simple,
        };
        
        let todos = planner.plan_to_todos(&plan);
        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].status, TodoStatus::InProgress);
        assert_eq!(todos[1].status, TodoStatus::Pending);
    }
}

