import type { RuleStarterTemplate } from '@/components/Dictionary/ruleStarterTemplates.types'

export const serviceIdentificationStarterTemplates: RuleStarterTemplate[] = [
  {
    key: 'service_http_header',
    label: 'HTTP Server 头',
    payload: {
      word: 'http_server_nginx',
      category: 'web_server',
      name: 'Nginx Server Header',
      service: 'http',
      product: 'nginx',
      vendor: 'NGINX',
      assetCategory: 'web_server',
      assetFamily: 'http_service',
      protocol: 'http',
      probeName: 'http_head',
      portsText: '80,8080,8000',
      sslPortsText: '443,8443',
      softmatch: false,
      priority: 100,
      confidence: 0.9,
      operator: 'or',
      matchers: [
        { part: 'header', type: 'contains', key: 'server', valueText: 'nginx' },
      ],
    },
  },
  {
    key: 'service_ssh_banner',
    label: 'SSH Banner',
    payload: {
      word: 'ssh_openssh',
      category: 'remote_access',
      name: 'OpenSSH Banner',
      service: 'ssh',
      product: 'OpenSSH',
      vendor: 'OpenBSD',
      assetCategory: 'remote_access',
      assetFamily: 'ssh',
      protocol: 'tcp',
      probeName: 'tcp_banner',
      portsText: '22',
      softmatch: false,
      priority: 100,
      confidence: 0.95,
      operator: 'or',
      matchers: [
        { part: 'banner', type: 'contains', valueText: 'OpenSSH' },
      ],
    },
  },
  {
    key: 'service_redis_banner',
    label: 'Redis Banner',
    payload: {
      word: 'redis_server',
      category: 'cache',
      name: 'Redis Service Banner',
      service: 'redis',
      product: 'Redis',
      vendor: 'Redis',
      assetCategory: 'cache',
      assetFamily: 'redis',
      protocol: 'tcp',
      probeName: 'tcp_banner',
      portsText: '6379',
      softmatch: true,
      priority: 90,
      confidence: 0.8,
      operator: 'or',
      matchers: [
        { part: 'banner', type: 'contains', valueText: 'redis' },
      ],
    },
  },
]

export const genericFingerprintStarterTemplates: RuleStarterTemplate[] = [
  {
    key: 'generic_banner_contains',
    label: '通用 Banner',
    payload: {
      word: 'service_banner_contains',
      category: 'generic',
      name: 'Generic Banner Match',
      protocol: 'tcp',
      probeName: 'tcp_banner',
      softmatch: true,
      priority: 50,
      confidence: 0.7,
      operator: 'or',
      matchers: [
        { part: 'banner', type: 'contains', valueText: '' },
      ],
    },
  },
]
