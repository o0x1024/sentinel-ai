import type { RuleStarterTemplate } from '@/components/Dictionary/ruleStarterTemplates.types'
import {
  genericFingerprintStarterTemplates,
  serviceIdentificationStarterTemplates,
} from '@/components/Dictionary/ruleStarterTemplates.service'
import { webFingerprintStarterTemplates } from '@/components/Dictionary/ruleStarterTemplates.web'
import { riskVerificationStarterTemplates } from '@/components/Dictionary/ruleStarterTemplates.poc'

export type { RuleStarterTemplate } from '@/components/Dictionary/ruleStarterTemplates.types'

const starterTemplates: Record<string, RuleStarterTemplate[]> = {
  'service_probe_rule:service_identification': serviceIdentificationStarterTemplates,
  'fingerprint_rule:service_identification': serviceIdentificationStarterTemplates,
  'fingerprint_rule:web_fingerprint': webFingerprintStarterTemplates,
  'fingerprint_rule:generic_fingerprint': genericFingerprintStarterTemplates,
  'poc_rule:risk_verification': riskVerificationStarterTemplates,
}

export function getRuleStarterTemplates(dictionaryType: string, dictionarySubtype?: string): RuleStarterTemplate[] {
  if (!dictionaryType || !dictionarySubtype) return []
  return starterTemplates[`${dictionaryType}:${dictionarySubtype}`] || []
}

export function getRuleStarterTemplate(dictionaryType: string, dictionarySubtype: string | undefined, key: string): RuleStarterTemplate | null {
  return getRuleStarterTemplates(dictionaryType, dictionarySubtype).find(template => template.key === key) || null
}
