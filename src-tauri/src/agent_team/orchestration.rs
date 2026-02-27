//! Team 编排计划（串行/并行）基础校验与规范化

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

const ALLOWED_STEP_TYPES: &[&str] = &["agent", "parallel", "serial"];

/// 规范化并校验编排计划。
///
/// 约束：
/// - 根对象必须包含 `steps: []`
/// - `steps` 至少 1 个
/// - 每个 step 必须有唯一 `id` 与合法 `type`
/// - `agent` step 需要 `member`
/// - `parallel|serial` step 需要非空 `children`
/// - 自动补全 `version`（默认 1）
pub fn normalize_and_validate_orchestration_plan(plan: Option<&Value>) -> Result<Option<Value>> {
    let Some(raw) = plan else {
        return Ok(None);
    };

    let mut normalized = raw.clone();
    let obj = normalized
        .as_object_mut()
        .ok_or_else(|| anyhow!("orchestration_plan must be a JSON object"))?;

    if obj.get("version").and_then(|v| v.as_i64()).is_none() {
        obj.insert("version".to_string(), json!(1));
    }

    let steps = obj
        .get("steps")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("orchestration_plan.steps must be an array"))?;
    if steps.is_empty() {
        return Err(anyhow!("orchestration_plan.steps cannot be empty"));
    }

    let mut seen_ids = std::collections::HashSet::new();
    for (index, step) in steps.iter().enumerate() {
        validate_step(step, &format!("orchestration_plan.steps[{}]", index), &mut seen_ids)?;
    }

    Ok(Some(normalized))
}

fn validate_step(
    step: &Value,
    path: &str,
    seen_ids: &mut std::collections::HashSet<String>,
) -> Result<()> {
    let step_obj = step
        .as_object()
        .ok_or_else(|| anyhow!("{} must be an object", path))?;

    let id = step_obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("{}.id is required", path))?;
    if !seen_ids.insert(id.to_string()) {
        return Err(anyhow!("orchestration_plan.steps contains duplicated id: {}", id));
    }

    let step_type = step_obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("{}.type is required", path))?;
    if !ALLOWED_STEP_TYPES.contains(&step_type) {
        return Err(anyhow!("{}.type '{}' is not supported", path, step_type));
    }

    match step_type {
        "agent" => {
            let _ = step_obj
                .get("member")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("{}.member is required", path))?;
        }
        "parallel" | "serial" => {
            let children = step_obj
                .get("children")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow!("{}.children is required", path))?;
            if children.is_empty() {
                return Err(anyhow!("{}.children cannot be empty", path));
            }
            for (idx, child) in children.iter().enumerate() {
                validate_step(child, &format!("{}.children[{}]", path, idx), seen_ids)?;
            }
        }
        _ => {}
    }

    Ok(())
}
