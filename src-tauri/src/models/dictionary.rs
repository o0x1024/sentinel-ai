use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 字典类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DictionaryType {
    /// 子域名字典
    Subdomain,
    /// 用户名字典
    Username,
    /// 密码字典
    Password,
    /// 路径字典
    Path,
    /// HTTP参数字典
    HttpParam,
    /// XSS载荷字典
    XssPayload,
    /// SQL注入载荷字典
    SqlInjectionPayload,
    /// 文件名字典
    Filename,
    /// 扩展名字典
    Extension,
    /// 端口字典
    Port,
    /// API端点字典
    ApiEndpoint,
    /// 自定义字典
    Custom(String),
}

impl ToString for DictionaryType {
    fn to_string(&self) -> String {
        match self {
            DictionaryType::Subdomain => "subdomain".to_string(),
            DictionaryType::Username => "username".to_string(),
            DictionaryType::Password => "password".to_string(),
            DictionaryType::Path => "path".to_string(),
            DictionaryType::HttpParam => "http_param".to_string(),
            DictionaryType::XssPayload => "xss_payload".to_string(),
            DictionaryType::SqlInjectionPayload => "sql_injection_payload".to_string(),
            DictionaryType::Filename => "filename".to_string(),
            DictionaryType::Extension => "extension".to_string(),
            DictionaryType::Port => "port".to_string(),
            DictionaryType::ApiEndpoint => "api_endpoint".to_string(),
            DictionaryType::Custom(name) => format!("custom_{}", name),
        }
    }
}

impl From<String> for DictionaryType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "subdomain" => DictionaryType::Subdomain,
            "username" => DictionaryType::Username,
            "password" => DictionaryType::Password,
            "path" => DictionaryType::Path,
            "http_param" => DictionaryType::HttpParam,
            "xss_payload" => DictionaryType::XssPayload,
            "sql_injection_payload" => DictionaryType::SqlInjectionPayload,
            "filename" => DictionaryType::Filename,
            "extension" => DictionaryType::Extension,
            "port" => DictionaryType::Port,
            "api_endpoint" => DictionaryType::ApiEndpoint,
            custom if custom.starts_with("custom_") => {
                DictionaryType::Custom(custom.strip_prefix("custom_").unwrap_or("").to_string())
            }
            _ => DictionaryType::Custom(s),
        }
    }
}

/// 服务类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceType {
    /// Web应用
    Web,
    /// SSH服务
    Ssh,
    /// 数据库服务
    Database,
    /// FTP服务
    Ftp,
    /// 邮件服务
    Mail,
    /// DNS服务
    Dns,
    /// API服务
    Api,
    /// 通用服务
    General,
    /// 自定义服务
    Custom(String),
}

impl ToString for ServiceType {
    fn to_string(&self) -> String {
        match self {
            ServiceType::Web => "web".to_string(),
            ServiceType::Ssh => "ssh".to_string(),
            ServiceType::Database => "database".to_string(),
            ServiceType::Ftp => "ftp".to_string(),
            ServiceType::Mail => "mail".to_string(),
            ServiceType::Dns => "dns".to_string(),
            ServiceType::Api => "api".to_string(),
            ServiceType::General => "general".to_string(),
            ServiceType::Custom(name) => name.clone(),
        }
    }
}

impl From<String> for ServiceType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "web" => ServiceType::Web,
            "ssh" => ServiceType::Ssh,
            "database" => ServiceType::Database,
            "ftp" => ServiceType::Ftp,
            "mail" => ServiceType::Mail,
            "dns" => ServiceType::Dns,
            "api" => ServiceType::Api,
            "general" => ServiceType::General,
            _ => ServiceType::Custom(s),
        }
    }
}

