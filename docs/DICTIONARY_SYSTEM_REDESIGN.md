# 通用字典管理系统重构需求分析

## 当前状况

目前系统只有子域名字典功能，存储在数据库的 `configurations` 表中，功能相对简单。

## 需求分析

### 1. 字典类型分类

#### 1.1 扫描类字典
- **子域名字典**: 用于子域名枚举
- **目录/路径字典**: 用于目录扫描、路径爆破
- **文件名字典**: 用于文件扫描
- **端口字典**: 常用端口列表
- **服务指纹字典**: 服务识别规则

#### 1.2 认证类字典
- **用户名字典**: 
  - 通用用户名字典
  - Web应用用户名字典
  - SSH服务用户名字典
  - 数据库用户名字典
  - FTP用户名字典
- **密码字典**:
  - 通用密码字典
  - 弱密码字典
  - 常用密码组合
  - 特定服务密码字典

#### 1.3 漏洞检测字典
- **XSS Payload字典**: 各种XSS攻击载荷
- **SQL注入字典**: SQL注入测试载荷
- **命令注入字典**: 命令执行测试载荷
- **路径遍历字典**: 目录遍历测试载荷
- **SSRF字典**: SSRF测试载荷

#### 1.4 参数类字典
- **HTTP参数字典**: 常见的HTTP参数名
- **Cookie参数字典**: 常见的Cookie名称
- **Header字典**: 常见的HTTP头部
- **API端点字典**: 常见的API路径

#### 1.5 服务专用字典
- **Web应用字典**: 针对Web应用的专用字典集合
- **数据库字典**: 针对各种数据库的字典集合
- **网络设备字典**: 针对路由器、交换机等设备
- **云服务字典**: 针对AWS、Azure等云服务

### 2. 字典属性设计

每个字典应包含以下属性：

```rust
pub struct Dictionary {
    pub id: String,
    pub name: String,
    pub category: DictionaryCategory,
    pub subcategory: Option<String>,
    pub service_type: Option<ServiceType>,
    pub description: String,
    pub words: Vec<String>,
    pub tags: Vec<String>,
    pub is_builtin: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub version: String,
    pub source: Option<String>, // 字典来源
    pub size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DictionaryCategory {
    Subdomain,
    Directory,
    Filename,
    Username,
    Password,
    XssPayload,
    SqlInjection,
    CommandInjection,
    PathTraversal,
    HttpParameter,
    ApiEndpoint,
    Port,
    Service,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceType {
    Web,
    Ssh,
    Ftp,
    Database(DatabaseType),
    Email,
    Dns,
    Cloud(CloudProvider),
    NetworkDevice,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseType {
    Mysql,
    Postgresql,
    Mssql,
    Oracle,
    MongoDB,
    Redis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    Aws,
    Azure,
    Gcp,
    Alibaba,
}
```

### 3. 数据库设计

#### 3.1 字典表 (dictionaries)

```sql
CREATE TABLE dictionaries (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    category VARCHAR(50) NOT NULL,
    subcategory VARCHAR(100),
    service_type VARCHAR(50),
    description TEXT,
    tags JSON,
    is_builtin BOOLEAN DEFAULT FALSE,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    created_by VARCHAR(100),
    version VARCHAR(20) DEFAULT '1.0.0',
    source VARCHAR(500),
    size INTEGER DEFAULT 0,
    INDEX idx_category (category),
    INDEX idx_service_type (service_type),
    INDEX idx_is_active (is_active)
);
```

#### 3.2 字典内容表 (dictionary_words)

```sql
CREATE TABLE dictionary_words (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    dictionary_id VARCHAR(36) NOT NULL,
    word VARCHAR(1000) NOT NULL,
    position INTEGER,
    metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (dictionary_id) REFERENCES dictionaries(id) ON DELETE CASCADE,
    INDEX idx_dictionary_id (dictionary_id),
    INDEX idx_word (word(100)),
    UNIQUE KEY unique_dict_word (dictionary_id, word(500))
);
```

#### 3.3 字典集合表 (dictionary_sets)

```sql
CREATE TABLE dictionary_sets (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    category VARCHAR(50),
    service_type VARCHAR(50),
    is_builtin BOOLEAN DEFAULT FALSE,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);
```

#### 3.4 字典集合关联表 (dictionary_set_relations)

