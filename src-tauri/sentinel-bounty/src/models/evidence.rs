//! Evidence management for findings and submissions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Evidence type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    /// HTTP request/response pair
    HttpTransaction,
    /// Screenshot or image
    Screenshot,
    /// Video recording
    Video,
    /// Log file
    LogFile,
    /// Source code snippet
    CodeSnippet,
    /// Network packet capture (PCAP)
    PacketCapture,
    /// Proof of Concept script
    PocScript,
    /// Exploit code
    Exploit,
    /// Configuration file
    Configuration,
    /// Other file type
    File,
    /// Text note
    Note,
}

/// Evidence item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Unique identifier
    pub id: String,
    /// Associated finding ID
    pub finding_id: String,
    /// Evidence type
    pub evidence_type: EvidenceType,
    /// Title or description
    pub title: String,
    /// Detailed description
    pub description: Option<String>,
    /// File path (if stored locally)
    pub file_path: Option<String>,
    /// File URL (if stored remotely)
    pub file_url: Option<String>,
    /// Content (for text-based evidence)
    pub content: Option<String>,
    /// MIME type
    pub mime_type: Option<String>,
    /// File size in bytes
    pub file_size: Option<i64>,
    /// HTTP request (if applicable)
    pub http_request: Option<HttpRequest>,
    /// HTTP response (if applicable)
    pub http_response: Option<HttpResponse>,
    /// Diff content (before/after)
    pub diff: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Display order
    pub display_order: i32,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Evidence {
    pub fn new(finding_id: String, evidence_type: EvidenceType, title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            finding_id,
            evidence_type,
            title,
            description: None,
            file_path: None,
            file_url: None,
            content: None,
            mime_type: None,
            file_size: None,
            http_request: None,
            http_response: None,
            diff: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
            display_order: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create HTTP transaction evidence
    pub fn from_http_transaction(
        finding_id: String,
        request: HttpRequest,
        response: HttpResponse,
        title: String,
    ) -> Self {
        let mut evidence = Self::new(finding_id, EvidenceType::HttpTransaction, title);
        evidence.http_request = Some(request);
        evidence.http_response = Some(response);
        evidence
    }
}

/// HTTP request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// HTTP response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: Option<i64>,
}

/// Create evidence request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEvidenceRequest {
    pub finding_id: String,
    pub evidence_type: EvidenceType,
    pub title: String,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub file_url: Option<String>,
    pub content: Option<String>,
    pub mime_type: Option<String>,
    pub http_request: Option<HttpRequest>,
    pub http_response: Option<HttpResponse>,
    pub diff: Option<String>,
    pub tags: Option<Vec<String>>,
    pub display_order: Option<i32>,
}

/// Update evidence request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEvidenceRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub file_url: Option<String>,
    pub content: Option<String>,
    pub mime_type: Option<String>,
    pub http_request: Option<HttpRequest>,
    pub http_response: Option<HttpResponse>,
    pub diff: Option<String>,
    pub tags: Option<Vec<String>>,
    pub display_order: Option<i32>,
}

/// Evidence filter for queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvidenceFilter {
    pub finding_ids: Option<Vec<String>>,
    pub evidence_types: Option<Vec<EvidenceType>>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
}

/// Evidence export format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceExport {
    pub evidence: Evidence,
    pub file_data: Option<Vec<u8>>,
}
