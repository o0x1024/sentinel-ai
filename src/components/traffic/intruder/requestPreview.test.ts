import { describe, expect, it } from 'vitest'
import { buildRequestPreviewDiff, buildRequestPreviewTraceDiffs } from './requestPreview'

describe('buildRequestPreviewDiff', () => {
  it('captures query parameter changes', () => {
    const before = [
      'GET /search?q=test&page=1 HTTP/1.1',
      'Host: example.com',
      '',
      '',
    ].join('\r\n')
    const after = [
      'GET /search?q=admin&page=1&nonce=abc HTTP/1.1',
      'Host: example.com',
      '',
      '',
    ].join('\r\n')

    const diff = buildRequestPreviewDiff(before, after)

    expect(diff.queryParameterDiff.unchangedCount).toBe(1)
    expect(diff.queryParameterDiff.changes).toEqual([
      {
        id: 'q-0',
        label: 'q',
        kind: 'changed',
        before: 'test',
        after: 'admin',
      },
      {
        id: 'nonce-0',
        label: 'nonce',
        kind: 'added',
        before: '',
        after: 'abc',
      },
    ])
  })

  it('captures form body parameter changes when content-type is form-urlencoded', () => {
    const before = [
      'POST /login HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/x-www-form-urlencoded',
      '',
      'user=alice&ts=1&sign=old',
    ].join('\r\n')
    const after = [
      'POST /login HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/x-www-form-urlencoded; charset=utf-8',
      '',
      'user=alice&ts=2&sign=new',
    ].join('\r\n')

    const diff = buildRequestPreviewDiff(before, after)

    expect(diff.formParameterDiff.unchangedCount).toBe(1)
    expect(diff.formParameterDiff.changes).toEqual([
      {
        id: 'ts-0',
        label: 'ts',
        kind: 'changed',
        before: '1',
        after: '2',
      },
      {
        id: 'sign-0',
        label: 'sign',
        kind: 'changed',
        before: 'old',
        after: 'new',
      },
    ])
  })

  it('handles repeated parameter names by index', () => {
    const before = [
      'GET /items?id=1&id=2 HTTP/1.1',
      'Host: example.com',
      '',
      '',
    ].join('\r\n')
    const after = [
      'GET /items?id=1&id=3&id=4 HTTP/1.1',
      'Host: example.com',
      '',
      '',
    ].join('\r\n')

    const diff = buildRequestPreviewDiff(before, after)

    expect(diff.queryParameterDiff.unchangedCount).toBe(1)
    expect(diff.queryParameterDiff.changes).toEqual([
      {
        id: 'id-1',
        label: 'id',
        kind: 'changed',
        before: '2',
        after: '3',
      },
      {
        id: 'id-2',
        label: 'id',
        kind: 'added',
        before: '',
        after: '4',
      },
    ])
  })

  it('builds trace diffs against the previous processor stage', () => {
    const original = [
      'POST /login?mode=basic HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/x-www-form-urlencoded',
      '',
      'user=alice&ts=1&sign=old',
    ].join('\r\n')
    const afterFirst = [
      'POST /login?mode=advanced HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/x-www-form-urlencoded',
      '',
      'user=alice&ts=1&sign=old',
    ].join('\r\n')
    const afterSecond = [
      'POST /login?mode=advanced HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/x-www-form-urlencoded',
      '',
      'user=alice&ts=2&sign=new',
    ].join('\r\n')

    const traceDiffs = buildRequestPreviewTraceDiffs(original, [afterFirst, afterSecond])

    expect(traceDiffs).toHaveLength(2)
    expect(traceDiffs[0].diff.queryParameterDiff.changes).toEqual([
      {
        id: 'mode-0',
        label: 'mode',
        kind: 'changed',
        before: 'basic',
        after: 'advanced',
      },
    ])
    expect(traceDiffs[0].diff.formParameterDiff.changes).toEqual([])
    expect(traceDiffs[1].diff.queryParameterDiff.changes).toEqual([])
    expect(traceDiffs[1].diff.formParameterDiff.changes).toEqual([
      {
        id: 'ts-0',
        label: 'ts',
        kind: 'changed',
        before: '1',
        after: '2',
      },
      {
        id: 'sign-0',
        label: 'sign',
        kind: 'changed',
        before: 'old',
        after: 'new',
      },
    ])
  })
})
