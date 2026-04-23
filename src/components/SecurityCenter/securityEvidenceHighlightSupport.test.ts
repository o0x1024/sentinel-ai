import { describe, expect, it } from 'vitest'
import {
  buildSecurityEvidenceRequestHtml,
  buildSecurityEvidenceResponseHtml,
  extractSecurityEvidenceHighlightBuckets,
  extractSecurityEvidenceSnippetFields,
} from './securityEvidenceHighlightSupport'

describe('securityEvidenceHighlightSupport', () => {
  it('extracts poc and matched response terms from evidence snippet', () => {
    const buckets = extractSecurityEvidenceHighlightBuckets(
      'location=body | technique=error-based | probe=utm_source\' | sql_error=DB::Exception',
    )
    const fields = extractSecurityEvidenceSnippetFields(
      'location=body | technique=error-based | target_path=contents[0].property_name | reference_status=200 | probe_status=500 | probe=utm_source\' | sql_error=DB::Exception',
    )

    expect(buckets.requestTerms).toEqual(["utm_source'"])
    expect(buckets.responseTerms).toEqual(['DB::Exception'])
    expect(fields.location).toBe('body')
    expect(fields.technique).toBe('error-based')
    expect(fields.targetPath).toBe('contents[0].property_name')
    expect(fields.referenceStatus).toBe('200')
    expect(fields.probeStatus).toBe('500')
  })

  it('highlights request and response raw text with extracted terms', () => {
    const evidenceSnippet = 'probe=utm_source\' | sql_error=DB::Exception'
    const requestHtml = buildSecurityEvidenceRequestHtml(
      'POST /demo HTTP/1.1\r\n\r\n{"property_name":"utm_source\'"}',
      evidenceSnippet,
    )
    const responseHtml = buildSecurityEvidenceResponseHtml(
      'HTTP/1.1 200 OK\r\n\r\n{"error_message":"DB::Exception"}',
      evidenceSnippet,
    )

    expect(requestHtml).toContain('security-evidence-hit--request')
    expect(requestHtml).toContain('utm_source&#39;')
    expect(responseHtml).toContain('security-evidence-hit--response')
    expect(responseHtml).toContain('DB::Exception')
  })
})
