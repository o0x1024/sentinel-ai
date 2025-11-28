use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
pub struct NodeCatalogItem {
    pub node_type: String,
    pub label: String,
    pub category: String,
    pub params_schema: serde_json::Value,
    pub input_ports: Vec<PortDef>,
    pub output_ports: Vec<PortDef>,
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
