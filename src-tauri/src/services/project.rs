use crate::models::database::{BountyProject, Submission};
use anyhow::Result;
use uuid::Uuid;
use std::collections::HashMap;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::Utc;

use crate::services::database::DatabaseService;

/// 项目推荐算法参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationParams {
    pub min_reward: Option<f64>,
    pub max_difficulty: Option<i32>,
    pub preferred_platforms: Option<Vec<String>>,
    pub skill_tags: Option<Vec<String>>,
    pub time_investment: Option<String>, // "low", "medium", "high"
    pub risk_tolerance: Option<String>, // "conservative", "moderate", "aggressive"
}

/// 项目分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAnalysis {
    pub project_id: String,
    pub roi_score: f64,
    pub difficulty_assessment: i32,
    pub competition_level: i32,
    pub success_probability: f64,
    pub estimated_time_hours: Option<i32>,
    pub recommended_tools: Vec<String>,
    pub risk_factors: Vec<String>,
    pub opportunities: Vec<String>,
}

/// 项目统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStats {
    pub total_projects: u64,
    pub active_projects: u64,
    pub completed_projects: u64,
    pub average_roi: f64,
    pub platform_distribution: HashMap<String, u64>,
    pub difficulty_distribution: HashMap<i32, u64>,
}

/// 项目服务
pub struct ProjectService {
    projects: RwLock<HashMap<String, BountyProject>>,
    submissions: RwLock<HashMap<String, Submission>>,
}

