<template>
  <div class="container p-6 space-y-6">
      <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div class="space-y-2">
          <div class="badge badge-outline badge-primary">{{ t('notifications.center.pageBadge') }}</div>
          <h1 class="text-3xl font-bold">{{ t('notifications.center.pageTitle') }}</h1>
          <p class="text-sm text-base-content/70 max-w-3xl">
            {{ t('notifications.center.pageDescription') }}
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <button class="btn btn-outline" @click="openPreferencesDialog">
            <i class="fas fa-sliders mr-2"></i>
            {{ t('notifications.center.preferencesTitle') }}
          </button>
          <button class="btn btn-ghost" @click="markCurrentAsRead">
            <i class="fas fa-envelope-open mr-2"></i>
            {{ t('notifications.center.markCurrentRead') }}
          </button>
          <button class="btn btn-ghost text-error" @click="clearCurrentItems">
            <i class="fas fa-trash mr-2"></i>
            {{ t('notifications.center.clearCurrent') }}
          </button>
          <button class="btn btn-primary" @click="router.push('/notifications')">
            <i class="fas fa-sliders mr-2"></i>
            {{ t('notifications.center.manageRules') }}
          </button>
        </div>
      </div>

      <div class="grid gap-4 md:grid-cols-3">
        <button
          v-for="tab in tabs"
          :key="tab.value"
          class="card border text-left transition-colors"
          :class="activeCategory === tab.value ? 'border-primary bg-primary/5' : 'border-base-300 bg-base-100 hover:bg-base-200/60'"
          @click="setCategory(tab.value)"
        >
          <div class="card-body gap-2">
            <div class="flex items-center justify-between gap-3">
              <span class="text-sm text-base-content/70">{{ tab.label }}</span>
              <span class="badge badge-outline">{{ tab.count }}</span>
            </div>
            <div class="text-2xl font-semibold">{{ tab.unread }}</div>
            <div class="flex items-center gap-2 text-xs text-base-content/60">
              <span>{{ t('notifications.center.unreadSummary') }}</span>
              <span v-if="tab.value === 'all'" class="badge badge-ghost badge-sm" :class="desktopPermissionBadgeClass">
                {{ desktopPermissionLabel }}
              </span>
            </div>
          </div>
        </button>
      </div>

      <div class="card border border-base-300 bg-base-100 shadow-sm">
        <div class="card-body gap-4">
          <div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div class="tabs tabs-boxed w-fit">
              <a
                v-for="tab in tabs"
                :key="`switch-${tab.value}`"
                class="tab"
                :class="{ 'tab-active': activeCategory === tab.value }"
                @click="setCategory(tab.value)"
              >
                {{ tab.label }}
              </a>
            </div>

            <div class="flex flex-col gap-3 sm:flex-row sm:items-center">
              <label class="form-control w-full sm:w-52">
                <div class="label py-1">
                  <span class="label-text text-xs text-base-content/70">{{ t('notifications.center.sourceFilter') }}</span>
                </div>
                <select class="select select-bordered select-sm" :value="activeSource" @change="onSourceChange">
                  <option value="all">{{ t('notifications.center.allSources') }}</option>
                  <option v-for="option in sourceOptions" :key="option.value" :value="option.value">
                    {{ option.label }}
                  </option>
                </select>
              </label>

              <label class="label cursor-pointer gap-3 justify-start sm:justify-end">
                <span class="label-text">{{ t('notifications.center.unreadOnly') }}</span>
                <input
                  type="checkbox"
                  class="toggle toggle-primary"
                  :checked="unreadOnly"
                  @change="onUnreadToggle"
                />
              </label>
            </div>
          </div>

          <div v-if="filteredItems.length === 0" class="rounded-2xl border border-dashed border-base-300 px-6 py-12 text-center">
            <div class="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-full bg-base-200">
              <i class="fas fa-bell-slash text-lg text-base-content/60"></i>
            </div>
            <h2 class="text-lg font-semibold">{{ t('notifications.center.emptyHistoryTitle') }}</h2>
            <p class="mt-2 text-sm text-base-content/70">
              {{ t('notifications.center.emptyHistoryDescription') }}
            </p>
          </div>

          <div v-else class="space-y-3">
            <div
              v-for="item in filteredItems"
              :key="item.id"
              role="button"
              tabindex="0"
              class="rounded-2xl border p-4 transition-colors"
              :class="item.read ? 'border-base-300 bg-base-100' : 'border-primary/30 bg-primary/5 hover:bg-primary/10'"
              @click="openItem(item)"
            >
              <div class="flex items-start gap-3">
                <div class="mt-1 flex h-10 w-10 items-center justify-center rounded-xl bg-base-200 text-base-content/80">
                  <i :class="item.icon"></i>
                </div>

                <div class="min-w-0 flex-1 space-y-3">
                  <div class="flex flex-col gap-2 lg:flex-row lg:items-start lg:justify-between">
                    <div class="min-w-0">
                      <div class="flex flex-wrap items-center gap-2">
                        <h3 class="truncate text-base font-semibold">{{ item.title }}</h3>
                        <span v-if="!item.read" class="badge badge-primary badge-sm">{{ t('notifications.center.unread') }}</span>
                        <span class="badge badge-outline badge-sm">{{ sourceLabel(item.source) }}</span>
                        <span class="badge badge-sm" :class="levelBadgeClass(item.level)">{{ levelLabel(item.level) }}</span>
                      </div>
                      <p class="mt-2 break-words text-sm leading-6 text-base-content/75">
                        {{ item.message }}
                      </p>
                    </div>

                    <div class="shrink-0 text-xs text-base-content/55">
                      {{ formatAbsoluteTime(item.createdAt) }}
                    </div>
                  </div>

                  <div class="flex flex-wrap items-center justify-between gap-3">
                    <div class="text-xs text-base-content/55">
                      {{ formatRelativeTime(item.createdAt) }}
                    </div>

                    <div class="flex items-center gap-2">
                      <button
                        v-if="!item.read"
                        class="btn btn-ghost btn-xs"
                        @click.stop="markAsRead(item.id)"
                      >
                        {{ t('notifications.center.markRead') }}
                      </button>
                      <button
                        class="btn btn-ghost btn-xs"
                        @click.stop="openItem(item)"
                      >
                        {{ t('notifications.center.openTarget') }}
                      </button>
                      <button
                        class="btn btn-ghost btn-xs text-error"
                        @click.stop="removeNotification(item.id)"
                      >
                        {{ t('notifications.center.removeItem') }}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <AppDialog ref="preferencesDialog" class="modal">
        <div class="modal-box max-w-4xl">
          <div class="flex items-start justify-between gap-4">
            <div>
              <h2 class="text-xl font-semibold">{{ t('notifications.center.preferencesTitle') }}</h2>
              <p class="mt-2 text-sm text-base-content/65">
                {{ t('notifications.center.preferencesDescription') }}
              </p>
            </div>
            <button class="btn btn-ghost btn-sm btn-circle" @click="closePreferencesDialog">
              <i class="fas fa-xmark"></i>
            </button>
          </div>

          <div class="mt-6 space-y-5">
            <div class="flex flex-wrap items-center gap-2">
              <span class="badge badge-outline">{{ enabledSourceCount }}/4</span>
              <span class="badge" :class="desktopPermissionBadgeClass">{{ desktopPermissionLabel }}</span>
            </div>

            <div class="grid gap-4 md:grid-cols-2">
              <label class="flex items-center justify-between gap-4 rounded-2xl border border-base-300 px-4 py-3">
                <div>
                  <div class="font-medium">{{ t('notifications.center.desktopEnabled') }}</div>
                  <div class="text-xs text-base-content/60">{{ t('notifications.center.desktopEnabledHint') }}</div>
                </div>
                <input
                  type="checkbox"
                  class="toggle toggle-primary"
                  :checked="preferences.desktopEnabled"
                  @change="onDesktopEnabledChange"
                />
              </label>

              <label class="flex items-center justify-between gap-4 rounded-2xl border border-base-300 px-4 py-3">
                <div>
                  <div class="font-medium">{{ t('notifications.center.soundEnabled') }}</div>
                  <div class="text-xs text-base-content/60">{{ t('notifications.center.soundEnabledHint') }}</div>
                </div>
                <input
                  type="checkbox"
                  class="toggle toggle-primary"
                  :checked="preferences.soundEnabled"
                  @change="onSoundEnabledChange"
                />
              </label>
            </div>

            <label class="form-control w-full max-w-xs">
              <div class="label pt-0">
                <span class="label-text font-medium">{{ t('notifications.center.desktopModeTitle') }}</span>
              </div>
              <select
                class="select select-bordered"
                :value="preferences.desktopMode"
                :disabled="!preferences.desktopEnabled"
                @change="onDesktopModeChange"
              >
                <option value="background">{{ t('notifications.center.desktopModes.background') }}</option>
                <option value="always">{{ t('notifications.center.desktopModes.always') }}</option>
              </select>
            </label>

            <div class="flex flex-wrap gap-2">
              <button class="btn btn-outline btn-sm" @click="handleRequestDesktopPermission">
                <i class="fas fa-unlock-keyhole mr-2"></i>
                {{ t('notifications.center.requestPermission') }}
              </button>
              <button class="btn btn-outline btn-sm" @click="handleTestDesktopNotification">
                <i class="fas fa-flask mr-2"></i>
                {{ t('notifications.center.testDesktopNotification') }}
              </button>
            </div>

            <div class="space-y-3">
              <div class="font-medium">{{ t('notifications.center.sourcePreferencesTitle') }}</div>
              <div class="grid gap-3 md:grid-cols-2">
                <label
                  v-for="source in sourceOptions"
                  :key="`pref-${source.value}`"
                  class="flex items-center justify-between gap-4 rounded-2xl border border-base-300 px-4 py-3"
                >
                  <div>
                    <div class="font-medium">{{ source.label }}</div>
                    <div class="text-xs text-base-content/60">
                      {{ t(`notifications.center.sourceDescriptions.${source.value}`) }}
                    </div>
                  </div>
                  <input
                    type="checkbox"
                    class="toggle toggle-primary"
                    :checked="Boolean(preferences.sources[source.value])"
                    @change="onSourcePreferenceChange(source.value, $event)"
                  />
                </label>
              </div>
            </div>
          </div>

          <div class="modal-action">
            <button class="btn" @click="closePreferencesDialog">
              {{ t('notifications.cancel') }}
            </button>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button>{{ t('notifications.cancel') }}</button>
        </form>
      </AppDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useNotificationCenter } from '@/composables/useNotificationCenter'
