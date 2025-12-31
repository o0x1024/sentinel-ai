//! Tauri commands for Test Explorer V1

use crate::engines::test_explorer_v1::{
    engine::TestExplorerV1Engine, planner::TaskPlanner, tools::TestExplorerToolState,
    tools::register_test_explorer_tools, types::*,
};
use crate::engines::create_client;
use crate::services::ai::AiService;
use anyhow::Result;
use serde_json::json;
use sentinel_tools::ToolServer;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Global state for test explorer sessions
pub struct TestExplorerState {
    sessions: Arc<RwLock<HashMap<String, Arc<RwLock<TestExplorerV1Engine>>>>>,
    tool_state: Arc<TestExplorerToolState>,
}

impl Default for TestExplorerState {
    fn default() -> Self {
        Self::new()
    }
}

impl TestExplorerState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            tool_state: Arc::new(TestExplorerToolState::new()),
        }
    }
}

/// Initialize test explorer (create browser and register tools)
#[tauri::command]
pub async fn test_explorer_init(
    app_handle: AppHandle,
    config: TestExplorerV1Config,
    use_planner: bool,
) -> Result<String, String> {
    info!("Initializing test explorer with config: {:?}", config);

    // Generate session ID
    let session_id = uuid::Uuid::new_v4().to_string();

    // Get AI service for planner
    let ai_service = app_handle
        .state::<Arc<RwLock<AiService>>>()
        .inner()
        .clone();

    // Create planner if requested
    let planner = if use_planner {
        let llm_client = create_client(&*ai_service.read().await);
        Some(TaskPlanner::new(llm_client))
    } else {
        None
    };

    // Create engine
    let engine = TestExplorerV1Engine::new(config.clone(), planner)
        .await
        .map_err(|e| format!("Failed to create engine: {}", e))?;

    // Get driver reference
    let driver = engine.driver();

    // Store session
    let state = app_handle.state::<TestExplorerState>();
    state
        .sessions
        .write()
        .await
        .insert(session_id.clone(), Arc::new(RwLock::new(engine)));

    // Register tools with the driver
    state.tool_state.set_driver(driver).await;

    // Register tools in global tool server
    let tool_server = app_handle.state::<Arc<ToolServer>>();
    register_test_explorer_tools(&tool_server, state.tool_state.clone())
        .await
        .map_err(|e| format!("Failed to register tools: {}", e))?;

    info!("Test explorer initialized with session ID: {}", session_id);

    Ok(session_id)
}

