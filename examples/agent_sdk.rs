//! Agent开发SDK
//! 提供宏和辅助工具简化自定义Agent的开发

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;

// 重新导出核心类型
pub use crate::workflow_engine::{AgentExecutor, ExecutionContext};

/// Agent能力描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub action: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub examples: Vec<AgentExample>,
    pub tags: Vec<String>,
}

/// Agent示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExample {
    pub name: String,
    pub description: String,
    pub inputs: HashMap<String, Value>,
    pub expected_outputs: HashMap<String, Value>,
}

/// Agent元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<AgentCapability>,
    pub dependencies: Vec<String>,
    pub configuration_schema: Option<Value>,
}

/// Agent配置特征
pub trait AgentConfig: Send + Sync + Clone + for<'de> Deserialize<'de> + Serialize {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
    
    fn get_schema() -> Value {
        serde_json::json!({})
    }
}

/// 默认空配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyConfig;

impl AgentConfig for EmptyConfig {}

/// Agent构建器特征
#[async_trait]
pub trait AgentBuilder<C: AgentConfig>: Send + Sync {
    type Agent: AgentExecutor;
    
    async fn build(config: C) -> Result<Self::Agent>;
    fn get_metadata() -> AgentMetadata;
}

/// Agent动作处理器
#[async_trait]
pub trait ActionHandler: Send + Sync {
    async fn handle(
        &self,
        inputs: &HashMap<String, Value>,
        context: &ExecutionContext,
    ) -> Result<HashMap<String, Value>>;
    
    fn get_capability(&self) -> AgentCapability;
}

/// 基础Agent实现
pub struct BaseAgent<C: AgentConfig> {
    pub agent_type: String,
    pub config: C,
    pub handlers: HashMap<String, Box<dyn ActionHandler>>,
}

impl<C: AgentConfig> BaseAgent<C> {
    pub fn new(agent_type: String, config: C) -> Self {
        Self {
            agent_type,
            config,
            handlers: HashMap::new(),
        }
    }
    
    pub fn add_handler(mut self, action: String, handler: Box<dyn ActionHandler>) -> Self {
        self.handlers.insert(action, handler);
        self
    }
    
    pub fn get_capabilities(&self) -> Vec<AgentCapability> {
        self.handlers.values().map(|h| h.get_capability()).collect()
    }
}

#[async_trait]
impl<C: AgentConfig> AgentExecutor for BaseAgent<C> {
    async fn execute(
        &self,
        action: &str,
        inputs: &HashMap<String, Value>,
        context: &ExecutionContext,
    ) -> Result<HashMap<String, Value>> {
        let handler = self.handlers.get(action)
            .ok_or_else(|| anyhow::anyhow!("不支持的动作: {}", action))?;
        
        handler.handle(inputs, context).await
    }
    
