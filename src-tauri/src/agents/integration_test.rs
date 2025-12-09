//! Agent 端到端集成测试
//!
//! 测试 Planner → TodoManager → Orchestrator 完整流程

#[cfg(test)]
mod tests {
    use crate::agents::{
        orchestrator::{AgentOrchestrator, OrchestratorConfig},
        planner::{PlanStep, PlannerConfig, TaskComplexity, TaskPlan, TaskPlanner},
        todo_manager::{Todo, TodoManager, TodoStatus},
    };
    use std::sync::Arc;

    // =========================================================================
    // TodoManager 集成测试
    // =========================================================================

    #[tokio::test]
    async fn test_todo_lifecycle() {
        let manager = TodoManager::new(None);
        let exec_id = "e2e-todo-lifecycle";

        // 1. 创建 todos
        let todos = vec![
            Todo::new("step-1", "信息收集")
                .with_status(TodoStatus::InProgress)
                .with_tool("whois_lookup"),
            Todo::new("step-2", "端口扫描").with_tool("port_scan"),
            Todo::new("step-3", "漏洞分析"),
        ];
        manager.write_todos(exec_id, todos, false).await.unwrap();

        // 验证初始状态
        let current = manager.get_todos(exec_id).await;
        assert_eq!(current.len(), 3);
        assert_eq!(current[0].status, TodoStatus::InProgress);
        assert_eq!(current[1].status, TodoStatus::Pending);

        // 2. 完成第一步，开始第二步
        manager
            .update_status(exec_id, "step-1", TodoStatus::Completed)
            .await
            .unwrap();
        manager
            .update_status(exec_id, "step-2", TodoStatus::InProgress)
            .await
            .unwrap();

        let current = manager.get_todos(exec_id).await;
        assert_eq!(current[0].status, TodoStatus::Completed);
        assert_eq!(current[1].status, TodoStatus::InProgress);

        // 3. 验证进行中的任务
        let in_progress = manager.get_in_progress(exec_id).await;
        assert!(in_progress.is_some());
        assert_eq!(in_progress.unwrap().id, "step-2");

        // 4. 获取统计
        let stats = manager.get_stats(exec_id).await;
        assert_eq!(stats.total, 3);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.pending, 1);

        // 5. 清理
        manager.clear(exec_id).await;
        let empty = manager.get_todos(exec_id).await;
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_nested_todos() {
        let manager = TodoManager::new(None);
        let exec_id = "e2e-nested-todos";

        // 创建嵌套任务
        let todos = vec![
            Todo::new("parent-1", "渗透测试").with_status(TodoStatus::InProgress),
            Todo::new("child-1-1", "子域名扫描")
                .with_parent("parent-1")
                .with_status(TodoStatus::InProgress),
            Todo::new("child-1-2", "端口扫描").with_parent("parent-1"),
            Todo::new("child-1-3", "服务识别").with_parent("parent-1"),
            Todo::new("parent-2", "报告生成"),
        ];
        manager.write_todos(exec_id, todos, false).await.unwrap();

        let all = manager.get_todos(exec_id).await;
        assert_eq!(all.len(), 5);

        // 验证父子关系
        let children: Vec<_> = all
            .iter()
            .filter(|t| t.metadata.as_ref().and_then(|m| m.parent_id.as_ref()) == Some(&"parent-1".to_string()))
            .collect();
        assert_eq!(children.len(), 3);

        // 验证根任务
        let roots: Vec<_> = all
            .iter()
            .filter(|t| t.metadata.as_ref().and_then(|m| m.parent_id.as_ref()).is_none())
            .collect();
        assert_eq!(roots.len(), 2);
    }

    // =========================================================================
    // Planner 集成测试
    // =========================================================================

    #[tokio::test]
    async fn test_planner_complexity_detection() {
        let planner = TaskPlanner::new(PlannerConfig::default());

        // 简单任务
        assert_eq!(
            planner.analyze_complexity("帮我查一下 example.com 的 WHOIS"),
            TaskComplexity::Simple
        );

        // 中等任务
        assert_eq!(
            planner.analyze_complexity("扫描 192.168.1.1 的开放端口"),
            TaskComplexity::Medium
        );

        // 复杂任务
        assert_eq!(
            planner.analyze_complexity("对 target.com 进行完整的渗透测试"),
            TaskComplexity::Complex
        );
        assert_eq!(
            planner.analyze_complexity("执行内网渗透，获取域控权限"),
            TaskComplexity::Complex
        );
    }

