use serde_json::{json, Value};

use sentinel_db::DatabaseService;

use crate::skills::{read_skill_markdown, skills_root};

const MAX_SKILL_CONTEXT_BODY_CHARS: usize = 1_600;
const MAX_SKILL_RECOMMENDATIONS: usize = 3;

pub fn build_logic_skill_context(db: &DatabaseService, payload: &Value) -> Value {
    let recommendations = payload
        .get("skillRecommendations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if recommendations.is_empty() {
        return Value::Array(vec![]);
    }

    let root = skills_root(db);
    let contexts = recommendations
        .into_iter()
        .take(MAX_SKILL_RECOMMENDATIONS)
        .filter_map(|item| build_skill_context_item(&root, &item))
        .collect::<Vec<_>>();

    Value::Array(contexts)
}

fn build_skill_context_item(root: &std::path::Path, item: &Value) -> Option<Value> {
    let skill_id = item.get("id").and_then(Value::as_str)?.to_string();
    let score = item.get("score").cloned().unwrap_or(Value::Null);
    let reasons = item
        .get("reasons")
        .cloned()
        .unwrap_or_else(|| Value::Array(vec![]));
    let skill_path = root.join(&skill_id).join("SKILL.md");

    let doc = read_skill_markdown(&skill_path).ok()?;
    Some(json!({
        "id": skill_id,
        "name": doc.frontmatter.name,
        "description": doc.frontmatter.description,
        "whenToUse": doc.frontmatter.when_to_use,
        "score": score,
        "reasons": reasons,
        "guidance": truncate_text(&doc.body),
    }))
}

fn truncate_text(input: &str) -> String {
    input.chars().take(MAX_SKILL_CONTEXT_BODY_CHARS).collect()
}
