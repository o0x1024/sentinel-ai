-- Enhance prompt_templates table with unified prompt system fields
-- Created: 2025-09-16
-- Description: Add extended fields for category, template_type, priority, tags, variables, version, etc.

-- Add new columns to existing prompt_templates table
ALTER TABLE prompt_templates ADD COLUMN category TEXT;
ALTER TABLE prompt_templates ADD COLUMN template_type TEXT;
ALTER TABLE prompt_templates ADD COLUMN target_architecture TEXT;
ALTER TABLE prompt_templates ADD COLUMN is_system INTEGER DEFAULT 0;
ALTER TABLE prompt_templates ADD COLUMN priority INTEGER DEFAULT 50;
ALTER TABLE prompt_templates ADD COLUMN tags TEXT; -- JSON array string
ALTER TABLE prompt_templates ADD COLUMN variables TEXT; -- JSON array string
ALTER TABLE prompt_templates ADD COLUMN version TEXT DEFAULT '1.0.0';

-- Create index for better query performance
CREATE INDEX IF NOT EXISTS idx_prompt_templates_category ON prompt_templates(category);
CREATE INDEX IF NOT EXISTS idx_prompt_templates_template_type ON prompt_templates(template_type);
CREATE INDEX IF NOT EXISTS idx_prompt_templates_is_system ON prompt_templates(is_system);
CREATE INDEX IF NOT EXISTS idx_prompt_templates_priority ON prompt_templates(priority);

-- Update existing templates to set default category as 'LlmArchitecture'
UPDATE prompt_templates 
SET category = 'LlmArchitecture' 
WHERE category IS NULL;

-- Set template_type based on stage for existing records
UPDATE prompt_templates 
SET template_type = 'Planner' 
WHERE stage IN ('planner', 'planning') AND template_type IS NULL;

UPDATE prompt_templates 
SET template_type = 'Executor' 
WHERE stage IN ('worker', 'execution') AND template_type IS NULL;

UPDATE prompt_templates 
SET template_type = 'Replanner' 
WHERE stage = 'replan' AND template_type IS NULL;

UPDATE prompt_templates 
SET template_type = 'Evaluator' 
WHERE stage = 'solver' AND template_type IS NULL;

-- Set default priority and tags for existing templates
UPDATE prompt_templates 
SET priority = 100, tags = '[]' 
WHERE priority IS NULL OR tags IS NULL;