    #[tokio::test]
    async fn test_planner_parse_json_plan() {
        let planner = TaskPlanner::new(PlannerConfig::default());

        let llm_response = r#"
        好的，我来为你制定渗透测试计划：
        
        {
            "plan": {
                "description": "对 example.com 进行渗透测试",
                "steps": [
                    {"id": "1", "description": "WHOIS 查询", "tool": {"name": "whois_lookup", "args": {"domain": "example.com"}}},
                    {"id": "2", "description": "DNS 枚举", "tool": {"name": "dns_enum"}},
                    {"id": "3", "description": "端口扫描", "tool": {"name": "port_scan"}},
                    {"id": "4", "description": "漏洞分析"}
                ],
                "expected_outcome": "发现潜在安全漏洞"
            }
        }
        "#;

        let plan = planner
            .parse_plan_from_response("渗透测试", llm_response)
            .unwrap();
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(plan.steps[0].tool, Some("whois_lookup".to_string()));
        assert_eq!(plan.complexity, TaskComplexity::Medium);
    }

    #[tokio::test]
    async fn test_planner_parse_text_plan() {
        let planner = TaskPlanner::new(PlannerConfig::default());

        let llm_response = r#"
        我将按以下步骤执行：
        
        1. 首先进行信息收集，获取目标基本信息
        2. 然后进行端口扫描，发现开放服务
        3. 接着进行服务识别，确定服务版本
        4. 最后进行漏洞扫描
        5. 生成安全报告
        "#;

        let plan = planner
            .parse_plan_from_response("安全测试", llm_response)
            .unwrap();
        assert_eq!(plan.steps.len(), 5);
        assert!(plan.steps[0].description.contains("信息收集"));
    }

    #[tokio::test]
    async fn test_plan_to_todos_conversion() {
        let planner = TaskPlanner::new(PlannerConfig::default());

        let plan = TaskPlan {
            task: "测试任务".to_string(),
            description: "测试计划".to_string(),
            steps: vec![
                PlanStep {
                    id: "1".to_string(),
                    description: "步骤一".to_string(),
                    tool: Some("tool_a".to_string()),
                    args: None,
                    depends_on: vec![],
                },
                PlanStep {
                    id: "2".to_string(),
                    description: "步骤二".to_string(),
                    tool: Some("tool_b".to_string()),
                    args: None,
                    depends_on: vec!["1".to_string()],
                },
                PlanStep {
                    id: "3".to_string(),
                    description: "步骤三".to_string(),
                    tool: None,
                    args: None,
                    depends_on: vec![],
                },
            ],
            expected_outcome: "完成".to_string(),
            complexity: TaskComplexity::Medium,
        };

        let todos = planner.plan_to_todos(&plan);
        assert_eq!(todos.len(), 3);

        // 第一个应该是 InProgress
        assert_eq!(todos[0].status, TodoStatus::InProgress);
        assert_eq!(todos[0].metadata.as_ref().unwrap().tool_name, Some("tool_a".to_string()));

        // 其余应该是 Pending
        assert_eq!(todos[1].status, TodoStatus::Pending);
        assert_eq!(todos[2].status, TodoStatus::Pending);
    }

    // =========================================================================
    // Orchestrator 集成测试
    // =========================================================================

    #[tokio::test]
    async fn test_orchestrator_simple_task() {
        let orchestrator = AgentOrchestrator::new(OrchestratorConfig::default(), None);

        let prep = orchestrator
            .prepare_task("orch-simple", "查询 example.com")
            .await
            .unwrap();

        assert_eq!(prep.complexity, TaskComplexity::Simple);
        assert!(!prep.todos_created); // 简单任务不自动创建 Todos
    }

    #[tokio::test]
    async fn test_orchestrator_complex_task() {
        let orchestrator = AgentOrchestrator::new(OrchestratorConfig::default(), None);

        let prep = orchestrator
            .prepare_task("orch-complex", "对 target.com 进行渗透测试")
            .await
            .unwrap();

        assert_eq!(prep.complexity, TaskComplexity::Complex);
        assert!(prep.todos_created); // 复杂任务自动创建 Todos

        // 验证初始 Todo
        let todos = orchestrator.get_todos("orch-complex").await;
        assert!(!todos.is_empty());
        assert_eq!(todos[0].status, TodoStatus::InProgress);
    }

    #[tokio::test]
    async fn test_orchestrator_force_todos() {
        let config = OrchestratorConfig {
            force_todos: true,
            ..Default::default()
        };
        let orchestrator = AgentOrchestrator::new(config, None);

        let prep = orchestrator
            .prepare_task("orch-force", "简单查询")
            .await
            .unwrap();

        assert!(prep.todos_created); // 强制创建
    }

    #[tokio::test]
    async fn test_orchestrator_update_plan() {
        let orchestrator = AgentOrchestrator::new(OrchestratorConfig::default(), None);
        let exec_id = "orch-update-plan";

        // 准备任务
        let _ = orchestrator.prepare_task(exec_id, "渗透测试").await.unwrap();

        // 模拟 LLM 返回计划
        let llm_response = r#"
        {
            "plan": {
                "description": "渗透测试计划",
                "steps": [
                    {"id": "1", "description": "信息收集"},
                    {"id": "2", "description": "端口扫描"},
                    {"id": "3", "description": "漏洞分析"}
                ]
            }
        }
        "#;

        let plan = orchestrator
            .update_plan(exec_id, "渗透测试", llm_response)
            .await
            .unwrap();

        assert!(plan.is_some());

        // 验证 Todos 已更新
        let todos = orchestrator.get_todos(exec_id).await;
        assert_eq!(todos.len(), 3);
    }

