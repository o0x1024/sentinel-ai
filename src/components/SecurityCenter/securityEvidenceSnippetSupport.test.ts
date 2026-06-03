import { describe, expect, it } from 'vitest'
import {
  parseSecurityEvidenceSnippetEntries,
  parseSecurityEvidenceSnippetFields,
} from './securityEvidenceSnippetSupport'

describe('securityEvidenceSnippetSupport', () => {
  it('parses structured snippet fields and labels', () => {
    const fields = parseSecurityEvidenceSnippetFields(
      "location=body | technique=error-based | target_path=contents[0].property_name | probe=utm_source'"
    )
    const entries = parseSecurityEvidenceSnippetEntries(
      "location=body | technique=error-based | target_path=contents[0].property_name | probe=utm_source'"
    )

    expect(fields).toEqual({
      location: 'body',
      technique: 'error-based',
      target_path: 'contents[0].property_name',
      probe: "utm_source'",
    })
    expect(entries).toEqual([
      { key: 'location', label: 'Location', value: 'body' },
      { key: 'technique', label: 'Technique', value: 'error-based' },
      {
        key: 'target_path',
        label: 'Target Path',
        value: 'contents[0].property_name',
      },
      { key: 'probe', label: 'PoC', value: "utm_source'" },
    ])
  })

  it('returns empty structures for plain text snippets', () => {
    expect(parseSecurityEvidenceSnippetFields('plain text snippet')).toEqual({})
    expect(parseSecurityEvidenceSnippetEntries('plain text snippet')).toEqual([])
  })
})
