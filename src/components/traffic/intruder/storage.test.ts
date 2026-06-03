import { describe, expect, it } from 'vitest'
import { createDefaultGrepPayloadSettings } from './analysis'
import {
  buildIntruderResourcePoolAutoName,
  createBuiltInResourcePools,
  exportIntruderResultsCsv,
  loadIntruderResourcePools,
  upsertIntruderResourcePoolEntry,
} from './storage'
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

  it('builds an automatic name from the resource pool config', () => {
    expect(buildIntruderResourcePoolAutoName(createBuiltInResourcePools())).toBe('资源池 1')
  })

  it('reuses an existing pool when the config already exists', () => {
    const builtIns = createBuiltInResourcePools()
    const result = upsertIntruderResourcePoolEntry(builtIns, {
      concurrencyEnabled: true,
      concurrency: 10,
      delayEnabled: false,
      delayMs: 0,
      randomDelayEnabled: false,
      randomDelayMs: 0,
      delayIncrementEnabled: false,
      delayIncrementMs: 0,
      autoThrottleEnabled: false,
      autoThrottleStatusCodes: [429, 503],
    })

    expect(result.pools).toHaveLength(1)
    expect(result.pool.id).toBe('default')
  })

  it('creates a custom pool with an automatic name when the config is new', () => {
    const result = upsertIntruderResourcePoolEntry(createBuiltInResourcePools(), {
      concurrencyEnabled: true,
      concurrency: 7,
      delayEnabled: true,
      delayMs: 120,
      randomDelayEnabled: false,
      randomDelayMs: 0,
      delayIncrementEnabled: false,
      delayIncrementMs: 0,
      autoThrottleEnabled: true,
      autoThrottleStatusCodes: [429],
    })

    expect(result.pools).toHaveLength(2)
    expect(result.pool).toMatchObject({
      name: '资源池 1',
      concurrency: 7,
      delayMs: 120,
      autoThrottleEnabled: true,
      autoThrottleStatusCodes: [429],
    })
  })

  it('keeps the same auto-generated name when updating an existing custom pool', () => {
    const result = upsertIntruderResourcePoolEntry([
      ...createBuiltInResourcePools(),
      {
        id: 'resource-pool-1',
        name: '资源池 3',
        concurrencyEnabled: true,
        concurrency: 3,
        delayEnabled: false,
        delayMs: 0,
        randomDelayEnabled: false,
        randomDelayMs: 0,
        delayIncrementEnabled: false,
        delayIncrementMs: 0,
        autoThrottleEnabled: false,
        autoThrottleStatusCodes: [],
        builtIn: false,
      },
    ], {
      id: 'resource-pool-1',
      concurrencyEnabled: true,
      concurrency: 8,
      delayEnabled: false,
      delayMs: 0,
      randomDelayEnabled: false,
      randomDelayMs: 0,
      delayIncrementEnabled: false,
      delayIncrementMs: 0,
      autoThrottleEnabled: false,
      autoThrottleStatusCodes: [],
    })

    expect(result.pool.name).toBe('资源池 3')
  })
})
