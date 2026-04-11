use super::team_v3_commands::{TeamV3MemberProfile, TeamV3Task};

pub(crate) fn build_team_member_execution_system_prompt(
    _member_id: &str,
    _profile: Option<&TeamV3MemberProfile>,
) -> String {
    let lines = vec![
        "你是 Team 成员，请以严谨、可执行的方式完成分配任务。".to_string(),
        "输出必须可追溯、可验证，避免空泛表述。".to_string(),
        "优先依据当前用户任务与共享上下文，不要编造未提供事实。\n".to_string(),
    ];
    lines.join("\n")
}

pub(crate) fn build_team_member_runtime_context(
    member_id: &str,
    profile: Option<&TeamV3MemberProfile>,
) -> String {
    let mut lines = vec![format!("成员 ID：{}", member_id)];
    if let Some(profile) = profile {
        if !profile.name.trim().is_empty() {
            lines.push(format!("成员名称：{}", profile.name.trim()));
        }
        if let Some(responsibility) = profile
            .responsibility
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("职责边界：{}", responsibility));
        }
        if let Some(decision_style) = profile
            .decision_style
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("决策风格：{}", decision_style));
        }
        if let Some(risk_preference) = profile
            .risk_preference
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("风险偏好：{}", risk_preference));
        }
        if let Some(system_prompt) = profile
            .system_prompt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("额外执行约束：{}", system_prompt));
        }
    }
    lines.join("\n")
}

pub(crate) fn build_team_v3_task_prompt(
    goal: &str,
    user_input: &str,
    task: &TeamV3Task,
    member_runtime_context: &str,
    dependency_context: &str,
    blackboard_context: &str,
    blackboard_snapshot_revision: i64,
    checkpoint_only: bool,
) -> String {
    let mut context_sections: Vec<String> = Vec::new();
    if !member_runtime_context.trim().is_empty() {
        context_sections.push(format!("成员运行时上下文：\n{}", member_runtime_context));
    }
    if !dependency_context.trim().is_empty() {
        context_sections.push(format!("依赖任务输出：\n{}", dependency_context));
    }
    if !blackboard_context.trim().is_empty() {
        context_sections.push(blackboard_context.to_string());
    }

    if context_sections.is_empty() {
        format!(
            "Team 总目标：{}\n用户输入：{}\nTeam 黑板快照版本：{}\n\n当前子任务：{}\n任务说明：{}\n\n请直接执行该子任务，并输出结构化结论（结论、依据、风险、下一步）。",
            goal,
            user_input,
            blackboard_snapshot_revision,
            task.title,
            task.instruction
        )
    } else if checkpoint_only {
        format!(
            "Team 总目标：{}\n用户输入：{}\nTeam 黑板快照版本：{}\n\n当前子任务：{}\n任务说明：{}\n\n{}\n\n你必须优先基于 Structured Memory（结构化记忆）完成收敛，必要时再参考 Task Output Evidence 与 Artifact 摘要。输出结构化结论（最终结论、关键证据、风险、下一步行动）。",
            goal,
            user_input,
            blackboard_snapshot_revision,
            task.title,
            task.instruction,
            context_sections.join("\n\n")
        )
    } else {
        format!(
            "Team 总目标：{}\n用户输入：{}\nTeam 黑板快照版本：{}\n\n当前子任务：{}\n任务说明：{}\n\n{}\n\n请优先基于 Structured Memory（结构化记忆）继续执行，必要时引用 Task Output Evidence 与 Artifact 摘要，并输出结构化结论（结论、依据、风险、下一步）。",
            goal,
            user_input,
            blackboard_snapshot_revision,
            task.title,
            task.instruction,
            context_sections.join("\n\n")
        )
    }
}

pub(crate) fn is_team_v3_summary_task(task: &TeamV3Task) -> bool {
    let key = task.task_key.to_lowercase();
    let title = task.title.to_lowercase();
    let instruction = task.instruction.to_lowercase();
    key.contains("summary")
        || key.contains("final")
        || title.contains("总结")
        || title.contains("汇总")
        || title.contains("summary")
        || instruction.contains("总结")
        || instruction.contains("汇总")
        || instruction.contains("synthes")
}