/// 字典模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Dictionary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub dict_type: String,
    pub service_type: Option<String>,
    pub category: Option<String>,
    pub is_builtin: bool,
    pub is_active: bool,
    pub word_count: i64,
    pub file_size: i64,
    pub checksum: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub source_url: Option<String>,
    pub tags: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Dictionary {
    pub fn new(
        name: String,
        dict_type: DictionaryType,
        service_type: Option<ServiceType>,
        description: Option<String>,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        Self {
            id,
            name,
            description,
            dict_type: dict_type.to_string(),
            service_type: service_type.map(|s| s.to_string()),
            category: None,
            is_builtin: false,
            is_active: true,
            word_count: 0,
            file_size: 0,
            checksum: None,
            version: "1.0.0".to_string(),
            author: None,
            source_url: None,
            tags: None,
            metadata: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn get_dict_type(&self) -> DictionaryType {
        DictionaryType::from(self.dict_type.clone())
    }

    pub fn get_service_type(&self) -> Option<ServiceType> {
        self.service_type
            .as_ref()
            .map(|s| ServiceType::from(s.clone()))
    }

    pub fn get_tags(&self) -> Vec<String> {
        self.tags
            .as_ref()
            .map(|tags| {
                tags.split(',')
                    .map(|tag| tag.trim().to_string())
                    .filter(|tag| !tag.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = if tags.is_empty() {
            None
        } else {
            Some(tags.join(","))
        };
    }
}

/// 字典词条模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DictionaryWord {
    pub id: String,
    pub dictionary_id: String,
    pub word: String,
    pub weight: f64,
    pub category: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

impl DictionaryWord {
    pub fn new(dictionary_id: String, word: String) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        Self {
            id,
            dictionary_id,
            word,
            weight: 1.0,
            category: None,
            metadata: None,
            created_at: now,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_category(mut self, category: String) -> Self {
        self.category = Some(category);
        self
    }

    pub fn with_metadata(mut self, metadata: String) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// 字典集合模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DictionarySet {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub service_type: Option<String>,
    pub scenario: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl DictionarySet {
    pub fn new(name: String, service_type: Option<ServiceType>) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        Self {
            id,
            name,
            description: None,
            service_type: service_type.map(|s| s.to_string()),
            scenario: None,
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn get_service_type(&self) -> Option<ServiceType> {
        self.service_type
            .as_ref()
            .map(|s| ServiceType::from(s.clone()))
    }
}

/// 字典集合关系模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DictionarySetRelation {
    pub id: String,
    pub set_id: String,
    pub dictionary_id: String,
    pub priority: i32,
    pub is_enabled: bool,
    pub created_at: String,
}

impl DictionarySetRelation {
    pub fn new(set_id: String, dictionary_id: String) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        Self {
            id,
            set_id,
            dictionary_id,
            priority: 0,
            is_enabled: true,
            created_at: now,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// 字典查询过滤器
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DictionaryFilter {
    pub dict_type: Option<DictionaryType>,
    pub service_type: Option<ServiceType>,
    pub category: Option<String>,
    pub is_builtin: Option<bool>,
    pub is_active: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub search_term: Option<String>,
}

/// 字典统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryStats {
    pub total_dictionaries: u64,
    pub total_words: u64,
    pub builtin_dictionaries: u64,
    pub custom_dictionaries: u64,
    pub active_dictionaries: u64,
    pub total_sets: u64,
    pub by_type: std::collections::HashMap<String, u64>,
    pub by_service: std::collections::HashMap<String, u64>,
}

/// 字典导入/导出格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryExport {
    pub dictionary: Dictionary,
    pub words: Vec<DictionaryWord>,
    pub export_time: String,
    pub format_version: String,
}

impl DictionaryExport {
    pub fn new(dictionary: Dictionary, words: Vec<DictionaryWord>) -> Self {
        Self {
            dictionary,
            words,
            export_time: chrono::Utc::now().to_rfc3339(),
            format_version: "1.0".to_string(),
        }
    }
}

/// 字典导入选项
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DictionaryImportOptions {
    pub merge_mode: MergeMode,
    pub skip_duplicates: bool,
    pub update_metadata: bool,
    pub preserve_weights: bool,
}

/// 合并模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeMode {
    /// 替换现有字典
    Replace,
    /// 合并到现有字典
    Merge,
    /// 创建新字典
    CreateNew,
}

impl Default for MergeMode {
    fn default() -> Self {
        MergeMode::Merge
    }
}
