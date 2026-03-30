import type { RuleStarterTemplate } from '@/components/Dictionary/ruleStarterTemplates.types'

export const webFingerprintStarterTemplates: RuleStarterTemplate[] = [
  {
    key: 'web_grafana_title',
    label: 'Grafana 标题',
    payload: {
      word: 'grafana_title',
      category: 'observability',
      name: 'Grafana Login Title',
      service: 'http',
      product: 'Grafana',
      vendor: 'Grafana Labs',
      assetCategory: 'dashboard',
      assetFamily: 'observability',
      protocol: 'http',
      probeName: 'http_head',
      portsText: '80,3000',
      sslPortsText: '443',
      softmatch: false,
      priority: 100,
      confidence: 0.9,
      operator: 'or',
      matchers: [
        { part: 'title', type: 'contains', valueText: 'Grafana' },
      ],
    },
  },
  {
    key: 'web_jenkins_header',
    label: 'Jenkins Header',
    payload: {
      word: 'jenkins_x_jenkins',
      category: 'ci',
      name: 'Jenkins Header',
      service: 'http',
      product: 'Jenkins',
      vendor: 'Jenkins',
      assetCategory: 'ci_cd',
      assetFamily: 'devops',
      protocol: 'http',
      probeName: 'http_head',
      portsText: '8080',
      sslPortsText: '8443',
      softmatch: false,
      priority: 100,
      confidence: 0.95,
      operator: 'or',
      matchers: [
        { part: 'header', type: 'exists', key: 'x-jenkins', valueText: '' },
      ],
    },
  },
]
