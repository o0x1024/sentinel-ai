//! Bug bounty report export commands.

use super::bounty_commands::ensure_bounty_feature;
use chrono::Utc;
use sentinel_db::DatabaseService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

// ============================================================================
// Export Report - One-click Submission Package
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReportRequest {
    pub finding_ids: Vec<String>,
    pub format: String,   // "markdown", "json", "html"
    pub language: String, // "en", "zh"
    pub include_evidence: bool,
    pub template_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReportResponse {
    pub filename: String,
    pub content: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingExportData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub finding_type: String,
    pub affected_url: Option<String>,
    pub affected_parameter: Option<String>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub evidence: Vec<EvidenceExportData>,
    pub cwe_id: Option<String>,
    pub cvss_score: Option<f32>,
    pub tags: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceExportData {
    pub id: String,
    pub evidence_type: String,
    pub title: String,
    pub description: Option<String>,
    pub content: Option<String>,
    pub http_request: Option<serde_json::Value>,
    pub http_response: Option<serde_json::Value>,
}

/// Export findings as a submission report
#[tauri::command]
pub async fn bounty_export_report(
    db_service: State<'_, Arc<DatabaseService>>,
    request: ExportReportRequest,
) -> Result<ExportReportResponse, String> {
    ensure_bounty_feature()?;

    let mut findings_data: Vec<FindingExportData> = Vec::new();

    for finding_id in &request.finding_ids {
        let finding = db_service
            .get_bounty_finding(finding_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Finding {} not found", finding_id))?;

        let mut evidence_data = Vec::new();
        if request.include_evidence {
            let evidences = db_service
                .list_bounty_evidence(finding_id)
                .await
                .map_err(|e| e.to_string())?;

            for ev in evidences {
                evidence_data.push(EvidenceExportData {
                    id: ev.id,
                    evidence_type: ev.evidence_type,
                    title: ev.title,
                    description: ev.description,
                    content: ev.content,
                    http_request: ev
                        .http_request_json
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    http_response: ev
                        .http_response_json
                        .and_then(|s| serde_json::from_str(&s).ok()),
                });
            }
        }

        let reproduction_steps: Option<Vec<String>> = finding
            .reproduction_steps_json
            .and_then(|s| serde_json::from_str(&s).ok());

        let tags: Vec<String> = finding
            .tags_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        findings_data.push(FindingExportData {
            id: finding.id,
            title: finding.title,
            description: finding.description,
            severity: finding.severity,
            finding_type: finding.finding_type,
            affected_url: finding.affected_url,
            affected_parameter: finding.affected_parameter,
            impact: finding.impact,
            remediation: finding.remediation,
            reproduction_steps,
            evidence: evidence_data,
            cwe_id: finding.cwe_id,
            cvss_score: finding.cvss_score.map(|s| s as f32),
            tags,
            created_at: finding.created_at,
        });
    }

    let (content, mime_type, ext) = match request.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&findings_data).map_err(|e| e.to_string())?;
            (json, "application/json", "json")
        }
        "html" => {
            let html = generate_html_report(&findings_data, &request.language);
            (html, "text/html", "html")
        }
        _ => {
            // Default to markdown
            let md = generate_markdown_report(&findings_data, &request.language);
            (md, "text/markdown", "md")
        }
    };

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("bounty_report_{}.{}", timestamp, ext);

    Ok(ExportReportResponse {
        filename,
        content,
        mime_type: mime_type.to_string(),
    })
}

