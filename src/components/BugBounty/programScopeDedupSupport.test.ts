import { describe, expect, it } from 'vitest'

import {
  findDuplicateProgramScopeTargets,
  hasDuplicateProgramScopeTarget,
  normalizeProgramScopeIdentityTarget,
  parseProgramScopeTargetLines,
} from './programScopeDedupSupport'

describe('programScopeDedupSupport', () => {
  it('normalizes scope target identity with whitespace and case folded', () => {
    expect(normalizeProgramScopeIdentityTarget('  EXAMPLE.com  ')).toBe('example.com')
  })

  it('detects duplicate targets within the same scope and target type', () => {
    const scopes = [
      { scope_type: 'in_scope', target_type: 'domain', target: 'example.com' },
      { scope_type: 'out_of_scope', target_type: 'domain', target: 'admin.example.com' },
    ]

    expect(
      hasDuplicateProgramScopeTarget(scopes, {
        scope_type: 'in_scope',
        target_type: 'domain',
        target: ' EXAMPLE.COM ',
      }),
    ).toBe(true)
  })

  it('does not treat a target in a different scope type as a duplicate', () => {
    const scopes = [
      { scope_type: 'out_of_scope', target_type: 'domain', target: 'example.com' },
    ]

    expect(
      hasDuplicateProgramScopeTarget(scopes, {
        scope_type: 'in_scope',
        target_type: 'domain',
        target: 'example.com',
      }),
    ).toBe(false)
  })

  it('parses multiple target lines while dropping blank lines and repeated targets', () => {
    expect(parseProgramScopeTargetLines(' example.com \n\nAPI.example.com\nexample.com\n')).toEqual([
      'example.com',
      'API.example.com',
    ])
  })

  it('finds existing duplicates across parsed target lines', () => {
    const scopes = [
      { scope_type: 'in_scope', target_type: 'domain', target: 'example.com' },
      { scope_type: 'in_scope', target_type: 'domain', target: 'admin.example.com' },
      { scope_type: 'out_of_scope', target_type: 'domain', target: 'api.example.com' },
    ]

    expect(
      findDuplicateProgramScopeTargets(
        scopes,
        { scope_type: 'in_scope', target_type: 'domain' },
        ['EXAMPLE.com', 'api.example.com', 'admin.example.com'],
      ),
    ).toEqual(['EXAMPLE.com', 'admin.example.com'])
  })
})
