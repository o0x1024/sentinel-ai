-- Prompt groups schema
-- Groups: one default group per architecture
CREATE TABLE IF NOT EXISTS prompt_groups (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  architecture TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  is_default INTEGER NOT NULL DEFAULT 0,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Ensure at most one default group per architecture via partial unique index
CREATE UNIQUE INDEX IF NOT EXISTS idx_prompt_groups_arch_default
ON prompt_groups(architecture)
WHERE is_default = 1;

-- Group items: stage to template mapping
CREATE TABLE IF NOT EXISTS prompt_group_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id INTEGER NOT NULL,
  stage TEXT NOT NULL,
  template_id INTEGER NOT NULL,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(group_id, stage),
  FOREIGN KEY(group_id) REFERENCES prompt_groups(id) ON DELETE CASCADE,
  FOREIGN KEY(template_id) REFERENCES prompt_templates(id) ON DELETE CASCADE
);



