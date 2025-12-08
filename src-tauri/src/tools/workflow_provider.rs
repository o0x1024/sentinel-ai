//! 工作流工具提供者
//!
//! 为Agent提供被标记为工具的工作流：
//! - workflow::<workflow_id>: 每个标记为工具的工作流对应一个工具
//! 
//! 输入输出约定：
//! - 第一个节点（Start）的 input_ports 定义工具的输入参数
//! - 最后一个节点（End）的输出作为工具的返回值

use sentinel_db::DatabaseService;
use sentinel_tools::unified_types::*;
use sentinel_tools::UnifiedToolManager;
use sentinel_workflow::{WorkflowEngine, WorkflowGraph, graph_to_definition, execute_workflow_steps};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use uuid::Uuid;

/// 工作流工具提供者
#[derive(Debug)]
pub struct WorkflowToolProvider {
    db: Arc<DatabaseService>,
    tool_manager: Arc<RwLock<UnifiedToolManager>>,
    app_handle: Option<tauri::AppHandle>,
}

impl WorkflowToolProvider {
    pub fn new(
        db: Arc<DatabaseService>,
        tool_manager: Arc<RwLock<UnifiedToolManager>>,
    ) -> Self {
        Self {
            db,
            tool_manager,
            app_handle: None,
        }
    }

    pub fn with_app_handle(mut self, handle: tauri::AppHandle) -> Self {
        self.app_handle = Some(handle);
        self
    }
}

#[async_trait::async_trait]
impl ToolProvider for WorkflowToolProvider {
    fn name(&self) -> &str {
        "workflow"
    }

    fn description(&self) -> &str {
        "Workflow tools that can be executed as AI tools"
    }

    async fn get_tools(&self) -> anyhow::Result<Vec<Arc<dyn UnifiedTool>>> {
        let mut tools: Vec<Arc<dyn UnifiedTool>> = Vec::new();

        // 从数据库获取标记为工具的工作流
        let workflows = self.db.list_workflow_tools().await.map_err(|e| {
            anyhow::anyhow!("Failed to list workflow tools: {}", e)
        })?;

        tracing::info!(
            "WorkflowToolProvider: found {} workflow tools",
            workflows.len()
        );

        for wf in workflows {
            let id = wf.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let name = wf.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            let description = wf.get("description").and_then(|v| v.as_str()).unwrap_or("Workflow tool");
            let version = wf.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0");

            // 加载完整的工作流定义以获取节点信息
            let graph_data = self.db.get_workflow_definition(id).await.ok().flatten();
            let (input_params, output_desc) = if let Some(data) = graph_data {
                extract_io_from_graph(&data)
            } else {
                (vec![], "Workflow output".to_string())
            };

            tracing::info!(
                "WorkflowToolProvider: register workflow tool => id={}, name='{}', input_params={}",
                id, name, input_params.len()
            );

            tools.push(Arc::new(WorkflowTool::new(
                self.db.clone(),
                self.tool_manager.clone(),
                self.app_handle.clone(),
                id.to_string(),
                name.to_string(),
                description.to_string(),
                version.to_string(),
                input_params,
                output_desc,
            )));
        }

        tracing::info!("WorkflowToolProvider: registered {} workflow tools", tools.len());
        Ok(tools)
    }

    async fn get_tool(&self, name: &str) -> anyhow::Result<Option<Arc<dyn UnifiedTool>>> {
        // 去除 workflow:: 前缀（如果有）
        let workflow_id = name.strip_prefix("workflow::").unwrap_or(name);

        // 从数据库获取工作流定义
        let wf_data = self.db.get_workflow_definition(workflow_id).await.map_err(|e| {
            anyhow::anyhow!("Failed to get workflow definition: {}", e)
        })?;

        if let Some(wf) = wf_data {
            // 检查是否标记为工具
            let is_tool = wf.get("is_tool").and_then(|v| v.as_bool()).unwrap_or(false);
            if !is_tool {
                tracing::warn!(
                    "WorkflowToolProvider::get_tool workflow '{}' is not marked as tool",
                    workflow_id
                );
                return Ok(None);
            }

            let name = wf.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            let description = wf.get("description").and_then(|v| v.as_str()).unwrap_or("Workflow tool");
            let version = wf.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0");

            // 提取输入输出定义
            let (input_params, output_desc) = extract_io_from_graph(&wf);

            tracing::info!(
                "WorkflowToolProvider::get_tool found workflow '{}' ({}) with {} input params",
                name, workflow_id, input_params.len()
            );

            return Ok(Some(Arc::new(WorkflowTool::new(
                self.db.clone(),
                self.tool_manager.clone(),
                self.app_handle.clone(),
                workflow_id.to_string(),
                name.to_string(),
                description.to_string(),
                version.to_string(),
                input_params,
                output_desc,
            ))));
        }

        tracing::warn!(
            "WorkflowToolProvider::get_tool not found for id='{}'",
            workflow_id
        );
        Ok(None)
    }

