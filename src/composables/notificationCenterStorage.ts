import type { AppNotificationItem } from '@/types/notification'

export const NOTIFICATION_STORAGE_KEY = 'sentinel-notification-center-items'
export const MAX_PERSISTED_NOTIFICATION_ITEMS = 80
export const MAX_PERSISTED_NOTIFICATION_BYTES = 128 * 1024

const MAX_TITLE_LENGTH = 160
const MAX_MESSAGE_LENGTH = 360
const MAX_ICON_LENGTH = 80
const MAX_ROUTE_PATH_LENGTH = 160
const MAX_ROUTE_QUERY_VALUE_LENGTH = 120
const MAX_METADATA_KEYS = 12
const MAX_METADATA_STRING_LENGTH = 160

type NotificationStorage = Pick<Storage, 'setItem'>

function truncateValue(value: unknown, maxLength: number) {
  const text = String(value ?? '').replace(/\s+/g, ' ').trim()
  if (text.length <= maxLength) return text
  return `${text.slice(0, maxLength - 1)}...`
}

function byteLength(value: string) {
  return new TextEncoder().encode(value).length
}

function compactMetadata(metadata: AppNotificationItem['metadata']) {
  if (!metadata || typeof metadata !== 'object') return {}

  return Object.fromEntries(
    Object.entries(metadata)
      .slice(0, MAX_METADATA_KEYS)
      .map(([key, value]) => {
        if (typeof value === 'number' || typeof value === 'boolean' || value == null) {
          return [key, value]
        }

        return [key, truncateValue(value, MAX_METADATA_STRING_LENGTH)]
      }),
  )
}

function compactRoute(route: AppNotificationItem['route']) {
  if (!route?.path) return null

  return {
    path: truncateValue(route.path, MAX_ROUTE_PATH_LENGTH),
    query: route.query
      ? Object.fromEntries(
        Object.entries(route.query).map(([key, value]) => [
          key,
          truncateValue(value, MAX_ROUTE_QUERY_VALUE_LENGTH),
        ]),
      )
      : undefined,
  }
}

function compactNotificationItem(item: AppNotificationItem): AppNotificationItem {
  return {
    id: item.id,
    eventKey: item.eventKey ? truncateValue(item.eventKey, 240) : undefined,
    category: item.category,
    source: item.source,
    level: item.level,
    title: truncateValue(item.title, MAX_TITLE_LENGTH),
    message: truncateValue(item.message, MAX_MESSAGE_LENGTH),
    icon: truncateValue(item.icon, MAX_ICON_LENGTH),
    read: item.read,
    createdAt: item.createdAt,
    route: compactRoute(item.route),
    metadata: compactMetadata(item.metadata),
  }
}

export function buildNotificationStoragePayload(
  items: AppNotificationItem[],
  maxBytes = MAX_PERSISTED_NOTIFICATION_BYTES,
) {
  const compactItems = items
    .slice(0, MAX_PERSISTED_NOTIFICATION_ITEMS)
    .map(compactNotificationItem)

  while (compactItems.length > 0) {
    const payload = JSON.stringify(compactItems)
    if (byteLength(payload) <= maxBytes) {
      return payload
    }
    compactItems.pop()
  }

  return '[]'
}

export function persistNotificationItems(storage: NotificationStorage, items: AppNotificationItem[]) {
  const payload = buildNotificationStoragePayload(items)

  try {
    storage.setItem(NOTIFICATION_STORAGE_KEY, payload)
    return true
  } catch (error) {
    console.warn('[notificationCenterStorage] Failed to persist notification items:', error)
    return false
  }
}
