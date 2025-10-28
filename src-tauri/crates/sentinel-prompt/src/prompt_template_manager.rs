//! Prompt模板管理器
//! 
//! 实现模板的加载、保存、版本控制和缓存功能，支持：
//! - 模板文件管理
//! - 版本控制
//! - 模板缓存
//! - 热重载
//! - 模板验证
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};
use tokio::fs;
use tokio::sync::RwLock;
use std::sync::Arc;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::sync::mpsc;

use serde_yaml;

/// 模板管理器
#[derive(Debug)]
pub struct PromptTemplateManager {
    /// 模板存储路径
    template_dir: PathBuf,
    /// 模板缓存
    template_cache: Arc<RwLock<HashMap<String, CachedTemplate>>>,
    /// 版本管理器
    version_manager: TemplateVersionManager,
    /// 文件监视器
    file_watcher: Option<notify::RecommendedWatcher>,
    /// 配置
    config: TemplateManagerConfig,
}

/// 缓存的模板
#[derive(Debug, Clone)]
pub struct CachedTemplate {
    /// 模板内容
    pub template: CustomTemplate,
    /// 文件路径
    pub file_path: PathBuf,
    /// 最后修改时间
    pub last_modified: std::time::SystemTime,
    /// 版本号
    pub version: String,
    /// 缓存时间
    pub cached_at: chrono::DateTime<chrono::Utc>,
    /// 使用次数
    pub usage_count: u64,
}

/// 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// 模板ID
    pub id: String,
    /// 模板名称
    pub name: String,
    /// 模板内容
    pub content: String,
    /// 模板类型
    pub template_type: TemplateType,
    /// 变量列表
    pub variables: Vec<String>,
    /// 模板元数据
    pub metadata: TemplateMetadata,
    /// 版本
    pub version: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 模板版本管理器
#[derive(Debug)]
pub struct TemplateVersionManager {
    /// 版本存储路径
    version_dir: PathBuf,
    /// 版本历史
    version_history: HashMap<String, Vec<TemplateVersion>>,
}

/// 模板版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVersion {
    /// 版本号
    pub version: String,
    /// 模板内容
    pub content: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 创建者
    pub author: String,
    /// 变更说明
    pub changelog: String,
    /// 标签
    pub tags: Vec<String>,
    /// 文件哈希
    pub content_hash: String,
}

/// 模板管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManagerConfig {
    /// 是否启用缓存
    pub enable_cache: bool,
    /// 缓存过期时间（秒）
    pub cache_ttl_seconds: u64,
    /// 是否启用热重载
    pub enable_hot_reload: bool,
    /// 最大缓存大小
    pub max_cache_size: usize,
    /// 是否启用版本控制
    pub enable_versioning: bool,
    /// 最大版本保留数量
    pub max_versions: usize,
    /// 模板验证规则
    pub validation_rules: ValidationRules,
    /// 模板目录
    pub template_dir: PathBuf,
    /// 是否启用验证
    pub validation_enabled: bool,
    /// 是否自动备份
    pub auto_backup: bool,
}

/// 验证规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
    /// 最大模板长度
    pub max_template_length: usize,
    /// 最小模板长度
    pub min_template_length: usize,
    /// 必需的变量
    pub required_variables: Vec<String>,
    /// 禁止的内容
    pub forbidden_content: Vec<String>,
    /// 必需的结构标记
    pub required_structure_markers: Vec<String>,
    /// 最大长度（兼容性字段）
    pub max_length: Option<usize>,
}

/// 模板搜索结果
#[derive(Debug, Clone)]
pub struct TemplateSearchResult {
    /// 模板ID
    pub template_id: String,
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 匹配分数
    pub match_score: f32,
    /// 模板类型
    pub template_type: TemplateType,
    /// 标签
    pub tags: Vec<String>,
}

/// 模板统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateStats {
    /// 总模板数量
    pub total_templates: usize,
    /// 缓存命中率
    pub cache_hit_rate: f32,
    /// 平均加载时间（毫秒）
    pub avg_load_time_ms: f64,
    /// 最常用模板
    pub most_used_templates: Vec<(String, u64)>,
    /// 版本统计
    pub version_stats: HashMap<String, usize>,
    /// 按类型统计模板
    pub templates_by_type: HashMap<String, usize>,
    /// 平均模板大小
    pub average_template_size: usize,
    /// 验证错误数量
    pub validation_errors: usize,
}


