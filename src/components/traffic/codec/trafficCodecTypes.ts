export interface TrafficCodecRule {
  id: string
  name: string
  enabled: boolean
  order: number
  match: CodecMatchRule
  scope: CodecScope
  pipeline: CodecPipeline
  reversible: boolean
  createdAt: string
  updatedAt: string
}

export interface CodecMatchRule {
  hosts: string[]
  paths: string[]
  methods: string[]
  contentTypes: string[]
}

export interface CodecScope {
  target: CodecScopeTarget
  fields: string[]
  headerName?: string
  pattern?: string
}

export type CodecScopeTarget =
  | 'json-field'
  | 'query-param'
  | 'form-field'
  | 'full-body'
  | 'header-value'
  | 'regex-match'

export interface CodecPipeline {
  steps: CodecStep[]
}

export interface CodecStep {
  id: string
  type: 'builtin' | 'plugin'
  codec: string
  pluginId?: string
  config: Record<string, string>
  enabled: boolean
}

export interface CodecResult {
  success: boolean
  content: string
  error?: string
  appliedRuleIds: string[]
}

export interface CodecRequestMeta {
  host: string
  path: string
  method: string
  contentType: string
}

export interface CodecExportData {
  version: string
  format: string
  exportedAt: string
  rules: CodecRuleExport[]
  keyPlaceholders: Record<string, KeyPlaceholderInfo>
}

export interface CodecRuleExport {
  name: string
  matchRule: CodecMatchRule
  scope: CodecScope
  pipeline: CodecPipeline
  reversible: boolean
}

export interface KeyPlaceholderInfo {
  description: string
  format: string
  length?: number
}

export const BUILTIN_CODECS = [
  { id: 'base64', label: 'Base64', category: 'encoding' },
  { id: 'url', label: 'URL Encode', category: 'encoding' },
  { id: 'hex', label: 'Hex', category: 'encoding' },
  { id: 'aes-cbc', label: 'AES-CBC', category: 'symmetric' },
  { id: 'aes-ecb', label: 'AES-ECB', category: 'symmetric' },
  { id: 'aes-gcm', label: 'AES-GCM', category: 'symmetric' },
  { id: 'sm4', label: 'SM4', category: 'symmetric' },
  { id: 'des', label: 'DES', category: 'symmetric' },
  { id: '3des', label: '3DES', category: 'symmetric' },
  { id: 'rsa-pkcs1v15', label: 'RSA PKCS#1 v1.5', category: 'asymmetric' },
  { id: 'rsa-oaep', label: 'RSA OAEP', category: 'asymmetric' },
  { id: 'gzip', label: 'Gzip', category: 'compression' },
  { id: 'deflate', label: 'Deflate', category: 'compression' },
  { id: 'xor', label: 'XOR', category: 'bitwise' },
] as const

export type BuiltinCodecId = (typeof BUILTIN_CODECS)[number]['id']
