import type { CodecRequestMeta } from '../codec/trafficCodecTypes'
import { extractCodecMetaFromRawRequest } from '../codec/trafficCodecContextMenuSupport'
import type { useTrafficCodec } from '../codec/useTrafficCodec'

export interface IntruderCodecSession {
  originalRaw: string
  meta: CodecRequestMeta
  appliedRuleIds: string[]
}

type TrafficCodec = ReturnType<typeof useTrafficCodec>

export async function decodeIntruderRequestTemplate(
  rawRequest: string,
  targetHost: string,
  codec: TrafficCodec,
): Promise<{ requestText: string; session: IntruderCodecSession | null }> {
  if (!rawRequest.trim() || !codec.codecViewEnabled.value) {
    return { requestText: rawRequest, session: null }
  }

  const meta = extractCodecMetaFromRawRequest(rawRequest, targetHost)
  if (!codec.hasActiveCodec(meta)) {
    return { requestText: rawRequest, session: null }
  }

  const result = await codec.decode(rawRequest, meta)
  if (!result.success || result.appliedRuleIds.length === 0) {
    return { requestText: rawRequest, session: null }
  }

  return {
    requestText: result.content,
    session: {
      originalRaw: rawRequest,
      meta,
      appliedRuleIds: result.appliedRuleIds,
    },
  }
}

export async function encodeIntruderRequestText(
  requestText: string,
  session: IntruderCodecSession | null | undefined,
  codec: TrafficCodec,
): Promise<string> {
  if (!session?.appliedRuleIds.length) {
    return requestText
  }

  const result = await codec.encode(requestText, session.meta)
  if (!result.success) {
    throw new Error(result.error || 'Codec encode failed')
  }

  return result.appliedRuleIds.length > 0 ? result.content : requestText
}

export async function batchEncodeIntruderRequestTexts(
  requestTexts: string[],
  session: IntruderCodecSession | null | undefined,
  codec: TrafficCodec,
): Promise<string[]> {
  if (!session?.appliedRuleIds.length || requestTexts.length === 0) {
    return requestTexts
  }

  const results = await codec.batchEncode(requestTexts, session.meta)
  return requestTexts.map((text, index) => {
    const result = results[index]
    if (!result?.success) {
      throw new Error(result?.error || 'Codec encode failed')
    }
    return result.appliedRuleIds.length > 0 ? result.content : text
  })
}
