use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use anyhow::Result;
use tauri::{AppHandle, Emitter, State};
use chrono::Utc;

use crate::engines::intelligent_dispatcher::workflow_engine::{WorkflowEngine, WorkflowDefinition, WorkflowMetadata, WorkflowStep, ExecutionStatus};
use crate::models::workflow_graph::{WorkflowGraph, NodeDef};
use crate::services::database::DatabaseService;
use crate::tools::unified_manager::ToolSystem;
use crate::tools::unified_types::ToolExecutionParams;
use sentinel_db::Database;
use crate::commands::rag_commands;
use crate::services::{PromptService, PromptServiceConfig, PromptBuildRequest, PromptBuildResponse, PromptBuildType};
use crate::commands::prompt_commands::PromptServiceState;
use sentinel_prompt::{PromptBuildContext, TargetInfo, ExecutionContext, HistoryItem};

#[derive(Debug, serde::Serialize)]
struct WorkflowEventPayload<'a> {
    execution_id: &'a str,
    workflow_id: &'a str,
    workflow_name: &'a str,
    version: &'a str,
    status: &'a str,
    current_step: Option<&'a str>,
    progress: Option<u32>,
}

fn graph_to_definition(graph: &WorkflowGraph) -> WorkflowDefinition {
    let mut depends_map: HashMap<String, Vec<String>> = HashMap::new();
    for e in &graph.edges {
        depends_map.entry(e.to_node.clone()).or_default().push(e.from_node.clone());
    }
    let steps: Vec<WorkflowStep> = graph.nodes.iter().map(|n| {
        WorkflowStep {
            id: n.id.clone(),
            name: n.node_name.clone(),
            agent_type: "node".to_string(),
            action: n.node_type.clone(),
            inputs: n.params.clone(),
            outputs: HashMap::new(),
            depends_on: depends_map.get(&n.id).cloned().unwrap_or_default(),
            condition: None,
            retry: None,
            timeout: Some(10.0),
            parallel: false,
            config: None,
        }
    }).collect();

    WorkflowDefinition {
        metadata: WorkflowMetadata {
            id: graph.id.clone(),
            name: graph.name.clone(),
            version: graph.version.clone(),
            description: format!("Graph with {} nodes", graph.nodes.len()),
            author: None,
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        steps,
        variables: HashMap::new(),
        error_handling: None,
        notifications: None,
    }
}

fn topo_order(nodes: &[NodeDef], edges: &[(String, String)]) -> Vec<String> {
    let mut indeg: HashMap<String, usize> = nodes.iter().map(|n| (n.id.clone(), 0)).collect();
    let mut adj: HashMap<String, Vec<String>> = nodes.iter().map(|n| (n.id.clone(), vec![])).collect();
    for (u, v) in edges {
        if let Some(x) = indeg.get_mut(v) { *x += 1; }
        adj.entry(u.clone()).or_default().push(v.clone());
    }
    let mut q: VecDeque<String> = indeg.iter().filter(|(_, &d)| d == 0).map(|(k, _)| k.clone()).collect();
    let mut order = Vec::new();
    while let Some(u) = q.pop_front() {
        order.push(u.clone());
        if let Some(nei) = adj.get(&u) {
            for v in nei { 
                if let Some(d) = indeg.get_mut(v) { 
                    *d -= 1; 
                    if *d == 0 { q.push_back(v.clone()); }
                }
            }
        }
    }
    if order.len() != nodes.len() { order } else { order }
}

#[tauri::command]
pub async fn start_workflow_run(
    graph: WorkflowGraph,
    app_handle: AppHandle,
    engine: State<'_, Arc<WorkflowEngine>>,
    db: State<'_, Arc<DatabaseService>>,
    tool_system: State<'_, Arc<ToolSystem>>,
    prompt_state: State<'_, PromptServiceState>,
) -> Result<String, String> {
    let def = graph_to_definition(&graph);
    let execution_id = engine.execute_workflow(&def, None).await.map_err(|e| e.to_string())?;
    let execution_id_for_spawn = execution_id.clone();

    let _ = app_handle.emit("workflow:run-start", &serde_json::json!({
        "execution_id": execution_id,
        "workflow_id": def.metadata.id,
        "workflow_name": def.metadata.name,
        "version": def.metadata.version,
        "status": "running"
    }));

    let db_clone = db.inner().clone();
    let app_handle_clone = app_handle.clone();
    let engine_clone = engine.inner().clone();
    let def_clone = def.clone();
    let tool_system_clone = tool_system.inner().clone();
    let prompt_state_clone = prompt_state.inner().clone();
    
    tokio::spawn(async move {
        // 插入运行记录
        if let Err(e) = db_clone.create_workflow_run(&execution_id_for_spawn, &def_clone.metadata.id, &def_clone.metadata.name, &def_clone.metadata.version, "running", Utc::now()).await {
            tracing::warn!("Failed to create workflow_run: {}", e);
        }

        // 计算拓扑顺序
        let edges: Vec<(String, String)> = graph.edges.iter().map(|e| (e.from_node.clone(), e.to_node.clone())).collect();
        let order = topo_order(&graph.nodes, &edges);
        let total = order.len().max(1) as u32;
        let mut completed = 0u32;
        let mut branch_results: HashMap<String, bool> = HashMap::new();

        for node_id in order {
            let _ = app_handle_clone.emit("workflow:step-start", &serde_json::json!({
                "execution_id": execution_id_for_spawn,
                "step_id": node_id
            }));

            engine_clone.update_current_step(&execution_id_for_spawn, &node_id).await;
            if let Err(e) = db_clone.save_workflow_run_step(&execution_id_for_spawn, &node_id, "running", Utc::now()).await { tracing::warn!("save step: {}", e); }

            let mut wrote_result = false;
            // 跳过不满足分支条件的节点
            let incoming = graph.edges.iter().filter(|e| e.to_node == node_id).collect::<Vec<_>>();
            let mut gated_by_branch = false;
            let mut branch_allowed = true;
            for e in &incoming {
                if let Some(from_step) = def_clone.steps.iter().find(|s| s.id == e.from_node) {
                    if from_step.action == "branch" {
                        gated_by_branch = true;
                        let selected = branch_results.get(&e.from_node).cloned().unwrap_or(true);
                        let expects_true = e.from_port == "true";
                        if selected != expects_true { branch_allowed = false; }
                    }
                }
            }
            if gated_by_branch && !branch_allowed {
                engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, serde_json::json!({"skipped": true})).await;
                wrote_result = true;
            }

            // 执行节点对应的工具或服务（若存在）
            if let Some(step_def) = def_clone.steps.iter().find(|s| s.id == node_id) {
                let action = step_def.action.clone();
                if action.starts_with("tool::") {
                    // 工具执行
                    let mut tool_name = action;
                    if let Some(stripped) = tool_name.strip_prefix("tool::") {
                        tool_name = stripped.to_string();
                    }

                    let params = ToolExecutionParams {
                        inputs: step_def.inputs.clone(),
                        context: std::collections::HashMap::new(),
                        timeout: Some(std::time::Duration::from_secs(30)),
                        execution_id: None,
                    };

                    match tool_system_clone.execute_tool(&tool_name, params).await {
                        Ok(exec_result) => {
                            engine_clone
                                .mark_step_completed_with_result(
                                    &execution_id_for_spawn,
                                    &node_id,
                                    exec_result.output,
                                )
                                .await;
                            wrote_result = true;
                        }
                        Err(err) => {
                            tracing::warn!("tool execute failed for {}: {}", tool_name, err);
                        }
                    }
                } else if action == "rag::ingest" {
                    // RAG 导入
                    let file_path = step_def.inputs.get("file_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let collection_id = step_def.inputs.get("collection_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let metadata_map = step_def.inputs.get("metadata").and_then(|v| v.as_object());
                    let mut metadata: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                    if let Some(obj) = metadata_map {
                        for (k, v) in obj.iter() {
                            if let Some(s) = v.as_str() { metadata.insert(k.clone(), s.to_string()); }
                        }
                    }

                    match rag_commands::rag_ingest_source(file_path, collection_id, Some(metadata)).await {
                        Ok(resp) => {
                            let result_json = serde_json::to_value(resp).unwrap_or(serde_json::json!({"status":"ok"}));
                            engine_clone
                                .mark_step_completed_with_result(
                                    &execution_id_for_spawn,
                                    &node_id,
                                    result_json,
                                )
                                .await;
                            wrote_result = true;
                        }
                        Err(e) => {
                            tracing::warn!("rag ingest failed: {}", e);
                        }
                    }
                } else if action == "rag::query" {
                    // RAG 查询
                    let query = step_def.inputs.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let collection_id = step_def.inputs.get("collection_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let top_k = step_def.inputs.get("top_k").and_then(|v| v.as_u64()).map(|n| n as usize);
                    let use_mmr = step_def.inputs.get("use_mmr").and_then(|v| v.as_bool());
                    let mmr_lambda = step_def.inputs.get("mmr_lambda").and_then(|v| v.as_f64()).map(|n| n as f32);
                    let filters_obj = step_def.inputs.get("filters").and_then(|v| v.as_object());
                    let mut filters: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                    if let Some(obj) = filters_obj {
                        for (k, v) in obj.iter() {
                            if let Some(s) = v.as_str() { filters.insert(k.clone(), s.to_string()); }
                        }
                    }

                    match rag_commands::rag_query(query, collection_id, top_k, use_mmr, mmr_lambda, Some(filters)).await {
                        Ok(resp) => {
                            let result_json = serde_json::to_value(resp).unwrap_or(serde_json::json!({"status":"ok"}));
                            engine_clone
                                .mark_step_completed_with_result(
                                    &execution_id_for_spawn,
                                    &node_id,
                                    result_json,
                                )
                                .await;
                            wrote_result = true;
                        }
                        Err(e) => {
                            tracing::warn!("rag query failed: {}", e);
                        }
                    }
                } else if action == "prompt::build" {
                    // Prompt 构建
                    // 获取或初始化服务（避免跨 await 持有锁）
                    let mut service_opt = {
                        let mut guard = prompt_state_clone.write().await;
                        if guard.is_none() {
                            match PromptService::new(PromptServiceConfig::default()).await {
                                Ok(service) => { *guard = Some(service); }
                                Err(e) => { tracing::warn!("init prompt service failed: {}", e); }
                            }
                        }
                        guard.take()
                    };

                    if let Some(service) = &service_opt {
                        // 会话
                        let session_id = step_def.inputs.get("session_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let session_id = match session_id {
                            Some(s) if !s.is_empty() => s,
                            _ => {
                                match service.create_session(None, None).await { Ok(id) => id, Err(e) => { tracing::warn!("create session failed: {}", e); "default_session".to_string() } }
                            }
                        };

                        // 构建类型
                        let build_type_str = step_def.inputs.get("build_type").and_then(|v| v.as_str()).unwrap_or("Planner");
                        let build_type = match build_type_str { "Executor" => PromptBuildType::Executor, "Replanner" => PromptBuildType::Replanner, "ReportGenerator" => PromptBuildType::ReportGenerator, _ => PromptBuildType::Planner };

                        // 构建上下文
                        let user_query = step_def.inputs.get("user_query").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let target_info = step_def.inputs.get("target_info").cloned().and_then(|v| serde_json::from_value::<TargetInfo>(v).ok());
                        let execution_context = step_def.inputs.get("execution_context").cloned().and_then(|v| serde_json::from_value::<ExecutionContext>(v).ok());
                        let history = step_def.inputs.get("history").and_then(|v| v.as_array()).map(|arr| {
                            arr.iter().filter_map(|item| serde_json::from_value::<HistoryItem>(item.clone()).ok()).collect::<Vec<HistoryItem>>()
                        }).unwrap_or_default();
                        let custom_vars_obj = step_def.inputs.get("custom_variables").and_then(|v| v.as_object());
                        let mut custom_variables: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
                        if let Some(obj) = custom_vars_obj { for (k, v) in obj.iter() { custom_variables.insert(k.clone(), v.clone()); } }

                        let ctx = PromptBuildContext {
                            user_query,
                            target_info,
                            available_tools: vec![],
                            execution_context,
                            history,
                            custom_variables,
                            rag_context: None,
                        };

                        let record_performance = step_def.inputs.get("record_performance").and_then(|v| v.as_bool()).unwrap_or(false);

                        let req = PromptBuildRequest { session_id, build_type, context: ctx, config_override: None, record_performance };

                        let build_res = service.build_prompt(req).await;
                        // 将服务放回状态
                        {
                            let mut guard = prompt_state_clone.write().await;
                            *guard = service_opt;
                        }

                        match build_res {
                            Ok(resp) => {
                                let result_json = serde_json::to_value(resp).unwrap_or(serde_json::json!({"status":"ok"}));
                                engine_clone
                                    .mark_step_completed_with_result(
                                        &execution_id_for_spawn,
                                        &node_id,
                                        result_json,
                                    )
                                    .await;
                                wrote_result = true;
                            }
                            Err(e) => {
                                tracing::warn!("prompt build failed: {}", e);
                            }
                        }
                    }
                } else if action == "branch" {
                    let expr = step_def.inputs.get("expr").and_then(|v| v.as_str()).unwrap_or("true");
                    let selected = match expr {
                        "true" => true,
                        "false" => false,
                        _ => true,
                    };
                    branch_results.insert(node_id.clone(), selected);
                    let result_json = serde_json::json!({"result": selected});
                    engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, result_json).await;
                    wrote_result = true;
                } else if action == "merge" {
                    let inputs = graph.edges.iter().filter(|e| e.to_node == node_id).collect::<Vec<_>>();
                    let mut merged = serde_json::Map::new();
                    for e in inputs {
                        if let Some(val) = engine_clone.get_step_result(&execution_id_for_spawn, &e.from_node).await { merged.insert(e.from_port.clone(), val); }
                    }
                    engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, serde_json::Value::Object(merged)).await;
                    wrote_result = true;
                } else if action == "retry" {
                    let times = step_def.inputs.get("times").and_then(|v| v.as_u64()).map(|n| n as u32).unwrap_or(3);
                    let delay_ms = step_def.inputs.get("delay_ms").and_then(|v| v.as_u64()).unwrap_or(500);
                    let tool_name = step_def.inputs.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");
                    let tool_params = step_def.inputs.get("tool_params").and_then(|v| v.as_object()).cloned().unwrap_or_default();
                    if !tool_name.is_empty() {
                        let mut last_err: Option<String> = None;
                        for _attempt in 0..times {
                            let mut inputs_map: HashMap<String, serde_json::Value> = HashMap::new();
                            for (k, v) in tool_params.iter() { inputs_map.insert(k.clone(), v.clone()); }
                            let params = ToolExecutionParams { inputs: inputs_map, context: HashMap::new(), timeout: Some(std::time::Duration::from_secs(30)), execution_id: None };
                            match tool_system_clone.execute_tool(tool_name, params).await {
                                Ok(exec_result) => {
                                    engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, exec_result.output).await;
                                    wrote_result = true;
                                    break;
                                }
                                Err(e) => { last_err = Some(e.to_string()); tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await; }
                            }
                        }
                        if !wrote_result { engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, serde_json::json!({"error": last_err.unwrap_or("unknown".to_string())})).await; wrote_result = true; }
                    }
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }

            // 步骤完成事件
            if !wrote_result {
                engine_clone.mark_step_completed(&execution_id_for_spawn, &node_id).await;
            }
            let _ = app_handle_clone.emit("workflow:step-complete", &serde_json::json!({
                "execution_id": execution_id_for_spawn,
                "step_id": node_id
            }));
            if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "completed", Utc::now()).await { tracing::warn!("update step: {}", e); }

            completed += 1;
            let progress = ((completed as f32 / total as f32) * 100.0) as u32;
            engine_clone.update_progress(&execution_id_for_spawn, progress).await;
            let _ = app_handle_clone.emit("workflow:progress", &serde_json::json!({
                "execution_id": execution_id_for_spawn,
                "progress": progress,
                "completed_steps": completed,
                "total_steps": total
            }));
            let _ = db_clone.update_workflow_run_progress(&execution_id_for_spawn, progress, completed, total).await;
        }

        engine_clone.mark_execution_completed(&execution_id_for_spawn).await;
        let _ = db_clone.update_workflow_run_status(&execution_id_for_spawn, "completed", Some(Utc::now()), None).await;
        let _ = app_handle_clone.emit("workflow:run-complete", &serde_json::json!({
            "execution_id": execution_id_for_spawn
        }));
    });

    Ok(execution_id)
}

#[tauri::command]
pub async fn get_workflow_run_status(
    execution_id: String,
    engine: State<'_, Arc<WorkflowEngine>>,
) -> Result<serde_json::Value, String> {
    match engine.get_execution_status(&execution_id).await {
        Ok(s) => Ok(serde_json::to_value(s).unwrap_or_default()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn list_workflow_runs(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<serde_json::Value>, String> {
    db.list_workflow_runs().await.map_err(|e| e.to_string())
}
