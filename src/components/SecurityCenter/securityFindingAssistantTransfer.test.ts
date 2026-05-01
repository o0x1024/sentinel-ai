import { beforeEach, describe, expect, it } from 'vitest'
import {
  buildReferencedSecurityFinding,
  consumePendingSecurityFindingAssistantAssets,
  queueSecurityFindingsForAssistant,
} from './securityFindingAssistantTransfer'
import type { Finding } from './vulnerabilityFindingTypes'

const createFinding = (description = 'description'): Finding => ({
  id: 'finding-1',
  plugin_id: 'agent:logic',
  vuln_type: 'idor',
  severity: 'high',
  confidence: 'high',
  title: 'IDOR in order detail',
  description,
  cwe: 'CWE-639',
  owasp: 'A01',
  remediation: 'Check tenant ownership before returning the order.',
  status: 'open',
  url: 'https://example.com/orders/7',
  method: 'GET',
  hit_count: 2,
  first_seen_at: '2026-04-14T08:00:00.000Z',
  last_seen_at: '2026-04-14T08:05:00.000Z',
  created_at: '2026-04-14T08:00:00.000Z',
  updated_at: '2026-04-14T08:05:00.000Z',
  evidence: [
    {
      id: 'evidence-1',
      vuln_id: 'finding-1',
      url: 'https://example.com/orders/7',
      method: 'GET',
      location: 'response_body',
      evidence_snippet: 'tenant B order returned to tenant A user',
      response_status: 200,
      timestamp: '2026-04-14T08:00:00.000Z',
    },
  ],
})

describe('securityFindingAssistantTransfer', () => {
  beforeEach(() => {
    consumePendingSecurityFindingAssistantAssets()
  })

  it('builds an AI assistant reference from finding details', () => {
    const asset = buildReferencedSecurityFinding(createFinding())

    expect(asset).toMatchObject({
      id: 'security-finding:finding-1',
      name: 'IDOR in order detail',
      value: 'https://example.com/orders/7',
      asset_type: 'security_finding',
      risk_level: 'high',
      status: 'open',
    })
    expect(asset.tags).toEqual(['idor', 'CWE-639', 'A01', 'confidence:high'])
    expect(asset.metadata?.evidence).toHaveLength(1)
  })

  it('queues and consumes pending finding references once', () => {
    queueSecurityFindingsForAssistant([createFinding()])

    expect(consumePendingSecurityFindingAssistantAssets()).toHaveLength(1)
    expect(consumePendingSecurityFindingAssistantAssets()).toHaveLength(0)
  })
})
