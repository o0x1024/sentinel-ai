import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Router } from 'vue-router'
import {
  TRAFFIC_LAUNCH_QUEUE_EVENT,
  useTrafficLaunchQueueStore,
} from '@/components/traffic/workbench/stores/useTrafficLaunchQueueStore'
import {
  buildHttpExchangeRequestFromSecurityEvidence,
  openSecurityEvidenceInTrafficWorkbench,
} from './securityEvidenceTransferSupport'
import type { Evidence } from './vulnerabilityFindingTypes'

const createEvidence = (overrides: Partial<Evidence> = {}): Evidence => ({
  id: 'evidence-1',
  vuln_id: 'finding-1',
  url: 'https://example.com/api/users?id=7',
  method: 'POST',
  location: 'response_body',
  evidence_snippet: 'snippet',
  request_headers: JSON.stringify([
    { name: 'Content-Type', value: 'application/json' },
    { name: 'X-Test', value: '1' },
  ]),
  request_body: '{"role":"admin"}',
  response_status: 200,
  response_headers: undefined,
  response_body: undefined,
  timestamp: '2026-04-14T08:00:00.000Z',
  ...overrides,
})

describe('securityEvidenceTransferSupport', () => {
  const launchQueue = useTrafficLaunchQueueStore()

  beforeEach(() => {
    launchQueue.consumeLaunchQueue()
  })

  it('builds repeater-compatible requests from direct evidence payloads', () => {
    const request = buildHttpExchangeRequestFromSecurityEvidence(createEvidence())

    expect(request).toMatchObject({
      absoluteUrl: 'https://example.com/api/users?id=7',
      endpoint: {
        scheme: 'https',
        host: 'example.com',
        port: 443,
      },
      request: {
        method: 'POST',
        target: '/api/users?id=7',
        versionPreference: 'HTTP/1.1',
        bodyText: '{"role":"admin"}',
      },
    })
    expect(request?.request.headers).toEqual([
      { name: 'Content-Type', value: 'application/json' },
      { name: 'X-Test', value: '1' },
    ])
  })

  it('uses baseline request data embedded in system agent context evidence', () => {
    const request = buildHttpExchangeRequestFromSecurityEvidence(
      createEvidence({
        location: 'system_agent_context',
        request_headers: undefined,
        request_body: JSON.stringify({
          sourceEvent: 'traffic.logic',
        }),
        response_body: JSON.stringify({
          baselineRequest: {
            method: 'GET',
            url: '/tenant/alpha/orders?limit=20',
            requestHeaders: JSON.stringify([{ name: 'Cookie', value: 'sid=abc' }]),
            requestBody: '',
          },
        }),
      })
    )

    expect(request).toMatchObject({
      absoluteUrl: 'https://example.com/tenant/alpha/orders?limit=20',
      request: {
        method: 'GET',
        target: '/tenant/alpha/orders?limit=20',
        bodyText: '',
      },
    })
    expect(request?.request.headers).toEqual([{ name: 'Cookie', value: 'sid=abc' }])
  })

  it('returns null when no transferable request can be reconstructed', () => {
    const request = buildHttpExchangeRequestFromSecurityEvidence(
      createEvidence({
        method: '',
        url: '',
        request_headers: undefined,
        request_body: undefined,
        response_status: undefined,
        response_headers: undefined,
        response_body: undefined,
      })
    )

    expect(request).toBeNull()
  })

  it('queues evidence requests for the repeater and notifies active traffic workbench views', async () => {
    const router = { push: vi.fn().mockResolvedValue(undefined) }
    const eventListener = vi.fn()
    window.addEventListener(TRAFFIC_LAUNCH_QUEUE_EVENT, eventListener)

    try {
      const handled = await openSecurityEvidenceInTrafficWorkbench(
        router as unknown as Router,
        createEvidence(),
        'repeater',
      )

      const snapshot = launchQueue.consumeLaunchQueue()
      expect(handled).toBe(true)
      expect(router.push).toHaveBeenCalledWith({ name: 'TrafficAnalysis' })
      expect(snapshot.repeaterRequests).toHaveLength(1)
      expect(snapshot.intruderRequests).toHaveLength(0)
      expect(eventListener).toHaveBeenCalledTimes(1)
    } finally {
      window.removeEventListener(TRAFFIC_LAUNCH_QUEUE_EVENT, eventListener)
    }
  })

  it('queues evidence requests for the intruder', async () => {
    const router = { push: vi.fn().mockResolvedValue(undefined) }

    const handled = await openSecurityEvidenceInTrafficWorkbench(
      router as unknown as Router,
      createEvidence(),
      'intruder',
    )

    const snapshot = launchQueue.consumeLaunchQueue()
    expect(handled).toBe(true)
    expect(snapshot.repeaterRequests).toHaveLength(0)
    expect(snapshot.intruderRequests).toHaveLength(1)
  })
})
