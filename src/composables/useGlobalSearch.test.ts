import { describe, expect, it } from 'vitest'
import { createFindingSearchEntries, createNotificationSearchEntries } from '@/composables/useGlobalSearch'
import type { Finding } from '@/components/SecurityCenter/vulnerabilityFindingTypes'
import type { AppNotificationItem } from '@/types/notification'

const notification: AppNotificationItem = {
  id: 'notification-1',
  category: 'message',
  source: 'ai_assistant',
  level: 'info',
  title: 'AI 分析已完成',
  message: 'example.com 的助手分析已经返回结果',
  icon: 'fas fa-robot',
  read: false,
  createdAt: '2026-04-08T00:00:00.000Z',
  route: {
    path: '/ai-assistant',
    query: { conversationId: 'conv-1' },
  },
}

const finding: Finding = {
  id: 'finding-1',
  plugin_id: 'plugin.example',
  vuln_type: 'xss',
  severity: 'high',
  confidence: 'high',
  title: 'Reflected XSS in search parameter',
  description: 'The q parameter reflects arbitrary HTML into the response.',
  status: 'open',
  url: 'https://example.com/search?q=test',
  hit_count: 1,
  first_seen_at: '2026-04-08T00:00:00.000Z',
  last_seen_at: '2026-04-08T00:00:00.000Z',
  created_at: '2026-04-08T00:00:00.000Z',
  updated_at: '2026-04-08T00:00:00.000Z',
  evidence: [],
}

describe('useGlobalSearch helpers', () => {
  it('maps notifications into searchable entries', () => {
    const [entry] = createNotificationSearchEntries([notification])

    expect(entry.id).toBe('notification:notification-1')
    expect(entry.category).toBe('notification')
    expect(entry.path).toBe('/ai-assistant')
    expect(entry.query).toEqual({ conversationId: 'conv-1' })
    expect(entry.keywords).toContain('AI 助手')
    expect(entry.keywords).toContain('未读')
  })

  it('maps findings into deep-linkable search entries', () => {
    const [entry] = createFindingSearchEntries([finding])

    expect(entry.id).toBe('finding:finding-1')
    expect(entry.category).toBe('finding')
    expect(entry.path).toBe('/security-center')
    expect(entry.query).toEqual({ tab: 'vulnerabilities', findingId: 'finding-1' })
    expect(entry.keywords).toContain('high')
    expect(entry.keywords).toContain('xss')
  })
})
