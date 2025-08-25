//! Agent取消逻辑测试

use crate::agents::manager::AgentManager;
use crate::agents::traits::*;
use crate::services::database::DatabaseService;
use std::sync::Arc;
use anyhow::Result;
use tokio::time::{sleep, Duration};

/// 测试Agent取消功能
pub async fn test_agent_cancellation() -> Result<()> {
    // 创建测试数据库服务
    let db_service = Arc::new(DatabaseService::new());
    // 创建Agent管理器
    let manager = Arc::new(AgentManager::new(db_service));
    
    // 初始化管理器
    manager.initialize().await?;
    
    println!("Testing agent cancellation functionality...");
    
    // 创建一个测试任务
    let request = crate::agents::manager::MultiAgentRequest {
        user_input: "Test cancellation task".to_string(),
        target: Some("127.0.0.1".to_string()),
        context: std::collections::HashMap::new(),
        selection_strategy: crate::agents::manager::AgentSelectionStrategy::Auto,
        priority: TaskPriority::Normal,
        user_id: "test_user".to_string(),
    };
    
    // 分发任务
    let session_id = manager.dispatch_task(request).await?;
    println!("Task dispatched with session ID: {}", session_id);
    
    // 等待一小段时间确保任务开始执行
    sleep(Duration::from_millis(100)).await;
    
    // 检查任务状态
    if let Some(status) = manager.get_session_status(&session_id).await {
        println!("Task status before cancellation: {:?}", status);
    }
    
    // 取消任务
    println!("Attempting to cancel task...");
    match manager.cancel_task(&session_id).await {
        Ok(_) => {
            println!("Task cancellation request sent successfully");
            
            // 等待一小段时间让取消生效
            sleep(Duration::from_millis(200)).await;
            
            // 检查取消后的状态
            if let Some(status) = manager.get_session_status(&session_id).await {
                println!("Task status after cancellation: {:?}", status);
            } else {
                println!("Task was removed from active sessions (cancelled successfully)");
            }
            
            // 检查已完成会话中是否有取消的任务
            let completed_sessions = manager.get_all_sessions().await;
            if let Some(session_info) = completed_sessions.get(&session_id) {
                println!("Cancelled task found in completed sessions:");
                println!("  Status: {:?}", session_info.status);
                println!("  Error: {:?}", session_info.error);
            }
            
            println!("Cancellation test completed successfully!");
        }
        Err(e) => {
            println!("Failed to cancel task: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

/// 测试并发取消
pub async fn test_concurrent_cancellation() -> Result<()> {
    println!("Testing concurrent cancellation...");
    
    // 创建测试数据库服务
    let db_service = Arc::new(DatabaseService::new());
    let manager = Arc::new(AgentManager::new(db_service));
    manager.initialize().await?;
    
    // 创建多个任务
    let mut session_ids = Vec::new();
    
    for i in 0..3 {
        let request = crate::agents::manager::MultiAgentRequest {
            user_input: format!("Test concurrent task {}", i),
            target: Some("127.0.0.1".to_string()),
            context: std::collections::HashMap::new(),
            selection_strategy: crate::agents::manager::AgentSelectionStrategy::Auto,
            priority: TaskPriority::Normal,
            user_id: "test_user".to_string(),
        };
        
        let session_id = manager.dispatch_task(request).await?;
        session_ids.push(session_id);
        println!("Dispatched task {}: {}", i, session_ids[i]);
    }
    
    // 等待任务开始
    sleep(Duration::from_millis(100)).await;
    
    // 同时取消所有任务
    let cancel_tasks: Vec<_> = session_ids.iter().map(|id| {
        let manager_clone = manager.clone();
        let id_clone = id.clone();
        tokio::spawn(async move {
            manager_clone.cancel_task(&id_clone).await
        })
    }).collect();
    
    // 等待所有取消操作完成
    for (i, task) in cancel_tasks.into_iter().enumerate() {
        match task.await {
            Ok(Ok(_)) => println!("Task {} cancelled successfully", i),
            Ok(Err(e)) => println!("Task {} cancellation failed: {}", i, e),
            Err(e) => println!("Task {} cancellation task failed: {}", i, e),
        }
    }
    
    // 等待一段时间让所有操作完成
    sleep(Duration::from_millis(500)).await;
    
    // 检查统计信息
    let stats = manager.get_statistics().await;
    println!("Final statistics:");
    println!("  Total tasks: {}", stats.total_tasks);
    println!("  Failed tasks: {}", stats.failed_tasks);
    println!("  Active sessions: {}", stats.active_sessions);
    
    println!("Concurrent cancellation test completed!");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cancellation_functionality() {
        if let Err(e) = test_agent_cancellation().await {
            panic!("Cancellation test failed: {}", e);
        }
    }
    
    #[tokio::test]
    async fn test_concurrent_cancellation_functionality() {
        if let Err(e) = test_concurrent_cancellation().await {
            panic!("Concurrent cancellation test failed: {}", e);
        }
    }
}