impl ProjectService {
    pub fn new() -> Self {
        Self {
            projects: RwLock::new(HashMap::new()),
            submissions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn list_projects(&self, _platform: Option<String>) -> Result<Vec<BountyProject>> {
        let projects = self.projects.read().await;
        Ok(projects.values().cloned().collect())
    }

    pub async fn get_project(&self, project_id: String) -> Result<BountyProject> {
        let projects = self.projects.read().await;
        projects.get(&project_id).cloned()
            .ok_or_else(|| anyhow::anyhow!("Project not found"))
    }

    pub async fn get_recommendations(&self, _limit: u32) -> Result<Vec<BountyProject>> {
        let projects = self.projects.read().await;
        Ok(projects.values().take(5).cloned().collect())
    }

    pub async fn calculate_roi(&self, _project_id: String) -> Result<f64> {
        Ok(0.75) // 模拟ROI计算
    }

    pub async fn create_submission(&self, project_id: String, vulnerability_id: String, description: String) -> Result<Submission> {
        let submission = Submission {
            id: Uuid::new_v4().to_string(),
            vulnerability_id,
            project_id,
            platform: "hackerone".to_string(),
            submission_id: None,
            title: "新漏洞提交".to_string(),
            description: Some(description),
            severity: "Medium".to_string(),
            status: "draft".to_string(),
            reward_amount: None,
            bonus_amount: None,
            currency: "USD".to_string(),
            submitted_at: chrono::Utc::now(),
            triaged_at: None,
            resolved_at: None,
            feedback: None,
            response_time: None,
            resolution_time: None,
            collaborators: None,
            attachments: None,
            notes: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let mut submissions = self.submissions.write().await;
        let submission_id = submission.id.clone();
        submissions.insert(submission_id, submission.clone());

        Ok(submission)
    }

    pub async fn list_submissions(&self, _project_id: Option<String>) -> Result<Vec<Submission>> {
        let submissions = self.submissions.read().await;
        Ok(submissions.values().cloned().collect())
    }

    pub async fn update_submission_status(&self, submission_id: String, _status: String, _reward: Option<f64>) -> Result<()> {
        let submissions = self.submissions.read().await;
        if submissions.contains_key(&submission_id) {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Submission not found"))
        }
    }

    pub async fn get_earnings_statistics(&self) -> Result<ProjectStats> {
        Ok(ProjectStats {
            total_projects: 0,
            active_projects: 0,
            completed_projects: 0,
            average_roi: 0.0,
            platform_distribution: HashMap::new(),
            difficulty_distribution: HashMap::new(),
        })
    }

    /// 分析项目并生成推荐评分
    pub async fn analyze_project(&self, project: &BountyProject) -> Result<ProjectAnalysis> {
        // 基础ROI计算
        let base_roi = self.calculate_base_roi(project);
        
        // 难度评估
        let difficulty = self.assess_difficulty(project);
        
        // 竞争程度分析
        let competition = self.analyze_competition(project);
        
        // 成功概率估算
        let success_prob = self.estimate_success_probability(project, difficulty, competition);
        
        // 推荐工具
        let recommended_tools = self.recommend_tools(project);
        
        // 风险因素识别
        let risk_factors = self.identify_risk_factors(project);
        
        // 机会点识别
        let opportunities = self.identify_opportunities(project);
        
        // 时间估算
        let estimated_time = self.estimate_time_investment(project, difficulty);

        Ok(ProjectAnalysis {
            project_id: project.id.clone(),
            roi_score: base_roi,
            difficulty_assessment: difficulty,
            competition_level: competition,
            success_probability: success_prob,
            estimated_time_hours: estimated_time,
            recommended_tools,
            risk_factors,
            opportunities,
        })
    }

    /// 根据参数推荐项目
    pub async fn recommend_projects(
        &self,
        database: &DatabaseService,
        params: &RecommendationParams,
        limit: Option<usize>
    ) -> Result<Vec<(BountyProject, ProjectAnalysis)>> {
        // 获取所有活跃项目
        let projects = database.get_projects().await?;
        
        let mut recommendations = Vec::new();
        
        for project in projects {
            // 基础筛选
            if !self.matches_criteria(&project, params) {
                continue;
            }
            
            // 分析项目
            let analysis = self.analyze_project(&project).await?;
            
            recommendations.push((project, analysis));
        }
        
        // 按ROI评分排序
        recommendations.sort_by(|a, b| b.1.roi_score.partial_cmp(&a.1.roi_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // 限制返回数量
        if let Some(limit) = limit {
            recommendations.truncate(limit);
        }
        
        Ok(recommendations)
    }

    /// 获取项目统计信息
    pub async fn get_project_statistics(&self, database: &DatabaseService) -> Result<ProjectStats> {
        let projects = database.get_projects().await?;
        
        let total_projects = projects.len() as u64;
        let active_projects = projects.iter().filter(|p| p.status == "active").count() as u64;
        let completed_projects = projects.iter().filter(|p| p.status == "completed").count() as u64;
        
        let total_roi: f64 = projects.iter().map(|p| p.roi_score).sum();
        let average_roi = if total_projects > 0 { total_roi / total_projects as f64 } else { 0.0 };
        
        let mut platform_distribution = HashMap::new();
        let mut difficulty_distribution = HashMap::new();
        
        for project in &projects {
            *platform_distribution.entry(project.platform.clone()).or_insert(0) += 1;
            *difficulty_distribution.entry(project.difficulty_level).or_insert(0) += 1;
        }
        
        Ok(ProjectStats {
            total_projects,
            active_projects,
            completed_projects,
            average_roi,
            platform_distribution,
            difficulty_distribution,
        })
    }

    /// 更新项目ROI评分
    pub async fn update_project_roi(&self, database: &DatabaseService, project_id: &str) -> Result<f64> {
        if let Ok(project) = database.get_project(project_id).await {
            let roi = self.calculate_base_roi(&project);
            // 这里应该更新数据库中的ROI评分
            // database.update_project_roi(project_id, roi).await?;
            Ok(roi)
        } else {
            Err(anyhow::anyhow!("Project not found"))
        }
    }

    /// 批量更新所有项目的ROI评分
    pub async fn batch_update_roi(&self, database: &DatabaseService) -> Result<u64> {
        let projects = database.get_projects().await?;
        let mut updated_count = 0;
        
        for project in projects {
            if let Ok(_) = self.update_project_roi(database, &project.id).await {
                updated_count += 1;
            }
        }
        
        Ok(updated_count)
    }

    // 私有辅助方法
    fn calculate_base_roi(&self, project: &BountyProject) -> f64 {
        let mut roi = 50.0; // 基础分数

        // 奖励范围影响 (假设reward_range是JSON字符串)
        if let Some(reward_str) = &project.reward_range {
            if let Ok(reward_data) = serde_json::from_str::<serde_json::Value>(reward_str) {
                if let Some(max_reward) = reward_data.get("max").and_then(|v| v.as_f64()) {
                    roi += (max_reward / 1000.0).min(30.0); // 最多加30分
                }
            }
        }

        // 成功率影响
        roi += project.success_rate * 20.0; // 最多加20分

        // 竞争程度影响（竞争越激烈分数越低）
        roi -= (project.competition_level as f64 - 1.0) * 5.0;

        // 难度影响（难度越高分数越低，但高难度项目竞争较少）
        roi -= (project.difficulty_level as f64 - 1.0) * 3.0; // 难度越高分数越低

        roi.max(0.0).min(100.0)
    }

    fn assess_difficulty(&self, project: &BountyProject) -> i32 {
        let mut difficulty = project.difficulty_level;

        // 根据域名数量调整难度
        if let Some(scope_str) = &project.scope_domains {
            if let Ok(domains) = serde_json::from_str::<Vec<String>>(scope_str) {
                if domains.len() > 10 {
                    difficulty += 1;
                }
            }
        }

        difficulty.min(5)
    }

    fn analyze_competition(&self, project: &BountyProject) -> i32 {
        let mut competition: f32 = 1.0;

        // 根据平台调整竞争程度
        match project.platform.as_str() {
            "hackerone" => competition = (competition + 1.0).min(5.0),
            "bugcrowd" => competition = (competition + 1.0).min(5.0),
            _ => competition = competition.max(1.0),
        }

        competition as i32
    }

    fn estimate_success_probability(&self, project: &BountyProject, difficulty: i32, competition: i32) -> f64 {
        let base_prob = 0.3; // 基础成功概率30%

        // 历史成功率影响
        let history_factor = 1.0 + project.success_rate;

        // 难度和竞争影响
        let difficulty_factor = 1.0 - (difficulty as f64 * 0.1);
        let competition_factor = 1.0 - (competition as f64 * 0.1);

        (base_prob * history_factor * difficulty_factor * competition_factor).min(1.0).max(0.01)
    }

    fn recommend_tools(&self, project: &BountyProject) -> Vec<String> {
        let mut tools = vec!["nmap".to_string(), "subfinder".to_string()];

        // 根据项目域名推荐工具
        if let Some(scope_str) = &project.scope_domains {
            if scope_str.contains("web") || scope_str.contains("http") {
                tools.push("httpx".to_string());
                tools.push("nuclei".to_string());
            }
        }

        // 根据平台推荐工具
        match project.platform.as_str() {
            "hackerone" => tools.push("h1-cli".to_string()),
            "bugcrowd" => tools.push("crowdfire".to_string()),
            _ => {}
        }

        tools
    }

    fn identify_risk_factors(&self, project: &BountyProject) -> Vec<String> {
        let mut risks = Vec::new();

        // 高竞争风险
        if project.competition_level >= 4 {
            risks.push("High competition environment".to_string());
        }

        // 高难度风险
        if project.difficulty_level >= 4 {
            risks.push("Technical difficulty is high".to_string());
        }

        // 低成功率风险
        if project.success_rate < 0.2 {
            risks.push("Low historical success rate".to_string());
        }

        risks
    }

    fn identify_opportunities(&self, project: &BountyProject) -> Vec<String> {
        let mut opportunities = Vec::new();

        // 低竞争机会
        if project.competition_level <= 2 {
            opportunities.push("Low competition level".to_string());
        }

        // 高ROI机会
        if project.roi_score >= 70.0 {
            opportunities.push("High ROI".to_string());
        }

        // 低难度机会
        if project.difficulty_level <= 2 {
            opportunities.push("Low technical门槛".to_string());
        }

        opportunities
    }

    fn estimate_time_investment(&self, project: &BountyProject, difficulty: i32) -> Option<i32> {
        let base_hours = match difficulty {
            1 => 8,   // 初级：8小时
            2 => 16,  // 中级：16小时
            3 => 32,  // 高级：32小时
            4 => 64,  // 专家：64小时
            _ => 128, // 超高级：128小时
        };

        // 根据域名数量调整时间
        if let Some(scope_str) = &project.scope_domains {
            if let Ok(domains) = serde_json::from_str::<Vec<String>>(scope_str) {
                let domain_factor = (domains.len() as f32 / 5.0).max(1.0);
                return Some((base_hours as f32 * domain_factor) as i32);
            }
        }

        Some(base_hours)
    }

    fn matches_criteria(&self, project: &BountyProject, params: &RecommendationParams) -> bool {
        // 奖励筛选
        if let Some(min_reward) = params.min_reward {
            if let Some(reward_str) = &project.reward_range {
                if let Ok(reward_data) = serde_json::from_str::<serde_json::Value>(reward_str) {
                    if let Some(max_reward) = reward_data.get("max").and_then(|v| v.as_f64()) {
                        if max_reward < min_reward {
                            return false;
                        }
                    }
                }
            }
        }

        // 难度筛选
        if let Some(max_difficulty) = params.max_difficulty {
            if project.difficulty_level > max_difficulty {
                return false;
            }
        }

        // 平台筛选
        if let Some(preferred_platforms) = &params.preferred_platforms {
            if !preferred_platforms.contains(&project.platform) {
                return false;
            }
        }

        // 状态筛选（只返回活跃项目）
        project.status == "active"
    }
}

impl Default for ProjectService {
    fn default() -> Self {
        Self::new()
    }
} 