/// Generate markdown report
fn generate_markdown_report(findings: &[FindingExportData], language: &str) -> String {
    let mut md = String::new();
    let is_zh = language == "zh";

    md.push_str(if is_zh {
        "# 漏洞报告\n\n"
    } else {
        "# Vulnerability Report\n\n"
    });
    md.push_str(&format!(
        "{}: {}\n\n",
        if is_zh { "生成时间" } else { "Generated" },
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    md.push_str(if is_zh {
        "## 概要\n\n"
    } else {
        "## Summary\n\n"
    });
    let severity_counts = count_severities(findings);
    md.push_str(&format!(
        "- {}: {}\n",
        if is_zh {
            "发现总数"
        } else {
            "Total Findings"
        },
        findings.len()
    ));
    for (sev, count) in &severity_counts {
        md.push_str(&format!("- {}: {}\n", sev.to_uppercase(), count));
    }
    md.push_str("\n---\n\n");

    for (i, finding) in findings.iter().enumerate() {
        md.push_str(&format!(
            "## {}. {} [{}]\n\n",
            i + 1,
            finding.title,
            finding.severity.to_uppercase()
        ));

        md.push_str(&format!(
            "**{}**: {}\n\n",
            if is_zh { "漏洞类型" } else { "Type" },
            finding.finding_type
        ));

        if let Some(ref url) = finding.affected_url {
            md.push_str(&format!(
                "**{}**: `{}`\n\n",
                if is_zh { "影响URL" } else { "Affected URL" },
                url
            ));
        }

        if let Some(ref cwe) = finding.cwe_id {
            md.push_str(&format!("**CWE**: {}\n\n", cwe));
        }

        md.push_str(&format!(
            "### {}\n\n{}\n\n",
            if is_zh { "描述" } else { "Description" },
            finding.description
        ));

        if let Some(ref impact) = finding.impact {
            md.push_str(&format!(
                "### {}\n\n{}\n\n",
                if is_zh { "影响" } else { "Impact" },
                impact
            ));
        }

        if let Some(ref steps) = finding.reproduction_steps {
            md.push_str(&format!(
                "### {}\n\n",
                if is_zh {
                    "复现步骤"
                } else {
                    "Reproduction Steps"
                }
            ));
            for step in steps {
                md.push_str(step);
                md.push_str("\n\n");
            }
        }

        if !finding.evidence.is_empty() {
            md.push_str(&format!(
                "### {}\n\n",
                if is_zh { "证据" } else { "Evidence" }
            ));
            for ev in &finding.evidence {
                md.push_str(&format!("#### {}\n\n", ev.title));
                if let Some(ref content) = ev.content {
                    md.push_str(&format!("```\n{}\n```\n\n", content));
                }
            }
        }

        if let Some(ref remediation) = finding.remediation {
            md.push_str(&format!(
                "### {}\n\n{}\n\n",
                if is_zh { "修复建议" } else { "Remediation" },
                remediation
            ));
        }

        md.push_str("\n---\n\n");
    }

    md
}

/// Generate HTML report
fn generate_html_report(findings: &[FindingExportData], language: &str) -> String {
    let is_zh = language == "zh";
    let title = if is_zh {
        "漏洞报告"
    } else {
        "Vulnerability Report"
    };

    let mut html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 900px; margin: 0 auto; padding: 20px; }}
        h1 {{ color: #333; border-bottom: 2px solid #333; padding-bottom: 10px; }}
        h2 {{ color: #444; margin-top: 30px; }}
        .finding {{ background: #f9f9f9; border-radius: 8px; padding: 20px; margin: 20px 0; }}
        .severity {{ display: inline-block; padding: 4px 12px; border-radius: 4px; font-weight: bold; color: white; }}
        .critical {{ background: #dc3545; }}
        .high {{ background: #fd7e14; }}
        .medium {{ background: #ffc107; color: #333; }}
        .low {{ background: #28a745; }}
        pre {{ background: #2d2d2d; color: #f8f8f2; padding: 15px; border-radius: 5px; overflow-x: auto; }}
        code {{ background: #e9ecef; padding: 2px 6px; border-radius: 3px; }}
    </style>
</head>
<body>
    <h1>{}</h1>
    <p>{}: {}</p>
"#,
        title,
        title,
        if is_zh { "生成时间" } else { "Generated" },
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    for (i, finding) in findings.iter().enumerate() {
        html.push_str(&format!(
            r#"<div class="finding">
            <h2>{}. {} <span class="severity {}">{}</span></h2>
            <p><strong>{}</strong>: {}</p>
            <h3>{}</h3><p>{}</p>
"#,
            i + 1,
            html_escape(&finding.title),
            finding.severity.to_lowercase(),
            finding.severity.to_uppercase(),
            if is_zh { "类型" } else { "Type" },
            html_escape(&finding.finding_type),
            if is_zh { "描述" } else { "Description" },
            html_escape(&finding.description)
        ));

        if let Some(ref remediation) = finding.remediation {
            html.push_str(&format!(
                "<h3>{}</h3><p>{}</p>",
                if is_zh { "修复建议" } else { "Remediation" },
                html_escape(remediation)
            ));
        }

        html.push_str("</div>");
    }

    html.push_str("</body></html>");
    html
}

fn count_severities(findings: &[FindingExportData]) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for f in findings {
        *counts.entry(f.severity.to_lowercase()).or_insert(0) += 1;
    }
    let order = ["critical", "high", "medium", "low", "info"];
    let mut result = Vec::new();
    for sev in order {
        if let Some(&count) = counts.get(sev) {
            result.push((sev.to_string(), count));
        }
    }
    result
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