import { useNotificationPreferences } from '@/composables/useNotificationPreferences'
import { useToast } from '@/composables/useToast'
import type {
  AppNotificationItem,
  NotificationCategory,
  NotificationDesktopMode,
  NotificationLevel,
  NotificationSource,
} from '@/types/notification'

type CategoryFilter = NotificationCategory | 'all'
type SourceFilter = NotificationSource | 'all'

const { t, locale } = useI18n()
const toast = useToast()
const router = useRouter()
const route = useRoute()
const preferencesDialog = ref<HTMLDialogElement | null>(null)
const {
  preferences,
  enabledSourceCount,
  setDesktopEnabled,
  setDesktopMode,
  setSoundEnabled,
  setSourceEnabled,
} = useNotificationPreferences()
const {
  initializeNotificationCenter,
  desktopPermission,
  requestDesktopPermission,
  testDesktopNotification,
  items,
  messageItems,
  notificationItems,
  unreadMessageCount,
  unreadNotificationCount,
  markAsRead,
  removeNotification,
  openNotification,
} = useNotificationCenter()

const tabs = computed(() => [
  {
    value: 'all' as const,
    label: t('notifications.center.allTab'),
    count: items.value.length,
    unread: unreadMessageCount.value + unreadNotificationCount.value,
  },
  {
    value: 'message' as const,
    label: t('notifications.center.messagesTab'),
    count: messageItems.value.length,
    unread: unreadMessageCount.value,
  },
  {
    value: 'notification' as const,
    label: t('notifications.center.notificationsTab'),
    count: notificationItems.value.length,
    unread: unreadNotificationCount.value,
  },
])