    #[tokio::test]
    async fn test_orchestrator_step_progression() {
        let orchestrator = AgentOrchestrator::new(OrchestratorConfig::default(), None);
        let exec_id = "orch-progression";

        // 手动添加 Todos
        let todos = vec![
            Todo::new("1", "步骤1").with_status(TodoStatus::InProgress),
            Todo::new("2", "步骤2"),
            Todo::new("3", "步骤3"),
        ];
        orchestrator
            .todo_manager()
            .write_todos(exec_id, todos, false)
            .await
            .unwrap();

        // 完成第一步
        orchestrator.mark_step_completed(exec_id, "1").await.unwrap();

        // 开始下一步
        let next = orchestrator.start_next_pending_step(exec_id).await.unwrap();
        assert_eq!(next, Some("2".to_string()));

        // 验证状态
        let current = orchestrator.get_current_step(exec_id).await;
        assert!(current.is_some());
        assert_eq!(current.unwrap().id, "2");

        // 使用 advance_todos 自动推进
        let next = orchestrator.advance_todos(exec_id).await.unwrap();
        assert_eq!(next, Some("3".to_string()));

        // 验证最终状态
        let todos = orchestrator.get_todos(exec_id).await;
        assert_eq!(todos[0].status, TodoStatus::Completed);
        assert_eq!(todos[1].status, TodoStatus::Completed);
        assert_eq!(todos[2].status, TodoStatus::InProgress);
    }

    #[tokio::test]
    async fn test_orchestrator_cleanup() {
        let orchestrator = AgentOrchestrator::new(OrchestratorConfig::default(), None);
        let exec_id = "orch-cleanup";

        // 添加数据
        orchestrator
            .todo_manager()
            .write_todos(exec_id, vec![Todo::new("1", "测试")], false)
            .await
            .unwrap();

        assert!(!orchestrator.get_todos(exec_id).await.is_empty());

        // 清理
        orchestrator.cleanup(exec_id).await;

        assert!(orchestrator.get_todos(exec_id).await.is_empty());
    }

    // =========================================================================
    // 完整流程测试
    // =========================================================================

    #[tokio::test]
    async fn test_full_agent_flow() {
        // 模拟完整的 Agent 执行流程
        let orchestrator = AgentOrchestrator::new(OrchestratorConfig::default(), None);
        let exec_id = "full-flow-test";
        let task = "对 example.com 进行全面的渗透测试和漏洞分析";

        // 1. 准备任务
        let prep = orchestrator.prepare_task(exec_id, task).await.unwrap();
        assert_eq!(prep.complexity, TaskComplexity::Complex);
        assert!(prep.todos_created);

        // 2. 模拟 LLM 返回计划
        let llm_plan = r#"
        {
            "plan": {
                "description": "安全扫描计划",
                "steps": [
                    {"id": "recon", "description": "信息收集", "tool": {"name": "whois_lookup"}},
                    {"id": "scan", "description": "端口扫描", "tool": {"name": "port_scan"}},
                    {"id": "vuln", "description": "漏洞扫描", "tool": {"name": "vuln_scan"}},
                    {"id": "report", "description": "生成报告"}
                ]
            }
        }
        "#;

        orchestrator
            .update_plan(exec_id, task, llm_plan)
            .await
            .unwrap();

        // 3. 验证计划已转换为 Todos
        let todos = orchestrator.get_todos(exec_id).await;
        assert_eq!(todos.len(), 4);
        assert_eq!(todos[0].id, "recon");
        assert_eq!(todos[0].status, TodoStatus::InProgress);

        // 4. 模拟执行流程
        // 完成 recon
        orchestrator.mark_step_completed(exec_id, "recon").await.unwrap();
        orchestrator.mark_step_started(exec_id, "scan").await.unwrap();

        let todos = orchestrator.get_todos(exec_id).await;
        assert_eq!(todos[0].status, TodoStatus::Completed);
        assert_eq!(todos[1].status, TodoStatus::InProgress);

        // 完成 scan
        orchestrator.mark_step_completed(exec_id, "scan").await.unwrap();
        orchestrator.mark_step_started(exec_id, "vuln").await.unwrap();

        // 模拟 vuln 失败
        orchestrator.mark_step_failed(exec_id, "vuln").await.unwrap();

        let todos = orchestrator.get_todos(exec_id).await;
        assert_eq!(todos[2].status, TodoStatus::Cancelled);

        // 5. 获取最终状态
        let stats = orchestrator.todo_manager().get_stats(exec_id).await;
        assert_eq!(stats.completed, 2);
        assert_eq!(stats.cancelled, 1);
        assert_eq!(stats.pending, 1);

        // 6. 清理
        orchestrator.cleanup(exec_id).await;
    }
}

