use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScanSession {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target: String,
    pub scan_type: String,
    pub status: ScanSessionStatus,
    pub config: serde_json::Value,
    pub progress: f32,
    pub current_stage: String,
    pub total_stages: i32,
    pub completed_stages: i32,
    pub results_summary: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "scan_session_status", rename_all = "lowercase")]
#[derive(Default)]
pub enum ScanSessionStatus {
    #[default]
    Created,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScanSessionRequest {
    pub name: String,
    pub description: Option<String>,
    pub target: String,
    pub scan_type: String,
    pub config: serde_json::Value,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateScanSessionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<ScanSessionStatus>,
    pub progress: Option<f32>,
    pub current_stage: Option<String>,
    pub total_stages: Option<i32>,
    pub completed_stages: Option<i32>,
    pub results_summary: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScanStage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub stage_name: String,
    pub stage_order: i32,
    pub status: ScanStageStatus,
    pub tool_name: String,
    pub config: serde_json::Value,
    pub results: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "scan_stage_status", rename_all = "lowercase")]
#[derive(Default)]
pub enum ScanStageStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub session_id: Uuid,
    pub overall_progress: f32,
    pub current_stage: String,
    pub completed_stages: i32,
    pub total_stages: i32,
    pub stages: Vec<ScanStageProgress>,
    pub estimated_time_remaining: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStageProgress {
    pub stage_name: String,
    pub status: ScanStageStatus,
    pub progress: f32,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
}



impl ScanSession {
    pub fn new(
        name: String,
        target: String,
        scan_type: String,
        config: serde_json::Value,
        created_by: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            target,
            scan_type,
            status: ScanSessionStatus::Created,
            config,
            progress: 0.0,
            current_stage: "初始化".to_string(),
            total_stages: 0,
            completed_stages: 0,
            results_summary: None,
            error_message: None,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            created_by,
        }
    }

    pub fn start(&mut self) {
        self.status = ScanSessionStatus::Running;
        self.started_at = Some(chrono::Utc::now());
    }

    pub fn complete(&mut self, results_summary: Option<serde_json::Value>) {
        self.status = ScanSessionStatus::Completed;
        self.completed_at = Some(chrono::Utc::now());
        self.progress = 100.0;
        self.results_summary = results_summary;
    }

    pub fn fail(&mut self, error_message: String) {
        self.status = ScanSessionStatus::Failed;
        self.completed_at = Some(chrono::Utc::now());
        self.error_message = Some(error_message);
    }

    pub fn cancel(&mut self) {
        self.status = ScanSessionStatus::Cancelled;
        self.completed_at = Some(chrono::Utc::now());
    }

    pub fn update_progress(&mut self, progress: f32, current_stage: String, completed_stages: i32) {
        self.progress = progress.clamp(0.0, 100.0);
        self.current_stage = current_stage;
        self.completed_stages = completed_stages;
    }
}

impl ScanStage {
    pub fn new(
        session_id: Uuid,
        stage_name: String,
        stage_order: i32,
        tool_name: String,
        config: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            stage_name,
            stage_order,
            status: ScanStageStatus::Pending,
            tool_name,
            config,
            results: None,
            error_message: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
        }
    }

    pub fn start(&mut self) {
        self.status = ScanStageStatus::Running;
        self.started_at = Some(chrono::Utc::now());
    }

    pub fn complete(&mut self, results: Option<serde_json::Value>) {
        self.status = ScanStageStatus::Completed;
        self.completed_at = Some(chrono::Utc::now());
        self.results = results;

        if let Some(started) = self.started_at {
            self.duration_ms = Some(
                chrono::Utc::now()
                    .signed_duration_since(started)
                    .num_milliseconds(),
            );
        }
    }

    pub fn fail(&mut self, error_message: String) {
        self.status = ScanStageStatus::Failed;
        self.completed_at = Some(chrono::Utc::now());
        self.error_message = Some(error_message);

        if let Some(started) = self.started_at {
            self.duration_ms = Some(
                chrono::Utc::now()
                    .signed_duration_since(started)
                    .num_milliseconds(),
            );
        }
    }
}