    fn get_capabilities(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
    
    fn get_agent_type(&self) -> String {
        self.agent_type.clone()
    }
}

/// Agent宏 - 简化Agent定义
#[macro_export]
macro_rules! define_agent {
    (
        name: $name:expr,
        version: $version:expr,
        description: $description:expr,
        author: $author:expr,
        config: $config_type:ty,
        actions: {
            $(
                $action_name:ident {
                    description: $action_desc:expr,
                    inputs: $input_schema:expr,
                    outputs: $output_schema:expr,
                    handler: $handler:expr
                }
            ),* $(,)?
        }
    ) => {
        pub struct Agent {
            base: $crate::agent_sdk::BaseAgent<$config_type>,
        }
        
        impl Agent {
            pub fn new(config: $config_type) -> Result<Self> {
                config.validate()?;
                
                let mut base = $crate::agent_sdk::BaseAgent::new(
                    $name.to_string(),
                    config,
                );
                
                $(
                    base = base.add_handler(
                        stringify!($action_name).to_string(),
                        Box::new($handler),
                    );
                )*
                
                Ok(Self { base })
            }
            
            pub fn get_metadata() -> $crate::agent_sdk::AgentMetadata {
                $crate::agent_sdk::AgentMetadata {
                    name: $name.to_string(),
                    version: $version.to_string(),
                    description: $description.to_string(),
                    author: $author.to_string(),
                    license: "MIT".to_string(),
                    repository: None,
                    documentation: None,
                    tags: vec![],
                    capabilities: vec![
                        $(
                            $crate::agent_sdk::AgentCapability {
                                action: stringify!($action_name).to_string(),
                                description: $action_desc.to_string(),
                                input_schema: $input_schema,
                                output_schema: $output_schema,
                                examples: vec![],
                                tags: vec![],
                            }
                        ),*
                    ],
                    dependencies: vec![],
                    configuration_schema: Some(<$config_type>::get_schema()),
                }
            }
        }
        
        #[async_trait::async_trait]
        impl $crate::workflow_engine::AgentExecutor for Agent {
            async fn execute(
                &self,
                action: &str,
                inputs: &std::collections::HashMap<String, serde_json::Value>,
                context: &$crate::workflow_engine::ExecutionContext,
            ) -> anyhow::Result<std::collections::HashMap<String, serde_json::Value>> {
                self.base.execute(action, inputs, context).await
            }
            
            fn get_capabilities(&self) -> Vec<String> {
                self.base.get_capabilities()
            }
            
            fn get_agent_type(&self) -> String {
                self.base.get_agent_type()
            }
        }
        
        #[async_trait::async_trait]
        impl $crate::agent_sdk::AgentBuilder<$config_type> for Agent {
            type Agent = Self;
            
            async fn build(config: $config_type) -> anyhow::Result<Self::Agent> {
                Self::new(config)
            }
            
            fn get_metadata() -> $crate::agent_sdk::AgentMetadata {
                Self::get_metadata()
            }
        }
    };
}

/// 动作处理器宏
#[macro_export]
macro_rules! action_handler {
    (
        $handler_name:ident,
        description: $description:expr,
        inputs: $input_schema:expr,
        outputs: $output_schema:expr,
        handler: |$inputs:ident: &HashMap<String, Value>, $context:ident: &ExecutionContext| $body:block
    ) => {
        pub struct $handler_name;
        
        #[async_trait::async_trait]
        impl $crate::agent_sdk::ActionHandler for $handler_name {
            async fn handle(
                &self,
                $inputs: &HashMap<String, Value>,
                $context: &$crate::workflow_engine::ExecutionContext,
            ) -> anyhow::Result<HashMap<String, Value>> {
                $body
            }
            
            fn get_capability(&self) -> $crate::agent_sdk::AgentCapability {
                $crate::agent_sdk::AgentCapability {
                    action: stringify!($handler_name).to_string(),
                    description: $description.to_string(),
                    input_schema: $input_schema,
                    output_schema: $output_schema,
                    examples: vec![],
                    tags: vec![],
                }
            }
        }
    };
}

/// 输入验证宏
#[macro_export]
macro_rules! validate_inputs {
    ($inputs:expr, { $($field:ident: $field_type:ty),* $(,)? }) => {
        {
            let mut validated = std::collections::HashMap::new();
            $(
                let value = $inputs.get(stringify!($field))
                    .ok_or_else(|| anyhow::anyhow!("缺少必需的输入字段: {}", stringify!($field)))?;
                let typed_value: $field_type = serde_json::from_value(value.clone())
                    .map_err(|e| anyhow::anyhow!("字段 {} 类型错误: {}", stringify!($field), e))?;
                validated.insert(stringify!($field).to_string(), serde_json::to_value(typed_value)?);
            )*
            validated
        }
    };
}

/// 输出构建宏
#[macro_export]
macro_rules! build_outputs {
    ({ $($field:ident: $value:expr),* $(,)? }) => {
        {
            let mut outputs = std::collections::HashMap::new();
            $(
                outputs.insert(stringify!($field).to_string(), serde_json::to_value($value)?);
            )*
            outputs
        }
    };
}

/// HTTP客户端辅助工具
pub struct HttpClient {
    client: reqwest::Client,
    base_url: Option<String>,
    default_headers: HashMap<String, String>,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: None,
            default_headers: HashMap::new(),
        }
    }
    
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }
    
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.default_headers.insert(key, value);
        self
    }
    
    pub async fn get(&self, path: &str) -> Result<Value> {
        let url = self.build_url(path);
        let mut request = self.client.get(&url);
        
        for (key, value) in &self.default_headers {
            request = request.header(key, value);
        }
        
        let response = request.send().await?;
        let json: Value = response.json().await?;
        Ok(json)
    }
    
    pub async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let url = self.build_url(path);
        let mut request = self.client.post(&url).json(body);
        
        for (key, value) in &self.default_headers {
            request = request.header(key, value);
        }
        
        let response = request.send().await?;
        let json: Value = response.json().await?;
        Ok(json)
    }
    
    fn build_url(&self, path: &str) -> String {
        match &self.base_url {
            Some(base) => format!("{}/{}", base.trim_end_matches('/'), path.trim_start_matches('/')),
            None => path.to_string(),
        }
    }
}

