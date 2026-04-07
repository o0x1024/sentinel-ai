/**
 * Example Intruder request processor plugin.
 * @plugin intruder_hmac_request_signer
 * @name Intruder HMAC Request Signer
 * @main_category intruder
 * @category request_processor
 *
 * Import this file into Plugin Management, then set:
 * - main category: intruder
 * - category: request_processor
 */

interface ToolInput {
  rawRequest: string
  config?: {
    signatureField?: string
    timestampField?: string
    secret?: string
    algorithm?: 'hmac-sha256'
    location?: 'query' | 'body'
    sortKeys?: boolean
    excludeFields?: string[]
    timestampMode?: 'seconds' | 'milliseconds' | 'fixed'
    fixedTimestamp?: string
  }
}

interface ToolOutput {
  success: boolean
  data?: {
    rawRequest: string
    canonical?: string
    signature?: string
  }
  error?: string
}

export function get_input_schema() {
  return {
    type: 'object',
    properties: {
      config: {
        type: 'object',
        required: ['secret'],
        properties: {
          secret: {
            type: 'string',
            description: 'HMAC secret key',
            'x-ui-group': {
              key: 'signing',
              label: 'Signing',
              description: 'Configure how the signature is calculated.',
            },
          },
          signatureField: {
            type: 'string',
            default: 'sign',
            description: 'Parameter name used to store the signature',
            'x-ui-group': 'request_target',
          },
          timestampField: {
            type: 'string',
            default: 'ts',
            description: 'Parameter name used to store the timestamp',
            'x-ui-group': 'timestamp',
          },
          algorithm: {
            type: 'string',
            enum: ['hmac-sha256'],
            default: 'hmac-sha256',
            description: 'Signing algorithm',
            'x-ui-group': 'signing',
            'x-ui-enum-descriptions': {
              'hmac-sha256': 'Uses HMAC-SHA256 to calculate the signature over the canonical parameter string.',
            },
          },
          location: {
            type: 'string',
            enum: ['query', 'body'],
            default: 'body',
            description: 'Where to read and rewrite parameters',
            'x-ui-group': {
              key: 'request_target',
              label: 'Request Target',
              description: 'Choose where the signer reads and rewrites parameters.',
            },
            'x-ui-enum-descriptions': {
              query: 'Read parameters from the URL query string and write the updated signature back to the URL.',
              body: 'Read parameters from an application/x-www-form-urlencoded body and write the updated signature back to the body.',
            },
          },
          sortKeys: {
            type: 'boolean',
            default: true,
            description: 'Sort keys before canonicalization',
            'x-ui-group': 'signing',
          },
          excludeFields: {
            type: 'array',
            items: { type: 'string' },
            description: 'Fields excluded from canonicalization',
            'x-ui-group': 'signing',
          },
          timestampMode: {
            type: 'string',
            enum: ['seconds', 'milliseconds', 'fixed'],
            default: 'seconds',
            description: 'How to populate the timestamp field',
            'x-ui-group': {
              key: 'timestamp',
              label: 'Timestamp',
              description: 'Control how the timestamp field is populated.',
            },
            'x-ui-enum-descriptions': {
              seconds: 'Use the current Unix timestamp in seconds for each request.',
              milliseconds: 'Use the current Unix timestamp in milliseconds for each request.',
              fixed: 'Use the exact fixedTimestamp value below for every request.',
            },
          },
          fixedTimestamp: {
            type: 'string',
            description: 'Timestamp value when timestampMode is fixed',
            'x-ui-group': {
              key: 'timestamp',
              label: 'Timestamp',
              collapsed: true,
            },
            'x-ui-disabled-when': {
              field: 'timestampMode',
              notEquals: 'fixed',
            },
            'x-ui-disabled-reason': 'Only used when timestamp mode is set to fixed.',
          },
        },
      },
    },
  }
}

