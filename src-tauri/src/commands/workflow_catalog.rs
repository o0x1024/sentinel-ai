use crate::models::workflow_graph::{NodeCatalogItem, PortDef, PortType};
use crate::tools::{get_global_tool_system, ToolInfo};
use crate::commands::passive_scan_commands::PassiveScanState;
use serde_json::json;

fn port(id: &str, name: &str, port_type: PortType, required: bool) -> PortDef {
    PortDef {
        id: id.to_string(),
        name: name.to_string(),
        port_type,
        required,
    }
}

fn trigger_nodes() -> Vec<NodeCatalogItem> {
    vec![
        NodeCatalogItem {
            node_type: "trigger_manual".to_string(),
            label: "手动触发".to_string(),
            category: "trigger".to_string(),
            params_schema: json!({"type": "object", "properties": {}}),
            input_ports: vec![],
            output_ports: vec![port("out", "输出", PortType::Json, true)],
        },
        NodeCatalogItem {
            node_type: "trigger_http".to_string(),
            label: "HTTP 触发".to_string(),
            category: "trigger".to_string(),
            params_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "method": {"type": "string", "enum": ["GET","POST"]}
                },
                "required": ["path"]
            }),
            input_ports: vec![],
            output_ports: vec![port("out", "请求体", PortType::Json, true)],
        },
        NodeCatalogItem {
            node_type: "trigger_schedule".to_string(),
            label: "定时触发".to_string(),
            category: "trigger".to_string(),
            params_schema: json!({
                "type": "object",
                "properties": {
                    "cron": {"type": "string"}
                },
                "required": ["cron"]
            }),
            input_ports: vec![],
            output_ports: vec![port("out", "输出", PortType::Json, true)],
        },
    ]
}

fn control_flow_nodes() -> Vec<NodeCatalogItem> {
    vec![
        NodeCatalogItem {
            node_type: "branch".to_string(),
            label: "条件分支".to_string(),
            category: "control".to_string(),
            params_schema: json!({
                "type": "object",
                "properties": {"expr": {"type": "string"}},
                "required": ["expr"]
            }),
            input_ports: vec![port("in", "输入", PortType::Json, true)],
            output_ports: vec![
                port("true", "为真", PortType::Json, true),
                port("false", "为假", PortType::Json, true),
            ],
        },
        NodeCatalogItem {
            node_type: "merge".to_string(),
            label: "合并".to_string(),
            category: "control".to_string(),
            params_schema: json!({"type": "object"}),
            input_ports: vec![
                port("in1", "输入1", PortType::Json, true),
                port("in2", "输入2", PortType::Json, true),
            ],
            output_ports: vec![port("out", "输出", PortType::Json, true)],
        },
        NodeCatalogItem {
            node_type: "parallel".to_string(),
            label: "并行".to_string(),
            category: "control".to_string(),
            params_schema: json!({
                "type": "object",
                "properties": {"max_concurrency": {"type": "integer"}}
            }),
            input_ports: vec![port("in", "输入", PortType::Array(Box::new(PortType::Json)), true)],
            output_ports: vec![port("out", "输出", PortType::Array(Box::new(PortType::Json)), true)],
        },
        NodeCatalogItem {
            node_type: "retry".to_string(),
            label: "重试".to_string(),
            category: "control".to_string(),
            params_schema: json!({
                "type": "object",
                "properties": {
                    "times": {"type": "integer"},
                    "delay_ms": {"type": "integer"},
                    "tool_name": {"type": "string"},
                    "tool_params": {"type": "object"}
                },
                "required": ["times"]
            }),
            input_ports: vec![port("in", "输入", PortType::Json, true)],
            output_ports: vec![port("out", "输出", PortType::Json, true)],
        },
    ]
}

fn data_nodes() -> Vec<NodeCatalogItem> {
    vec![
        NodeCatalogItem {
            node_type: "json_transform".to_string(),
            label: "JSON 变换".to_string(),
            category: "data".to_string(),
            params_schema: json!({
                "type": "object",
                "properties": {"mapping": {"type": "object"}},
                "required": ["mapping"]
            }),
            input_ports: vec![port("in", "输入", PortType::Json, true)],
            output_ports: vec![port("out", "输出", PortType::Json, true)],
        },
        NodeCatalogItem {
            node_type: "http_request".to_string(),
            label: "HTTP 请求".to_string(),
            category: "data".to_string(),
            params_schema: json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string"},
                    "method": {"type": "string", "enum": ["GET","POST","PUT","DELETE"]},
                    "headers": {"type": "object"},
                    "body": {"type": "object"}
                },
                "required": ["url"]
            }),
            input_ports: vec![],
            output_ports: vec![port("response", "响应", PortType::Json, true)],
        },
    ]
}

fn output_nodes() -> Vec<NodeCatalogItem> {
    vec![
        NodeCatalogItem {
            node_type: "output".to_string(),
            label: "输出".to_string(),
            category: "output".to_string(),
            params_schema: json!({"type": "object", "properties": {"save_artifact": {"type": "boolean"}}}),
            input_ports: vec![port("in", "输入", PortType::Json, true)],
            output_ports: vec![port("artifact", "产物", PortType::Artifact, false)],
        },
        NodeCatalogItem {
            node_type: "notify".to_string(),
            label: "通知".to_string(),
            category: "output".to_string(),
            params_schema: json!({
                "type": "object",
                "properties": {
                    "notification_rule_id": {
                        "type": "string",
                        "title": "通知规则",
                        "description": "选择已配置的通知规则"
                    },
                    "title": {
                        "type": "string",
                        "title": "标题",
                        "description": "通知标题"
                    },
                    "content": {
                        "type": "string",
                        "title": "内容",
                        "description": "通知内容，支持模板变量"
                    },
                    "use_input_as_content": {
                        "type": "boolean",
                        "title": "使用输入作为内容",
                        "description": "如果启用，将使用上游节点的输出作为通知内容",
                        "default": false
                    }
                },
                "required": ["notification_rule_id"]
            }),
            input_ports: vec![port("in", "输入", PortType::Json, true)],
            output_ports: vec![],
        },
    ]
}

