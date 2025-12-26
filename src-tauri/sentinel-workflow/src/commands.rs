use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use anyhow::Result;
use tauri::{AppHandle, Emitter, State};
use chrono::Utc;

use crate::engine::{WorkflowEngine, WorkflowDefinition, WorkflowMetadata, WorkflowStep};
use sentinel_db::DatabaseService;
use sentinel_db::Database;
use sentinel_traffic::PluginManager;
use rig::tool::ToolSet;
use serde::{Deserialize, Serialize};
use sentinel_rag::{RagService, RagQueryRequest, IngestRequest, config::RagConfig};
use sentinel_prompt::{PromptBuilder, PromptConfigManager, PromptBuildContext, TargetInfo, ExecutionContext, HistoryItem};
use sentinel_db::core::models::rag_config::RagConfig as CoreRagConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortType {
    String,
    Integer,
    Float,
    Boolean,
    Json,
    Array(Box<PortType>),
    Object(HashMap<String, PortType>),
    Artifact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDef {
    pub id: String,
    pub name: String,
    pub port_type: PortType,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDef {
    pub name: String,
    pub var_type: PortType,
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialRef {
    pub name: String,
    pub provider: String,
    pub ref_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDef {
    pub id: String,
    pub node_type: String,
    pub node_name: String,
    pub x: f64,
    pub y: f64,
    pub params: HashMap<String, serde_json::Value>,
    pub input_ports: Vec<PortDef>,
    pub output_ports: Vec<PortDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDef {
    pub id: String,
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraph {
    pub id: String,
    pub name: String,
    pub version: String,
    pub nodes: Vec<NodeDef>,
    pub edges: Vec<EdgeDef>,
    pub variables: Vec<VariableDef>,
    pub credentials: Vec<CredentialRef>,
}

pub fn graph_to_definition(graph: &WorkflowGraph) -> WorkflowDefinition {
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

pub fn topo_order(nodes: &[NodeDef], edges: &[(String, String)]) -> Vec<String> {
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

fn convert_core_to_rag(core: CoreRagConfig) -> RagConfig {
    RagConfig {
        database_path: core.database_path,
        chunk_size_chars: core.chunk_size_chars,
        chunk_overlap_chars: core.chunk_overlap_chars,
        top_k: core.top_k,
        mmr_lambda: core.mmr_lambda,
        batch_size: core.batch_size,
        max_concurrent: core.max_concurrent,
        embedding_provider: core.embedding_provider,
        embedding_model: core.embedding_model,
        embedding_dimensions: core.embedding_dimensions,
        embedding_api_key: core.embedding_api_key,
        embedding_base_url: core.embedding_base_url,
        reranking_provider: core.reranking_provider,
        reranking_model: core.reranking_model,
        reranking_enabled: core.reranking_enabled,
        similarity_threshold: core.similarity_threshold,
        augmentation_enabled: core.augmentation_enabled,
        context_window_size: core.context_window_size,
        chunk_expansion_enabled: core.chunk_expansion_enabled,
        chunk_expansion_before: core.chunk_expansion_before,
        chunk_expansion_after: core.chunk_expansion_after,
        chunking_strategy: match core.chunking_strategy {
            sentinel_db::core::models::rag_config::ChunkingStrategy::FixedSize => sentinel_rag::config::ChunkingStrategy::FixedSize,
            sentinel_db::core::models::rag_config::ChunkingStrategy::RecursiveCharacter => sentinel_rag::config::ChunkingStrategy::RecursiveCharacter,
            sentinel_db::core::models::rag_config::ChunkingStrategy::Semantic => sentinel_rag::config::ChunkingStrategy::Semantic,
            sentinel_db::core::models::rag_config::ChunkingStrategy::StructureAware => sentinel_rag::config::ChunkingStrategy::StructureAware,
        },
        min_chunk_size_chars: core.min_chunk_size_chars,
        max_chunk_size_chars: core.max_chunk_size_chars,
    }
}

/// 执行工作流步骤（供 start_workflow_run 和调度器共用）
pub async fn execute_workflow_steps(
    execution_id: String,
    graph: WorkflowGraph,
    def: WorkflowDefinition,
    db: Arc<DatabaseService>,
    app_handle: AppHandle,
    engine: Arc<WorkflowEngine>,
    toolset: Arc<ToolSet>,
    plugin_manager: Option<Arc<PluginManager>>,
) {
    let execution_id_for_spawn = execution_id;
    let db_clone = db;
    let app_handle_clone = app_handle;
    let engine_clone = engine;
    let def_clone = def;
    let toolset_clone = toolset;
    let plugin_manager_clone = plugin_manager;
        if let Err(e) = db_clone.create_workflow_run(&execution_id_for_spawn, &def_clone.metadata.id, &def_clone.metadata.name, &def_clone.metadata.version, "running", Utc::now()).await {
            tracing::warn!("Failed to create workflow_run: {}", e);
        }

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
                let result = serde_json::json!({"skipped": true});
                engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, result.clone()).await;
                if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "completed", Utc::now(), Some(result.to_string()), None).await {
                    tracing::warn!("failed to update step status: {}", e);
                }
                wrote_result = true;
            }

            if let Some(step_def) = def_clone.steps.iter().find(|s| s.id == node_id) {
                let action = step_def.action.clone();
                if action.starts_with("tool::") {
                    let mut tool_name = action;
                    if let Some(stripped) = tool_name.strip_prefix("tool::") {
                        tool_name = stripped.to_string();
                    }

                    // 使用 rig-core ToolSet 调用工具
                    let params_json = serde_json::to_string(&step_def.inputs).unwrap_or_default();
                    match toolset_clone.call(&tool_name, params_json).await {
                        Ok(result) => {
                            let result_value = serde_json::from_str(&result).unwrap_or(serde_json::json!({"result": result}));
                            engine_clone
                                .mark_step_completed_with_result(
                                    &execution_id_for_spawn,
                                    &node_id,
                                    result_value.clone(),
                                )
                                .await;
                            if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "completed", Utc::now(), Some(result_value.to_string()), None).await {
                                tracing::warn!("failed to update step status: {}", e);
                            }
                            wrote_result = true;
                        }
                        Err(err) => {
                            tracing::warn!("tool execute failed for {}: {}", tool_name, err);
                            if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "failed", Utc::now(), None, Some(&err.to_string())).await {
                                tracing::warn!("failed to update step status: {}", e);
                            }
                        }
                    }
                } else if action.starts_with("plugin::") {
                    // 处理Agent插件工具节点 - 通过 PluginManager.execute_agent 执行
                    let plugin_id = action.strip_prefix("plugin::").unwrap_or(&action);
                    tracing::info!("Executing Agent plugin '{}' with inputs: {:?}", plugin_id, step_def.inputs);
                    
                    if let Some(ref pm) = plugin_manager_clone {
                        // 确保插件已注册到内存中（从数据库加载）
                        if pm.get_plugin(plugin_id).await.is_none() {
                            tracing::info!("Plugin '{}' not in memory, loading from database...", plugin_id);
                            
                            // 从数据库加载插件元数据和代码
                            if let Ok(Some(plugin_data)) = db_clone.get_plugin_from_registry(plugin_id).await {
                                let enabled = matches!(plugin_data.status, sentinel_traffic::PluginStatus::Enabled);
                                let name = &plugin_data.metadata.name;
                                let version = &plugin_data.metadata.version;
                                let author = plugin_data.metadata.author.as_deref();
                                let main_category = &plugin_data.metadata.main_category;
                                let category = &plugin_data.metadata.category;
                                let description = plugin_data.metadata.description.as_deref();
                                let tags = plugin_data.metadata.tags.clone();
                                
                                // 从数据库获取代码
                                let code = db_clone.get_plugin_code(plugin_id).await.ok().flatten().unwrap_or_default();
                                
                                let metadata = sentinel_traffic::PluginMetadata {
                                    id: plugin_id.to_string(),
                                    name: name.to_string(),
                                    version: version.to_string(),
                                    author: author.map(|s| s.to_string()),
                                    main_category: main_category.to_string(),
                                    category: category.to_string(),
                                    description: description.map(|s| s.to_string()),
                                    default_severity: sentinel_traffic::types::Severity::Medium,
                                    tags,
                                };
                                
                                // 注册到内存并缓存代码
                                let _ = pm.register_plugin(plugin_id.to_string(), metadata, enabled).await;
                                let _ = pm.set_plugin_code(plugin_id.to_string(), code.to_string()).await;
                                tracing::info!("Plugin '{}' loaded from database and registered", plugin_id);
                            } else {
                                tracing::warn!("Plugin '{}' not found in database", plugin_id);
                            }
                        }
                        
                        // 构建输入参数
                        let input_value = serde_json::json!(step_def.inputs);
                        
                        match pm.execute_agent(plugin_id, &input_value).await {
                            Ok((findings, output)) => {
                                let result_value = serde_json::json!({
                                    "success": true,
                                    "findings": findings.len(),
                                    "output": output,
                                    "plugin_id": plugin_id
                                });
                                engine_clone
                                    .mark_step_completed_with_result(
                                        &execution_id_for_spawn,
                                        &node_id,
                                        result_value.clone(),
                                    )
                                    .await;
                                if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "completed", Utc::now(), Some(result_value.to_string()), None).await {
                                    tracing::warn!("failed to update step status: {}", e);
                                }
                                wrote_result = true;
                                tracing::info!("Agent plugin '{}' executed successfully, {} findings", plugin_id, findings.len());
                            }
                            Err(err) => {
                                tracing::warn!("Agent plugin '{}' execution failed: {}", plugin_id, err);
                                let error_result = serde_json::json!({
                                    "success": false,
                                    "error": err.to_string(),
                                    "plugin_id": plugin_id
                                });
                                engine_clone
                                    .mark_step_completed_with_result(
                                        &execution_id_for_spawn,
                                        &node_id,
                                        error_result.clone(),
                                    )
                                    .await;
                                if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "failed", Utc::now(), Some(error_result.to_string()), Some(&err.to_string())).await {
                                    tracing::warn!("failed to update step status: {}", e);
                                }
                                wrote_result = true;
                            }
                        }
                    } else {
                        tracing::warn!("PluginManager not available for executing plugin '{}'", plugin_id);
                        let error_result = serde_json::json!({
                            "success": false,
                            "error": "PluginManager not available",
                            "plugin_id": plugin_id
                        });
                        engine_clone
                            .mark_step_completed_with_result(
                                &execution_id_for_spawn,
                                &node_id,
                                error_result.clone(),
                            )
                            .await;
                        if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "failed", Utc::now(), Some(error_result.to_string()), Some("PluginManager not available")).await {
                            tracing::warn!("failed to update step status: {}", e);
                        }
                        wrote_result = true;
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
                    let tool_params = step_def.inputs.get("tool_params").cloned().unwrap_or_default();
                    if !tool_name.is_empty() {
                        let mut last_err: Option<String> = None;
                        for _attempt in 0..times {
                            let params_json = serde_json::to_string(&tool_params).unwrap_or_default();
                            match toolset_clone.call(tool_name, params_json).await {
                                Ok(result) => {
                                    let result_value = serde_json::from_str(&result).unwrap_or(serde_json::json!({"result": result}));
                                    engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, result_value.clone()).await;
                                    if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "completed", Utc::now(), Some(result_value.to_string()), None).await {
                                        tracing::warn!("failed to update step status: {}", e);
                                    }
                                    wrote_result = true;
                                    break;
                                }
                                Err(e) => { last_err = Some(e.to_string()); tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await; }
                            }
                        }
                        if !wrote_result {
                            let error_val = serde_json::json!({"error": last_err.clone().unwrap_or("unknown".to_string())});
                            engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, error_val.clone()).await;
                            if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "failed", Utc::now(), Some(error_val.to_string()), last_err.as_deref()).await {
                                tracing::warn!("failed to update step status: {}", e);
                            }
                            wrote_result = true;
                        }
                    }
                } else if action == "rag::ingest" {
                    let file_path = step_def.inputs.get("file_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let collection_id = step_def.inputs.get("collection_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let metadata_obj = step_def.inputs.get("metadata").and_then(|v| v.as_object());
                    let mut metadata: HashMap<String, String> = HashMap::new();
                    if let Some(obj) = metadata_obj { for (k, v) in obj.iter() { if let Some(s) = v.as_str() { metadata.insert(k.clone(), s.to_string()); } } }

                    let rag_config = match db_clone.get_rag_config().await { Ok(Some(core_cfg)) => convert_core_to_rag(core_cfg), _ => RagConfig::default() };
                    match RagService::new(rag_config, db_clone.clone()).await {
                        Ok(service) => {
                            let req = IngestRequest { file_path, collection_id, metadata: if metadata.is_empty() { None } else { Some(metadata) } };
                            match service.ingest_source(req).await {
                                Ok(resp) => {
                                    let result_json = serde_json::to_value(resp).unwrap_or(serde_json::json!({"status":"ok"}));
                                    engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, result_json.clone()).await;
                                    if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "completed", Utc::now(), Some(result_json.to_string()), None).await {
                                        tracing::warn!("failed to update step status: {}", e);
                                    }
                                    wrote_result = true;
                                }
                                Err(err) => {
                                    tracing::warn!("rag ingest failed: {}", err);
                                    let err_msg = err.to_string();
                                    let error_val = serde_json::json!({"error": err_msg});
                                    engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, error_val.clone()).await;
                                    if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "failed", Utc::now(), Some(error_val.to_string()), Some(&err_msg)).await {
                                        tracing::warn!("failed to update step status: {}", e);
                                    }
                                    wrote_result = true;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("init rag service failed: {}", e);
                            let error_val = serde_json::json!({"error": e.to_string()});
                            engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, error_val.clone()).await;
                            if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "failed", Utc::now(), Some(error_val.to_string()), Some(&e.to_string())).await {
                                tracing::warn!("failed to update step status: {}", e);
                            }
                            wrote_result = true;
                        }
                    }
                } else if action == "rag::query" {
                    let query = step_def.inputs.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let collection_id = step_def.inputs.get("collection_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let top_k = step_def.inputs.get("top_k").and_then(|v| v.as_u64()).map(|n| n as usize);
                    let use_mmr = step_def.inputs.get("use_mmr").and_then(|v| v.as_bool());
                    let mmr_lambda = step_def.inputs.get("mmr_lambda").and_then(|v| v.as_f64()).map(|n| n as f32);
                    let filters_obj = step_def.inputs.get("filters").and_then(|v| v.as_object());
                    let mut filters: HashMap<String, String> = HashMap::new();
                    if let Some(obj) = filters_obj { for (k, v) in obj.iter() { if let Some(s) = v.as_str() { filters.insert(k.clone(), s.to_string()); } } }

                    let rag_config = match db_clone.get_rag_config().await { Ok(Some(core_cfg)) => convert_core_to_rag(core_cfg), _ => RagConfig::default() };
                    match RagService::new(rag_config, db_clone.clone()).await {
                        Ok(service) => {
                            let req = RagQueryRequest { query, collection_id, top_k, use_mmr, mmr_lambda, filters: if filters.is_empty() { None } else { Some(filters) }, use_embedding: Some(true), reranking_enabled: Some(true), similarity_threshold: None };
                            match service.query(req).await {
                                Ok(resp) => {
                                    let result_json = serde_json::to_value(resp).unwrap_or(serde_json::json!({"status":"ok"}));
                                    engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, result_json.clone()).await;
                                    if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "completed", Utc::now(), Some(result_json.to_string()), None).await {
                                        tracing::warn!("failed to update step status: {}", e);
                                    }
                                    wrote_result = true;
                                }
                                Err(e) => {
                                    tracing::warn!("rag query failed: {}", e);
                                    let error_val = serde_json::json!({"error": e.to_string()});
                                    engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, error_val.clone()).await;
                                    if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "failed", Utc::now(), Some(error_val.to_string()), Some(&e.to_string())).await {
                                        tracing::warn!("failed to update step status: {}", e);
                                    }
                                    wrote_result = true;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("init rag service failed: {}", e);
                            let error_val = serde_json::json!({"error": e.to_string()});
                            engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, error_val.clone()).await;
                            if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "failed", Utc::now(), Some(error_val.to_string()), Some(&e.to_string())).await {
                                tracing::warn!("failed to update step status: {}", e);
                            }
                            wrote_result = true;
                        }
                    }
                } else if action == "prompt::build" {
                    let build_type = step_def.inputs.get("build_type").and_then(|v| v.as_str()).unwrap_or("Planner");
                    let user_query = step_def.inputs.get("user_query").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let target_info = step_def.inputs.get("target_info").cloned().and_then(|v| serde_json::from_value::<TargetInfo>(v).ok());
                    let execution_context = step_def.inputs.get("execution_context").cloned().and_then(|v| serde_json::from_value::<ExecutionContext>(v).ok());
                    let history = step_def.inputs.get("history").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter().filter_map(|item| serde_json::from_value::<HistoryItem>(item.clone()).ok()).collect::<Vec<HistoryItem>>()
                    }).unwrap_or_default();
                    let custom_vars_obj = step_def.inputs.get("custom_variables").and_then(|v| v.as_object());
                    let mut custom_variables: HashMap<String, serde_json::Value> = HashMap::new();
                    if let Some(obj) = custom_vars_obj { for (k, v) in obj.iter() { custom_variables.insert(k.clone(), v.clone()); } }

                    let cfg_mgr = PromptConfigManager::new();
                    let builder = PromptBuilder::new(cfg_mgr);
                    let ctx = PromptBuildContext { user_query, target_info, available_tools: vec![], execution_context, history, custom_variables, rag_context: None };
                    let build_res = match build_type { "Executor" => builder.build_executor_prompt(&ctx, step_def.inputs.get("step_instructions").and_then(|v| v.as_str()).unwrap_or("")).await, "Replanner" => builder.build_replanner_prompt(&ctx, step_def.inputs.get("execution_results").and_then(|v| v.as_str()).unwrap_or("") , step_def.inputs.get("original_plan").and_then(|v| v.as_str()).unwrap_or("")).await, "ReportGenerator" => builder.build_report_prompt(&ctx, step_def.inputs.get("execution_summary").and_then(|v| v.as_str()).unwrap_or("") , step_def.inputs.get("target_audience").and_then(|v| v.as_str()).unwrap_or("")).await, _ => builder.build_planner_prompt(&ctx).await };
                    match build_res {
                        Ok(res) => {
                            let result_json = serde_json::to_value(res).unwrap_or(serde_json::json!({"status":"ok"}));
                            engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, result_json).await;
                            wrote_result = true;
                        }
                        Err(e) => { tracing::warn!("prompt build failed: {}", e); }
                    }
                } else if action == "notify" {
                    // 通知节点处理
                    let notification_rule_id = step_def.inputs.get("notification_rule_id").and_then(|v| v.as_str()).unwrap_or("");
                    let use_input_as_content = step_def.inputs.get("use_input_as_content").and_then(|v| v.as_bool()).unwrap_or(false);
                    
                    // 获取通知内容
                    let title = step_def.inputs.get("title").and_then(|v| v.as_str()).unwrap_or("Workflow Notification").to_string();
                    let mut content = step_def.inputs.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    
                    // 如果启用了使用输入作为内容，从上游节点获取数据
                    if use_input_as_content {
                        let input_edges = graph.edges.iter().filter(|e| e.to_node == node_id).collect::<Vec<_>>();
                        if let Some(edge) = input_edges.first() {
                            if let Some(upstream_result) = engine_clone.get_step_result(&execution_id_for_spawn, &edge.from_node).await {
                                content = serde_json::to_string_pretty(&upstream_result).unwrap_or(content);
                            }
                        }
                    }
                    
                    // 发送通知
                    if !notification_rule_id.is_empty() {
                        // 从inputs中获取通知配置（前端保存workflow时已经附加）
                        let channel = step_def.inputs.get("_notification_channel")
                            .and_then(|v| v.as_str())
                            .unwrap_or("webhook")
                            .to_string();
                        let config = step_def.inputs.get("_notification_config")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!({}));
                        
                        tracing::info!(
                            "Sending notification: rule_id={}, channel={}, title={}", 
                            notification_rule_id, channel, title
                        );
                        
                        // 使用 sentinel-notify 发送通知
                        match sentinel_notify::send(
                            &channel,
                            config.clone(),
                            sentinel_notify::NotificationMessage {
                                title: title.clone(),
                                content: content.clone(),
                            }
                        ).await {
                            Ok(_) => {
                                tracing::info!("Notification sent successfully for node: {}", node_id);
                                let result_json = serde_json::json!({
                                    "status": "sent",
                                    "title": title,
                                    "content": content,
                                    "channel": channel,
                                    "notification_rule_id": notification_rule_id
                                });
                                engine_clone.mark_step_completed_with_result(
                                    &execution_id_for_spawn,
                                    &node_id,
                                    result_json.clone()
                                ).await;
                                if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "completed", Utc::now(), Some(result_json.to_string()), None).await {
                                    tracing::warn!("failed to update step status: {}", e);
                                }
                                wrote_result = true;
                            }
                            Err(e) => {
                                tracing::warn!("Failed to send notification for node {}: {}", node_id, e);
                                let error_json = serde_json::json!({
                                    "status": "failed",
                                    "error": e.to_string(),
                                    "title": title,
                                    "content": content,
                                    "channel": channel,
                                    "notification_rule_id": notification_rule_id
                                });
                                engine_clone.mark_step_completed_with_result(
                                    &execution_id_for_spawn,
                                    &node_id,
                                    error_json.clone()
                                ).await;
                                if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "failed", Utc::now(), Some(error_json.to_string()), Some(&e.to_string())).await {
                                    tracing::warn!("failed to update step status: {}", e);
                                }
                                wrote_result = true;
                            }
                        }
                    } else {
                        tracing::warn!("No notification_rule_id provided for notify node: {}", node_id);
                        let result_json = serde_json::json!({
                            "status": "skipped",
                            "reason": "no_rule_id"
                        });
                        engine_clone.mark_step_completed_with_result(
                            &execution_id_for_spawn,
                            &node_id,
                            result_json.clone()
                        ).await;
                        if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "completed", Utc::now(), Some(result_json.to_string()), Some("no_rule_id")).await {
                            tracing::warn!("failed to update step status: {}", e);
                        }
                        wrote_result = true;
                    }
                } else if action == "ai_chat" || action == "ai_agent" {
                    // AI Chat / AI Agent 节点执行
                    tracing::info!("Executing AI node '{}' with action '{}', inputs: {:?}", node_id, action, step_def.inputs);
                    
                    // 获取上游输入
                    let upstream_input = {
                        let mut input_val = serde_json::Value::Null;
                        for edge in &graph.edges {
                            if edge.to_node == node_id && edge.to_port == "in" {
                                if let Some(val) = engine_clone.get_step_result(&execution_id_for_spawn, &edge.from_node).await {
                                    input_val = val;
                                    tracing::info!("AI node '{}' got upstream input from '{}'", node_id, edge.from_node);
                                    break;
                                }
                            }
                        }
                        input_val
                    };
                    
                    // 获取参数
                    let prompt_template = step_def.inputs.get("prompt")
                        .or_else(|| step_def.inputs.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    // 替换模板变量 {{input}}
                    let prompt = if prompt_template.contains("{{input}}") {
                        let input_str = match &upstream_input {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Null => String::new(),
                            other => serde_json::to_string_pretty(other).unwrap_or_default(),
                        };
                        prompt_template.replace("{{input}}", &input_str)
                    } else if prompt_template.is_empty() && !upstream_input.is_null() {
                        // 如果没有prompt但有上游输入，使用上游输入作为prompt
                        match &upstream_input {
                            serde_json::Value::String(s) => s.clone(),
                            other => serde_json::to_string_pretty(other).unwrap_or_default(),
                        }
                    } else {
                        prompt_template
                    };
                    
                    tracing::info!("AI node '{}' prompt: '{}'", node_id, if prompt.len() > 100 { &prompt[..100] } else { &prompt });
                    
                    // 获取其他参数
                    let system_prompt = step_def.inputs.get("system_prompt").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let provider = step_def.inputs.get("provider").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                    let model = step_def.inputs.get("model").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                    
                    // 调用 LLM 服务
                    let llm_config = sentinel_llm::LlmConfig::new(
                        provider.clone().unwrap_or_else(|| "openai".to_string()),
                        model.clone().unwrap_or_else(|| "gpt-4".to_string()),
                    ).with_timeout(120);
                    let llm_client = sentinel_llm::LlmClient::new(llm_config);
                    
                    let result = match llm_client.completion(system_prompt.as_deref(), &prompt).await {
                        Ok(response) => {
                            tracing::info!("AI node '{}' got response: {} chars", node_id, response.len());
                            serde_json::json!({
                                "success": true,
                                "response": response,
                                "prompt": prompt,
                                "provider": provider,
                                "model": model,
                            })
                        }
                        Err(e) => {
                            tracing::error!("AI node '{}' failed: {}", node_id, e);
                            serde_json::json!({
                                "success": false,
                                "error": e.to_string(),
                                "prompt": prompt,
                                "provider": provider,
                                "model": model,
                            })
                        }
                    };
                    
                    engine_clone.mark_step_completed_with_result(&execution_id_for_spawn, &node_id, result.clone()).await;
                    let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                    let error_msg = result.get("error").and_then(|v| v.as_str());
                    if let Err(e) = db_clone.update_workflow_run_step_status(
                        &execution_id_for_spawn, 
                        &node_id, 
                        if success { "completed" } else { "failed" }, 
                        Utc::now(), 
                        Some(result.to_string()),
                        error_msg
                    ).await {
                        tracing::warn!("failed to update step status: {}", e);
                    }
                    wrote_result = true;
                    tracing::info!("AI node '{}' completed", node_id);
                } else {
                    // 未知节点类型
                    tracing::warn!("Unknown action type '{}' for node '{}', marking as completed", action, node_id);
                }
            } else {
                tracing::warn!("Step definition not found for node '{}'", node_id);
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }

            if !wrote_result {
                engine_clone.mark_step_completed(&execution_id_for_spawn, &node_id).await;
                if let Err(e) = db_clone.update_workflow_run_step_status(&execution_id_for_spawn, &node_id, "completed", Utc::now(), None, None).await {
                    tracing::warn!("failed to update step status: {}", e);
                }
            }
            
            // 获取步骤结果并发送事件
            let step_result = engine_clone.get_step_result(&execution_id_for_spawn, &node_id).await;
            let _ = app_handle_clone.emit("workflow:step-complete", &serde_json::json!({
                "execution_id": execution_id_for_spawn,
                "step_id": node_id,
                "result": step_result
            }));

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
}

