import { describe, expect, it, vi } from 'vitest'
import {
  buildNotificationStoragePayload,
  MAX_PERSISTED_NOTIFICATION_BYTES,
  MAX_PERSISTED_NOTIFICATION_ITEMS,
  NOTIFICATION_STORAGE_KEY,
  persistNotificationItems,
} from './notificationCenterStorage'
import type { AppNotificationItem } from '@/types/notification'

function createNotification(index: number, message = 'message'): AppNotificationItem {
  return {
    id: `notification-${index}`,
    eventKey: `event-${index}`,
    category: 'message',
    source: 'ai_assistant',
    level: 'info',
    title: `Notification ${index}`,
    message,
    icon: 'fas fa-message',
    read: false,
    createdAt: new Date(2026, 0, index + 1).toISOString(),
    route: {
      path: '/ai-assistant',
      query: {
        conversationId: `conversation-${index}`,
      },
    },
    metadata: {
      conversation_id: `conversation-${index}`,
      response: message,
    },
  }
}

describe('notificationCenterStorage', () => {
  it('builds a bounded payload from large notification messages', () => {
    const items = Array.from({ length: MAX_PERSISTED_NOTIFICATION_ITEMS + 20 }, (_, index) =>
      createNotification(index, 'x'.repeat(10_000)),
    )

    const payload = buildNotificationStoragePayload(items)
    const stored = JSON.parse(payload) as AppNotificationItem[]

    expect(new TextEncoder().encode(payload).length).toBeLessThanOrEqual(MAX_PERSISTED_NOTIFICATION_BYTES)
    expect(stored.length).toBeLessThanOrEqual(MAX_PERSISTED_NOTIFICATION_ITEMS)
    expect(stored[0].message.length).toBeLessThan(500)
    expect(String(stored[0].metadata?.response).length).toBeLessThan(220)
  })

  it('does not throw when localStorage quota is exceeded', () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const storage = {
      setItem: vi.fn(() => {
        throw new DOMException('The quota has been exceeded.', 'QuotaExceededError')
      }),
    }

    expect(() => persistNotificationItems(storage, [createNotification(1)])).not.toThrow()
    expect(persistNotificationItems(storage, [createNotification(1)])).toBe(false)
    expect(storage.setItem).toHaveBeenCalledWith(NOTIFICATION_STORAGE_KEY, expect.any(String))
    warnSpy.mockRestore()
  })
})
