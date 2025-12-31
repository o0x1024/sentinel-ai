#[cfg(test)]
mod task_planner_tests {
    use sentinel_tools::buildin_tools::task_planner::{TaskPlannerTool, TaskPlannerArgs, TaskStatus};
    use rig::tool::Tool;

    #[tokio::test]
    async fn test_replan() {
        let tool = TaskPlannerTool::new();
        let exec_id = "test-replan-001".to_string();

        // Add initial tasks
        let result = tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "add_tasks".to_string(),
            tasks: Some(vec![
                "Task 1".to_string(),
                "Task 2".to_string(),
            ]),
            task_index: None,
            status: None,
            result: None,
            new_description: None,
        }).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.plan.as_ref().unwrap().tasks.len(), 2);

        // Replan with new tasks
        let result = tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "replan".to_string(),
            tasks: Some(vec![
                "New Task 1".to_string(),
                "New Task 2".to_string(),
                "New Task 3".to_string(),
            ]),
            task_index: None,
            status: None,
            result: None,
            new_description: None,
        }).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.plan.as_ref().unwrap().tasks.len(), 3);
        assert_eq!(output.plan.as_ref().unwrap().tasks[0].description, "New Task 1");
        assert_eq!(output.plan.as_ref().unwrap().tasks[0].status, TaskStatus::InProgress);
    }

    #[tokio::test]
    async fn test_update_task() {
        let tool = TaskPlannerTool::new();
        let exec_id = "test-update-001".to_string();

        // Add tasks
        tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "add_tasks".to_string(),
            tasks: Some(vec![
                "Original description".to_string(),
            ]),
            task_index: None,
            status: None,
            result: None,
            new_description: None,
        }).await.unwrap();

        // Update task description
        let result = tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "update_task".to_string(),
            tasks: None,
            task_index: Some(0),
            status: None,
            result: None,
            new_description: Some("Updated description".to_string()),
        }).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.plan.as_ref().unwrap().tasks[0].description, "Updated description");
    }

    #[tokio::test]
    async fn test_delete_task() {
        let tool = TaskPlannerTool::new();
        let exec_id = "test-delete-001".to_string();

        // Add tasks
        tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "add_tasks".to_string(),
            tasks: Some(vec![
                "Task 1".to_string(),
                "Task 2".to_string(),
                "Task 3".to_string(),
            ]),
            task_index: None,
            status: None,
            result: None,
            new_description: None,
        }).await.unwrap();

        // Delete middle task
        let result = tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "delete_task".to_string(),
            tasks: None,
            task_index: Some(1),
            status: None,
            result: None,
            new_description: None,
        }).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.plan.as_ref().unwrap().tasks.len(), 2);
        assert_eq!(output.plan.as_ref().unwrap().tasks[0].description, "Task 1");
        assert_eq!(output.plan.as_ref().unwrap().tasks[1].description, "Task 3");
    }

    #[tokio::test]
    async fn test_insert_task() {
        let tool = TaskPlannerTool::new();
        let exec_id = "test-insert-001".to_string();

        // Add tasks
        tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "add_tasks".to_string(),
            tasks: Some(vec![
                "Task 1".to_string(),
                "Task 3".to_string(),
            ]),
            task_index: None,
            status: None,
            result: None,
            new_description: None,
        }).await.unwrap();

        // Insert task in the middle
        let result = tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "insert_task".to_string(),
            tasks: None,
            task_index: Some(1),
            status: None,
            result: None,
            new_description: Some("Task 2".to_string()),
        }).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.plan.as_ref().unwrap().tasks.len(), 3);
        assert_eq!(output.plan.as_ref().unwrap().tasks[0].description, "Task 1");
        assert_eq!(output.plan.as_ref().unwrap().tasks[1].description, "Task 2");
        assert_eq!(output.plan.as_ref().unwrap().tasks[2].description, "Task 3");
    }

    #[tokio::test]
    async fn test_delete_current_task() {
        let tool = TaskPlannerTool::new();
        let exec_id = "test-delete-current-001".to_string();

        // Add tasks
        tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "add_tasks".to_string(),
            tasks: Some(vec![
                "Task 1".to_string(),
                "Task 2".to_string(),
                "Task 3".to_string(),
            ]),
            task_index: None,
            status: None,
            result: None,
            new_description: None,
        }).await.unwrap();

        // Current task is 0 (InProgress by default)
        // Delete current task
        let result = tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "delete_task".to_string(),
            tasks: None,
            task_index: Some(0),
            status: None,
            result: None,
            new_description: None,
        }).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.plan.as_ref().unwrap().tasks.len(), 2);
        // Current task should move to next (now at index 0)
        assert_eq!(output.plan.as_ref().unwrap().current_task_index, Some(0));
        assert_eq!(output.plan.as_ref().unwrap().tasks[0].status, TaskStatus::InProgress);
    }

    #[tokio::test]
    async fn test_error_cases() {
        let tool = TaskPlannerTool::new();
        let exec_id = "test-error-001".to_string();

        // Test missing parameters for replan
        let result = tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "replan".to_string(),
            tasks: None,
            task_index: None,
            status: None,
            result: None,
            new_description: None,
        }).await;
        assert!(result.is_err());

        // Test index out of bounds
        let result = tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "delete_task".to_string(),
            tasks: None,
            task_index: Some(999),
            status: None,
            result: None,
            new_description: None,
        }).await;
        assert!(result.is_err());

        // Test unknown action
        let result = tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "unknown_action".to_string(),
            tasks: None,
            task_index: None,
            status: None,
            result: None,
            new_description: None,
        }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ctf_workflow() {
        let tool = TaskPlannerTool::new();
        let exec_id = "ctf-workflow-001".to_string();

        // Initial plan
        tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "add_tasks".to_string(),
            tasks: Some(vec![
                "Test SQL injection".to_string(),
                "Extract database info".to_string(),
                "Find flag".to_string(),
            ]),
            task_index: None,
            status: None,
            result: None,
            new_description: None,
        }).await.unwrap();

        // Complete first task
        tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "update_status".to_string(),
            tasks: None,
            task_index: Some(0),
            status: Some(TaskStatus::Completed),
            result: Some("Found SQL injection, keywords filtered".to_string()),
            new_description: None,
        }).await.unwrap();

        // Insert bypass task
        tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "insert_task".to_string(),
            tasks: None,
            task_index: Some(1),
            status: None,
            result: None,
            new_description: Some("Apply bypass techniques".to_string()),
        }).await.unwrap();

        // Verify plan
        let result = tool.call(TaskPlannerArgs {
            execution_id: exec_id.clone(),
            action: "get_plan".to_string(),
            tasks: None,
            task_index: None,
            status: None,
            result: None,
            new_description: None,
        }).await.unwrap();

        let plan = result.plan.unwrap();
        assert_eq!(plan.tasks.len(), 4);
        assert_eq!(plan.tasks[0].status, TaskStatus::Completed);
        assert_eq!(plan.tasks[1].description, "Apply bypass techniques");
        assert_eq!(plan.tasks[1].status, TaskStatus::Pending);
        // Current task should be at index 2 (was 1, shifted by insert)
        assert_eq!(plan.current_task_index, Some(2));
        assert_eq!(plan.tasks[2].status, TaskStatus::InProgress);
    }
}