/// Execute test explorer with direct mode (LLM + tools)
#[tauri::command]
pub async fn test_explorer_execute_direct(
    app_handle: AppHandle,
    session_id: String,
    prompt: String,
) -> Result<ExecutionResult, String> {
    info!("Executing test explorer direct mode: {}", prompt);

    let state = app_handle.state::<TestExplorerState>();
    let sessions = state.sessions.read().await;

    let engine = sessions
        .get(&session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let mut engine = engine.write().await;
    engine
        .execute_direct(&prompt)
        .await
        .map_err(|e| format!("Execution failed: {}", e))
}

/// Execute test explorer with planning mode
#[tauri::command]
pub async fn test_explorer_execute_planning(
    app_handle: AppHandle,
    session_id: String,
    goal: String,
) -> Result<ExecutionResult, String> {
    info!("Executing test explorer planning mode: {}", goal);

    let state = app_handle.state::<TestExplorerState>();
    let sessions = state.sessions.read().await;

    let engine = sessions
        .get(&session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let mut engine = engine.write().await;
    engine
        .execute_with_planning(&goal)
        .await
        .map_err(|e| format!("Execution failed: {}", e))
}

/// Execute with streaming events
#[tauri::command]
pub async fn test_explorer_execute_streaming(
    app_handle: AppHandle,
    session_id: String,
    goal: String,
    use_planner: bool,
) -> Result<(), String> {
    info!("Executing test explorer streaming mode: {}", goal);

    let state = app_handle.state::<TestExplorerState>();
    let sessions = state.sessions.read().await;

    let engine = sessions
        .get(&session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?
        .clone();

    let app = app_handle.clone();
    let sid = session_id.clone();

    tokio::spawn(async move {
        let mut engine = engine.write().await;

        let result = engine
            .execute_streaming(&goal, use_planner, |event| {
                // Emit event to frontend
                let _ = app.emit_to(
                    tauri::EventTarget::any(),
                    &format!("test_explorer:event:{}", sid),
                    &event
                );
            })
            .await;

        match result {
            Ok(exec_result) => {
                let _ = app.emit_to(
                    tauri::EventTarget::any(),
                    &format!("test_explorer:complete:{}", sid),
                    &json!({ "success": true, "result": exec_result })
                );
            }
            Err(e) => {
                error!("Streaming execution failed: {}", e);
                let _ = app.emit_to(
                    tauri::EventTarget::any(),
                    &format!("test_explorer:complete:{}", sid),
                    &json!({ "success": false, "error": e.to_string() })
                );
            }
        }
    });

    Ok(())
}

/// Get captured API requests
#[tauri::command]
pub async fn test_explorer_get_apis(
    app_handle: AppHandle,
    session_id: String,
) -> Result<Vec<ApiRequest>, String> {
    let state = app_handle.state::<TestExplorerState>();
    let sessions = state.sessions.read().await;

    let engine = sessions
        .get(&session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let engine = engine.read().await;
    let apis = engine.driver().get_captured_requests().await;

    Ok(apis)
}

/// Get execution history
#[tauri::command]
pub async fn test_explorer_get_history(
    app_handle: AppHandle,
    session_id: String,
) -> Result<Vec<ExecutionStep>, String> {
    let state = app_handle.state::<TestExplorerState>();
    let sessions = state.sessions.read().await;

    let engine = sessions
        .get(&session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let engine = engine.read().await;
    Ok(engine.history().to_vec())
}

/// Export captured APIs as HAR
#[tauri::command]
pub async fn test_explorer_export_har(
    app_handle: AppHandle,
    _session_id: String,
) -> Result<serde_json::Value, String> {
    let state = app_handle.state::<TestExplorerState>();

    // Get driver from tool state
    let driver = state
        .tool_state
        .get_driver()
        .await
        .ok_or_else(|| "Browser not initialized".to_string())?;

    // Get network listener and export HAR
    let requests = driver.get_captured_requests().await;

    // Build HAR format
    let entries: Vec<serde_json::Value> = requests
        .iter()
        .map(|req| {
            json!({
                "startedDateTime": chrono::DateTime::<chrono::Utc>::from(req.timestamp).to_rfc3339(),
                "time": 0,
                "request": {
                    "method": req.method,
                    "url": req.url,
                    "httpVersion": "HTTP/1.1",
                    "headers": req.headers.iter().map(|(k, v)| {
                        json!({"name": k, "value": v})
                    }).collect::<Vec<_>>(),
                    "queryString": [],
                    "postData": req.request_body.as_ref().map(|body| {
                        json!({
                            "mimeType": "application/json",
                            "text": body
                        })
                    }),
                },
                "response": {
                    "status": req.response_status.unwrap_or(0),
                    "statusText": "",
                    "httpVersion": "HTTP/1.1",
                    "headers": req.response_headers.as_ref().map(|headers| {
                        headers.iter().map(|(k, v)| {
                            json!({"name": k, "value": v})
                        }).collect::<Vec<_>>()
                    }).unwrap_or_default(),
                    "content": {
                        "size": req.response_body.as_ref().map(|b| b.len()).unwrap_or(0),
                        "mimeType": "application/json",
                        "text": req.response_body.as_ref().unwrap_or(&String::new())
                    },
                },
            })
        })
        .collect();

    Ok(json!({
        "log": {
            "version": "1.2",
            "creator": {
                "name": "Sentinel AI Test Explorer V1",
                "version": "1.0.0"
            },
            "entries": entries
        }
    }))
}

/// Close a test explorer session
#[tauri::command]
pub async fn test_explorer_close(
    app_handle: AppHandle,
    session_id: String,
) -> Result<(), String> {
    info!("Closing test explorer session: {}", session_id);

    let state = app_handle.state::<TestExplorerState>();
    let mut sessions = state.sessions.write().await;

    sessions
        .remove(&session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    Ok(())
}

