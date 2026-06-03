use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::commands::tool_commands::skills::{create_skill_internal, CreateSkillRequest};
use crate::skills::candidates::{
    create_suppression_rule_from_candidate, delete_skill_candidate_suppression_rule,
    expire_skill_candidate_suppression_rule as expire_skill_candidate_suppression_rule_internal,
    extend_skill_candidate_suppression_rule as extend_skill_candidate_suppression_rule_internal,
    get_skill_candidate,
    list_skill_candidate_suppression_rules as list_skill_candidate_suppression_rules_internal,
    list_skill_candidates as list_skill_candidates_internal, mark_skill_candidate_promoted,
    update_skill_candidate_status, SkillCandidate, SkillCandidateSuppressionRule,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoteSkillCandidateRequest {
    pub candidate_id: String,
    pub skill_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSkillCandidateRequest {
    pub candidate_id: String,
    pub status: String,
    pub suppression_category: Option<String>,
    pub review_note: Option<String>,
}

fn default_rejection_review_note(category: &str) -> String {
    match category {
        "duplicate_noise" => "Rejected as duplicate low-signal draft noise",
        "stale_preference" => "Rejected as a stale or non-reusable preference draft",
        "overfit_procedure" => "Rejected as an overfit procedure that should stay contextual",
        "incorrect_pattern" => "Rejected as an incorrect or misleading reusable pattern",
        _ => "Rejected by SkillsManager review gate",
    }
    .to_string()
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn build_rejection_review_context(
    suppression_category: Option<String>,
    review_note: Option<String>,
) -> Result<(String, String), String> {
    let category = normalize_optional_text(suppression_category).ok_or_else(|| {
        "Rejected skill candidates must include a suppression category".to_string()
    })?;
    let note = normalize_optional_text(review_note)
        .unwrap_or_else(|| default_rejection_review_note(category.as_str()));
    Ok((category, note))
}

pub async fn list_skill_candidates(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<SkillCandidate>, String> {
    list_skill_candidates_internal(db_service.inner().as_ref()).map_err(|e| e.to_string())
}

pub async fn list_skill_candidate_suppression_rules(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<SkillCandidateSuppressionRule>, String> {
    list_skill_candidate_suppression_rules_internal(db_service.inner().as_ref())
        .map_err(|e| e.to_string())
}

pub async fn delete_skill_candidate_suppression_rule_by_id(
    rule_id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    delete_skill_candidate_suppression_rule(db_service.inner().as_ref(), rule_id.as_str())
        .map_err(|e| e.to_string())
}

pub async fn extend_skill_candidate_suppression_rule(
    rule_id: String,
    days: i64,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<SkillCandidateSuppressionRule, String> {
    extend_skill_candidate_suppression_rule_internal(
        db_service.inner().as_ref(),
        rule_id.as_str(),
        days,
    )
    .map_err(|e| e.to_string())
}

pub async fn expire_skill_candidate_suppression_rule(
    rule_id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<SkillCandidateSuppressionRule, String> {
    expire_skill_candidate_suppression_rule_internal(db_service.inner().as_ref(), rule_id.as_str())
        .map_err(|e| e.to_string())
}

pub async fn promote_skill_candidate(
    payload: PromoteSkillCandidateRequest,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<sentinel_db::Skill, String> {
    let candidate = get_skill_candidate(db_service.inner().as_ref(), payload.candidate_id.as_str())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Skill candidate not found".to_string())?;

    if candidate.status == "promoted" {
        return Err("Skill candidate has already been promoted".to_string());
    }

    let skill_name = payload
        .skill_name
        .clone()
        .unwrap_or_else(|| candidate.suggested_skill_name.clone());

    let skill = create_skill_internal(
        CreateSkillRequest {
            name: skill_name,
            description: candidate.description.clone(),
            content: candidate.content.clone(),
            argument_hint: format!("Derived from memory candidate {}", candidate.id),
            disable_model_invocation: false,
            user_invocable: true,
            allowed_tools: Vec::new(),
            model: String::new(),
            context: String::new(),
            agent: String::new(),
            hooks: Some(serde_json::json!({
                "memory_candidate_id": candidate.id,
                "memory_kind": candidate.memory_kind,
                "memory_scope": candidate.scope,
                "memory_source": candidate.source,
                "memory_confidence": candidate.confidence,
            })),
        },
        db_service.inner().as_ref(),
    )
    .await?;

    mark_skill_candidate_promoted(
        db_service.inner().as_ref(),
        payload.candidate_id.as_str(),
        skill.id.as_str(),
    )
    .map_err(|e| e.to_string())?;

    if let Err(error) = delete_skill_candidate_suppression_rule(
        db_service.inner().as_ref(),
        payload.candidate_id.as_str(),
    ) {
        tracing::warn!(
            candidate_id = %payload.candidate_id,
            "Failed to clear suppression rule after promotion: {}",
            error
        );
    }

    Ok(skill)
}

pub async fn review_skill_candidate(
    payload: ReviewSkillCandidateRequest,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<SkillCandidate, String> {
    let normalized = payload.status.trim().to_lowercase();
    if normalized != "rejected" && normalized != "archived" {
        return Err("Skill candidate status must be 'rejected' or 'archived'".to_string());
    }

    let candidate = get_skill_candidate(db_service.inner().as_ref(), payload.candidate_id.as_str())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Skill candidate not found".to_string())?;

    if candidate.status == "promoted" {
        return Err("Promoted skill candidates cannot be reviewed again".to_string());
    }

    let (suppression_category, review_note) = if normalized == "rejected" {
        let (category, note) = build_rejection_review_context(
            payload.suppression_category.clone(),
            payload.review_note.clone(),
        )?;
        (Some(category), Some(note))
    } else {
        (None, normalize_optional_text(payload.review_note.clone()))
    };

    update_skill_candidate_status(
        db_service.inner().as_ref(),
        payload.candidate_id.as_str(),
        normalized.as_str(),
        review_note.clone(),
        None,
    )
    .and_then(|candidate| {
        if normalized == "rejected" {
            create_suppression_rule_from_candidate(
                db_service.inner().as_ref(),
                &candidate,
                suppression_category.as_deref(),
                review_note,
            )?;
        }
        Ok(candidate)
    })
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{build_rejection_review_context, default_rejection_review_note};

    #[test]
    fn rejection_review_context_requires_category() {
        let result = build_rejection_review_context(None, None);
        assert!(result.is_err());
    }

    #[test]
    fn rejection_review_context_defaults_note_from_category() {
        let (category, note) =
            build_rejection_review_context(Some("duplicate_noise".to_string()), None).unwrap();
        assert_eq!(category, "duplicate_noise");
        assert_eq!(note, default_rejection_review_note("duplicate_noise"));
    }

    #[test]
    fn rejection_review_context_trims_user_note() {
        let (_, note) = build_rejection_review_context(
            Some("incorrect_pattern".to_string()),
            Some("  custom review note  ".to_string()),
        )
        .unwrap();
        assert_eq!(note, "custom review note");
    }
}
