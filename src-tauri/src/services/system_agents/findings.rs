use anyhow::Result;
use chrono::Utc;
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use sentinel_db::{DatabaseService, TrafficEvidenceRecord, TrafficFinding};

use crate::services::system_agents::safety::SystemAgentSafetyPolicy;
use crate::services::system_agents::types::SystemAgentEvent;

pub async fn persist_passive_agent_finding(
    db: &DatabaseService,
    app_handle: &AppHandle,
    profile_id: &str,
    safety_policy_json: &str,
    event: &SystemAgentEvent,
    output: &Value,
) -> Result<Option<String>> {
    let finding = match build_finding_from_output(db, profile_id, event, output).await? {
        Some(finding) => finding,
        None => return Ok(None),
    };
    let safety_policy = SystemAgentSafetyPolicy::from_json_str(safety_policy_json);

    let signature = finding.calculate_signature();
    if db.check_traffic_signature_exists(&signature).await? {
        db.update_traffic_vulnerability_hit(&signature).await?;
        return Ok(None);
    }

    if safety_policy.is_shadow_mode_enabled() {
        let _ = app_handle.emit(
            "system-agent:shadow-finding",
            serde_json::json!({
                "profileId": profile_id,
                "riskType": finding.vuln_type,
                "severity": finding.severity,
                "url": finding.url,
                "summary": output
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or("Passive system agent detected a potential shadow-mode risk."),
                "timestamp": Utc::now().to_rfc3339(),
            }),
        );
        return Ok(None);
    }

    let vuln_id = finding.id.clone();
    let vuln_type = finding.vuln_type.clone();
    let severity = finding.severity.clone();
    let url = finding.url.clone();
    let summary = format!("{} - {}", finding.title, finding.description);

    db.insert_traffic_vulnerability(&finding).await?;
    insert_system_agent_context_evidence(
        db,
        &finding.id,
        &finding.url,
        &finding.method,
        event,
        output,
    )
    .await?;

    let _ = app_handle.emit(
        "scan:finding",
        serde_json::json!({
            "vuln_id": vuln_id,
            "vuln_type": vuln_type,
            "severity": severity,
            "url": url,
            "summary": summary,
            "timestamp": Utc::now().to_rfc3339(),
        }),
    );

    Ok(Some(finding.id))
}

async fn insert_system_agent_context_evidence(
    db: &DatabaseService,
    vuln_id: &str,
    url: &str,
    method: &str,
    event: &SystemAgentEvent,
    output: &Value,
) -> Result<()> {
    let event_payload =
        serde_json::to_string_pretty(&event.payload).unwrap_or_else(|_| event.payload.to_string());
    let output_payload =
        serde_json::to_string_pretty(output).unwrap_or_else(|_| output.to_string());
    let meta_payload = serde_json::json!({
        "sourceEvent": event.event_name,
        "eventSource": event.source,
        "capturedAt": event.timestamp.to_rfc3339(),
    });

    let evidence = TrafficEvidenceRecord {
        id: format!("{}-ctx-{}", vuln_id, Uuid::new_v4()),
        vuln_id: vuln_id.to_string(),
        url: url.to_string(),
        method: method.to_string(),
        location: "system_agent_context".to_string(),
        evidence_snippet: output
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("System agent triage context")
            .to_string(),
        request_headers: None,
        request_body: Some(event_payload),
        response_status: None,
        response_headers: Some(meta_payload.to_string()),
        response_body: Some(output_payload),
        timestamp: Utc::now(),
    };

    db.insert_traffic_evidence(&evidence).await
}

