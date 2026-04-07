/**
 * Example Intruder payload processor plugin.
 * @plugin intruder_sql_comment_tamper
 * @name Intruder SQL Comment Tamper
 * @main_category intruder
 * @category payload_processor
 *
 * Import this file into Plugin Management, then set:
 * - main category: intruder
 * - category: payload_processor
 */

interface ToolInput {
  payload: string
  originalPayload?: string
  baseValue?: string
  positionIndex?: number
  config?: {
    uppercaseKeywords?: boolean
    replaceSpacesWithComments?: boolean
    commentToken?: string
    skipIfContains?: string
  }
}

interface ToolOutput {
  success: boolean
  data?: {
    payload?: string
    skip?: boolean
    changed?: boolean
  }
  error?: string
}

const SQL_KEYWORDS = [
  'select',
  'union',
  'from',
  'where',
  'and',
  'or',
  'sleep',
  'benchmark',
  'waitfor',
  'having',
  'order',
  'group',
  'by',
  'into',
  'update',
  'delete',
  'insert',
]

export function get_input_schema() {
  return {
    type: 'object',
    properties: {
      config: {
        type: 'object',
        properties: {
          uppercaseKeywords: {
            type: 'boolean',
            default: true,
            description: 'Uppercase common SQL keywords before sending',
            'x-ui-group': {
              key: 'transforms',
              label: 'Transforms',
              description: 'Choose how the payload should be rewritten before sending.',
            },
          },
          replaceSpacesWithComments: {
            type: 'boolean',
            default: true,
            description: 'Replace spaces with inline SQL comments like /**/',
            'x-ui-group': 'transforms',
          },
          commentToken: {
            type: 'string',
            default: '/**/',
            description: 'Token used when replacing spaces',
            'x-ui-group': 'transforms',
          },
          skipIfContains: {
            type: 'string',
            description: 'Skip payloads that already contain this marker',
            'x-ui-group': {
              key: 'filters',
              label: 'Filters',
              description: 'Skip payloads that already match a marker.',
              collapsed: true,
            },
          },
        },
      },
    },
  }
}

function normalizeConfig(config?: ToolInput['config']) {
  return {
    uppercaseKeywords: config?.uppercaseKeywords !== false,
    replaceSpacesWithComments: config?.replaceSpacesWithComments !== false,
    commentToken: config?.commentToken || '/**/',
    skipIfContains: config?.skipIfContains || '',
  }
}

function uppercaseSqlKeywords(value: string) {
  return SQL_KEYWORDS.reduce((current, keyword) => {
    const pattern = new RegExp(`\\b${keyword}\\b`, 'gi')
    return current.replace(pattern, keyword.toUpperCase())
  }, value)
}

function replaceSpacesWithComments(value: string, commentToken: string) {
  return value.replace(/\s+/g, commentToken)
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
  try {
    const payload = typeof input?.payload === 'string' ? input.payload : ''
    const config = normalizeConfig(input?.config)

    if (config.skipIfContains && payload.includes(config.skipIfContains)) {
      return {
        success: true,
        data: {
          skip: true,
        },
      }
    }

    let nextPayload = payload

    if (config.uppercaseKeywords) {
      nextPayload = uppercaseSqlKeywords(nextPayload)
    }

    if (config.replaceSpacesWithComments) {
      nextPayload = replaceSpacesWithComments(nextPayload, config.commentToken)
    }

    return {
      success: true,
      data: {
        payload: nextPayload,
        changed: nextPayload !== payload,
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