function parseRawRequest(rawRequest: string) {
  const normalized = rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
  const splitIndex = normalized.indexOf('\n\n')
  const headerPart = splitIndex === -1 ? normalized : normalized.slice(0, splitIndex)
  const body = splitIndex === -1 ? '' : normalized.slice(splitIndex + 2)
  const lines = headerPart.split('\n')
  const requestLine = lines.shift() || 'GET / HTTP/1.1'
  const [method = 'GET', path = '/', protocol = 'HTTP/1.1'] = requestLine.split(/\s+/)
  return {
    method,
    path,
    protocol,
    headers: lines,
    body,
  }
}

function buildRawRequest(parts: { method: string; path: string; protocol: string; headers: string[]; body: string }) {
  return [`${parts.method} ${parts.path} ${parts.protocol}`, ...parts.headers, '', parts.body].join('\r\n')
}

function toRecord(searchParams: URLSearchParams): Record<string, string> {
  const record: Record<string, string> = {}
  searchParams.forEach((value, key) => {
    record[key] = value
  })
  return record
}

function applyRecord(target: URLSearchParams, values: Record<string, string>) {
  target.forEach((_, key) => target.delete(key))
  Object.entries(values).forEach(([key, value]) => target.append(key, value))
}

function buildCanonicalString(values: Record<string, string>, options: Required<ToolInput>['config']) {
  const excluded = new Set([
    options.signatureField,
    ...(options.excludeFields || []),
  ])
  const keys = Object.keys(values).filter((key) => !excluded.has(key))
  if (options.sortKeys) {
    keys.sort()
  }
  return keys.map((key) => `${key}=${values[key] ?? ''}`).join('&')
}

function resolveTimestamp(config: Required<ToolInput>['config']) {
  if (config.timestampMode === 'fixed') {
    return config.fixedTimestamp || ''
  }
  const now = Date.now()
  return config.timestampMode === 'milliseconds' ? String(now) : String(Math.floor(now / 1000))
}

async function hmacSha256(secret: string, message: string) {
  const encoder = new TextEncoder()
  const key = await crypto.subtle.importKey(
    'raw',
    encoder.encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  )
  const signature = await crypto.subtle.sign('HMAC', key, encoder.encode(message))
  return Array.from(new Uint8Array(signature))
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('')
}

function normalizeConfig(config?: ToolInput['config']) {
  return {
    signatureField: config?.signatureField || 'sign',
    timestampField: config?.timestampField || 'ts',
    secret: config?.secret || '',
    algorithm: config?.algorithm || 'hmac-sha256',
    location: config?.location || 'body',
    sortKeys: config?.sortKeys !== false,
    excludeFields: Array.isArray(config?.excludeFields) ? config.excludeFields : [],
    timestampMode: config?.timestampMode || 'seconds',
    fixedTimestamp: config?.fixedTimestamp || '',
  }
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
  try {
    const config = normalizeConfig(input?.config)
    if (!config.secret) {
      return {
        success: false,
        error: 'config.secret is required',
      }
    }

    const parsed = parseRawRequest(input.rawRequest)
    const url = new URL(parsed.path, 'https://intruder.local')
    const bodyParams = new URLSearchParams(parsed.body)
    const sourceParams = config.location === 'query' ? url.searchParams : bodyParams
    const values = toRecord(sourceParams)

    values[config.timestampField] = resolveTimestamp(config)
    const canonical = buildCanonicalString(values, config)
    const signature = await hmacSha256(config.secret, canonical)
    values[config.signatureField] = signature

    applyRecord(sourceParams, values)

    const nextPath = `${url.pathname}${url.search}`
    const nextBody = config.location === 'body' ? bodyParams.toString() : parsed.body

    return {
      success: true,
      data: {
        rawRequest: buildRawRequest({
          ...parsed,
          path: nextPath,
          body: nextBody,
        }),
        canonical,
        signature,
      },
    }
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

globalThis.get_input_schema = get_input_schema
globalThis.analyze = analyze