impl Default for TemplateManagerConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            cache_ttl_seconds: 3600, // 1小时
            enable_hot_reload: true,
            max_cache_size: 100,
            enable_versioning: true,
            max_versions: 10,
            validation_rules: ValidationRules {
                max_template_length: 10000,
                min_template_length: 50,
                required_variables: vec![],
                forbidden_content: vec![
                    "<script>".to_string(),
                    "javascript:".to_string(),
                    "eval(".to_string(),
                ],
                required_structure_markers: vec![
                    "**".to_string(), // Markdown格式标记
                ],
                max_length: Some(10000),
            },
            template_dir: PathBuf::from("templates"),
            validation_enabled: true,
            auto_backup: true,
        }
    }
}

impl PromptTemplateManager {
    /// 创建新的模板管理器
    pub async fn new<P: AsRef<Path>>(template_dir: P, config: TemplateManagerConfig) -> Result<Self> {
        let template_dir = template_dir.as_ref().to_path_buf();
        
        // 确保目录存在
        fs::create_dir_all(&template_dir).await?;
        
        let version_dir = template_dir.join("versions");
        fs::create_dir_all(&version_dir).await?;

        let version_manager = TemplateVersionManager::new(version_dir)?;
        
        let mut manager = Self {
            template_dir,
            template_cache: Arc::new(RwLock::new(HashMap::new())),
            version_manager,
            file_watcher: None,
            config,
        };

        // 初始化文件监视器
        if manager.config.enable_hot_reload {
            manager.setup_file_watcher().await?;
        }

        // 预加载模板
        manager.preload_templates().await?;

        Ok(manager)
    }

    /// 加载模板
    pub async fn load_template(&self, template_id: &str) -> Result<CustomTemplate> {
        // 检查缓存
        if self.config.enable_cache {
            let cache = self.template_cache.read().await;
            if let Some(cached) = cache.get(template_id) {
                // 检查缓存是否过期
                let cache_age = chrono::Utc::now().signed_duration_since(cached.cached_at);
                if cache_age.num_seconds() < self.config.cache_ttl_seconds as i64 {
                    return Ok(cached.template.clone());
                }
            }
        }

        // 从文件加载
        let template = self.load_template_from_file(template_id).await?;
        
        // 更新缓存
        if self.config.enable_cache {
            self.update_cache(template_id, &template).await?;
        }

        Ok(template)
    }

    /// 保存模板
    pub async fn save_template(&mut self, template_id: &str, template: &CustomTemplate) -> Result<()> {
        // 验证模板
        self.validate_template(template)?;

        // 创建版本（如果启用版本控制）
        if self.config.enable_versioning {
            self.version_manager.create_version(template_id, template).await?;
        }

        // 保存到文件
        let file_path = self.get_template_file_path(template_id);
        let content = serde_yaml::to_string(template)?;
        fs::write(&file_path, content).await?;

        // 更新缓存
        if self.config.enable_cache {
            self.update_cache(template_id, template).await?;
        }

        Ok(())
    }

    /// 删除模板
    pub async fn delete_template(&mut self, template_id: &str) -> Result<()> {
        let file_path = self.get_template_file_path(template_id);
        
        // 删除文件
        if file_path.exists() {
            fs::remove_file(&file_path).await?;
        }

        // 从缓存中移除
        let mut cache = self.template_cache.write().await;
        cache.remove(template_id);

        // 删除版本历史
        if self.config.enable_versioning {
            self.version_manager.delete_versions(template_id).await?;
        }

        Ok(())
    }

    /// 列出所有模板
    pub async fn list_templates(&self) -> Result<Vec<String>> {
        let mut templates = Vec::new();
        let mut entries = fs::read_dir(&self.template_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                if let Some(stem) = path.file_stem() {
                    if let Some(template_id) = stem.to_str() {
                        templates.push(template_id.to_string());
                    }
                }
            }
        }

