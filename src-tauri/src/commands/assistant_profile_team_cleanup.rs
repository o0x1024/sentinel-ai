use std::collections::HashSet;

use crate::commands::assistant_profile_commands::{AssistantProfilePayload, TeamProfilePayload};

pub(crate) fn prune_team_profiles_for_assistant_profiles(
    team_profiles: Vec<TeamProfilePayload>,
    assistant_profiles: &[AssistantProfilePayload],
) -> (Vec<TeamProfilePayload>, bool) {
    let assistant_ids = assistant_profiles
        .iter()
        .map(|profile| profile.id.clone())
        .collect::<HashSet<_>>();
    let mut changed = false;
    let mut pruned_profiles = Vec::new();

    for mut profile in team_profiles {
        let specialist_count_before = profile.specialist_profile_ids.len();
        profile
            .specialist_profile_ids
            .retain(|profile_id| assistant_ids.contains(profile_id));
        changed = changed || profile.specialist_profile_ids.len() != specialist_count_before;

        let has_required_members = assistant_ids.contains(&profile.orchestrator_profile_id)
            && assistant_ids.contains(&profile.monitor_profile_id)
            && !profile.specialist_profile_ids.is_empty();
        if has_required_members {
            pruned_profiles.push(profile);
        } else {
            changed = true;
        }
    }

    (pruned_profiles, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assistant(id: &str) -> AssistantProfilePayload {
        AssistantProfilePayload {
            id: id.to_string(),
            label: id.to_string(),
            description: String::new(),
            team_role: "assistant".to_string(),
            default_model: None,
            default_rag_enabled: false,
            default_web_search_enabled: false,
            default_tools_enabled: false,
            default_tenth_man_enabled: false,
            default_tool_selection_strategy: "Keyword".to_string(),
            default_max_tools: 1,
            default_harness_max_continuations: 6,
            default_preselected_tools: vec![],
            default_disabled_tools: vec![],
            default_manual_tools: vec![],
            default_team_orchestration_preset_id: None,
            default_team_recovery_preset_id: None,
            default_team_profile_id: None,
            context_mode: "claude-like".to_string(),
            run_mode: "assistant".to_string(),
        }
    }

    fn team(
        id: &str,
        orchestrator: &str,
        specialists: Vec<&str>,
        monitor: &str,
    ) -> TeamProfilePayload {
        TeamProfilePayload {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            orchestrator_profile_id: orchestrator.to_string(),
            specialist_profile_ids: specialists.into_iter().map(str::to_string).collect(),
            monitor_profile_id: monitor.to_string(),
            default_model: None,
            default_team_orchestration_preset_id: None,
            default_team_recovery_preset_id: None,
            context_mode: "claude-like".to_string(),
            memory_policy: serde_json::json!({}),
            tool_policy_matrix: serde_json::json!({}),
            harness_policy: serde_json::json!({}),
            concurrency_policy: serde_json::json!({}),
            safety_policy: serde_json::json!({}),
        }
    }

    #[test]
    fn prunes_deleted_specialists_and_removes_invalid_teams() {
        let assistants = vec![
            assistant("lead"),
            assistant("specialist-a"),
            assistant("monitor"),
        ];
        let teams = vec![
            team(
                "keep",
                "lead",
                vec!["specialist-a", "deleted-specialist"],
                "monitor",
            ),
            team(
                "drop-orchestrator",
                "deleted-lead",
                vec!["specialist-a"],
                "monitor",
            ),
            team(
                "drop-empty-specialists",
                "lead",
                vec!["deleted-specialist"],
                "monitor",
            ),
        ];

        let (pruned, changed) = prune_team_profiles_for_assistant_profiles(teams, &assistants);

        assert!(changed);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0].id, "keep");
        assert_eq!(pruned[0].specialist_profile_ids, vec!["specialist-a"]);
    }
}