const sourceOptions = computed(() => [
  { value: 'ai_assistant' as const, label: t('notifications.center.sources.ai_assistant') },
  { value: 'workflow' as const, label: t('notifications.center.sources.workflow') },
  { value: 'monitor' as const, label: t('notifications.center.sources.monitor') },
  { value: 'bug_bounty_workflow' as const, label: t('notifications.center.sources.bug_bounty_workflow') },
])

const desktopPermissionLabel = computed(() => {
  return t(`notifications.center.permissionStates.${desktopPermission.value}`)
})

const desktopPermissionBadgeClass = computed(() => {
  switch (desktopPermission.value) {
    case 'granted':
      return 'badge-success'
    case 'denied':
      return 'badge-error'
    case 'unsupported':
      return 'badge-warning'
    default:
      return 'badge-ghost'
  }
})

const activeCategory = computed<CategoryFilter>(() => {
  const raw = String(route.query.category || 'all')
  return raw === 'message' || raw === 'notification' ? raw : 'all'
})

const activeSource = computed<SourceFilter>(() => {
  const raw = String(route.query.source || 'all')
  return sourceOptions.value.some((option) => option.value === raw) ? (raw as NotificationSource) : 'all'
})

const unreadOnly = computed(() => String(route.query.unread || '') === '1')