/// 数据库辅助工具
pub struct DatabaseHelper {
    // 这里可以集成具体的数据库客户端
}

impl DatabaseHelper {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<HashMap<String, Value>>> {
        // 实际实现中应该连接到真实的数据库
        // 这里只是示例
        Ok(vec![])
    }
    
    pub async fn execute(&self, sql: &str, params: &[Value]) -> Result<u64> {
        // 实际实现中应该连接到真实的数据库
        Ok(0)
    }
}

/// 文件系统辅助工具
pub struct FileSystemHelper;

impl FileSystemHelper {
    pub async fn read_file(&self, path: &str) -> Result<String> {
        tokio::fs::read_to_string(path).await.map_err(Into::into)
    }
    
    pub async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        tokio::fs::write(path, content).await.map_err(Into::into)
    }
    
    pub async fn list_directory(&self, path: &str) -> Result<Vec<String>> {
        let mut entries = tokio::fs::read_dir(path).await?;
        let mut files = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                files.push(name.to_string());
            }
        }
        
        Ok(files)
    }
    
    pub async fn create_directory(&self, path: &str) -> Result<()> {
        tokio::fs::create_dir_all(path).await.map_err(Into::into)
    }
}

/// 日志辅助工具
pub struct Logger {
    context: String,
}

impl Logger {
    pub fn new(context: String) -> Self {
        Self { context }
    }
    
    pub fn info(&self, message: &str) {
        println!("[INFO] [{}] {}", self.context, message);
    }
    
    pub fn warn(&self, message: &str) {
        println!("[WARN] [{}] {}", self.context, message);
    }
    
    pub fn error(&self, message: &str) {
        println!("[ERROR] [{}] {}", self.context, message);
    }
    
    pub fn debug(&self, message: &str) {
        println!("[DEBUG] [{}] {}", self.context, message);
    }
}

/// Agent测试辅助工具
pub struct AgentTester<A: AgentExecutor> {
    agent: A,
}

impl<A: AgentExecutor> AgentTester<A> {
    pub fn new(agent: A) -> Self {
        Self { agent }
    }
    