fn rag_nodes() -> Vec<NodeCatalogItem> {
    vec![
        NodeCatalogItem {
            node_type: "rag::ingest".to_string(),
            label: "RAG 导入".to_string(),
            category: "rag".to_string(),
            params_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"},
                    "collection_id": {"type": "string"},
                    "metadata": {"type": "object"}
                },
                "required": ["file_path"]
            }),
            input_ports: vec![],
            output_ports: vec![port("ingest_result", "导入结果", PortType::Json, true)],
        },
        NodeCatalogItem {
            node_type: "rag::query".to_string(),
            label: "RAG 查询".to_string(),
            category: "rag".to_string(),
            params_schema: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "collection_id": {"type": "string"},
                    "top_k": {"type": "integer"},
                    "use_mmr": {"type": "boolean"},
                    "mmr_lambda": {"type": "number"},
                    "filters": {"type": "object"}
                },
                "required": ["query"]
            }),
            input_ports: vec![],
            output_ports: vec![port("rag_context", "检索结果", PortType::Json, true)],
        },
    ]
}

fn prompt_nodes() -> Vec<NodeCatalogItem> {
    vec![
        NodeCatalogItem {
            node_type: "prompt::build".to_string(),
            label: "Prompt 构建".to_string(),
            category: "prompt".to_string(),
            params_schema: json!({
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "build_type": {"type": "string", "enum": ["Planner", "Executor", "Replanner", "ReportGenerator"]},
                    "user_query": {"type": "string"},
                    "target_info": {"type": "object"},
                    "available_tools": {"type": "array"},
                    "execution_context": {"type": "object"},
                    "history": {"type": "array"},
                    "custom_variables": {"type": "object"},
                    "record_performance": {"type": "boolean"}
                },
                "required": ["user_query"]
            }),
            input_ports: vec![],
            output_ports: vec![port("prompt_result", "构建结果", PortType::Json, true)],
        },
    ]
}

fn tool_node_from_tool(info: &ToolInfo) -> NodeCatalogItem {
    // 简化映射：将工具参数schema直接用作节点参数schema
    let params_schema = info.parameters.schema.clone();

    NodeCatalogItem {
        node_type: format!("tool::{}", info.name),
        label: info.name.clone(),
        category: "tool".to_string(),
        params_schema,
        input_ports: vec![port("params", "参数", PortType::Json, false)],
        output_ports: vec![port("result", "结果", PortType::Json, true)],
    }
}

fn plugin_tool_node_from_plugin(plugin: &sentinel_passive::PluginRecord) -> NodeCatalogItem {
    // 为Agent插件创建工具节点
    // 使用 plugin:: 前缀以区分普通工具
    
    // 尝试从全局工具系统获取插件的参数schema
    let params_schema = if let Ok(tool_system) = get_global_tool_system() {
        let tool_name = format!("plugin::{}", plugin.metadata.id);
        
        // 同步获取工具列表并查找对应的插件工具
        let tools = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                tool_system.list_tools().await
            })
        });
        
        if let Some(tool) = tools.iter().find(|t| t.name == tool_name) {
            // 使用插件工具的实际参数schema
            tool.parameters.schema.clone()
        } else {
            // 如果找不到工具，使用默认的通用参数schema
            json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "object",
                        "title": "插件输入参数",
                        "description": "根据插件需求填写的JSON对象",
                        "additionalProperties": true
                    }
                }
            })
        }
    } else {
        // 工具系统未初始化，使用默认schema
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "object",
                    "title": "插件输入参数",
                    "description": "根据插件需求填写的JSON对象",
                    "additionalProperties": true
                }
            }
        })
    };

    NodeCatalogItem {
        node_type: format!("plugin::{}", plugin.metadata.id),
        label: format!("{} (插件)", plugin.metadata.name),
        category: "tool".to_string(),
        params_schema,
        input_ports: vec![port("inputs", "输入", PortType::Json, false)],
        output_ports: vec![port("result", "结果", PortType::Json, true)],
    }
}

#[tauri::command]
pub async fn list_node_catalog(
    passive_state: tauri::State<'_, PassiveScanState>,
) -> Result<Vec<NodeCatalogItem>, String> {
    let mut catalog: Vec<NodeCatalogItem> = Vec::new();

    // 基础节点
    catalog.extend(trigger_nodes());
    catalog.extend(control_flow_nodes());
    catalog.extend(data_nodes());
    catalog.extend(output_nodes());
    catalog.extend(rag_nodes());
    catalog.extend(prompt_nodes());

    // 工具节点（从全局工具系统）
    if let Ok(tool_system) = get_global_tool_system() {
        let tools = tool_system.list_tools().await;
        for t in tools {
            // 仅添加可用工具
            if t.available {
                catalog.push(tool_node_from_tool(&t));
            }
        }
    }

    // Agent插件工具节点（直接从插件系统）
    match passive_state.list_plugins_internal().await {
        Ok(plugins) => {
            for plugin in plugins {
                // 只添加已启用的agent类别插件
                if plugin.status == sentinel_passive::PluginStatus::Enabled
                    && plugin.metadata.main_category == "agent"
                {
                    catalog.push(plugin_tool_node_from_plugin(&plugin));
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to load agent plugins for workflow catalog: {}", e);
        }
    }

    Ok(catalog)
}
