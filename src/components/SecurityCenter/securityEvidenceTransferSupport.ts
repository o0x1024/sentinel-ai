import type { Router } from 'vue-router'
import type { HttpExchangeRequest } from '@/components/traffic/http/model'
import { parseStoredHeaderEntries } from '@/components/traffic/http/headers'
import { endpointFromUrl } from '@/components/traffic/http/url'
import { DEFAULT_HTTP_VERSION } from '@/components/traffic/http/version'
import { useTrafficLaunchQueueStore } from '@/components/traffic/workbench/stores/useTrafficLaunchQueueStore'
import { getWorkbenchEvidenceExchange } from './securityWorkbenchSystemAgentContent'
import type { Evidence } from './vulnerabilityFindingTypes'

export type SecurityEvidenceTransferTarget = 'draft' | 'attackWorkspace'
export interface SecurityEvidenceTransferMessages {
  triggerLabel: string
  createDraft: string
  createAttackWorkspace: string
  noTransferableRequest: string
  draftCreated: string
  attackWorkspaceCreated: string
  transferFailed: string
}

const isHttpUrl = (value?: string | null) => Boolean(value && /^https?:\/\//i.test(value.trim()))

const resolveAbsoluteRequestUrl = (requestUrl?: string | null, fallbackUrl?: string | null) => {
  const normalizedRequestUrl = requestUrl?.trim() || ''
  const normalizedFallbackUrl = fallbackUrl?.trim() || ''

  if (isHttpUrl(normalizedRequestUrl)) {
    return normalizedRequestUrl
  }

  if (!isHttpUrl(normalizedFallbackUrl)) {
    return ''
  }

  if (!normalizedRequestUrl) {
    return normalizedFallbackUrl
  }

  try {
    return new URL(normalizedRequestUrl, normalizedFallbackUrl).toString()
  } catch {
    return ''
  }
}

export function buildHttpExchangeRequestFromSecurityEvidence(
  evidence: Evidence
): HttpExchangeRequest | null {
  const exchange = getWorkbenchEvidenceExchange(evidence)
  if (!exchange) {
    return null
  }

  const absoluteUrl = resolveAbsoluteRequestUrl(exchange.requestUrl, evidence.url)
  if (!absoluteUrl) {
    return null
  }

  try {
    const parsedUrl = new URL(absoluteUrl)
    return {
      endpoint: endpointFromUrl(parsedUrl.toString()),
      absoluteUrl: parsedUrl.toString(),
      request: {
        method: exchange.requestMethod || evidence.method || 'GET',
        target: `${parsedUrl.pathname || '/'}${parsedUrl.search}`,
        versionPreference: DEFAULT_HTTP_VERSION,
        headers: parseStoredHeaderEntries(exchange.requestHeaders || undefined),
        bodyText: exchange.requestBody || '',
      },
    }
  } catch {
    return null
  }
}

export function findFirstTransferableSecurityEvidence(
  evidences: Evidence[] | null | undefined
): Evidence | null {
  if (!evidences?.length) {
    return null
  }

  return (
    evidences.find(evidence => Boolean(buildHttpExchangeRequestFromSecurityEvidence(evidence))) ||
    null
  )
}

export async function openSecurityEvidenceInTrafficWorkbench(
  router: Router,
  evidence: Evidence,
  target: SecurityEvidenceTransferTarget
): Promise<boolean> {
  const request = buildHttpExchangeRequestFromSecurityEvidence(evidence)
  if (!request) {
    return false
  }

  const launchQueue = useTrafficLaunchQueueStore()
  if (target === 'draft') {
    launchQueue.queueDraftRequest(request)
  } else {
    launchQueue.queueAttackWorkspaceRequest(request)
  }

  await router.push({ name: 'TrafficAnalysis' })
  return true
}
