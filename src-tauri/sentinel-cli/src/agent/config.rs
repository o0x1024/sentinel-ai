use crate::args::AgentRuntimeArgs;
use crate::solver::SolveFilters;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRuntimeConfig {
    pub allow_hint: bool,
    pub stop_when_done: bool,
    pub max_requests_per_target: usize,
    pub max_level: Option<i32>,
    pub difficulties: Vec<String>,
    pub include_solved: bool,
    pub limit: Option<usize>,
    pub max_concurrent_challenges: usize,
    pub max_attempts_per_challenge: usize,
    pub failure_cooldown_secs: u64,
    pub loop_interval_secs: u64,
    pub heartbeat_interval_secs: u64,
    pub heartbeat_timeout_secs: u64,
    pub restart_delay_secs: u64,
    pub max_steps_per_challenge: usize,
    pub llm_timeout_secs: u64,
    pub max_challenge_duration_secs: u64,
}

impl AgentRuntimeConfig {
    pub fn to_supervisor_args(&self) -> Vec<String> {
        self.to_cli_args("run")
    }

    pub fn solve_filters(&self) -> SolveFilters {
        SolveFilters {
            max_level: self.max_level,
            difficulties: self.difficulties.clone(),
            include_solved: self.include_solved,
            limit: self.limit,
        }
    }

    pub fn to_worker_args(&self) -> Vec<String> {
        self.to_cli_args("worker")
    }

    pub fn load_llm_config(&self, execution_id: &str) -> Result<ContestLlmConfig> {
        let provider = required_env("SENTINEL_LLM_PROVIDER")?;
        let model = required_env("SENTINEL_LLM_MODEL")?;
        Ok(ContestLlmConfig {
            provider: provider.clone(),
            rig_provider: std::env::var("SENTINEL_LLM_RIG_PROVIDER")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(provider),
            model,
            base_url: optional_env("SENTINEL_LLM_BASE_URL"),
            api_key: optional_env("SENTINEL_LLM_API_KEY"),
            timeout_secs: self.llm_timeout_secs,
            max_turns: self.max_steps_per_challenge,
            execution_id: execution_id.to_string(),
        })
    }

    fn to_cli_args(&self, mode: &str) -> Vec<String> {
        let mut args = vec!["agent".to_string(), mode.to_string()];
        if self.allow_hint {
            args.push("--allow-hint".to_string());
        }
        if !self.stop_when_done {
            args.push("--stop-when-done=false".to_string());
        }
        args.push(format!(
            "--max-requests-per-target={}",
            self.max_requests_per_target
        ));
        if let Some(max_level) = self.max_level {
            args.push(format!("--max-level={max_level}"));
        }
        for difficulty in &self.difficulties {
            args.push("--difficulty".to_string());
            args.push(difficulty.clone());
        }
        if self.include_solved {
            args.push("--include-solved".to_string());
        }
        if let Some(limit) = self.limit {
            args.push(format!("--limit={limit}"));
        }
        args.push(format!(
            "--max-concurrent-challenges={}",
            self.max_concurrent_challenges
        ));
        args.push(format!(
            "--max-attempts-per-challenge={}",
            self.max_attempts_per_challenge
        ));
        args.push(format!(
            "--failure-cooldown-secs={}",
            self.failure_cooldown_secs
        ));
        args.push(format!("--loop-interval-secs={}", self.loop_interval_secs));
        args.push(format!(
            "--heartbeat-interval-secs={}",
            self.heartbeat_interval_secs
        ));
        args.push(format!(
            "--heartbeat-timeout-secs={}",
            self.heartbeat_timeout_secs
        ));
        args.push(format!("--restart-delay-secs={}", self.restart_delay_secs));
        args.push(format!(
            "--max-steps-per-challenge={}",
            self.max_steps_per_challenge
        ));
        args.push(format!("--llm-timeout-secs={}", self.llm_timeout_secs));
        args.push(format!(
            "--max-challenge-duration-secs={}",
            self.max_challenge_duration_secs
        ));
        args
    }
}

impl From<AgentRuntimeArgs> for AgentRuntimeConfig {
    fn from(value: AgentRuntimeArgs) -> Self {
        Self {
            allow_hint: value.allow_hint,
            stop_when_done: value.stop_when_done,
            max_requests_per_target: value.max_requests_per_target,
            max_level: value.max_level,
            difficulties: value.difficulties,
            include_solved: value.include_solved,
            limit: value.limit,
            max_concurrent_challenges: value.max_concurrent_challenges.max(1),
            max_attempts_per_challenge: value.max_attempts_per_challenge.max(1),
            failure_cooldown_secs: value.failure_cooldown_secs.max(30),
            loop_interval_secs: value.loop_interval_secs,
            heartbeat_interval_secs: value.heartbeat_interval_secs,
            heartbeat_timeout_secs: value.heartbeat_timeout_secs,
            restart_delay_secs: value.restart_delay_secs,
            max_steps_per_challenge: value.max_steps_per_challenge,
            llm_timeout_secs: value.llm_timeout_secs,
            max_challenge_duration_secs: value.max_challenge_duration_secs,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestLlmConfig {
    pub provider: String,
    pub rig_provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub timeout_secs: u64,
    pub max_turns: usize,
    pub execution_id: String,
}

fn required_env(key: &str) -> Result<String> {
    optional_env(key).ok_or_else(|| anyhow!("required env var missing: {}", key))
}

fn optional_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
