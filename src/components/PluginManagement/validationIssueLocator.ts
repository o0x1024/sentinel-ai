export interface ValidationIssueTarget {
  from: number
  to: number
}

const ISSUE_CODE_PATTERNS: Record<string, RegExp[]> = {
  markdown_code_fence: [/```/],
  scan_transaction_export: [
    /export\s+async\s+function\s+scan_transaction\b/,
    /export\s+function\s+scan_transaction\b/,
    /function\s+scan_transaction\b/,
  ],
  scan_transaction_global_binding: [/globalThis\.scan_transaction\s*=/],
  get_input_schema_export: [
    /export\s+function\s+get_input_schema\b/,
    /function\s+get_input_schema\b/,
  ],
  get_input_schema_global_binding: [/globalThis\.get_input_schema\s*=/],
  analyze_export: [
    /export\s+async\s+function\s+analyze\b/,
    /export\s+function\s+analyze\b/,
    /function\s+analyze\b/,
  ],
  analyze_global_binding: [/globalThis\.analyze\s*=/],
  payloads_return_shape: [/payloads\s*:/],
  raw_request_return_shape: [/rawRequest\s*:/, /\brawRequest\b/],
  request_processor_raw_request_usage: [/\brawRequest\b/],
  payload_processor_return_shape: [/payload\s*:/, /skip\s*:/],
  runtime_schema_execution_failed: [
    /export\s+function\s+get_input_schema\b/,
    /function\s+get_input_schema\b/,
    /globalThis\.get_input_schema\s*=/,
  ],
  runtime_schema_shape: [/get_input_schema\b/, /type\s*:/],
}

const ISSUE_PATTERNS: Array<{ test: RegExp; patterns: RegExp[] }> = [
  {
    test: /get_input_schema|schema/i,
    patterns: [
      /export\s+function\s+get_input_schema\b/,
      /function\s+get_input_schema\b/,
      /globalThis\.get_input_schema\s*=/,
    ],
  },
  {
    test: /scan_transaction/i,
    patterns: [
      /export\s+async\s+function\s+scan_transaction\b/,
      /export\s+function\s+scan_transaction\b/,
      /function\s+scan_transaction\b/,
      /globalThis\.scan_transaction\s*=/,
    ],
  },
  {
    test: /\banalyze\b|globalThis\.analyze/i,
    patterns: [
      /export\s+async\s+function\s+analyze\b/,
      /export\s+function\s+analyze\b/,
      /function\s+analyze\b/,
      /globalThis\.analyze\s*=/,
    ],
  },
  {
    test: /data\.payloads|payload_generator|payloads/i,
    patterns: [/payloads\s*:/],
  },
  {
    test: /data\.rawRequest|request_processor|rawRequest/i,
    patterns: [/rawRequest\s*:/, /\brawRequest\b/],
  },
  {
    test: /data\.payload|data\.skip|payload_processor|\bskip\b/i,
    patterns: [/payload\s*:/, /skip\s*:/],
  },
  {
    test: /Markdown|代码块标记|```/,
    patterns: [/```/],
  },
]

export function locateValidationIssue(
  code: string,
  sectionKey: string,
  issueCode: string,
  message: string,
): ValidationIssueTarget | null {
  const directPatterns = ISSUE_CODE_PATTERNS[issueCode] || []

  for (const pattern of directPatterns) {
    const match = pattern.exec(code)
    if (match?.index != null) {
      return {
        from: match.index,
        to: match.index + match[0].length,
      }
    }
  }

  const normalizedMessage = `${sectionKey} ${message}`

  for (const rule of ISSUE_PATTERNS) {
    if (!rule.test.test(normalizedMessage)) continue

    for (const pattern of rule.patterns) {
      const match = pattern.exec(code)
      if (match?.index != null) {
        return {
          from: match.index,
          to: match.index + match[0].length,
        }
      }
    }
  }

  const firstNonWhitespace = code.search(/\S/)
  if (firstNonWhitespace >= 0) {
    return {
      from: firstNonWhitespace,
      to: Math.min(code.length, firstNonWhitespace + 1),
    }
  }

  return null
}