    pub async fn test_action(
        &self,
        action: &str,
        inputs: HashMap<String, Value>,
        expected_outputs: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, Value>> {
        let context = ExecutionContext {
            variables: HashMap::new(),
            outputs: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        let outputs = self.agent.execute(action, &inputs, &context).await?;
        
        if let Some(expected) = expected_outputs {
            for (key, expected_value) in expected {
                let actual_value = outputs.get(&key)
                    .ok_or_else(|| anyhow::anyhow!("输出中缺少字段: {}", key))?;
                
                if actual_value != &expected_value {
                    return Err(anyhow::anyhow!(
                        "字段 {} 的值不匹配: 期望 {:?}, 实际 {:?}",
                        key, expected_value, actual_value
                    ));
                }
            }
        }
        
        Ok(outputs)
    }
    
    pub fn get_capabilities(&self) -> Vec<String> {
        self.agent.get_capabilities()
    }
}

/// Agent注册表
pub struct AgentRegistry {
    agents: HashMap<String, Box<dyn AgentExecutor>>,
    metadata: HashMap<String, AgentMetadata>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
    
    pub fn register<A: AgentExecutor + 'static>(
        &mut self,
        agent_type: String,
        agent: A,
        metadata: AgentMetadata,
    ) {
        self.agents.insert(agent_type.clone(), Box::new(agent));
        self.metadata.insert(agent_type, metadata);
    }
    
    pub fn get_agent(&self, agent_type: &str) -> Option<&dyn AgentExecutor> {
        self.agents.get(agent_type).map(|a| a.as_ref())
    }
    
    pub fn get_metadata(&self, agent_type: &str) -> Option<&AgentMetadata> {
        self.metadata.get(agent_type)
    }
    
    pub fn list_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }
    
    pub fn search_agents(&self, query: &str) -> Vec<String> {
        self.metadata
            .iter()
            .filter(|(_, metadata)| {
                metadata.name.contains(query) ||
                metadata.description.contains(query) ||
                metadata.tags.iter().any(|tag| tag.contains(query))
            })
            .map(|(agent_type, _)| agent_type.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestConfig {
        api_key: String,
        timeout: u64,
    }
    
    impl AgentConfig for TestConfig {
        fn validate(&self) -> Result<()> {
            if self.api_key.is_empty() {
                return Err(anyhow::anyhow!("API密钥不能为空"));
            }
            Ok(())
        }
        
        fn get_schema() -> Value {
            json!({
                "type": "object",
                "properties": {
                    "api_key": { "type": "string" },
                    "timeout": { "type": "integer", "minimum": 1 }
                },
                "required": ["api_key"]
            })
        }
    }
    
    action_handler!(
        TestHandler,
        description: "测试处理器",
        inputs: json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            }
        }),
        outputs: json!({
            "type": "object",
            "properties": {
                "response": { "type": "string" }
            }
        }),
        handler: |inputs: &HashMap<String, Value>, _context: &ExecutionContext| {
            let validated = validate_inputs!(inputs, {
                message: String
            });
            
            let message = validated.get("message").unwrap().as_str().unwrap();
            let response = format!("收到消息: {}", message);
            
            Ok(build_outputs!({
                response: response
            }))
        }
    );
    
    define_agent!(
        name: "test_agent",
        version: "1.0.0",
        description: "测试Agent",
        author: "测试作者",
        config: TestConfig,
        actions: {
            test_action {
                description: "测试动作",
                inputs: json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" }
                    }
                }),
                outputs: json!({
                    "type": "object",
                    "properties": {
                        "response": { "type": "string" }
                    }
                }),
                handler: TestHandler
            }
        }
    );
    
    #[tokio::test]
    async fn test_agent_creation() {
        let config = TestConfig {
            api_key: "test_key".to_string(),
            timeout: 30,
        };
        
        let agent = Agent::new(config).unwrap();
        assert_eq!(agent.get_agent_type(), "test_agent");
        assert!(agent.get_capabilities().contains(&"test_action".to_string()));
    }
    
    #[tokio::test]
    async fn test_agent_execution() {
        let config = TestConfig {
            api_key: "test_key".to_string(),
            timeout: 30,
        };
        
        let agent = Agent::new(config).unwrap();
        let tester = AgentTester::new(agent);
        
        let inputs = HashMap::from([
            ("message".to_string(), json!("Hello, World!"))
        ]);
        
        let expected_outputs = HashMap::from([
            ("response".to_string(), json!("收到消息: Hello, World!"))
        ]);
        
        let outputs = tester.test_action("test_action", inputs, Some(expected_outputs)).await.unwrap();
        assert_eq!(outputs.get("response").unwrap(), &json!("收到消息: Hello, World!"));
    }
}