use crate::engines::vision_explorer_v2::core::{
    Agent, Event, PageContext, PerceptionEngine, TaskResult,
};
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct PerceptionAgent {
    id: String,
    engine: Box<dyn PerceptionEngine>,
    event_tx: mpsc::Sender<Event>,
}

impl PerceptionAgent {
    pub fn new(
        id: String,
        engine: Box<dyn PerceptionEngine>,
        event_tx: mpsc::Sender<Event>,
    ) -> Self {
        Self {
            id,
            engine,
            event_tx,
        }
    }
}

#[async_trait]
impl Agent for PerceptionAgent {
    fn id(&self) -> String {
        self.id.clone()
    }

    async fn handle_event(&self, event: &Event) -> Result<()> {
        match event {
            Event::TaskAssigned {
                agent_id,
                task_id,
                target_node_id: _,
                payload, // Payload should contain PageContext? Or we fetch it?
            } if agent_id == &self.id => {
                // In V2, we assume we want to analyze the "Current" context?
                // OR the payload contains the context to analyze?
                // The Planner's 'add_node' logic created a node with URL/DOM.
                // But `PerceptionEngine::analyze` takes `PageContext` which has screenshot etc.
                // WE DON'T STORE SCREENSHOT IN GRAPH NODE (usually) to save RAM.

                // So the Payload from Planner needs to contain the Context?
                // Planner: `target_node_id` is passed.
                // But Planner doesn't have the heavy context (Screenshot).
                // Actually Planner received `PageContext` in `TaskCompleted`.
                // It likely dropped the screenshot.

                // Solution: The `Navigator` emits `PageContext` via `TaskCompleted`.
                // The `Planner` sees it.
                // BUT `Analyst` needs that same `PageContext`.
                // If `Planner` sends `TaskAssigned(Analyst)`, it must pass the `PageContext`.
                // So Planner needs to validly construct it or pass it through.

                // Hack/Fix: `Planner` receives `TaskCompleted` (Context).
                // It IMMEDIATELY triggers `Analyst` with that `Context` as payload.

                // Checking Planner logic again:
                // Planner logic: if Unvisited -> return TaskAssigned(Analyst, payload: None).
                // This assumes Analyst can fetch state or we pass it.
                // Since this is a distributed/event-driven system, passing large payloads is OK locally.

                // Let's assume payload IS the context.

                // Execute analysis and always send TaskCompleted
                let analysis_result = if let Some(val) = payload {
                    if let Ok(context) = serde_json::from_value::<PageContext>(val.clone()) {
                        self.engine.analyze(&context).await
                    } else {
                        log::error!("PerceptionAgent received task without valid PageContext payload");
                        Err(anyhow::anyhow!("Invalid PageContext payload"))
                    }
                } else {
                    log::error!("PerceptionAgent received task without payload");
                    Err(anyhow::anyhow!("Missing PageContext payload"))
                };

                // Always send TaskCompleted
                let result = match analysis_result {
                    Ok(perception_result) => {
                        log::info!("PerceptionAgent {}: Analysis complete", self.id);
                        TaskResult {
                            success: true,
                            message: "Analysis complete".to_string(),
                            new_nodes: vec![],
                            data: Some(serde_json::to_value(perception_result)?),
                        }
                    }
                    Err(e) => {
                        log::error!("PerceptionAgent {} error: {}", self.id, e);
                        TaskResult {
                            success: false,
                            message: format!("Analysis failed: {}", e),
                            new_nodes: vec![],
                            data: None,
                        }
                    }
                };

                self.event_tx
                    .send(Event::TaskCompleted {
                        agent_id: self.id.clone(),
                        task_id: task_id.clone(),
                        result,
                    })
                    .await?;

                Ok(())
            }
            _ => Ok(()),
        }
    }
}
