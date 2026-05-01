import type { ReferencedAsset } from '@/types/agentReferences'
import type { Evidence, Finding } from './vulnerabilityFindingTypes'

const MAX_TEXT_FIELD_LENGTH = 4000

let pendingSecurityFindingAssets: ReferencedAsset[] = []

const truncateText = (value?: string | null) => {
  if (!value) return undefined
  if (value.length <= MAX_TEXT_FIELD_LENGTH) return value
  return `${value.slice(0, MAX_TEXT_FIELD_LENGTH)}... [truncated]`
}

const compactEvidence = (evidence: Evidence) => ({
  id: evidence.id,
  url: evidence.url,
  method: evidence.method,
  location: evidence.location,
  evidence_snippet: truncateText(evidence.evidence_snippet),
  request_headers: truncateText(evidence.request_headers),
  request_body: truncateText(evidence.request_body),
  response_status: evidence.response_status,
  response_headers: truncateText(evidence.response_headers),
  response_body: truncateText(evidence.response_body),
  timestamp: evidence.timestamp,
})

export function buildReferencedSecurityFinding(finding: Finding): ReferencedAsset {
  const tags = [
    finding.vuln_type,
    finding.cwe,
    finding.owasp,
    finding.confidence ? `confidence:${finding.confidence}` : '',
  ].filter((item): item is string => Boolean(item))

  return {
    id: `security-finding:${finding.id}`,
    name: finding.title || finding.id,
    value: finding.url || finding.id,
    asset_type: 'security_finding',
    mentionText: `漏洞:${finding.title || finding.id}`,
    risk_level: finding.severity,
    status: finding.status,
    description: truncateText(finding.description),
    tags,
    metadata: {
      finding_id: finding.id,
      plugin_id: finding.plugin_id,
      vulnerability_type: finding.vuln_type,
      severity: finding.severity,
      confidence: finding.confidence,
      method: finding.method,
      url: finding.url,
      hit_count: finding.hit_count,
      first_seen_at: finding.first_seen_at,
      last_seen_at: finding.last_seen_at,
      created_at: finding.created_at,
      updated_at: finding.updated_at,
      analysis_stage: finding.analysisStage,
      analysis_stage_label: finding.analysisStageLabel,
      remediation: truncateText(finding.remediation),
      evidence: (finding.evidence || []).map(compactEvidence),
    },
  }
}

export function queueSecurityFindingsForAssistant(findings: Finding[]) {
  pendingSecurityFindingAssets.push(...findings.map(buildReferencedSecurityFinding))
}

export function consumePendingSecurityFindingAssistantAssets() {
  const assets = pendingSecurityFindingAssets
  pendingSecurityFindingAssets = []
  return assets
}