        Ok(templates)
    }

    /// 搜索模板
    pub async fn search_templates(&self, query: &str, template_type: Option<TemplateType>) -> Result<Vec<TemplateSearchResult>> {
        let template_ids = self.list_templates().await?;
        let mut results = Vec::new();

        for template_id in template_ids {
            if let Ok(template) = self.load_template(&template_id).await {
                // 类型过滤
                if let Some(ref filter_type) = template_type {
                    if &template.template_type != filter_type {
                        continue;
                    }
                }

                // 计算匹配分数
                let score = self.calculate_match_score(&template, query);
                if score > 0.1 {
                    results.push(TemplateSearchResult {
                        template_id: template_id.clone(),
                        name: template.name.clone(),
                        description: template.description.clone(),
                        match_score: score,
                        template_type: template.template_type.clone(),
                        tags: template.tags.clone(),
                    });
                }
            }
        }

        // 按匹配分数排序
        results.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap());
        
        Ok(results)
    }

    /// 获取模板版本历史
    pub async fn get_template_versions(&self, template_id: &str) -> Result<Vec<TemplateVersion>> {
        self.version_manager.get_versions(template_id).await
    }

    /// 恢复模板版本
    pub async fn restore_template_version(&mut self, template_id: &str, version: &str) -> Result<()> {
        let template = self.version_manager.get_version_content(template_id, version).await?;
        self.save_template(template_id, &template).await
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> Result<TemplateStats> {
        let cache = self.template_cache.read().await;
        let total_templates = self.list_templates().await?.len();
        
        // 计算缓存命中率（简化实现）
        let cache_hit_rate = if total_templates > 0 {
            cache.len() as f32 / total_templates as f32
        } else {
            0.0
        };

        // 获取最常用模板
        let mut usage_stats: Vec<_> = cache.iter()
            .map(|(id, cached)| (id.clone(), cached.usage_count))
            .collect();
        usage_stats.sort_by(|a, b| b.1.cmp(&a.1));
        let most_used_templates = usage_stats.into_iter().take(10).collect();

        // 版本统计
        let version_stats = self.version_manager.get_version_stats().await?;

        Ok(TemplateStats {
            total_templates,
            cache_hit_rate,
            avg_load_time_ms: 0.0, // 需要实际测量
            most_used_templates,
            version_stats,
            templates_by_type: HashMap::new(),
            average_template_size: 0,
            validation_errors: 0,
        })
    }

    /// 清理缓存
    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.template_cache.write().await;
        cache.clear();
        Ok(())
    }

    /// 预加载模板
    async fn preload_templates(&self) -> Result<()> {
        let template_ids = self.list_templates().await?;
        
        for template_id in template_ids {
            if let Err(e) = self.load_template(&template_id).await {
                eprintln!("Failed to preload template {}: {}", template_id, e);
            }
        }

        Ok(())
    }

    /// 从文件加载模板
    async fn load_template_from_file(&self, template_id: &str) -> Result<CustomTemplate> {
        let file_path = self.get_template_file_path(template_id);
        
        if !file_path.exists() {
            return Err(anyhow!("Template file not found: {}", template_id));
        }

        let content = fs::read_to_string(&file_path).await?;
        let template: CustomTemplate = serde_yaml::from_str(&content)?;
        
        // 验证模板
        self.validate_template(&template)?;

        Ok(template)
    }

    /// 更新缓存
    async fn update_cache(&self, template_id: &str, template: &CustomTemplate) -> Result<()> {
        let mut cache = self.template_cache.write().await;
        
        // 检查缓存大小限制
        if cache.len() >= self.config.max_cache_size {
            // 移除最旧的缓存项
            if let Some((oldest_key, _)) = cache.iter()
                .min_by_key(|(_, cached)| cached.cached_at)
                .map(|(k, v)| (k.clone(), v.clone())) {
                cache.remove(&oldest_key);
            }
        }

        let file_path = self.get_template_file_path(template_id);
        let last_modified = fs::metadata(&file_path).await?
            .modified().unwrap_or(std::time::SystemTime::now());

        let cached_template = CachedTemplate {
            template: template.clone(),
            file_path,
            last_modified,
            version: template.version.clone(),
            cached_at: chrono::Utc::now(),
            usage_count: cache.get(template_id).map_or(1, |c| c.usage_count + 1),
        };

        cache.insert(template_id.to_string(), cached_template);
        Ok(())
    }

    /// 获取模板文件路径
    fn get_template_file_path(&self, template_id: &str) -> PathBuf {
        self.template_dir.join(format!("{}.yaml", template_id))
    }

    /// 验证模板
    fn validate_template(&self, template: &CustomTemplate) -> Result<()> {
        let rules = &self.config.validation_rules;

        // 检查长度
        if template.content.len() > rules.max_template_length {
            return Err(anyhow!("Template too long: {} > {}", 
                template.content.len(), rules.max_template_length));
        }

        if template.content.len() < rules.min_template_length {
            return Err(anyhow!("Template too short: {} < {}", 
                template.content.len(), rules.min_template_length));
        }

        // 检查禁止内容
        for forbidden in &rules.forbidden_content {
            if template.content.contains(forbidden) {
                return Err(anyhow!("Template contains forbidden content: {}", forbidden));
            }
        }

        // 检查必需的结构标记
        for marker in &rules.required_structure_markers {
            if !template.content.contains(marker) {
                return Err(anyhow!("Template missing required structure marker: {}", marker));
            }
        }

        Ok(())
    }

    /// 计算匹配分数
    fn calculate_match_score(&self, template: &CustomTemplate, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let mut score: f32 = 0.0;

        // 名称匹配
        if template.name.to_lowercase().contains(&query_lower) {
            score += 0.4;
        }

        // 描述匹配
        if template.description.to_lowercase().contains(&query_lower) {
            score += 0.3;
        }

        // 标签匹配
        for tag in &template.tags {
            if tag.to_lowercase().contains(&query_lower) {
                score += 0.2;
                break;
            }
        }

        // 内容匹配
        if template.content.to_lowercase().contains(&query_lower) {
            score += 0.1;
        }

        score.min(1.0)
    }

    /// 设置文件监视器
    async fn setup_file_watcher(&mut self) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })?;
        watcher.watch(&self.template_dir, RecursiveMode::NonRecursive)?;

        let cache = Arc::clone(&self.template_cache);
        let _template_dir = self.template_dir.clone();

        // 启动监视线程
        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in event.paths {
                            if let Some(template_id) = Self::extract_template_id(&path) {
                                // 清除缓存中的对应项
                                let mut cache = cache.write().await;
                                cache.remove(&template_id);
                                println!("Template {} cache invalidated due to file change", template_id);
                            }
                        }
                    },
                    EventKind::Remove(_) => {
                        for path in event.paths {
                            if let Some(template_id) = Self::extract_template_id(&path) {
                                let mut cache = cache.write().await;
                                cache.remove(&template_id);
                                println!("Template {} removed from cache", template_id);
                            }
                        }
                    },
                    _ => {}
                }
            }
        });

        self.file_watcher = Some(watcher);
        Ok(())
    }

    /// 从路径提取模板ID
    fn extract_template_id(path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .map(|s| s.to_string())
    }
}

