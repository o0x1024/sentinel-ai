# LLM Compiler 提示词文件说明

## 📦 包含文件

### 1. `llm_compiler_prompts.sql`
**用途**: 数据库导入文件  
**内容**: 包含所有 LLM Compiler 架构的提示词模板和默认配置

**包含的提示词**:
- ✅ Planning 阶段 - 规划器提示词
- ✅ Execution 阶段 - 最终响应生成提示词
- ✅ Evaluation 阶段 - 评估决策提示词
- ✅ Replanning 阶段 - 重规划提示词

### 2. `llm_compiler_prompts_guide.md`
**用途**: 详细使用指南  
**内容**: 提示词设计理念、使用场景、最佳实践、示例代码

## 🚀 快速开始

### 方法1: 通过命令行导入

```bash
# 确保数据库文件存在
cd /Users/a1024/code/ai/sentinel-ai/src-tauri

# 导入提示词
sqlite3 sentinel-ai.db < ../docs/llm_compiler_prompts.sql

# 验证导入
sqlite3 sentinel-ai.db "SELECT COUNT(*) FROM prompt_templates WHERE architecture='LLMCompiler';"
# 应该输出: 4
```

### 方法2: 通过应用界面导入

1. 启动 Sentinel AI 应用
2. 进入 **设置 > 提示词管理**
3. 点击 **导入提示词**
4. 选择 `llm_compiler_prompts.sql` 文件
5. 确认导入

## 📖 查看和编辑

### 在数据库中查看

```sql
-- 查看所有 LLMCompiler 提示词
SELECT id, name, stage, is_default 
FROM prompt_templates 
WHERE architecture = 'LLMCompiler';

-- 查看某个阶段的提示词内容
SELECT name, content 
FROM prompt_templates 
WHERE architecture = 'LLMCompiler' AND stage = 'planner';

-- 查看默认提示词组
SELECT g.name, gi.stage, t.name AS template_name
FROM prompt_groups g
JOIN prompt_group_items gi ON g.id = gi.group_id
JOIN prompt_templates t ON gi.template_id = t.id
WHERE g.architecture = 'LLMCompiler';
```

### 在应用界面中编辑

1. 进入 **设置 > 提示词管理**
2. 选择 **LLMCompiler** 架构
3. 点击要编辑的阶段
4. 修改提示词内容
5. 点击 **保存**

## 🎯 使用场景

### 1. 安全渗透测试
适用于端口扫描、漏洞探测、信息收集等自动化任务

```
示例任务:
"扫描以下3个目标的常见端口：192.168.1.1, 192.168.1.2, 192.168.1.3"

LLM Compiler 会:
1. 生成3个并行端口扫描任务
2. 同时执行所有扫描
3. 评估结果完整性
4. 如需要，重规划深度扫描
```

### 2. 子域名枚举
并行查询多个 DNS 服务器，提高效率

```
示例任务:
"枚举 example.com 的所有子域名"

LLM Compiler 会:
1. 生成多个并行枚举任务（不同DNS、不同字典）
2. 合并去重结果
3. 评估是否需要补充查询
4. 生成完整子域名列表
```

### 3. Web应用扫描
先探测服务，再根据结果进行深度扫描

```
示例任务:
"扫描 https://example.com 的安全问题"

LLM Compiler 会:
1. 先执行基础信息收集
2. 根据技术栈选择合适的扫描工具
3. 并行扫描不同漏洞类型
4. 汇总生成安全报告
```

## ⚙️ 自定义提示词

### 创建自定义版本

```sql
-- 复制默认提示词创建自定义版本
INSERT INTO prompt_templates (name, description, architecture, stage, content, is_default, is_active)
SELECT 
    'LLMCompiler 规划器 - 快速模式',
    '优化速度的规划器版本，减少任务数量',
    architecture,
    stage,
    -- 在这里修改 content
    REPLACE(content, '建议 3-10 个', '建议 2-5 个'),
    0,  -- 不设为默认
    1
FROM prompt_templates
WHERE architecture = 'LLMCompiler' AND stage = 'planner' AND is_default = 1;
```

### 创建场景专用提示词组

```sql
-- 创建"快速扫描"场景的提示词组
INSERT INTO prompt_groups (architecture, name, description, is_default)
VALUES (
    'LLMCompiler',
    'LLMCompiler 快速扫描模式',
    '优化速度的提示词组合，适用于快速扫描场景',
    0
);

-- 关联自定义提示词
INSERT INTO prompt_group_items (group_id, stage, template_id)
SELECT 
    (SELECT id FROM prompt_groups WHERE name = 'LLMCompiler 快速扫描模式'),
    'planner',
    (SELECT id FROM prompt_templates WHERE name = 'LLMCompiler 规划器 - 快速模式');
```

## 🔧 故障排除

### 问题1: 导入失败

```bash
# 检查文件是否存在
ls -lh docs/llm_compiler_prompts.sql

# 检查数据库连接
sqlite3 src-tauri/sentinel-ai.db "SELECT COUNT(*) FROM prompt_templates;"

# 清理后重新导入（谨慎使用）
sqlite3 src-tauri/sentinel-ai.db "DELETE FROM prompt_templates WHERE architecture='LLMCompiler';"
sqlite3 src-tauri/sentinel-ai.db < docs/llm_compiler_prompts.sql
```

### 问题2: 提示词未生效

```sql
-- 检查是否为默认提示词
SELECT stage, is_default, is_active 
FROM prompt_templates 
WHERE architecture='LLMCompiler';

-- 检查提示词组配置
SELECT * FROM prompt_groups WHERE architecture='LLMCompiler';

-- 确保 is_default=1 且 is_active=1
UPDATE prompt_templates 
SET is_default=1, is_active=1 
WHERE architecture='LLMCompiler';
```

### 问题3: 执行结果不符合预期

1. 查看日志确认使用的提示词版本
```bash
tail -f src-tauri/logs/sentinel-ai.log | grep "LLMCompiler"
```

2. 在应用中查看实际发送给 LLM 的提示词
   - 进入**提示词管理** > **调试模式**
   - 执行测试任务
   - 查看完整的 prompt 内容

3. 调整提示词内容并重试

## 📊 性能建议

### 优化并行度
```
轻量任务（信息查询）: 并行度 3-5
中等任务（端口扫描）: 并行度 2-3
重型任务（漏洞扫描）: 并行度 1-2
```

### 优化任务数量
```
简单目标: 3-5个任务
中等复杂度: 5-8个任务
复杂目标: 8-10个任务（不建议超过10）
```

### 优化超时设置
```
快速查询: 5-10秒
常规扫描: 15-30秒
深度扫描: 30-60秒
```

## 📚 延伸阅读

- 详细使用指南: `llm_compiler_prompts_guide.md`
- LLM Compiler 架构文档: `../src-tauri/docs/llm_compiler_architecture.md`
- 提示词工程最佳实践: `../docs/prompt_engineering_guide.md`

## 🤝 反馈和贡献

如果你有任何问题或改进建议：

1. 提交 Issue: 描述问题或建议
2. 提交 PR: 改进提示词模板
3. 分享经验: 在讨论区分享使用心得

## 📝 版本历史

- **v1.0.0** (2025-11-13)
  - 初始版本
  - 包含4个核心阶段的提示词
  - 支持安全测试场景优化

---

**维护者**: Sentinel AI Team  
**最后更新**: 2025-11-13  
**状态**: ✅ 已测试，可用于生产环境