```sql
CREATE TABLE dictionary_set_relations (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    set_id VARCHAR(36) NOT NULL,
    dictionary_id VARCHAR(36) NOT NULL,
    priority INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (set_id) REFERENCES dictionary_sets(id) ON DELETE CASCADE,
    FOREIGN KEY (dictionary_id) REFERENCES dictionaries(id) ON DELETE CASCADE,
    UNIQUE KEY unique_set_dict (set_id, dictionary_id)
);
```

### 4. 服务层设计

#### 4.1 字典管理服务 (DictionaryService)

```rust
pub struct DictionaryService {
    db: Arc<DatabaseService>,
}

impl DictionaryService {
    // 基础CRUD操作
    pub async fn create_dictionary(&self, dict: &Dictionary) -> Result<String>;
    pub async fn get_dictionary(&self, id: &str) -> Result<Option<Dictionary>>;
    pub async fn update_dictionary(&self, dict: &Dictionary) -> Result<()>;
    pub async fn delete_dictionary(&self, id: &str) -> Result<()>;
    
    // 查询操作
    pub async fn list_dictionaries(&self, filter: DictionaryFilter) -> Result<Vec<Dictionary>>;
    pub async fn search_dictionaries(&self, query: &str) -> Result<Vec<Dictionary>>;
    pub async fn get_dictionaries_by_category(&self, category: DictionaryCategory) -> Result<Vec<Dictionary>>;
    pub async fn get_dictionaries_by_service(&self, service: ServiceType) -> Result<Vec<Dictionary>>;
    
    // 字典内容操作
    pub async fn add_words(&self, dict_id: &str, words: &[String]) -> Result<()>;
    pub async fn remove_words(&self, dict_id: &str, words: &[String]) -> Result<()>;
    pub async fn get_words(&self, dict_id: &str) -> Result<Vec<String>>;
    pub async fn replace_words(&self, dict_id: &str, words: &[String]) -> Result<()>;
    
    // 字典集合操作
    pub async fn create_dictionary_set(&self, set: &DictionarySet) -> Result<String>;
    pub async fn add_dictionary_to_set(&self, set_id: &str, dict_id: &str, priority: i32) -> Result<()>;
    pub async fn get_set_dictionaries(&self, set_id: &str) -> Result<Vec<Dictionary>>;
    
    // 导入导出
    pub async fn import_dictionary(&self, content: &str, format: ImportFormat, metadata: DictionaryMetadata) -> Result<String>;
    pub async fn export_dictionary(&self, dict_id: &str, format: ExportFormat) -> Result<String>;
    
    // 内置字典管理
    pub async fn initialize_builtin_dictionaries(&self) -> Result<()>;
    pub async fn update_builtin_dictionary(&self, category: DictionaryCategory, service: Option<ServiceType>) -> Result<()>;
}
```

### 5. 前端界面设计

#### 5.1 主界面布局

- **侧边栏**: 字典分类树形结构
- **主内容区**: 字典列表和详情
- **工具栏**: 搜索、过滤、导入导出等操作

#### 5.2 功能模块

1. **字典管理页面**
   - 字典列表（支持分页、搜索、过滤）
   - 字典详情（查看、编辑字典信息和内容）
   - 字典创建（向导式创建）

2. **字典集合管理**
   - 集合列表
   - 集合配置（添加/移除字典，设置优先级）

3. **导入导出功能**
   - 支持多种格式（TXT, JSON, CSV, XML）
   - 批量导入
   - 在线字典库集成

4. **字典统计**
   - 使用频率统计
   - 字典大小分析
   - 性能指标

### 6. API接口设计

#### 6.1 字典管理接口

```typescript
// 获取字典列表
get_dictionaries(filter: DictionaryFilter): Promise<Dictionary[]>

// 创建字典
create_dictionary(request: CreateDictionaryRequest): Promise<string>

// 更新字典
update_dictionary(id: string, request: UpdateDictionaryRequest): Promise<void>

// 删除字典
delete_dictionary(id: string): Promise<void>

// 获取字典内容
get_dictionary_words(id: string, pagination?: Pagination): Promise<string[]>

// 添加词汇
add_dictionary_words(id: string, words: string[]): Promise<void>

// 移除词汇
remove_dictionary_words(id: string, words: string[]): Promise<void>

// 导入字典
import_dictionary(content: string, format: ImportFormat, metadata: DictionaryMetadata): Promise<string>

// 导出字典
export_dictionary(id: string, format: ExportFormat): Promise<string>
```