#[tauri::command]
pub async fn start_workflow_run(
    graph: WorkflowGraph,
    app_handle: AppHandle,
    engine: State<'_, Arc<WorkflowEngine>>,
    db: State<'_, Arc<DatabaseService>>,
    plugin_manager: State<'_, Arc<PluginManager>>,
) -> Result<String, String> {
    // Multi-point license verification
    #[cfg(not(debug_assertions))]
    if !sentinel_license::is_licensed() {
        return Err("License required for this feature".to_string());
    }

    let def = graph_to_definition(&graph);
    let execution_id = engine.execute_workflow(&def, None).await.map_err(|e| e.to_string())?;

    let _ = app_handle.emit("workflow:run-start", &serde_json::json!({
        "execution_id": execution_id,
        "workflow_id": def.metadata.id,
        "workflow_name": def.metadata.name,
        "version": def.metadata.version,
        "status": "running"
    }));

    let execution_id_for_spawn = execution_id.clone();
    let db_clone = db.inner().clone();
    let app_handle_clone = app_handle.clone();
    let engine_clone = engine.inner().clone();
    let def_clone = def.clone();
    let graph_clone = graph.clone();
    let plugin_manager_clone = Some(plugin_manager.inner().clone());
    
    // 使用 rig-core ToolSet
    let toolset = Arc::new(sentinel_tools::create_buildin_toolset());

    tokio::spawn(async move {
        execute_workflow_steps(
            execution_id_for_spawn,
            graph_clone,
            def_clone,
            db_clone,
            app_handle_clone,
            engine_clone,
            toolset,
            plugin_manager_clone,
        ).await;
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

/// 停止工作流执行
#[tauri::command]
pub async fn stop_workflow_run(
    execution_id: String,
    app_handle: AppHandle,
    engine: State<'_, Arc<WorkflowEngine>>,
) -> Result<(), String> {
    tracing::info!("Stopping workflow run: {}", execution_id);
    
    engine.cancel_execution(&execution_id).await.map_err(|e| e.to_string())?;
    
    // 发送停止事件
    let _ = app_handle.emit("workflow:run-stop", &serde_json::json!({
        "execution_id": execution_id,
        "status": "cancelled"
    }));
    
    Ok(())
}

#[tauri::command]
pub async fn list_workflow_runs(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<serde_json::Value>, String> {
    db.list_workflow_runs().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_workflow_runs_paginated(
    page: i64,
    page_size: i64,
    search: Option<String>,
    workflow_id: Option<String>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<serde_json::Value, String> {
    let (runs, total) = db.list_workflow_runs_paginated(page, page_size, search.as_deref(), workflow_id.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "data": runs,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))
}

#[tauri::command]
pub async fn get_workflow_run_detail(
    run_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Option<serde_json::Value>, String> {
    db.get_workflow_run_detail(&run_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_workflow_run(
    run_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db.delete_workflow_run(&run_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_workflow_definition(
    graph: WorkflowGraph,
    description: Option<String>,
    tags: Option<String>,
    is_template: bool,
    is_tool: Option<bool>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let graph_json = serde_json::to_string(&graph).map_err(|e| e.to_string())?;
    db.save_workflow_definition(
        &graph.id,
        &graph.name,
        description.as_deref(),
        &graph_json,
        is_template,
        is_tool.unwrap_or(false),
        None,
        tags.as_deref(),
        &graph.version,
        "system",
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(graph.id)
}

#[tauri::command]
pub async fn get_workflow_definition(
    id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Option<serde_json::Value>, String> {
    db.get_workflow_definition(&id).await.map_err(|e: anyhow::Error| e.to_string())
}

#[tauri::command]
pub async fn list_workflow_definitions(
    is_template: Option<bool>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<serde_json::Value>, String> {
    db.list_workflow_definitions(is_template.unwrap_or(false)).await.map_err(|e: anyhow::Error| e.to_string())
}

/// 列出所有标记为工具的工作流
#[tauri::command]
pub async fn list_workflow_tools(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<serde_json::Value>, String> {
    db.list_workflow_tools().await.map_err(|e: anyhow::Error| e.to_string())
}

#[tauri::command]
pub async fn delete_workflow_definition(
    id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db.delete_workflow_definition(&id).await.map_err(|e: anyhow::Error| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowValidationIssue {
    pub code: String,
    pub message: String,
    pub node_id: Option<String>,
    pub edge_id: Option<String>,
}

#[tauri::command]
pub async fn validate_workflow_graph(graph: WorkflowGraph) -> Result<Vec<WorkflowValidationIssue>, String> {
    let mut issues = Vec::new();

    // 检查是否为空工作流
    if graph.nodes.is_empty() {
        issues.push(WorkflowValidationIssue {
            code: "empty_workflow".to_string(),
            message: "Workflow has no nodes".to_string(),
            node_id: None,
            edge_id: None,
        });
        return Ok(issues);
    }

    // 检查节点ID唯一性
    let mut node_ids = std::collections::HashSet::new();
    for node in &graph.nodes {
        if !node_ids.insert(&node.id) {
            issues.push(WorkflowValidationIssue {
                code: "duplicate_node_id".to_string(),
                message: format!("Duplicate node ID: {}", node.id),
                node_id: Some(node.id.clone()),
                edge_id: None,
            });
        }
    }

    // 构建端口映射
    let mut node_port_map: HashMap<String, (std::collections::HashSet<String>, std::collections::HashSet<String>)> = HashMap::new();
    for node in &graph.nodes {
        let inputs: std::collections::HashSet<String> = node.input_ports.iter().map(|p| p.id.clone()).collect();
        let outputs: std::collections::HashSet<String> = node.output_ports.iter().map(|p| p.id.clone()).collect();
        node_port_map.insert(node.id.clone(), (inputs, outputs));
    }

    // 检查边的有效性
    for edge in &graph.edges {
        // 检查from_node存在
        if !node_ids.contains(&edge.from_node) {
            issues.push(WorkflowValidationIssue {
                code: "edge_from_missing".to_string(),
                message: format!("from_node not found: {}", edge.from_node),
                edge_id: Some(edge.id.clone()),
                node_id: None,
            });
        } else if let Some((_, outputs)) = node_port_map.get(&edge.from_node) {
            if !outputs.contains(&edge.from_port) {
                issues.push(WorkflowValidationIssue {
                    code: "edge_from_port_missing".to_string(),
                    message: format!("from_port not found: {}", edge.from_port),
                    edge_id: Some(edge.id.clone()),
                    node_id: Some(edge.from_node.clone()),
                });
            }
        }

        // 检查to_node存在
        if !node_ids.contains(&edge.to_node) {
            issues.push(WorkflowValidationIssue {
                code: "edge_to_missing".to_string(),
                message: format!("to_node not found: {}", edge.to_node),
                edge_id: Some(edge.id.clone()),
                node_id: None,
            });
        } else if let Some((inputs, _)) = node_port_map.get(&edge.to_node) {
            if !inputs.contains(&edge.to_port) {
                issues.push(WorkflowValidationIssue {
                    code: "edge_to_port_missing".to_string(),
                    message: format!("to_port not found: {}", edge.to_port),
                    edge_id: Some(edge.id.clone()),
                    node_id: Some(edge.to_node.clone()),
                });
            }
        }
    }

    // 检查循环依赖
    let mut indegree: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for node in &graph.nodes {
        indegree.insert(node.id.clone(), 0);
        adj.insert(node.id.clone(), Vec::new());
    }
    for edge in &graph.edges {
        if node_ids.contains(&edge.from_node) && node_ids.contains(&edge.to_node) {
            *indegree.entry(edge.to_node.clone()).or_insert(0) += 1;
            adj.entry(edge.from_node.clone()).or_default().push(edge.to_node.clone());
        }
    }

    let mut queue: VecDeque<String> = indegree.iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited = 0;

    while let Some(u) = queue.pop_front() {
        visited += 1;
        if let Some(neighbors) = adj.get(&u) {
            for v in neighbors {
                if let Some(deg) = indegree.get_mut(v) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(v.clone());
                    }
                }
            }
        }
    }

    if visited != graph.nodes.len() {
        issues.push(WorkflowValidationIssue {
            code: "cycle_detected".to_string(),
            message: "Workflow contains circular dependencies".to_string(),
            node_id: None,
            edge_id: None,
        });
    }

    // 检查必需输入端口
    for node in &graph.nodes {
        let required_inputs: Vec<&PortDef> = node.input_ports.iter().filter(|p| p.required).collect();
        let connected_ports: std::collections::HashSet<String> = graph.edges.iter()
            .filter(|e| e.to_node == node.id)
            .map(|e| e.to_port.clone())
            .collect();

        for port in required_inputs {
            if !connected_ports.contains(&port.id) {
                issues.push(WorkflowValidationIssue {
                    code: "missing_required_input".to_string(),
                    message: format!("Required input port '{}' is not connected", port.name),
                    node_id: Some(node.id.clone()),
                    edge_id: None,
                });
            }
        }
    }

    // 检查孤立节点（没有输入也没有输出的节点）
    for node in &graph.nodes {
        let has_input = graph.edges.iter().any(|e| e.to_node == node.id);
        let has_output = graph.edges.iter().any(|e| e.from_node == node.id);
        
        if !has_input && !has_output && graph.nodes.len() > 1 {
            issues.push(WorkflowValidationIssue {
                code: "isolated_node".to_string(),
                message: format!("Node '{}' is isolated (no connections)", node.node_name),
                node_id: Some(node.id.clone()),
                edge_id: None,
            });
        }
    }

    Ok(issues)
}

// ==================== 定时调度相关 ====================

use crate::scheduler::{WorkflowScheduler, ScheduleConfig, ScheduleInfo, ScheduleExecutor};

/// 调度执行器实现
pub struct WorkflowScheduleExecutor {
    engine: Arc<WorkflowEngine>,
    db: Arc<DatabaseService>,
    app_handle: AppHandle,
    plugin_manager: Option<Arc<PluginManager>>,
}

impl WorkflowScheduleExecutor {
    pub fn new(
        engine: Arc<WorkflowEngine>,
        db: Arc<DatabaseService>,
        app_handle: AppHandle,
    ) -> Self {
        Self { engine, db, app_handle, plugin_manager: None }
    }
    
    pub fn with_plugin_manager(mut self, pm: Arc<PluginManager>) -> Self {
        self.plugin_manager = Some(pm);
        self
    }
}

#[async_trait::async_trait]
impl ScheduleExecutor for WorkflowScheduleExecutor {
    async fn execute_workflow(&self, workflow_id: &str) -> Result<String, String> {
        // 从数据库加载工作流定义
        let wf_data = self.db.get_workflow_definition(workflow_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Workflow not found: {}", workflow_id))?;
        
        // graph 在 "graph" 字段中
        let graph_value = wf_data.get("graph")
            .ok_or_else(|| "Workflow graph not found".to_string())?;
        
        let graph: WorkflowGraph = serde_json::from_value(graph_value.clone())
            .map_err(|e| format!("Failed to parse workflow graph: {}", e))?;
        
        let def = graph_to_definition(&graph);
        let execution_id = self.engine.execute_workflow(&def, None)
            .await
            .map_err(|e| e.to_string())?;
        
        // 发送事件通知前端
        let _ = self.app_handle.emit("workflow:schedule-triggered", &serde_json::json!({
            "workflow_id": workflow_id,
            "execution_id": execution_id,
        }));
        
        // 使用与 start_workflow_run 相同的执行逻辑
        let execution_id_for_spawn = execution_id.clone();
        let db_clone = self.db.clone();
        let app_handle_clone = self.app_handle.clone();
        let engine_clone = self.engine.clone();
        let def_clone = def.clone();
        let toolset = Arc::new(sentinel_tools::create_buildin_toolset());
        let plugin_manager_clone = self.plugin_manager.clone();
        
        tokio::spawn(async move {
            execute_workflow_steps(
                execution_id_for_spawn,
                graph,
                def_clone,
                db_clone,
                app_handle_clone,
                engine_clone,
                toolset,
                plugin_manager_clone,
            ).await;
        });
        
        Ok(execution_id)
    }
}

/// 启动工作流定时调度
#[tauri::command]
pub async fn start_workflow_schedule(
    workflow_id: String,
    workflow_name: String,
    config: ScheduleConfig,
    scheduler: State<'_, Arc<WorkflowScheduler>>,
) -> Result<(), String> {
    tracing::info!(
        "[Scheduler] Starting schedule for workflow: {} ({}) - type: {}, interval: {:?}s",
        workflow_name, workflow_id, config.trigger_type, config.interval_seconds
    );
    
    let result = scheduler.start_schedule(workflow_id.clone(), workflow_name.clone(), config).await;
    
    match &result {
        Ok(_) => tracing::info!("[Scheduler] Schedule started successfully for: {}", workflow_name),
        Err(e) => tracing::error!("[Scheduler] Failed to start schedule for {}: {}", workflow_name, e),
    }
    
    result
}

/// 停止工作流定时调度
#[tauri::command]
pub async fn stop_workflow_schedule(
    workflow_id: String,
    scheduler: State<'_, Arc<WorkflowScheduler>>,
) -> Result<(), String> {
    tracing::info!("Stopping schedule for workflow: {}", workflow_id);
    scheduler.stop_schedule(&workflow_id).await
}

/// 列出所有定时调度任务
#[tauri::command]
pub async fn list_workflow_schedules(
    scheduler: State<'_, Arc<WorkflowScheduler>>,
) -> Result<Vec<ScheduleInfo>, String> {
    Ok(scheduler.list_schedules().await)
}

/// 获取单个调度任务信息
#[tauri::command]
pub async fn get_workflow_schedule(
    workflow_id: String,
    scheduler: State<'_, Arc<WorkflowScheduler>>,
) -> Result<Option<ScheduleInfo>, String> {
    Ok(scheduler.get_schedule(&workflow_id).await)
}


