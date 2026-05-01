use crate::types::Finding;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::warn;

/// 插件执行上下文（用于收集插件发现的漏洞）
#[derive(Clone, Default)]
pub struct PluginContext {
    pub findings: Arc<Mutex<Vec<Finding>>>,
    pub last_result: Arc<Mutex<Option<serde_json::Value>>>,
    pub plugin_id: Arc<Mutex<Option<String>>>,
    pub plugin_main_category: Arc<Mutex<Option<String>>>,
    pub execution_context: Arc<Mutex<Option<String>>>,
    pub run_id: Arc<Mutex<Option<String>>>,
    pub monitor_type: Arc<Mutex<Option<String>>>,
    pub traffic_request_id: Arc<Mutex<Option<String>>>,
    pub finding_sink: Arc<Mutex<Option<mpsc::UnboundedSender<Finding>>>>,
}

impl PluginContext {
    pub fn new() -> Self {
        Self {
            findings: Arc::new(Mutex::new(Vec::new())),
            last_result: Arc::new(Mutex::new(None)),
            plugin_id: Arc::new(Mutex::new(None)),
            plugin_main_category: Arc::new(Mutex::new(None)),
            execution_context: Arc::new(Mutex::new(None)),
            run_id: Arc::new(Mutex::new(None)),
            monitor_type: Arc::new(Mutex::new(None)),
            traffic_request_id: Arc::new(Mutex::new(None)),
            finding_sink: Arc::new(Mutex::new(None)),
        }
    }

    pub fn take_findings(&self) -> Vec<Finding> {
        let mut findings = self.findings.lock().unwrap();
        std::mem::take(&mut *findings)
    }

    pub fn take_last_result(&self) -> Option<serde_json::Value> {
        let mut last = self.last_result.lock().unwrap();
        std::mem::take(&mut *last)
    }

    pub fn set_plugin_id(&self, plugin_id: Option<String>) {
        let mut current = self.plugin_id.lock().unwrap();
        *current = plugin_id;
    }

    pub fn plugin_id(&self) -> Option<String> {
        self.plugin_id.lock().unwrap().clone()
    }

    pub fn set_plugin_main_category(&self, main_category: Option<String>) {
        let mut current = self.plugin_main_category.lock().unwrap();
        *current = main_category;
    }

    pub fn plugin_main_category(&self) -> Option<String> {
        self.plugin_main_category.lock().unwrap().clone()
    }

    pub fn set_execution_context(&self, execution_context: Option<String>) {
        let mut current = self.execution_context.lock().unwrap();
        *current = execution_context;
    }

    pub fn execution_context(&self) -> Option<String> {
        self.execution_context.lock().unwrap().clone()
    }

    pub fn set_run_id(&self, run_id: Option<String>) {
        let mut current = self.run_id.lock().unwrap();
        *current = run_id;
    }

    pub fn run_id(&self) -> Option<String> {
        self.run_id.lock().unwrap().clone()
    }

    pub fn set_monitor_type(&self, monitor_type: Option<String>) {
        let mut current = self.monitor_type.lock().unwrap();
        *current = monitor_type;
    }

    pub fn set_traffic_request_id(&self, request_id: Option<String>) {
        let mut current = self.traffic_request_id.lock().unwrap();
        *current = request_id;
    }

    pub fn traffic_request_id(&self) -> Option<String> {
        self.traffic_request_id.lock().unwrap().clone()
    }

    pub fn set_finding_sink(&self, finding_sink: Option<mpsc::UnboundedSender<Finding>>) {
        let mut current = self.finding_sink.lock().unwrap();
        *current = finding_sink;
    }

    pub fn emit_finding(&self, finding: Finding) -> bool {
        if let Some(sink) = self.finding_sink.lock().unwrap().clone() {
            match sink.send(finding.clone()) {
                Ok(_) => {
                    let mut findings = self.findings.lock().unwrap();
                    findings.push(finding);
                    return true;
                }
                Err(error) => {
                    warn!("Failed to stream finding to traffic pipeline: {}", error);
                    return false;
                }
            }
        }

        false
    }
}
