import { describe, expect, it } from 'vitest'
import { createDefaultGrepPayloadSettings } from './analysis'
import { createBuiltInResourcePools, exportIntruderResultsCsv, loadIntruderResourcePools } from './storage'
import type { IntruderAttackResult } from './types'

function createResult(): IntruderAttackResult {
  return {
    id: 'result-1',
    index: 1,
    payloadSummary: 'P1=admin',
    payloadValues: ['admin'],
    statusCode: 200,
    responseLength: 512,
    wordCount: 10,
    lineCount: 4,
    responseTimeMs: 120,
    rawRequest: 'GET / HTTP/1.1',
    rawResponse: 'HTTP/1.1 200 OK',
    redirectCount: 0,
    finalUrl: 'https://example.com/',
    redirectChain: [],
    payloadReflectionCount: 0,
    grepMatches: {},
    grepExtracts: {},
  }
}

describe('intruder result export', () => {
  it('exports columns in the selected order', () => {
    const csv = exportIntruderResultsCsv(
      [createResult()],
      [],
      [],
      createDefaultGrepPayloadSettings(),
      ['statusCode', 'index', 'payloadSummary'],
    )

    expect(csv).toContain('Status,Request,Payload')
    expect(csv).toContain('200,0,P1=admin')
  })

  it('exports the payload reflection column when enabled', () => {
    const csv = exportIntruderResultsCsv(
      [{ ...createResult(), payloadReflectionCount: 3 }],
      [],
      [],
      { ...createDefaultGrepPayloadSettings(), enabled: true },
      ['payloadReflectionCount', 'statusCode'],
    )

    expect(csv).toContain('Reflections,Status')
    expect(csv).toContain('3,200')
  })
})

describe('intruder resource pools', () => {
  it('defines only the default built-in pool with throttling metadata', () => {
    const pools = createBuiltInResourcePools()
    expect(pools).toHaveLength(1)
    expect(pools[0]).toMatchObject({
      id: 'default',
      concurrencyEnabled: true,
      autoThrottleEnabled: false,
      autoThrottleStatusCodes: [429, 503],
    })
  })

  it('normalizes legacy persisted custom pools', () => {
    localStorage.setItem('trafficAnalysis.intruder.resourcePools.v1', JSON.stringify([
      { id: 'custom-1', name: 'Legacy', concurrency: 7, delayMs: 120, randomDelayMs: 30 },
    ]))

    const pools = loadIntruderResourcePools(createBuiltInResourcePools())
    const legacyPool = pools.find((pool) => pool.id === 'custom-1')
    localStorage.removeItem('trafficAnalysis.intruder.resourcePools.v1')

    expect(legacyPool).toMatchObject({
      id: 'custom-1',
      name: 'Legacy',
      concurrencyEnabled: true,
      concurrency: 7,
      delayEnabled: false,
      autoThrottleEnabled: false,
    })
  })
})