impl TemplateVersionManager {
    /// 创建新的版本管理器
    pub fn new(version_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            version_dir,
            version_history: HashMap::new(),
        })
    }

    /// 创建新版本
    pub async fn create_version(&mut self, template_id: &str, template: &CustomTemplate) -> Result<()> {
        let version = self.generate_version_number(template_id).await?;
        let content_hash = self.calculate_content_hash(&template.content);
        
        let template_version = TemplateVersion {
            version: version.clone(),
            content: template.content.clone(),
            created_at: chrono::Utc::now(),
            author: "system".to_string(), // 可以从配置或上下文获取
            changelog: "Auto-generated version".to_string(),
            tags: template.tags.clone(),
            content_hash,
        };

        // 保存版本文件
        let version_file = self.version_dir.join(format!("{}_{}.yaml", template_id, version));
        let content = serde_yaml::to_string(&template_version)?;
        fs::write(version_file, content).await?;

        // 更新内存中的版本历史
        self.version_history.entry(template_id.to_string())
            .or_insert_with(Vec::new)
            .push(template_version);

        // 清理旧版本
        self.cleanup_old_versions(template_id).await?;

        Ok(())
    }

    /// 获取版本列表
    pub async fn get_versions(&self, template_id: &str) -> Result<Vec<TemplateVersion>> {
        if let Some(versions) = self.version_history.get(template_id) {
            Ok(versions.clone())
        } else {
            // 从文件系统加载
            self.load_versions_from_disk(template_id).await
        }
    }

    /// 获取特定版本内容
    pub async fn get_version_content(&self, template_id: &str, version: &str) -> Result<CustomTemplate> {
        let version_file = self.version_dir.join(format!("{}_{}.yaml", template_id, version));
        
        if !version_file.exists() {
            return Err(anyhow!("Version not found: {} v{}", template_id, version));
        }

        let content = fs::read_to_string(version_file).await?;
        let template_version: TemplateVersion = serde_yaml::from_str(&content)?;
        
        // 重构为CustomTemplate
        Ok(CustomTemplate {
            id: template_id.to_string(),
            name: template_id.to_string(),
            description: "Restored from version".to_string(),
            content: template_version.content,
            template_type: TemplateType::Custom,
            creator: template_version.author,
            created_at: template_version.created_at,
            version: template_version.version,
            tags: template_version.tags,
            usage_stats: UsageStats::default(),
            variables: vec![], // 需要重新解析
            metadata: HashMap::new(),
            category: None,
            target_architecture: None,
            is_system: false,
            priority: 0,
        })
    }

    /// 删除版本历史
    pub async fn delete_versions(&mut self, template_id: &str) -> Result<()> {
        // 删除内存中的版本历史
        self.version_history.remove(template_id);

        // 删除文件系统中的版本文件
        let mut entries = fs::read_dir(&self.version_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with(&format!("{}_", template_id)) {
                    fs::remove_file(path).await?;
                }
            }
        }

        Ok(())
    }

    /// 获取版本统计
    pub async fn get_version_stats(&self) -> Result<HashMap<String, usize>> {
        let mut stats = HashMap::new();
        
        for (template_id, versions) in &self.version_history {
            stats.insert(template_id.clone(), versions.len());
        }

        Ok(stats)
    }

    /// 生成版本号
    async fn generate_version_number(&self, template_id: &str) -> Result<String> {
        let versions = self.get_versions(template_id).await.unwrap_or_default();
        let next_version = versions.len() + 1;
        Ok(format!("v{}", next_version))
    }

    /// 计算内容哈希
    fn calculate_content_hash(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// 从磁盘加载版本
    async fn load_versions_from_disk(&self, template_id: &str) -> Result<Vec<TemplateVersion>> {
        let mut versions = Vec::new();
        let mut entries = fs::read_dir(&self.version_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with(&format!("{}_", template_id)) && filename.ends_with(".yaml") {
                    let content = fs::read_to_string(&path).await?;
                    if let Ok(version) = serde_yaml::from_str::<TemplateVersion>(&content) {
                        versions.push(version);
                    }
                }
            }
        }

        // 按版本号排序
        versions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        
        Ok(versions)
    }

    /// 清理旧版本
    async fn cleanup_old_versions(&mut self, template_id: &str) -> Result<()> {
        const MAX_VERSIONS: usize = 10; // 可以配置
        
        if let Some(versions) = self.version_history.get_mut(template_id) {
            if versions.len() > MAX_VERSIONS {
                // 保留最新的版本
                versions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                let to_remove = versions.split_off(MAX_VERSIONS);
                
                // 删除文件
                for version in to_remove {
                    let version_file = self.version_dir.join(format!("{}_{}.yaml", template_id, version.version));
                    if version_file.exists() {
                        fs::remove_file(version_file).await?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_template_manager() {
        let temp_dir = TempDir::new().unwrap();
        let config = TemplateManagerConfig::default();
        let manager = PromptTemplateManager::new(temp_dir.path(), config).await.unwrap();
        
        let template = CustomTemplate {
            id: "test_template".to_string(),
            name: "test_template".to_string(),
            description: "Test template".to_string(),
            content: "**Test**: {variable}".to_string(),
            template_type: TemplateType::Custom,
            creator: "test_user".to_string(),
            created_at: chrono::Utc::now(),
            version: "v1".to_string(),
            tags: vec!["test".to_string()],
            usage_stats: UsageStats::default(),
            variables: vec!["variable".to_string()],
            metadata: HashMap::new(),
            category: None,
            target_architecture: None,
            is_system: false,
            priority: 0,
        };

        // 测试保存和加载
        let mut manager = manager;
        manager.save_template("test", &template).await.unwrap();
        let loaded = manager.load_template("test").await.unwrap();
        assert_eq!(loaded.name, template.name);
    }

    #[test]
    fn test_version_manager() {
        let temp_dir = TempDir::new().unwrap();
        let version_manager = TemplateVersionManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        // 测试版本号生成
        assert!(version_manager.calculate_content_hash("test").len() > 0);
    }
}