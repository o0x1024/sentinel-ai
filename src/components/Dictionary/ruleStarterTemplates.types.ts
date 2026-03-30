export interface RuleStarterTemplate {
  key: string
  label: string
  payload: {
    word?: string
    category?: string
    severity?: string
    name?: string
    service?: string
    product?: string
    vendor?: string
    assetCategory?: string
    assetFamily?: string
    protocol?: string
    probeName?: string
    portsText?: string
    sslPortsText?: string
    softmatch?: boolean
    priority?: number
    confidence?: number
    operator?: string
    findingType?: string
    requestMethod?: string
    requestPath?: string
    timeoutMs?: number
    safeMode?: boolean
    targetTypesText?: string
    fingerprintScopeText?: string
    productScopeText?: string
    vendorScopeText?: string
    portScopeText?: string
    matchers?: Array<{
      part: string
      type: string
      key?: string
      valueText?: string
    }>
  }
}