const filteredItems = computed(() => {
  return items.value.filter((item) => {
    if (activeCategory.value !== 'all' && item.category !== activeCategory.value) return false
    if (activeSource.value !== 'all' && item.source !== activeSource.value) return false
    if (unreadOnly.value && item.read) return false
    return true
  })
})

const buildNextQuery = (overrides: Partial<Record<'category' | 'source' | 'unread', string | undefined>>) => {
  const nextQuery: Record<string, string> = {}
  const category = overrides.category ?? String(route.query.category || 'all')
  const source = overrides.source ?? String(route.query.source || 'all')
  const unread = overrides.unread ?? String(route.query.unread || '')

  if (category && category !== 'all') nextQuery.category = category
  if (source && source !== 'all') nextQuery.source = source
  if (unread === '1') nextQuery.unread = '1'
  return nextQuery
}

const setCategory = async (category: CategoryFilter) => {
  await router.replace({
    path: '/notification-center',
    query: buildNextQuery({ category }),
  })
}

const onSourceChange = async (event: Event) => {
  const value = (event.target as HTMLSelectElement).value as SourceFilter
  await router.replace({
    path: '/notification-center',
    query: buildNextQuery({ source: value }),
  })
}

const onUnreadToggle = async (event: Event) => {
  const checked = (event.target as HTMLInputElement).checked
  await router.replace({
    path: '/notification-center',
    query: buildNextQuery({ unread: checked ? '1' : undefined }),
  })
}

const onDesktopEnabledChange = (event: Event) => {
  setDesktopEnabled((event.target as HTMLInputElement).checked)
}

const onSoundEnabledChange = (event: Event) => {
  setSoundEnabled((event.target as HTMLInputElement).checked)
}

const onDesktopModeChange = (event: Event) => {
  setDesktopMode((event.target as HTMLSelectElement).value as NotificationDesktopMode)
}

const onSourcePreferenceChange = (source: NotificationSource, event: Event) => {
  setSourceEnabled(source, (event.target as HTMLInputElement).checked)
}

const openPreferencesDialog = () => {
  preferencesDialog.value?.showModal()
}

const closePreferencesDialog = () => {
  preferencesDialog.value?.close()
}

const handleRequestDesktopPermission = async () => {
  const granted = await requestDesktopPermission()
  if (granted) {
    toast.success(t('notifications.center.permissionGranted'))
    return
  }

  toast.warning(t(`notifications.center.permissionRequestResults.${desktopPermission.value}`))
}

const handleTestDesktopNotification = async () => {
  const ok = await testDesktopNotification()
  if (ok) {
    toast.success(t('notifications.center.testDesktopTriggered'))
    return
  }

  toast.warning(t(`notifications.center.permissionRequestResults.${desktopPermission.value}`))
}

const openItem = async (item: AppNotificationItem) => {
  await openNotification(router, item)
}

const markCurrentAsRead = () => {
  filteredItems.value
    .filter((item) => !item.read)
    .forEach((item) => markAsRead(item.id))
}

const clearCurrentItems = () => {
  filteredItems.value.forEach((item) => removeNotification(item.id))
}

const formatRelativeTime = (value: string) => {
  const timestamp = new Date(value).getTime()
  const deltaMs = Date.now() - timestamp

  if (Number.isNaN(timestamp) || deltaMs < 60_000) {
    return t('notifications.center.justNow')
  }

  const minutes = Math.floor(deltaMs / 60_000)
  if (minutes < 60) {
    return t('notifications.center.minutesAgo', { count: minutes })
  }

  const hours = Math.floor(minutes / 60)
  if (hours < 24) {
    return t('notifications.center.hoursAgo', { count: hours })
  }

  const days = Math.floor(hours / 24)
  return t('notifications.center.daysAgo', { count: days })
}

const formatAbsoluteTime = (value: string) => {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat(locale.value === 'zh' ? 'zh-CN' : 'en-US', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date)
}

const sourceLabel = (source: NotificationSource) => {
  return t(`notifications.center.sources.${source}`)
}

const levelLabel = (level: NotificationLevel) => {
  return t(`notifications.center.levels.${level}`)
}

const levelBadgeClass = (level: NotificationLevel) => {
  switch (level) {
    case 'success':
      return 'badge-success'
    case 'warning':
      return 'badge-warning'
    case 'error':
      return 'badge-error'
    default:
      return 'badge-info'
  }
}

onMounted(async () => {
  await initializeNotificationCenter(router)
})
</script>
