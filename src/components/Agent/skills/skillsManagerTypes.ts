import type { SuppressionRuleCategory } from '../skillsManagerHelpers'

export type { SuppressionRuleCategory } from '../skillsManagerHelpers'

export interface Skill {
  id: string
  name: string
  description: string
  source_path: string
  argument_hint: string
  disable_model_invocation: boolean
  user_invocable: boolean
  allowed_tools: string[]
  model: string
  context: string
  agent: string
  hooks: Record<string, any> | null
  content?: string
  created_at?: string
  updated_at?: string
}

export interface SkillForm extends Skill {
  content: string
  hooks_raw: string
}

export interface SkillCandidate {
  id: string
  title: string
  suggested_skill_name: string
  description: string
  content: string
  memory_kind: string
  source_memory_id?: string | null
  scope: string
  source: string
  stability: string
  confidence: number
  memory_excerpt: string
  capture_reasons: string[]
  status: string
  promoted_skill_id?: string | null
  review_note?: string | null
  reviewed_at_ms?: number | null
  created_at_ms: number
  updated_at_ms: number
}

export interface SkillCandidateSuppressionRule {
  id: string
  memory_kind: string
  source: string
  category: SuppressionRuleCategory | string
  note?: string | null
  excerpt: string
  match_terms: string[]
  hit_count?: number
  last_matched_at_ms?: number | null
  last_matched_excerpt?: string | null
  ttl_days?: number
  expires_at_ms?: number | null
  created_at_ms: number
  updated_at_ms: number
}

export interface SkillFileEntry {
  path: string
  size: number
}

export interface RejectionCategoryOption {
  category: SuppressionRuleCategory
  label: string
  buttonClass: string
  icon: string
}