    async fn refresh(&self) -> anyhow::Result<()> {
        tracing::info!("Refreshing workflow tools");
        Ok(())
    }
}

/// 从工作流图中提取输入输出定义
/// - 输入：入口节点（Start 节点或第一个节点）的 params
/// - 输出：End 节点的 output_ports 描述
fn extract_io_from_graph(wf_data: &Value) -> (Vec<ParameterDefinition>, String) {
    let mut input_params = Vec::new();
    let mut output_desc = "Workflow output".to_string();

    if let Some(graph) = wf_data.get("graph") {
        if let Some(nodes) = graph.get("nodes").and_then(|v| v.as_array()) {
            if nodes.is_empty() {
                return (input_params, output_desc);
            }

            // 查找入口节点：优先 start 节点，否则找第一个没有入边的节点
            let mut entry_node = nodes.iter().find(|n| {
                n.get("node_type").and_then(|t| t.as_str()) == Some("start")
            });

            if entry_node.is_none() {
                // 获取所有目标节点（有入边的节点）
                let edges = graph.get("edges").and_then(|v| v.as_array());
                let target_nodes: std::collections::HashSet<&str> = edges
                    .map(|e| e.iter()
                        .filter_map(|edge| edge.get("to_node").and_then(|v| v.as_str()))
                        .collect())
                    .unwrap_or_default();
                
                // 找第一个没有入边的节点
                entry_node = nodes.iter().find(|n| {
                    let node_id = n.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    !target_nodes.contains(node_id)
                });

                // 如果所有节点都有入边，使用第一个节点
                if entry_node.is_none() {
                    entry_node = nodes.first();
                }
            }

            if let Some(entry) = entry_node {
                // 优先从节点的 params 提取参数（包含实际配置值）
                if let Some(params) = entry.get("params").and_then(|v| v.as_object()) {
                    for (key, value) in params {
                        input_params.push(ParameterDefinition {
                            name: key.clone(),
                            param_type: value_to_param_type(value),
                            description: format!("Parameter: {}", key),
                            required: false,
                            default_value: Some(value.clone()),
                        });
                    }
                }

                // 如果 params 为空，尝试从 input_ports 提取
                if input_params.is_empty() {
                    if let Some(ports) = entry.get("input_ports").and_then(|v| v.as_array()) {
                        for port in ports {
                            let name = port.get("name").and_then(|v| v.as_str()).unwrap_or("input");
                            // 跳过通用的输入端口
                            if name == "输入" || name == "inputs" {
                                continue;
                            }
                            let port_type = port.get("port_type").and_then(|v| v.as_str()).unwrap_or("String");
                            let required = port.get("required").and_then(|v| v.as_bool()).unwrap_or(false);

                            input_params.push(ParameterDefinition {
                                name: name.to_string(),
                                param_type: port_type_to_param_type(port_type),
                                description: format!("Input: {}", name),
                                required,
                                default_value: None,
                            });
                        }
                    }
                }
            }

            // 找到 End 节点获取输出描述
            let end_node = nodes.iter().find(|n| {
                n.get("node_type").and_then(|t| t.as_str()) == Some("end")
            });

            // 如果没有 end 节点，使用最后一个节点
            let output_node = end_node.or_else(|| nodes.last());

            if let Some(out) = output_node {
                if let Some(ports) = out.get("output_ports").and_then(|v| v.as_array()) {
                    let port_names: Vec<&str> = ports.iter()
                        .filter_map(|p| p.get("name").and_then(|v| v.as_str()))
                        .collect();
                    if !port_names.is_empty() {
                        output_desc = format!("Output: {}", port_names.join(", "));
                    }
                }
            }
        }
    }

    // 如果没有提取到输入参数，添加默认的 input 参数
    if input_params.is_empty() {
        input_params.push(ParameterDefinition {
            name: "input".to_string(),
            param_type: ParameterType::String,
            description: "Input data for the workflow".to_string(),
            required: false,
            default_value: None,
        });
    }

    (input_params, output_desc)
}