async fn build_finding_from_output(
    db: &DatabaseService,
    profile_id: &str,
    event: &SystemAgentEvent,
    output: &Value,
) -> Result<Option<TrafficFinding>> {
    let risk_type = output
        .get("riskType")
        .and_then(Value::as_str)
        .unwrap_or("none");
    if matches!(risk_type, "" | "none") {
        return Ok(None);
    }

    let confidence = output
        .get("confidence")
        .and_then(Value::as_str)
        .unwrap_or("low")
        .to_lowercase();
    if confidence == "low" {
        return Ok(None);
    }

    let summary = output
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("Passive system agent detected a potential security risk.");
    let signals = output
        .get("signals")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let payload = &event.payload;
    let request_id = payload.get("requestId").and_then(Value::as_i64);
    let proxy_request = match request_id {
        Some(request_id) => db.get_proxy_request_by_id(request_id).await?,
        None => None,
    };

    let url = proxy_request
        .as_ref()
        .map(|record| record.url.clone())
        .or_else(|| {
            payload
                .get("url")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string());
    let method = proxy_request
        .as_ref()
        .map(|record| record.method.clone())
        .or_else(|| {
            payload
                .get("method")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "GET".to_string());
    let location = payload
        .get("clusterKey")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            payload
                .get("pathTemplate")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| url.clone());

    let title = build_title(
        risk_type,
        &method,
        payload.get("pathTemplate").and_then(Value::as_str),
    );
    let description = if signals.is_empty() {
        summary.to_string()
    } else {
        format!("{summary}\n\nSignals:\n- {}", signals.join("\n- "))
    };

    let evidence = if signals.is_empty() {
        summary.to_string()
    } else {
        signals.join(" | ")
    };

    let (severity, confidence_label) = map_confidence(&confidence);
    let (cwe, owasp, remediation) = map_risk_metadata(risk_type);

    Ok(Some(TrafficFinding {
        id: format!("saf-{}", Uuid::new_v4()),
        plugin_id: format!("agent:{profile_id}"),
        vuln_type: risk_type.to_string(),
        severity: severity.to_string(),
        confidence: confidence_label.to_string(),
        title,
        description,
        cwe,
        owasp,
        remediation,
        url,
        method,
        location,
        evidence,
        request_headers: proxy_request
            .as_ref()
            .and_then(|record| record.request_headers.clone()),
        request_body: proxy_request
            .as_ref()
            .and_then(|record| record.request_body.clone()),
        response_status: proxy_request.as_ref().map(|record| record.status_code),
        response_headers: proxy_request
            .as_ref()
            .and_then(|record| record.response_headers.clone()),
        response_body: proxy_request
            .as_ref()
            .and_then(|record| record.response_body.clone()),
        created_at: Utc::now(),
    }))
}

fn build_title(risk_type: &str, method: &str, path: Option<&str>) -> String {
    let path = path.unwrap_or("/");
    match risk_type {
        "idor" | "bola" | "bfla" => format!(
            "[AI] Potential {} risk on {} {}",
            risk_type.to_uppercase(),
            method,
            path
        ),
        "logic" | "workflow" | "race" => format!(
            "[AI] Potential {} risk on {} {}",
            risk_type.to_uppercase(),
            method,
            path
        ),
        _ => format!("[AI] Potential {} risk on {} {}", risk_type, method, path),
    }
}

fn map_confidence(confidence: &str) -> (&'static str, &'static str) {
    match confidence {
        "high" => ("high", "High"),
        "medium" => ("medium", "Medium"),
        _ => ("low", "Low"),
    }
}

fn map_risk_metadata(risk_type: &str) -> (Option<String>, Option<String>, Option<String>) {
    match risk_type {
        "idor" | "bola" => (
            Some("CWE-639".to_string()),
            Some("API1:2023 Broken Object Level Authorization".to_string()),
            Some("在服务端强制校验对象归属关系，禁止仅依赖客户端传入的对象 ID。".to_string()),
        ),
        "bfla" => (
            Some("CWE-285".to_string()),
            Some("API5:2023 Broken Function Level Authorization".to_string()),
            Some("在服务端为每个敏感功能点增加角色/权限校验。".to_string()),
        ),
        "logic" | "workflow" => (
            Some("CWE-840".to_string()),
            Some("A04:2021 Insecure Design".to_string()),
            Some("补齐关键业务状态校验，阻止跳步骤、重复执行和逆序执行。".to_string()),
        ),
        "race" => (
            Some("CWE-362".to_string()),
            Some("A04:2021 Insecure Design".to_string()),
            Some("对关键状态更新增加幂等和并发保护，避免竞态条件。".to_string()),
        ),
        _ => (None, None, None),
    }
}
