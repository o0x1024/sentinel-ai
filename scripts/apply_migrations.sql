-- 应用数据库迁移脚本
.read src-tauri/migrations/20250916_enhance_prompt_templates.sql

-- 查看更新后的表结构
.schema prompt_templates

-- 查看现有数据
SELECT id, name, category, template_type, priority, tags, variables, version FROM prompt_templates LIMIT 10;
