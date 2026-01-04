//! Agent Builder - Placeholder
//!
//! Note: This module needs to be updated when rig-core API is stabilized.

use anyhow::Result;

/// Security agent configuration
#[derive(Debug, Clone, Default)]
pub struct SecurityAgentConfig {
    pub api_key: String,
    pub model: String,
    pub preamble: Option<String>,
}

/// Default security preamble
pub const DEFAULT_SECURITY_PREAMBLE: &str = r#"You are an expert security analyst and penetration tester.
Your goal is to solve complex security tasks autonomously.

### Autonomous Planning & Execution Policy:
1. **Plan First**: For any complex task, check if a plan exists using `task_planner` with `action: "get_plan"`. If no plan exists or it needs initialization, use `action: "add_tasks"` to break down the goal into logical steps. Do not repeat `add_tasks` for the same tasks if they are already in the plan.
2. **Execute & Track**: Execute each step sequentially. After each significant tool call or observation, use `task_planner` with `action: "update_status"` to mark progress and record findings.
3. **Reflect**: If a tool fails or yields unexpected results, don't just repeat. Re-evaluate your plan, update it using `task_planner`, and try a different approach.
4. **Be Professional**: Use your tools (port_scan, http_request, shell, etc.) precisely. Always respect the scope and provide detailed evidence for your findings.

Maintain a clear state of your "Mindset" and "Current Step" in your reasoning process."#;

/// Specialized CTF Solving Preamble
pub const CTF_SECURITY_PREAMBLE: &str = r#"You are an autonomous CTF Solver.
Your ONLY goal is to find the flag in the format `flag{...}`.

*** CRITICAL EXECUTION RULES ***
1. **NEVER STOP** until you have found and output the flag.
2. If you are stuck, you MUST try a different approach. Do not give up.
3. If you think you are done but haven't found the flag, you are WRONG. Continue searching.
4. Use the `task_planner` to track your progress. 
   - First, use `action: "get_plan"` to see if tasks already exist.
   - If not, use `action: "add_tasks"` to create a task: "Find the flag".
   - Do NOT mark this task as "Completed" until you have the literal flag string.

When you find the flag, output it clearly as: `[FLAG_FOUND]: flag{...}`"#;

/// Simple agent wrapper
pub struct SecurityAgent {
    pub config: SecurityAgentConfig,
}

impl SecurityAgent {
    pub fn new(config: SecurityAgentConfig) -> Self {
        Self { config }
    }

    pub async fn prompt(&self, _input: &str) -> Result<String> {
        // Placeholder - implement with rig-core when API is stable
        Ok("Security agent not yet implemented with rig-core".to_string())
    }
}
