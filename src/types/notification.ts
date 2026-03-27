export type NotificationCategory = 'message' | 'notification'

export type NotificationSource =
  | 'ai_assistant'
  | 'workflow'
  | 'monitor'
  | 'bug_bounty_workflow'

export type NotificationLevel = 'info' | 'success' | 'warning' | 'error'
export type NotificationDesktopMode = 'background' | 'always'

export interface NotificationPreferences {
  desktopEnabled: boolean
  desktopMode: NotificationDesktopMode
  soundEnabled: boolean
  sources: Record<NotificationSource, boolean>
}

export interface NotificationRouteTarget {
  path: string
  query?: Record<string, string>
}

export interface AppNotificationItem {
  id: string
  eventKey?: string
  category: NotificationCategory
  source: NotificationSource
  level: NotificationLevel
  title: string
  message: string
  icon: string
  read: boolean
  createdAt: string
  route?: NotificationRouteTarget | null
  metadata?: Record<string, unknown>
}
