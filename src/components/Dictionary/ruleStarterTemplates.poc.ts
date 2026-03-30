import type { RuleStarterTemplate } from '@/components/Dictionary/ruleStarterTemplates.types'

export const riskVerificationStarterTemplates: RuleStarterTemplate[] = [
  {
    key: 'risk_http_exposure',
    label: 'HTTP 暴露验证',
    payload: {
      word: 'http_exposure_check',
      category: 'exposure',
      severity: 'medium',
      name: 'HTTP Exposure Check',
      findingType: 'information_exposure',
      requestMethod: 'GET',
      requestPath: '/',
      timeoutMs: 5000,
      safeMode: true,
      targetTypesText: 'web',
      fingerprintScopeText: '',
      productScopeText: '',
      vendorScopeText: '',
      portScopeText: '80,443,8080,8443',
      matchers: [
        { part: 'status', type: 'in', valueText: '200,401,403' },
      ],
    },
  },
]