/// 将端口类型转换为参数类型
fn port_type_to_param_type(port_type: &str) -> ParameterType {
    match port_type.to_lowercase().as_str() {
        "string" => ParameterType::String,
        "integer" | "int" | "number" => ParameterType::Number,
        "float" | "double" => ParameterType::Number,
        "boolean" | "bool" => ParameterType::Boolean,
        "array" => ParameterType::Array,
        "object" | "json" => ParameterType::Object,
        _ => ParameterType::String,
    }
}

/// 从值推断参数类型
fn value_to_param_type(value: &Value) -> ParameterType {
    match value {
        Value::String(_) => ParameterType::String,
        Value::Number(_) => ParameterType::Number,
        Value::Bool(_) => ParameterType::Boolean,
        Value::Array(_) => ParameterType::Array,
        Value::Object(_) => ParameterType::Object,
        Value::Null => ParameterType::String,
    }
}

// ============================================================================
// 工作流工具实现
// ============================================================================

/// 工作流工具
#[derive(Debug)]
struct WorkflowTool {
    db: Arc<DatabaseService>,
    tool_manager: Arc<RwLock<UnifiedToolManager>>,
    app_handle: Option<tauri::AppHandle>,
    workflow_id: String,
    workflow_name: String,
    workflow_description: String,
    full_tool_name: String,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl WorkflowTool {
    fn new(
        db: Arc<DatabaseService>,
        tool_manager: Arc<RwLock<UnifiedToolManager>>,
        app_handle: Option<tauri::AppHandle>,
        workflow_id: String,
        workflow_name: String,
        workflow_description: String,
        version: String,
        input_params: Vec<ParameterDefinition>,
        output_desc: String,
    ) -> Self {
        let full_tool_name = format!("workflow::{}", workflow_id);

        // 构建参数 schema
        let mut properties = serde_json::Map::new();
        let mut required_params = Vec::new();
        let mut optional_params = Vec::new();

        for param in &input_params {
            let type_str = match param.param_type {
                ParameterType::String => "string",
                ParameterType::Number => "number",
                ParameterType::Boolean => "boolean",
                ParameterType::Array => "array",
                ParameterType::Object => "object",
            };
            properties.insert(param.name.clone(), json!({
                "type": type_str,
                "description": param.description
            }));

            if param.required {
                required_params.push(param.name.clone());
            } else {
                optional_params.push(param.name.clone());
            }
        }

        let parameters = ToolParameters {
            parameters: input_params,
            schema: json!({
                "type": "object",
                "properties": properties,
                "required": required_params.clone()
            }),
            required: required_params,
            optional: optional_params,
        };

        let metadata = ToolMetadata {
            version: version.clone(),
            author: "Workflow".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["workflow".to_string(), "automation".to_string()],
            install_command: None,
            requirements: vec![output_desc],
        };

        Self {
            db,
            tool_manager,
            app_handle,
            workflow_id,
            workflow_name,
            workflow_description,
            full_tool_name,
            parameters,
            metadata,
        }
    }

    /// 执行工作流并等待完成
    async fn run_workflow(&self, mut graph: WorkflowGraph, inputs: HashMap<String, Value>) -> anyhow::Result<Value> {
        // 将输入参数注入到入口节点
        // 优先找 start 节点，否则找第一个没有入边的节点
        let target_nodes: std::collections::HashSet<String> = graph.edges.iter()
            .map(|e| e.to_node.clone())
            .collect();
        
        let entry_node_id = graph.nodes.iter()
            .find(|n| n.node_type == "start")
            .map(|n| n.id.clone())
            .or_else(|| {
                graph.nodes.iter()
                    .find(|n| !target_nodes.contains(&n.id))
                    .map(|n| n.id.clone())
            })
            .or_else(|| graph.nodes.first().map(|n| n.id.clone()));

        if let Some(entry_id) = entry_node_id {
            for node in &mut graph.nodes {
                if node.id == entry_id {
                    // 将 AI 传入的参数合并到入口节点的 params
                    for (key, value) in &inputs {
                        node.params.insert(key.clone(), value.clone());
                    }
                    tracing::info!(
                        "Injected {} input params to entry node: {} ({})",
                        inputs.len(),
                        node.node_name,
                        node.id
                    );
                    break;
                }
            }
        }

        let engine = Arc::new(WorkflowEngine::new());
        let def = graph_to_definition(&graph);
        
        // 启动执行
        let execution_id = engine.execute_workflow(&def, None).await?;
        
        tracing::info!(
            "Workflow execution started: {} (execution_id={})",
            self.workflow_name,
            execution_id
        );

        // 发送开始事件
        if let Some(ref handle) = self.app_handle {
            use tauri::Emitter;
            let _ = handle.emit("workflow:run-start", &json!({
                "execution_id": execution_id,
                "workflow_id": self.workflow_id,
                "workflow_name": self.workflow_name,
                "version": def.metadata.version,
                "status": "running",
                "source": "tool",
                "inputs": inputs
            }));
        }

        // 执行工作流步骤
        if let Some(ref handle) = self.app_handle {
            execute_workflow_steps(
                execution_id.clone(),
                graph.clone(),
                def.clone(),
                self.db.clone(),
                handle.clone(),
                engine.clone(),
                self.tool_manager.clone(),
            ).await;
        } else {
            tracing::warn!("No app_handle available, workflow events won't be emitted");
            return Ok(json!({
                "status": "error",
                "message": "Workflow tool requires app_handle for execution"
            }));
        }

        // 等待执行完成（轮询状态）
        let mut attempts = 0;
        let max_attempts = 120; // 最多等待 2 分钟
        
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            attempts += 1;

            match engine.get_execution_status(&execution_id).await {
                Ok(status) => {
                    let status_str = format!("{:?}", status.status);
                    tracing::debug!("Workflow status: {} (attempt {})", status_str, attempts);
                    
                    match status.status {
                        sentinel_workflow::engine::ExecutionStatus::Completed => {
                            tracing::info!("Workflow completed: {}", execution_id);
                            
                            // 从 End 节点获取输出
                            let output = self.extract_end_node_output(&graph, &status).await;
                            
                            return Ok(json!({
                                "status": "completed",
                                "execution_id": execution_id,
                                "workflow_id": self.workflow_id,
                                "workflow_name": self.workflow_name,
                                "output": output,
                                "completed_steps": status.completed_steps,
                                "total_steps": status.total_steps,
                            }));
                        }
                        sentinel_workflow::engine::ExecutionStatus::Failed => {
                            let error = status.error.unwrap_or_else(|| "Unknown error".to_string());
                            tracing::error!("Workflow failed: {} - {}", execution_id, error);
                            return Err(anyhow::anyhow!("Workflow failed: {}", error));
                        }
                        sentinel_workflow::engine::ExecutionStatus::Cancelled => {
                            return Err(anyhow::anyhow!("Workflow was cancelled"));
                        }
                        _ => {
                            // Still running
                            if attempts >= max_attempts {
                                return Err(anyhow::anyhow!("Workflow execution timeout"));
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get execution status: {}", e);
                    if attempts >= max_attempts {
                        return Err(anyhow::anyhow!("Workflow execution timeout: {}", e));
                    }
                }
            }
        }
    }

    /// 从出口节点提取输出
    /// - 如果有 End 节点，从 End 节点获取
    /// - 否则从最后一个执行的节点获取
    async fn extract_end_node_output(
        &self, 
        graph: &WorkflowGraph, 
        status: &sentinel_workflow::engine::WorkflowExecutionStatus
    ) -> Value {
        // 1. 首先尝试找 End 节点
        let end_node = graph.nodes.iter().find(|n| n.node_type == "end");
        
        if let Some(end) = end_node {
            if let Some(step_detail) = status.step_details.get(&end.id) {
                if let Some(result) = &step_detail.result {
                    return result.clone();
                }
            }
            return json!(end.params);
        }

        // 2. 没有 End 节点，找最后一个执行的节点（按拓扑顺序）
        // 计算拓扑顺序
        let edges: Vec<(String, String)> = graph.edges.iter()
            .map(|e| (e.from_node.clone(), e.to_node.clone()))
            .collect();
        let order = sentinel_workflow::topo_order(&graph.nodes, &edges);
        
        // 从后往前找，获取第一个有执行结果的节点
        for node_id in order.iter().rev() {
            if let Some(step_detail) = status.step_details.get(node_id) {
                if let Some(result) = &step_detail.result {
                    tracing::info!("Extracted output from node: {}", node_id);
                    return result.clone();
                }
            }
        }

        // 3. 如果 step_details 为空，尝试从 status.result 获取
        if let Some(result) = &status.result {
            return result.clone();
        }

        // 4. 最后返回一个默认结果
        json!({
            "status": "completed",
            "message": "Workflow executed successfully"
        })
    }
}

#[async_trait::async_trait]
impl UnifiedTool for WorkflowTool {
    fn name(&self) -> &str {
        &self.full_tool_name
    }

    fn description(&self) -> &str {
        &self.workflow_description
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Custom("workflow".to_string())
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        let start_time = std::time::Instant::now();
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);

        tracing::info!(
            "Executing workflow tool: {} ({}) with inputs: {:?}",
            self.workflow_name,
            self.workflow_id,
            params.inputs.keys().collect::<Vec<_>>()
        );

        // 从数据库加载工作流定义
        let wf_data = self.db.get_workflow_definition(&self.workflow_id).await.map_err(|e| {
            anyhow::anyhow!("Failed to load workflow: {}", e)
        })?;

        let wf_data = wf_data.ok_or_else(|| {
            anyhow::anyhow!("Workflow not found: {}", self.workflow_id)
        })?;

        // 获取工作流图
        let graph_value = wf_data.get("graph").ok_or_else(|| {
            anyhow::anyhow!("Workflow graph not found")
        })?;

        // 解析工作流图
        let graph: WorkflowGraph = serde_json::from_value(graph_value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse workflow graph: {}", e))?;

        // 执行工作流，传入输入参数
        let result = self.run_workflow(graph, params.inputs).await;
        let duration = start_time.elapsed();

        match result {
            Ok(output) => {
                tracing::info!(
                    "Workflow tool executed successfully: {} ({}ms)",
                    self.workflow_name,
                    duration.as_millis()
                );
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: self.full_tool_name.clone(),
                    tool_id: self.workflow_id.clone(),
                    success: true,
                    output,
                    error: None,
                    execution_time_ms: duration.as_millis() as u64,
                    metadata: HashMap::from([
                        ("workflow_id".to_string(), json!(self.workflow_id)),
                        ("workflow_name".to_string(), json!(self.workflow_name)),
                    ]),
                    started_at: Utc::now(),
                    completed_at: Some(Utc::now()),
                    status: ExecutionStatus::Completed,
                })
            }
            Err(e) => {
                tracing::error!(
                    "Workflow tool execution failed: {} - {}",
                    self.workflow_name,
                    e
                );
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: self.full_tool_name.clone(),
                    tool_id: self.workflow_id.clone(),
                    success: false,
                    output: json!(null),
                    error: Some(e.to_string()),
                    execution_time_ms: duration.as_millis() as u64,
                    metadata: HashMap::from([
                        ("workflow_id".to_string(), json!(self.workflow_id)),
                        ("workflow_name".to_string(), json!(self.workflow_name)),
                    ]),
                    started_at: Utc::now(),
                    completed_at: Some(Utc::now()),
                    status: ExecutionStatus::Failed,
                })
            }
        }
    }
}