pub(crate) fn build_team_v3_planner_prompt(
    goal: &str,
    user_input: &str,
    member_catalog: &[String],
    blackboard_context: &str,
) -> String {
    let mut sections = vec![
        format!("Team 目标：{}", goal),
        format!("用户输入：{}", user_input),
        format!(
            "当前成员（可复用、可重命名、可新增）：\n{}",
            if member_catalog.is_empty() {
                "- 暂无预设成员，请按任务需要自行设计成员".to_string()
            } else {
                member_catalog.join("\n")
            }
        ),
    ];
    if !blackboard_context.trim().is_empty() {
        sections.push(blackboard_context.to_string());
    }
    sections.push(
        "请基于当前目标生成 Team 执行计划，并仅输出合法 JSON 不要输出 Markdown、解释文字或代码块围栏。字段要求：\
        summary: 简短拆解说明；\
        agents: 成员数组（每个成员包含 id/name/responsibility/system_prompt/decision_style/risk_preference/weight）；\
        id: 稳定成员标识，建议使用小写英文和连字符；\
        name: 成员显示名；\
        responsibility/system_prompt/decision_style/risk_preference: 成员角色信息；\
        weight: 成员权重（可选）；\
        tasks: 任务数组；\
        task_key: 英文短键；\
        title: 人类可读标题；\
        instruction: 可直接执行的指令；\
        depends_on: 依赖 task_key 数组；\
        owner_agent_id: 必须引用 agents.id；\
        priority: 数字，越小越先执行。"
            .to_string(),
    );
    sections.join("\n\n")
}

pub(crate) fn build_team_v3_planner_system_prompt(main_agent_id: &str) -> String {
    format!(
        r#"你是 Team 主调度代理 {main_agent_id}，负责把用户目标分解为可执行任务图并定义执行成员。

输出规则：
1) 仅输出 JSON，不要输出 Markdown、解释文字或代码块围栏。
2) JSON 结构必须为：{{"summary":"...","agents":[{{...}}],"tasks":[{{...}}]}}
3) agents 数量 1-8；tasks 数量 2-12。
4) task.owner_agent_id 必须严格使用 agents.id，不得引用未定义成员。
5) 能并行的任务不要相互依赖；必须有至少一个收敛任务依赖关键前置任务。
6) 如果输入已有成员，可复用其 id；也可按任务需要新增成员。

Few-shot 示例 1（双成员并行后收敛）：
输入目标：分析仓库并给出改造建议。
输出：
{{"summary":"双线分析后统一汇总","agents":[
  {{"id":"product-analyst","name":"产品分析师","responsibility":"聚焦用户价值与业务目标","system_prompt":"优先解释用户场景、价值与优先级。","decision_style":"evidence-first","risk_preference":"balanced","weight":1.0}},
  {{"id":"architecture-analyst","name":"架构分析师","responsibility":"评估实现路径与扩展性","system_prompt":"优先分析模块边界、依赖与扩展成本。","decision_style":"structured","risk_preference":"conservative","weight":1.0}}
],"tasks":[
  {{"task_key":"analyze-product","title":"分析产品价值","instruction":"提炼目标用户、核心价值与关键用例。","depends_on":[],"owner_agent_id":"product-analyst","priority":10}},
  {{"task_key":"analyze-architecture","title":"分析技术架构","instruction":"识别架构模式、关键模块与技术风险。","depends_on":[],"owner_agent_id":"architecture-analyst","priority":10}},
  {{"task_key":"deliver-summary","title":"汇总结论","instruction":"合并前置分析并输出行动建议。","depends_on":["analyze-product","analyze-architecture"],"owner_agent_id":"product-analyst","priority":30}}
]}}

Few-shot 示例 2（三成员串行链路）：
输入目标：排查线上故障并给出修复方案。
输出：
{{"summary":"先定位根因，再修复并验证","agents":[
  {{"id":"incident-investigator","name":"故障定位","responsibility":"定位根因与影响范围","system_prompt":"强调证据链完整性。","decision_style":"diagnostic","risk_preference":"balanced","weight":1.0}},
  {{"id":"fix-designer","name":"修复设计","responsibility":"制定修复与回滚策略","system_prompt":"优先最小化风险与变更面。","decision_style":"structured","risk_preference":"conservative","weight":1.0}},
  {{"id":"verifier","name":"验证负责人","responsibility":"设计验证与观测方案","system_prompt":"强调可观测性与验收标准。","decision_style":"checklist","risk_preference":"conservative","weight":1.0}}
],"tasks":[
  {{"task_key":"locate-root-cause","title":"定位根因","instruction":"分析现象、日志与变更定位问题根因。","depends_on":[],"owner_agent_id":"incident-investigator","priority":10}},
  {{"task_key":"propose-fix","title":"制定修复方案","instruction":"基于根因给出可执行修复方案和回滚策略。","depends_on":["locate-root-cause"],"owner_agent_id":"fix-designer","priority":20}},
  {{"task_key":"verify-fix","title":"设计验证步骤","instruction":"制定验证清单和观测指标，确认修复有效。","depends_on":["propose-fix"],"owner_agent_id":"verifier","priority":30}}
]}}"#
    )
}