### 7. 实施计划

#### 阶段1: 数据库重构
1. 创建新的字典表结构
2. 数据迁移脚本（将现有子域名字典迁移到新结构）
3. 更新DatabaseService

#### 阶段2: 服务层重构
1. 实现DictionaryService
2. 重构SubdomainScanner以使用新的字典服务
3. 添加内置字典初始化

#### 阶段3: API接口实现
1. 实现字典管理命令
2. 更新现有扫描工具以支持新字典系统
3. API测试

#### 阶段4: 前端界面开发
1. 字典管理页面
2. 字典集合管理
3. 导入导出功能
4. 集成到现有界面

#### 阶段5: 内置字典和优化
1. 添加各类内置字典
2. 性能优化
3. 文档完善
4. 测试和调试

### 8. 技术考虑

#### 8.1 性能优化
- 字典内容分页加载
- 缓存常用字典
- 索引优化
- 异步加载

#### 8.2 安全考虑
- 字典内容验证
- 导入文件安全检查
- 权限控制
- 敏感信息过滤

#### 8.3 扩展性
- 插件化字典加载
- 在线字典库支持
- 自定义字典格式
- API扩展接口

这个重构将使字典管理系统更加通用、灵活和强大，能够支持各种安全测试场景的需求。

## 9. 实施进度记录

### 已完成项目 ✅

#### 阶段2: 服务层重构 (2024-12-21)
- ✅ 实现了完整的DictionaryService服务层
- ✅ 创建了字典管理相关的数据模型和类型定义
- ✅ 实现了字典的CRUD操作、词条管理、导入导出功能
- ✅ 添加了字典集合管理和内置字典初始化功能
- ✅ 保持了与原有子域名字典API的兼容性

#### 阶段3: API接口实现 (2024-12-21)
- ✅ 实现了完整的字典管理Tauri命令集
- ✅ 包括字典CRUD、词条操作、导入导出、统计、集合管理等功能
- ✅ 更新了模块导入和命令注册
- ✅ 项目编译和测试通过

#### 阶段4: 前端界面开发 (2024-12-21)
- ✅ 创建了完整的字典管理前端页面 (DictionaryManagement.vue)
- ✅ 实现了字典的创建、编辑、删除、复制功能
- ✅ 实现了词条的增删改查、清空、批量操作
- ✅ 实现了字典的导入导出功能
- ✅ 实现了内置字典初始化功能
- ✅ 支持按字典类型筛选和国际化
- ✅ 添加了路由配置和导航链接
- ✅ 更新了中英文国际化翻译
- ✅ 集成到应用主界面和侧边栏

### 技术特点

1. **现代化UI设计**: 使用DaisyUI组件库，界面简洁美观
2. **完整的功能覆盖**: 支持字典的全生命周期管理
3. **良好的用户体验**: 支持拖拽排序、批量操作、实时搜索
4. **国际化支持**: 完整的中英文翻译
5. **类型安全**: 使用TypeScript确保类型安全
6. **响应式设计**: 适配不同屏幕尺寸

### 待完成项目 📋

#### 阶段1: 数据库重构
- ⏳ 创建新的字典表结构
- ⏳ 数据迁移脚本（将现有子域名字典迁移到新结构）
- ⏳ 更新DatabaseService以支持新表结构

#### 阶段5: 内置字典和优化
- ⏳ 添加各类内置字典（子域名、目录、文件名、用户名等）
- ⏳ 性能优化（缓存、分页、索引）
- ⏳ 完善文档和测试

### 系统架构

当前实现的字典管理系统采用了分层架构：

```
前端层 (Vue.js + DaisyUI)
    ↓
API层 (Tauri Commands)
    ↓
服务层 (DictionaryService)
    ↓
数据层 (SQLite Database)
```

### 下一步计划

1. **数据库表结构实现**: 根据设计文档创建实际的数据库表
2. **数据迁移**: 将现有的子域名字典数据迁移到新系统
3. **内置字典**: 添加常用的安全测试字典
4. **性能优化**: 针对大型字典的加载和搜索优化
5. **集成测试**: 确保与现有扫描工具的完美集成