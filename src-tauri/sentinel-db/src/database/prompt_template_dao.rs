use anyhow::Result;
use sqlx::sqlite::SqlitePool;

pub async fn insert_default_templates(pool: &SqlitePool) -> Result<()> {
    let defaults: &[(&str, &str, &str, &str)] = &[
        ("rewoo", "planner", "ReWOO Planner", "你是一个智能任务规划器。请根据用户的问题，制定一个详细的执行计划。\n\n用户问题：{user_question}\n\n请按照以下格式输出计划：\nPlan: 步骤描述\n#E1 = 工具名称[参数]\n#E2 = 工具名称[参数, #E1]"),
        ("rewoo", "worker", "ReWOO Worker", "你是一个工具执行器。请根据计划中的步骤，调用相应的工具并返回结果。\n当前步骤：{current_step}\n可用工具：{available_tools}"),
        ("rewoo", "solver", "ReWOO Solver", "你是一个结果整合器。请根据执行结果，为用户提供最终答案。\n原始问题：{original_question}\n执行计划：{plan}\n执行结果：{results}"),
        ("llmcompiler", "planning", "LLMC Planning", "你是一个并行任务规划器。请将复杂任务分解为可并行执行的子任务。\n用户任务：{user_task}"),
        ("llmcompiler", "execution", "LLMC Execution", "你是一个任务执行器。请执行指定的子任务。\n当前任务：{current_task}\n依赖结果：{dependencies}"),
        ("llmcompiler", "replan", "LLMC Replan", "你是一个重新规划器。当执行出现问题时，请调整执行计划。\n原始计划：{original_plan}\n执行状态：{execution_status}\n错误信息：{error_info}"),
        ("planexecute", "planning", "P&E Planning", "你是一个策略规划师。请为用户的目标制定详细的执行计划。\n用户目标：{user_goal}"),
        ("planexecute", "execution", "P&E Execution", "你是一个执行专家。请执行计划中的当前步骤。\n执行计划：{plan}\n当前步骤：{current_step}\n前置结果：{previous_results}"),
        ("planexecute", "replan", "P&E Replan", "你是一个重新规划师。请评估执行结果并在必要时调整计划。\n执行计划：{plan}\n执行结果：{results}\n目标达成情况：{goal_achievement}"),
    ];

    for (arch, stage, name, content) in defaults {
        sqlx::query(r#"INSERT INTO prompt_templates (name, description, architecture, stage, content, is_default, is_active) VALUES (?, ?, ?, ?, ?, 1, 1)"#)
            .bind(*name)
            .bind(Option::<&str>::None)
            .bind(*arch)
            .bind(*stage)
            .bind(*content)
            .execute(pool)
            .await?;
    }
    Ok(())
}


